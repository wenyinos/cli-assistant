//! XDG path resolution for the command-line assistant.
//!
//! Maps to Python `utils/environment.py`. Resolves state, data, and config
//! directories following XDG Base Directory conventions with
//! application-specific defaults.

use std::path::PathBuf;

use crate::constants::APP_NAME;

/// Returns the XDG config directory path.
///
/// Default: `/etc/xdg/command-line-assistant`
///
/// Falls back to `$XDG_CONFIG_HOME/command-line-assistant` if the system-wide
/// path is not accessible, or `~/.config/command-line-assistant` as a last resort.
pub fn get_xdg_config_path() -> PathBuf {
    // Primary: system-wide XDG config directory
    let system_path = PathBuf::from("/etc/xdg").join(APP_NAME);
    if system_path.is_dir() {
        return system_path;
    }

    // Secondary: $XDG_CONFIG_HOME
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        let path = PathBuf::from(xdg_config).join(APP_NAME);
        if path.is_dir() {
            return path;
        }
    }

    // Fallback: ~/.config/<app>
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join(APP_NAME)
}

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
    fn config_path_contains_app_name() {
        let path = get_xdg_config_path();
        assert!(path.ends_with(APP_NAME));
    }

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
