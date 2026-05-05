//! c – Command Line Assistant client binary.

mod cli;
mod dbus_client;
mod rendering;
mod terminal;

use std::io::{self, IsTerminal, Read};
use std::process;

use clap::Parser;

use cla_common::environment::get_xdg_state_path;
use cla_common::files;
use cla_dbus::structures::{AttachmentInput, Question, StdinInput, TerminalInput};

use cli::{Cli, Commands};
use dbus_client::DbusClient;
use rendering::Renderer;

/// Maximum question size in bytes (32KB).
const MAX_QUESTION_SIZE: usize = 32_000;

/// Legal notice shown once per session.
const LEGAL_NOTICE: &str = "This feature uses AI technology. Do not include any personal information or \
    other sensitive information in your input. Interactions may be used to \
    improve Red Hat's products or services.";

/// Always shown after responses.
const ALWAYS_LEGAL_MESSAGE: &str = "Always review AI-generated content prior to use.";

/// Default chat name.
const DEFAULT_CHAT_NAME: &str = "default";

/// Default chat description.
const DEFAULT_CHAT_DESCRIPTION: &str = "Default Command Line Assistant Chat.";

/// Known subcommands — if none of these appear in argv, default to "chat".
const SUBCOMMANDS: &[&str] = &["chat", "history", "feedback", "shell"];

/// Global flags that should be preserved before the subcommand.
const GLOBAL_FLAGS: &[&str] = &["-p", "--plain", "--debug", "--version", "-v", "-h", "--help"];

/// Inject "chat" as the default subcommand when the user runs `c "question"`
/// without explicitly naming a subcommand. Mirrors Python's `add_default_command`.
fn add_default_command(args: Vec<String>) -> Vec<String> {
    if args.len() <= 1 {
        return args;
    }

    let mut global_flags: Vec<String> = Vec::new();
    let mut rest: Vec<String> = Vec::new();

    for arg in &args[1..] {
        if GLOBAL_FLAGS.contains(&arg.as_str()) {
            global_flags.push(arg.clone());
        } else {
            rest.push(arg.clone());
        }
    }

    // Check if any remaining arg is a known subcommand.
    let has_subcommand = rest.iter().any(|a| SUBCOMMANDS.contains(&a.as_str()));

    if has_subcommand {
        args
    } else {
        // Insert "chat" before the rest.
        let mut result = vec![args[0].clone()];
        result.extend(global_flags);
        result.push("chat".to_string());
        result.extend(rest);
        result
    }
}

#[tokio::main]
async fn main() {
    // Pre-process args: inject "chat" as default subcommand if none given.
    let args = add_default_command(std::env::args().collect());
    let cli = Cli::parse_from(args);

    // Setup logging if debug mode is enabled.
    if cli.debug {
        tracing_subscriber::fmt()
            .with_env_filter("debug")
            .init();
    }

    let renderer = Renderer::new(cli.plain);

    // Read stdin if available.
    let stdin = read_stdin();

    // If no command specified, default to chat.
    let command = cli.command.unwrap_or(Commands::Chat {
        query_string: None,
        attachment: None,
        interactive: false,
        with_output: None,
        list: false,
        delete: None,
        delete_all: false,
        name: None,
        description: None,
    });

    let exit_code = match command {
        Commands::Chat {
            query_string,
            attachment,
            interactive,
            with_output,
            list,
            delete,
            delete_all,
            name,
            description,
        } => {
            handle_chat(
                &renderer,
                query_string,
                stdin,
                attachment,
                interactive,
                with_output,
                list,
                delete,
                delete_all,
                name,
                description,
                cli.plain,
            )
            .await
        }
        Commands::History {
            from_chat,
            first,
            last,
            filter,
            all,
            clear,
            clear_all,
        } => {
            handle_history(
                &renderer,
                &from_chat,
                first,
                last,
                filter.as_deref(),
                all,
                clear,
                clear_all,
            )
            .await
        }
        Commands::Feedback => handle_feedback(&renderer),
        Commands::Shell {
            enable_capture,
            enable_interactive,
            disable_interactive,
        } => handle_shell(&renderer, enable_capture, enable_interactive, disable_interactive),
    };

    process::exit(exit_code);
}

// ---------------------------------------------------------------------------
// Chat command
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
async fn handle_chat(
    renderer: &Renderer,
    query_string: Option<String>,
    stdin: Option<String>,
    attachment: Option<String>,
    interactive: bool,
    with_output: Option<usize>,
    list: bool,
    delete: Option<String>,
    delete_all: bool,
    name: Option<String>,
    description: Option<String>,
    plain: bool,
) -> i32 {
    let dbus = match DbusClient::new().await {
        Ok(c) => c,
        Err(e) => {
            renderer.error(&format!("Failed to connect to daemon: {}", e));
            return 69; // EX_UNAVAILABLE
        }
    };

    let euid = unsafe { libc::geteuid() };
    let user_id = match dbus.get_user_id(euid).await {
        Ok(id) => id,
        Err(e) => {
            renderer.error(&format!("Failed to get user ID: {}", e));
            return 1;
        }
    };

    let name = name.unwrap_or_else(|| DEFAULT_CHAT_NAME.to_string());
    let description = description.unwrap_or_else(|| DEFAULT_CHAT_DESCRIPTION.to_string());

    // Handle list/delete operations
    if list {
        return match dbus.get_all_chats(&user_id).await {
            Ok(chats) => {
                if chats.chats.is_empty() {
                    renderer.normal("No chats available.");
                } else {
                    renderer.normal(&format!("Found a total of {} chats:", chats.chats.len()));
                    for (i, chat) in chats.chats.iter().enumerate() {
                        renderer.normal(&format!(
                            "{}. Chat: {} - {} (created at: {})",
                            i, chat.name, chat.description, chat.created_at
                        ));
                    }
                }
                0
            }
            Err(e) => {
                renderer.error(&format!("Failed to list chats: {}", e));
                1
            }
        };
    }

    if let Some(chat_name) = delete {
        return match dbus.delete_chat(&user_id, &chat_name).await {
            Ok(()) => {
                renderer.normal(&format!("Chat {} deleted successfully.", chat_name));
                0
            }
            Err(e) => {
                renderer.error(&format!("Failed to delete chat: {}", e));
                1
            }
        };
    }

    if delete_all {
        return match dbus.delete_all_chats(&user_id).await {
            Ok(()) => {
                renderer.normal("Deleted all chats successfully.");
                0
            }
            Err(e) => {
                renderer.error(&format!("Failed to delete all chats: {}", e));
                1
            }
        };
    }

    // Ensure chat exists
    let chat_id = match dbus.get_chat_id(&user_id, &name).await {
        Ok(id) => id,
        Err(_) => match dbus.create_chat(&user_id, &name, &description).await {
            Ok(id) => id,
            Err(e) => {
                renderer.error(&format!("Failed to create chat: {}", e));
                return 1;
            }
        },
    };

    if interactive {
        return handle_interactive_chat(renderer, &dbus, &user_id, &chat_id, stdin, plain).await;
    }

    // Gather input
    let question_text = gather_input(query_string, stdin.clone(), attachment.as_deref(), with_output);

    if question_text.trim().len() < 2 {
        renderer.error("Your query needs to have at least 2 characters.");
        return 1;
    }

    // Trim to max size
    let question_text = if question_text.len() > MAX_QUESTION_SIZE {
        renderer.warning(&format!(
            "Question exceeds {}KB limit. Trimming to fit.",
            MAX_QUESTION_SIZE / 1000
        ));
        question_text[..MAX_QUESTION_SIZE].to_string()
    } else {
        question_text
    };

    // Show legal notice once
    show_legal_notice_once(renderer);

    // Build question payload
    let question = Question {
        message: question_text,
        stdin: stdin.map(|s| StdinInput { stdin: s }),
        attachment: attachment.map(|path| {
            let contents = std::fs::read_to_string(&path).unwrap_or_default();
            let mimetype = files::guess_mimetype(std::path::Path::new(&path));
            AttachmentInput { contents, mimetype }
        }),
        terminal: with_output.map(|_| {
            let blocks = terminal::parse_terminal_output();
            let output = terminal::find_output_by_index(
                -(with_output.unwrap_or(1) as isize),
                &blocks,
            );
            TerminalInput { output }
        }),
        systeminfo: None,
    };

    // Show spinner and submit
    eprint!("⁺₊+ Asking RHEL Lightspeed...");
    let response = match dbus.ask_question(&user_id, question).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!();
            renderer.error(&format!("Failed to get response: {}", e));
            return 1;
        }
    };
    eprintln!();

    // Display response
    display_response(renderer, &response.message);

    // Write history
    let _ = dbus
        .write_history(&chat_id, &user_id, &response.message, &response.message)
        .await;

    0
}

async fn handle_interactive_chat(
    renderer: &Renderer,
    dbus: &DbusClient,
    user_id: &str,
    chat_id: &str,
    stdin: Option<String>,
    _plain: bool,
) -> i32 {
    renderer.normal("Welcome to the interactive mode for command line assistant!");
    renderer.normal("To exit, press Ctrl + C or type '.exit'.");
    renderer.normal("The current session does not include running context.");
    renderer.normal("");

    loop {
        eprint!(">>> ");
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }

        let input = input.trim();

        if input == ".exit" {
            break;
        }

        if input.is_empty() {
            renderer.error("Your question can't be empty. Please, try again.");
            continue;
        }

        let question = Question {
            message: input.to_string(),
            stdin: stdin.clone().map(|s| StdinInput { stdin: s }),
            attachment: None,
            terminal: None,
            systeminfo: None,
        };

        eprint!("⁺₊+ Asking RHEL Lightspeed...");
        match dbus.ask_question(user_id, question).await {
            Ok(response) => {
                eprintln!();
                display_response(renderer, &response.message);
                let _ = dbus
                    .write_history(chat_id, user_id, &response.message, &response.message)
                    .await;
            }
            Err(e) => {
                eprintln!();
                renderer.error(&format!("Failed to get response: {}", e));
            }
        }
    }

    0
}

// ---------------------------------------------------------------------------
// History command
// ---------------------------------------------------------------------------

async fn handle_history(
    renderer: &Renderer,
    from_chat: &str,
    first: bool,
    last: bool,
    filter: Option<&str>,
    _all: bool,
    clear: bool,
    clear_all: bool,
) -> i32 {
    let dbus = match DbusClient::new().await {
        Ok(c) => c,
        Err(e) => {
            renderer.error(&format!("Failed to connect to daemon: {}", e));
            return 69;
        }
    };

    let euid = unsafe { libc::geteuid() };
    let user_id = match dbus.get_user_id(euid).await {
        Ok(id) => id,
        Err(e) => {
            renderer.error(&format!("Failed to get user ID: {}", e));
            return 1;
        }
    };

    // Check chat availability
    if !clear_all {
        match dbus.is_chat_available(&user_id, from_chat).await {
            Ok(false) => {
                renderer.error(&format!(
                    "Nothing to clean as {} chat is not available. Try asking something first.",
                    from_chat
                ));
                return 1;
            }
            Err(e) => {
                renderer.error(&format!("Failed to check chat availability: {}", e));
                return 1;
            }
            _ => {}
        }
    }

    if clear {
        return match dbus.clear_history(&user_id, from_chat).await {
            Ok(()) => {
                renderer.normal("History cleaned successfully.");
                0
            }
            Err(e) => {
                renderer.error(&format!("Failed to clear history: {}", e));
                1
            }
        };
    }

    if clear_all {
        return match dbus.clear_all_history(&user_id).await {
            Ok(()) => {
                renderer.normal("All histories cleared successfully.");
                0
            }
            Err(e) => {
                renderer.error(&format!("Failed to clear all history: {}", e));
                1
            }
        };
    }

    let result = if first {
        dbus.get_first_conversation(&user_id, from_chat).await
    } else if last {
        dbus.get_last_conversation(&user_id, from_chat).await
    } else if let Some(f) = filter {
        dbus.get_filtered_conversation(&user_id, f, from_chat)
            .await
    } else {
        dbus.get_history(&user_id).await
    };

    match result {
        Ok(history) => {
            if history.histories.is_empty() {
                renderer.normal("No history entries found");
            } else {
                for entry in &history.histories {
                    renderer.markdown(&format!("## 🤔 Question\n{}\n", entry.question));
                    renderer.normal("");
                    renderer.markdown(&format!("## 🤖 Answer\n{}\n", entry.response));
                    renderer.markdown(&format!("*Created at: {}*\n", entry.created_at));
                    if history.histories.len() > 1 {
                        renderer.normal(&"═".repeat(60));
                    }
                }
            }
            0
        }
        Err(e) => {
            renderer.error(&format!("Failed to get history: {}", e));
            1
        }
    }
}

// ---------------------------------------------------------------------------
// Feedback command
// ---------------------------------------------------------------------------

fn handle_feedback(renderer: &Renderer) -> i32 {
    renderer.notice(
        "Do not include any personal information or other \
         sensitive information in your feedback. Feedback may \
         be used to improve Red Hat's products or services.",
    );
    renderer.normal("To submit feedback, use the following email address: <cla-feedback@redhat.com>.");
    0
}

// ---------------------------------------------------------------------------
// Shell command
// ---------------------------------------------------------------------------

fn handle_shell(
    renderer: &Renderer,
    enable_capture: bool,
    enable_interactive: bool,
    disable_interactive: bool,
) -> i32 {
    if enable_interactive {
        return write_bashrc_integration(renderer, "cla-interactive.bashrc", BASH_INTERACTIVE);
    }

    if disable_interactive {
        return remove_bashrc_integration(renderer, "cla-interactive.bashrc");
    }

    if enable_capture {
        renderer.normal("Starting terminal reader. Press Ctrl + D to stop the capturing.");
        renderer.normal(&format!(
            "Terminal capture log is being written to {}",
            terminal::terminal_capture_file().display()
        ));
        // TODO: Implement PTY spawn (requires nix crate integration)
        renderer.warning("Terminal capture is not yet implemented in the Rust version.");
        return 0;
    }

    renderer.warning("No operation specified. Use --help to see available options.");
    1
}

const BASH_INTERACTIVE: &str = r#"
# Command Line Assistant Interactive Mode Integration
__c_interactive() {
    local old_tty=$(stty -g)
    local c_binary=/usr/bin/c

    cleanup() {
        stty "$old_tty"
    }

    trap cleanup EXIT
    stty sane
    stty echo
    stty icanon

    if command -v $c_binary >/dev/null 2>&1; then
        $c_binary chat --interactive
    else
        echo "Error: Command Line Assistant is not installed"
        return 1
    fi

    cleanup
}

bind -x '"\C-g": __c_interactive'
"#;

fn write_bashrc_integration(renderer: &Renderer, filename: &str, contents: &str) -> i32 {
    let bashrc_d = dirs::home_dir()
        .unwrap_or_default()
        .join(".bashrc.d");

    if let Err(e) = files::create_folder(&bashrc_d, true, 0o700) {
        renderer.error(&format!("Failed to create bashrc.d directory: {}", e));
        return 1;
    }

    let file_path = bashrc_d.join(filename);
    if file_path.exists() {
        renderer.warning(&format!(
            "The integration is already present and enabled at {}! \
             Restart your terminal or source ~/.bashrc in case it's not working.",
            file_path.display()
        ));
        return 2;
    }

    if let Err(e) = files::write_file(contents.as_bytes(), &file_path, 0o644) {
        renderer.error(&format!("Failed to write integration file: {}", e));
        return 1;
    }

    renderer.normal(&format!(
        "Integration successfully added at {}. \
         In order to use it, please restart your terminal or source ~/.bashrc",
        file_path.display()
    ));
    0
}

fn remove_bashrc_integration(renderer: &Renderer, filename: &str) -> i32 {
    let file_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".bashrc.d")
        .join(filename);

    if !file_path.exists() {
        renderer.warning("It seems that the integration is not enabled. Skipping operation.");
        return 2;
    }

    match std::fs::remove_file(&file_path) {
        Ok(()) => {
            renderer.normal("Integration disabled successfully.");
            0
        }
        Err(e) => {
            renderer.error(&format!("Failed to remove integration: {}", e));
            1
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read stdin if it's piped (not a terminal).
fn read_stdin() -> Option<String> {
    if io::stdin().is_terminal() {
        return None;
    }

    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok()?;
    let trimmed = buf.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Gather input from various sources (query, stdin, attachment, terminal output).
fn gather_input(
    query: Option<String>,
    stdin: Option<String>,
    attachment: Option<&str>,
    with_output: Option<usize>,
) -> String {
    // Priority: query > stdin > attachment > terminal output
    if let Some(q) = query {
        if !q.trim().is_empty() {
            return q;
        }
    }
    if let Some(s) = stdin {
        if !s.trim().is_empty() {
            return s;
        }
    }
    if let Some(path) = attachment {
        if let Ok(contents) = std::fs::read_to_string(path) {
            return contents;
        }
    }
    if with_output.is_some() {
        let blocks = terminal::parse_terminal_output();
        let output =
            terminal::find_output_by_index(-(with_output.unwrap_or(1) as isize), &blocks);
        if !output.is_empty() {
            return output;
        }
    }
    String::new()
}

/// Show the legal notice once per parent PID.
fn show_legal_notice_once(renderer: &Renderer) {
    let state_dir = get_xdg_state_path();
    let legal_file = state_dir.join("legal");
    let ppid = format!("{}", getppid());

    // Check if already shown for this parent PID
    if let Ok(content) = std::fs::read_to_string(&legal_file) {
        if content.trim() == ppid {
            return;
        }
    }

    // Show and record
    let _ = files::create_folder(&state_dir, true, 0o700);
    let _ = files::write_file(ppid.as_bytes(), &legal_file, 0o600);
    renderer.notice(LEGAL_NOTICE);
}

/// Display a response with borders and legal notice.
fn display_response(renderer: &Renderer, response: &str) {
    renderer.notice(&"─".repeat(72));
    renderer.normal("");
    renderer.markdown(response);
    renderer.normal("");
    renderer.notice(&"─".repeat(72));
    renderer.notice(ALWAYS_LEGAL_MESSAGE);
}

/// Get parent PID.
fn getppid() -> u32 {
    unsafe { libc::getppid() as u32 }
}
