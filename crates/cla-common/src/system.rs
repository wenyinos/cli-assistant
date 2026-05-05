//! System information helpers.
//!
//! Provides OS name, version, architecture, and machine-id reading — used by
//! the daemon to populate `SystemInfo` in LLM payloads.

use std::path::Path;

use crate::constants::MACHINE_ID_PATH;

/// Read `/etc/machine-id` and return the raw 32-char hex string.
///
/// Falls back to an empty string if the file cannot be read.
pub fn read_machine_id() -> String {
    std::fs::read_to_string(Path::new(MACHINE_ID_PATH))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Return the OS pretty-name from `/etc/os-release`.
///
/// Parses the `PRETTY_NAME="..."` field. Falls back to `"Linux"`.
pub fn os_name() -> String {
    parse_os_release_field("PRETTY_NAME").unwrap_or_else(|| "Linux".to_string())
}

/// Return the OS version ID from `/etc/os-release`.
///
/// Parses `VERSION_ID="..."`. Falls back to `"unknown"`.
pub fn os_version() -> String {
    parse_os_release_field("VERSION_ID").unwrap_or_else(|| "unknown".to_string())
}

/// Return the platform identifier from `/etc/os-release`.
///
/// Parses `PLATFORM_ID="..."`. Falls back to `"unknown"`.
pub fn os_id() -> String {
    parse_os_release_field("ID").unwrap_or_else(|| "unknown".to_string())
}

/// Return the CPU architecture string (e.g. `x86_64`, `aarch64`).
pub fn arch() -> &'static str {
    std::env::consts::ARCH
}

/// Parse a single `KEY="VALUE"` field from `/etc/os-release`.
fn parse_os_release_field(key: &str) -> Option<String> {
    let content = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim();
            // Strip optional `=` and surrounding quotes.
            let value = rest.strip_prefix('=').unwrap_or(rest).trim();
            let value = value.trim_matches('"');
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_is_nonempty() {
        assert!(!arch().is_empty());
    }

    #[test]
    fn os_name_returns_string() {
        // Just verify it doesn't panic.
        let _ = os_name();
    }
}
