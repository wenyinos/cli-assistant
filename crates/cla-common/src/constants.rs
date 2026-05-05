//! Application-wide constants.

/// Current version of the command-line assistant, sourced from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration file path.
pub const DEFAULT_CONFIG_PATH: &str = "/etc/cli-assistant/config.toml";

/// System machine-id path.
pub const MACHINE_ID_PATH: &str = "/etc/machine-id";

/// Application name used for XDG directory naming.
pub const APP_NAME: &str = "command-line-assistant";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_nonempty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn config_path_is_absolute() {
        assert!(DEFAULT_CONFIG_PATH.starts_with('/'));
    }
}
