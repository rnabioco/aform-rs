//! Configuration file handling for aform.
//!
//! Loads settings from `~/.config/aform/aform.toml` or `./aform.toml`.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::color::Theme;

/// Application configuration loaded from aform.toml.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// UI theme colors.
    pub theme: Theme,
}

impl Config {
    /// Load configuration from file, falling back to defaults if not found.
    ///
    /// Search order:
    /// 1. `./aform.toml` (current directory)
    /// 2. `~/.config/aform/aform.toml` (XDG config)
    ///
    /// Returns `(config, was_file_loaded)` tuple.
    pub fn load() -> (Self, bool) {
        // Try current directory first
        if let Some(config) = Self::load_from_path(&PathBuf::from("aform.toml")) {
            return (config, true);
        }

        // Try XDG config directory
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("aform").join("aform.toml");
            if let Some(config) = Self::load_from_path(&config_path) {
                return (config, true);
            }
        }

        // Fall back to defaults
        (Self::default(), false)
    }

    /// Load configuration from a specific path.
    fn load_from_path(path: &PathBuf) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}
