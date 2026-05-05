//! D-Bus service constants for the Command Line Assistant.
//!
//! All identifiers follow the `com.redhat.lightspeed` namespace convention,
//! mirroring the Python `constants.py` definitions.

// ---------------------------------------------------------------------------
// Namespace component arrays (mirrors Python tuples)
// ---------------------------------------------------------------------------

/// Root service namespace: `("com", "redhat", "lightspeed")`.
pub const SERVICE_NAMESPACE: &[&str] = &["com", "redhat", "lightspeed"];

/// Chat namespace: `("com", "redhat", "lightspeed", "chat")`.
pub const CHAT_NAMESPACE: &[&str] = &["com", "redhat", "lightspeed", "chat"];

/// History namespace: `("com", "redhat", "lightspeed", "history")`.
pub const HISTORY_NAMESPACE: &[&str] = &["com", "redhat", "lightspeed", "history"];

/// User namespace: `("com", "redhat", "lightspeed", "user")`.
pub const USER_NAMESPACE: &[&str] = &["com", "redhat", "lightspeed", "user"];

// ---------------------------------------------------------------------------
// D-Bus well-known bus names (dot-separated identifiers)
// ---------------------------------------------------------------------------

/// D-Bus bus name for the chat service.
pub const CHAT_IDENTIFIER: &str = "com.redhat.lightspeed.chat";

/// D-Bus bus name for the history service.
pub const HISTORY_IDENTIFIER: &str = "com.redhat.lightspeed.history";

/// D-Bus bus name for the user service.
pub const USER_IDENTIFIER: &str = "com.redhat.lightspeed.user";

// Convenience aliases for the client crate.
/// D-Bus well-known bus name for the chat service.
pub const CHAT_BUS_NAME: &str = CHAT_IDENTIFIER;
/// D-Bus well-known bus name for the history service.
pub const HISTORY_BUS_NAME: &str = HISTORY_IDENTIFIER;
/// D-Bus well-known bus name for the user service.
pub const USER_BUS_NAME: &str = USER_IDENTIFIER;

// ---------------------------------------------------------------------------
// D-Bus object paths
// ---------------------------------------------------------------------------

/// Object path for the chat interface.
pub const CHAT_OBJECT_PATH: &str = "/com/redhat/lightspeed/chat";

/// Object path for the history interface.
pub const HISTORY_OBJECT_PATH: &str = "/com/redhat/lightspeed/history";

/// Object path for the user interface.
pub const USER_OBJECT_PATH: &str = "/com/redhat/lightspeed/user";

// ---------------------------------------------------------------------------
// D-Bus interface names (same as bus names by convention)
// ---------------------------------------------------------------------------

/// Interface name for chat operations.
pub const CHAT_INTERFACE: &str = "com.redhat.lightspeed.chat";

/// Interface name for history operations.
pub const HISTORY_INTERFACE: &str = "com.redhat.lightspeed.history";

/// Interface name for user operations.
pub const USER_INTERFACE: &str = "com.redhat.lightspeed.user";
