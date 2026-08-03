//! XDG path resolution for the command-line assistant.
//!
//! Maps to Python `utils/environment.py`. Resolves state and data directories
//! following XDG Base Directory conventions with application-specific
//! defaults. Configuration is deliberately loaded from the fixed
//! `/etc/cli-assistant/config.toml` (see `config.rs`).

use std::path::PathBuf;

use crate::constants::APP_NAME;

/// Returns the XDG state directory path.
///
/// Default: `~/.local/state/command-line-assistant`
///
/// Uses `$XDG_STATE_HOME` if set, otherwise `~/.local/state`.
pub fn get_xdg_state_path() -> PathBuf {
    if let Ok(xdg_state) = std::env::var("XDG_STATE_HOME") {
        return PathBuf::from(xdg_state).join(APP_NAME);
    }

    // XDG spec default: ~/.local/state
    dirs::state_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("state")))
        .unwrap_or_else(|| PathBuf::from("~/.local/state"))
        .join(APP_NAME)
}

/// Returns the XDG data directory path.
///
/// Default: `~/.local/share/command-line-assistant`
///
/// Uses `$XDG_DATA_HOME` if set, otherwise `~/.local/share`.
pub fn get_xdg_data_path() -> PathBuf {
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg_data).join(APP_NAME);
    }

    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join(APP_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_path_contains_app_name() {
        let path = get_xdg_state_path();
        assert!(path.ends_with(APP_NAME));
    }

    #[test]
    fn data_path_contains_app_name() {
        let path = get_xdg_data_path();
        assert!(path.ends_with(APP_NAME));
    }
}
