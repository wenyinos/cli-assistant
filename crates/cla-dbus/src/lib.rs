//! # cla-dbus
//!
//! D-Bus interfaces and data structures for the Command Line Assistant.
//!
//! This crate defines the D-Bus service layer used for IPC between the CLI client
//! and the background daemon. It follows the `com.redhat.lightspeed` namespace.
//!
//! ## Crate Structure
//!
//! - [`constants`] — D-Bus service identifiers, bus names, and object paths.
//! - [`structures`] — Serializable data types exchanged over D-Bus.
//! - [`interfaces`] — Trait definitions for D-Bus service interfaces (Chat, History, User).
//! - [`exceptions`] — D-Bus specific error types.

pub mod constants;
pub mod exceptions;
pub mod interfaces;
pub mod structures;

// Re-export commonly used types at crate root.
pub use constants::*;
pub use exceptions::ClaDbusError;
pub use structures::*;
