//! CLI argument definitions using clap derive macros.

use clap::{Parser, Subcommand};

/// Command Line Assistant powered by RHEL Lightspeed.
///
/// An AI-driven assistant for RHEL system management, available directly
/// from the command line.
#[derive(Debug, Parser)]
#[command(name = "c", version, about, disable_help_subcommand = true)]
pub struct Cli {
    /// Enable plain output (no colors, animations, or rich content).
    #[arg(short, long, global = true)]
    pub plain: bool,

    /// Enable debug logging information.
    #[arg(long, global = true)]
    pub debug: bool,

    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Ask a question to the LLM.
    Chat {
        /// The question to ask.
        query_string: Option<String>,

        /// File attachment to read and send alongside the query.
        #[arg(short, long)]
        attachment: Option<String>,

        /// Start an interactive chat session.
        #[arg(short, long)]
        interactive: bool,

        /// Add output from terminal as context (1 = latest, 2 = second latest, etc.).
        /// Requires terminal capture to be enabled via `c shell --enable-capture`.
        #[arg(short = 'w', long)]
        with_output: Option<usize>,

        /// List all chats.
        #[arg(short, long)]
        list: bool,

        /// Delete a chat session by name.
        #[arg(short, long)]
        delete: Option<String>,

        /// Delete all chats.
        #[arg(long)]
        delete_all: bool,

        /// Give a name to the chat session.
        #[arg(short, long)]
        name: Option<String>,

        /// Give a description to the chat session.
        #[arg(long)]
        description: Option<String>,
    },

    /// Manage conversation history.
    History {
        /// Specify which chat to retrieve history from (default: "default").
        #[arg(long, default_value = "default")]
        from_chat: String,

        /// Get the first conversation from history.
        #[arg(short = 'f', long)]
        first: bool,

        /// Get the last conversation from history.
        #[arg(short, long)]
        last: bool,

        /// Search for a specific keyword in the history.
        #[arg(long)]
        filter: Option<String>,

        /// Get all conversation history.
        #[arg(short, long)]
        all: bool,

        /// Clear the entire history for a given chat.
        #[arg(short, long)]
        clear: bool,

        /// Clear the entire history.
        #[arg(long)]
        clear_all: bool,
    },

    /// Submit feedback about the Command Line Assistant.
    Feedback,

    /// Manage shell integrations.
    Shell {
        /// Enable terminal capture for the current terminal session.
        #[arg(long)]
        enable_capture: bool,

        /// Enable the shell integration for interactive mode.
        #[arg(long)]
        enable_interactive: bool,

        /// Disable the shell integration for interactive mode.
        #[arg(long)]
        disable_interactive: bool,
    },
}
