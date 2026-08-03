//! Terminal capture module.
//!
//! Provides PTY-based terminal capture and JSON output parsing, mirroring
//! Python's `terminal/reader.py` and `terminal/parser.py`.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::io::BorrowedFd;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use cla_common::environment::get_xdg_state_path;
use cla_common::files;
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use nix::pty::{forkpty, ForkptyResult, Winsize};
use nix::sys::wait::waitpid;
use nix::unistd;

/// Marker emitted by the captured shell prompt so command boundaries are visible.
const PROMPT_MARKER: &[u8] = b"\x1b]";

/// Path to the terminal capture log file.
pub fn terminal_capture_file() -> PathBuf {
    get_xdg_state_path().join("terminal.log")
}

/// Return true when a terminal capture session is currently running.
pub fn capture_active() -> bool {
    files::NamedFileLock::new("terminal")
        .map(|lock| lock.is_locked())
        .unwrap_or(false)
}

/// Spawn a shell under a PTY and write command/output blocks to `terminal.log`.
pub fn start_capture() -> anyhow::Result<()> {
    let state_dir = get_xdg_state_path();
    fs::create_dir_all(&state_dir)?;
    let capture_path = state_dir.join("terminal.log");
    fs::write(&capture_path, b"")?;

    let winsize = current_winsize();
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

    let result = unsafe { forkpty(Some(&winsize), None) };
    let fork_result = result.map_err(|e| anyhow::anyhow!("failed to open PTY: {}", e))?;

    match fork_result {
        ForkptyResult::Parent { child, master } => {
            let file = OpenOptions::new().append(true).open(&capture_path)?;
            let mut recorder = TerminalRecorder::new(file);
            let mut buffer = [0u8; 4096];
            let stdin_fd = unsafe { BorrowedFd::borrow_raw(libc::STDIN_FILENO) };
            let mut stdin_open = true;

            loop {
                let mut fds = [
                    PollFd::new(master.as_fd(), PollFlags::POLLIN | PollFlags::POLLHUP),
                    PollFd::new(stdin_fd, PollFlags::POLLIN),
                ];
                if !stdin_open {
                    fds[1].set_events(PollFlags::empty());
                }

                poll(&mut fds, PollTimeout::NONE)
                    .map_err(|e| anyhow::anyhow!("failed to poll terminal capture: {}", e))?;

                if fds[0].any().unwrap_or(false) {
                    match unistd::read(master.as_raw_fd(), &mut buffer) {
                        Ok(0) | Err(nix::errno::Errno::EIO) => break,
                        Ok(n) => recorder.handle(&buffer[..n]),
                        Err(e) => {
                            let _ = fs::remove_file(&capture_path);
                            return Err(anyhow::anyhow!("failed to read PTY: {}", e));
                        }
                    }
                }

                if stdin_open && fds[1].any().unwrap_or(false) {
                    match unistd::read(libc::STDIN_FILENO, &mut buffer) {
                        Ok(0) => {
                            let _ = unistd::write(&master, b"\x04");
                            stdin_open = false;
                        }
                        Ok(n) => {
                            if let Err(e) = unistd::write(&master, &buffer[..n]) {
                                let _ = fs::remove_file(&capture_path);
                                return Err(anyhow::anyhow!("failed to write to PTY: {}", e));
                            }
                        }
                        Err(e) => {
                            let _ = fs::remove_file(&capture_path);
                            return Err(anyhow::anyhow!("failed to read stdin: {}", e));
                        }
                    }
                }
            }

            recorder.write_json_block();
            let _ = waitpid(child, None);
            let _ = fs::remove_file(&capture_path);
            Ok(())
        }
        ForkptyResult::Child => {
            inject_prompt_marker();
            let err = Command::new(&shell).exec();
            Err(anyhow::anyhow!("failed to exec shell {}: {}", shell, err))
        }
    }
}

/// Get the current terminal size, falling back to 80x24.
fn current_winsize() -> Winsize {
    let mut size: Winsize = unsafe { std::mem::zeroed() };
    unsafe {
        libc::ioctl(0, libc::TIOCGWINSZ, &mut size);
    }
    if size.ws_col == 0 || size.ws_row == 0 {
        size.ws_col = 80;
        size.ws_row = 24;
    }
    size
}

/// Ensure every Bash prompt starts with the terminal capture marker.
fn inject_prompt_marker() {
    let marker = "printf '\\033]'";
    let existing = std::env::var("PROMPT_COMMAND").unwrap_or_default();
    let combined = if existing.is_empty() {
        marker.to_string()
    } else {
        format!("{}; {}", existing, marker)
    };
    std::env::set_var("PROMPT_COMMAND", combined);
}

/// Stateful recorder that separates a command from its output by prompt markers.
pub struct TerminalRecorder {
    file: File,
    in_command: bool,
    current_command: Vec<u8>,
    current_output: Vec<u8>,
}

impl TerminalRecorder {
    pub fn new(file: File) -> Self {
        Self {
            file,
            in_command: true,
            current_command: Vec::new(),
            current_output: Vec::new(),
        }
    }

    pub fn handle(&mut self, data: &[u8]) {
        if data.starts_with(PROMPT_MARKER) {
            if !self.in_command {
                self.write_json_block();
            }
            self.in_command = true;
        } else if self.in_command
            && (data.contains(&b'\n') || data.windows(2).any(|w| w == b"\r\n"))
        {
            self.in_command = false;
        }

        if self.in_command {
            self.current_command.extend_from_slice(data);
        } else {
            self.current_output.extend_from_slice(data);
        }
    }

    pub fn write_json_block(&mut self) {
        if self.current_command.is_empty() {
            self.current_command.clear();
            self.current_output.clear();
            return;
        }

        let block = TerminalBlock {
            command: String::from_utf8_lossy(&self.current_command)
                .trim()
                .to_string(),
            output: String::from_utf8_lossy(&self.current_output)
                .trim()
                .to_string(),
        };
        if let Ok(json) = serde_json::to_string(&block) {
            let _ = writeln!(self.file, "{}", json);
            let _ = self.file.flush();
        }

        self.current_command.clear();
        self.current_output.clear();
    }
}

/// A parsed command/output pair from the terminal capture log.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminalBlock {
    pub command: String,
    pub output: String,
}

/// Parse the terminal capture log into a list of command/output blocks.
///
/// Returns blocks in reverse order (most recent first).
pub fn parse_terminal_output() -> Vec<TerminalBlock> {
    let path = terminal_capture_file();
    if !path.exists() {
        return Vec::new();
    }

    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut blocks: Vec<TerminalBlock> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<TerminalBlock>(&line) {
            Ok(mut block) => {
                // Strip ANSI escape sequences
                block.command = strip_ansi(&block.command);
                block.output = strip_ansi(&block.output);

                // Skip "exit" entries
                if block.output.trim_end() == "exit" {
                    continue;
                }
                blocks.push(block);
            }
            Err(_) => continue,
        }
    }

    blocks.reverse();
    blocks
}

/// Find terminal output by index (negative index = reverse search).
pub fn find_output_by_index(index: isize, blocks: &[TerminalBlock]) -> String {
    let idx = if index < 0 {
        // Negative index: -1 = last, -2 = second to last, etc.
        let abs_idx = index.unsigned_abs();
        if abs_idx > blocks.len() || abs_idx == 0 {
            return String::new();
        }
        blocks.len() - abs_idx
    } else {
        let idx = index as usize;
        if idx >= blocks.len() {
            return String::new();
        }
        idx
    };

    blocks
        .get(idx)
        .map(|b| b.output.clone())
        .unwrap_or_default()
}

/// Strip ANSI escape sequences from text.
fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '\x1b' {
            result.push(c);
            continue;
        }

        let is_csi = chars.peek() == Some(&'[');
        let is_osc = chars.peek() == Some(&']');
        if is_csi {
            chars.next();
        }
        if is_osc {
            chars.next();
        }

        for next in chars.by_ref() {
            if is_csi {
                if ('@'..='~').contains(&next) {
                    break;
                }
            } else if is_osc {
                if next == '\x07' {
                    break;
                }
            } else if next.is_ascii_alphabetic() {
                break;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorder(file: File) -> TerminalRecorder {
        TerminalRecorder::new(file)
    }

    #[test]
    fn recorder_separates_command_and_output() {
        let (file, path) = tempfile_file();
        let mut recorder = recorder(file);

        recorder.handle(b"\x1b]prompt$ ");
        recorder.handle(b"ls -l\r\n");
        recorder.handle(b"total 0\r\n");
        recorder.handle(b"\x1b]next prompt$ ");
        recorder.write_json_block();

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(content.contains("ls -l"));
        assert!(content.contains("total 0"));
    }

    #[test]
    fn find_output_uses_reverse_index() {
        let blocks = vec![
            TerminalBlock {
                command: "first".to_string(),
                output: "one".to_string(),
            },
            TerminalBlock {
                command: "second".to_string(),
                output: "two".to_string(),
            },
        ];

        assert_eq!(find_output_by_index(-1, &blocks), "two");
        assert_eq!(find_output_by_index(-2, &blocks), "one");
        assert_eq!(find_output_by_index(0, &blocks), "one");
        assert_eq!(find_output_by_index(2, &blocks), "");
    }

    #[test]
    fn strip_ansi_removes_csi_and_osc_markers() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(strip_ansi("\x1b]0;title\x07body"), "body");
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    fn tempfile_file() -> (File, PathBuf) {
        let path = std::env::temp_dir().join(format!("cla-terminal-{}.log", uuid::Uuid::new_v4()));
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("open terminal test log");
        (file, path)
    }
}
