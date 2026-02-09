use crate::keybindings::KeyTable;
use crate::options::Options;
use anyhow::Result;
use std::path::PathBuf;
use tracing::{debug, warn};

/// Top-level configuration.
pub struct Config {
    pub options: Options,
    pub key_table: KeyTable,
}

impl Config {
    /// Create a default configuration.
    pub fn default_config() -> Self {
        Config {
            options: Options::default(),
            key_table: KeyTable::default_tmux_bindings(),
        }
    }

    /// Load configuration from the default config file (~/.wtmux.conf).
    pub fn load() -> Result<Self> {
        let mut config = Self::default_config();

        if let Some(path) = Self::config_path() {
            if path.exists() {
                debug!("Loading config from: {}", path.display());
                let content = std::fs::read_to_string(&path)?;
                config.apply_config_string(&content)?;
            } else {
                debug!("No config file found at: {}", path.display());
            }
        }

        Ok(config)
    }

    /// Get the default config file path.
    pub fn config_path() -> Option<PathBuf> {
        std::env::var("USERPROFILE")
            .ok()
            .map(|home| PathBuf::from(home).join(".wtmux.conf"))
    }

    /// Apply configuration from a string (used by source-file command).
    pub fn apply_config_string(&mut self, content: &str) -> Result<()> {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Err(e) = self.apply_config_line(line) {
                warn!("Config error: {} (line: {})", e, line);
            }
        }
        Ok(())
    }

    fn apply_config_line(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return Ok(());
        }

        match parts[0] {
            "set-option" | "set" => {
                crate::parser::parse_set_option(&mut self.options, parts[1])?;
            }
            "bind-key" | "bind" => {
                crate::parser::parse_bind_key(&mut self.key_table, parts[1])?;
            }
            "unbind-key" | "unbind" => {
                crate::parser::parse_unbind_key(&mut self.key_table, parts[1])?;
            }
            "source-file" | "source" => {
                let path = parts[1].trim();
                let content = std::fs::read_to_string(path)?;
                self.apply_config_string(&content)?;
            }
            _ => {
                warn!("Unknown config command: {}", parts[0]);
            }
        }

        Ok(())
    }
}
