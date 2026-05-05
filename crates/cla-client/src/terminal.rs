//! Terminal capture module.
//!
//! Provides PTY-based terminal capture and JSON output parsing, mirroring
//! Python's `terminal/reader.py` and `terminal/parser.py`.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use cla_common::environment::get_xdg_state_path;

/// Path to the terminal capture log file.
pub fn terminal_capture_file() -> PathBuf {
    get_xdg_state_path().join("terminal.log")
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

    blocks.get(idx).map(|b| b.output.clone()).unwrap_or_default()
}

/// Strip ANSI escape sequences from text.
fn strip_ansi(text: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap_or_else(|_| {
        // Fallback: simple pattern
        regex::Regex::new(r"\x1b\[.*?m").unwrap()
    });
    re.replace_all(text, "").to_string()
}

// Simple regex fallback if the `regex` crate isn't available.
// For now we use a manual stripper.
mod regex {
    pub struct Regex;
    impl Regex {
        pub fn new(_pattern: &str) -> Result<Self, ()> {
            Ok(Self)
        }
        pub fn replace_all<'a>(&self, text: &'a str, _replacement: &str) -> std::borrow::Cow<'a, str> {
            // Manual ANSI stripping
            let mut result = String::with_capacity(text.len());
            let mut chars = text.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '\x1b' {
                    // Skip until we hit a letter
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if next.is_ascii_alphabetic() {
                            break;
                        }
                    }
                } else {
                    result.push(c);
                }
            }
            std::borrow::Cow::Owned(result)
        }
    }
}
