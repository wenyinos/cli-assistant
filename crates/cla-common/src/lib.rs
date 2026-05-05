//! # `cla-common`
//!
//! Shared types, configuration, error handling, session management, and utility
//! functions for the command-line assistant — a Rust port of the Python CLI
//! assistant.
//!
//! ## Modules
//!
//! | Module | Description |
//! |---|---|
//! | [`config`] | TOML configuration loading and schema types |
//! | [`constants`] | Application-wide constants (version, paths) |
//! | [`environment`] | XDG path resolution |
//! | [`errors`] | Error types with domain-specific exit codes |
//! | [`files`] | File utilities: create, write, lock, MIME guessing |
//! | [`session`] | User session identity (machine-id + UUID v5) |
//! | [`system`] | System information helpers (OS name, version, arch) |

pub mod config;
pub mod constants;
pub mod environment;
pub mod errors;
pub mod files;
pub mod session;
pub mod system;

// Re-export key types at crate root for ergonomic imports.
pub use config::AppConfig;
/// Convenience alias — the rest of the codebase uses `Config`.
pub type Config = AppConfig;
pub use errors::{ClaError, Result};
pub use session::UserSessionManager;
pub use constants::VERSION;
