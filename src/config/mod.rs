pub mod file;
pub mod flags;

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub region: Option<String>,
    pub editor: String,
    pub secrets_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            region: None,
            editor: std::env::var("EDITOR").unwrap_or_else(|_| "vi".into()),
            secrets_path: PathBuf::from("./.secrets"),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let flags = flags::parse_flags();
        let file_config = file::load_config_file()?;
        let mut config = Config::default();

        // Apply config file overrides
        if let Some(file_cfg) = file_config {
            if let Some(region) = file_cfg.region {
                config.region = Some(region);
            }
            if let Some(editor) = file_cfg.editor {
                config.editor = editor;
            }
            if let Some(path) = file_cfg.secrets_path {
                config.secrets_path = path;
            }
        }

        // Apply flag overrides (highest priority)
        if let Some(region) = flags.region {
            config.region = Some(region);
        }
        if let Some(editor) = flags.editor {
            config.editor = editor;
        }
        if let Some(path) = flags.secrets_path {
            config.secrets_path = path;
        }

        Ok(config)
    }
}
