pub mod file;
pub mod flags;

use crate::cli::Cli;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub provider: Option<String>,
    pub region: Option<String>,
    pub editor: String,
    pub secrets_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: Some("aws".to_string()),
            region: None,
            editor: std::env::var("EDITOR").unwrap_or_else(|_| "vi".into()),
            secrets_path: PathBuf::from("./.secrets"),
        }
    }
}

impl Config {
    pub fn load_from_cli(cli: &Cli) -> Result<Self, Box<dyn std::error::Error>> {
        let flags = flags::Flags::from(cli);
        let file_config = file::load_config_file()?;
        let mut config = Config::default();

        // Apply config file overrides
        if let Some(file_cfg) = file_config {
            if let Some(provider) = file_cfg.provider {
                config.provider = Some(provider);
            }
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
        if let Some(provider) = flags.provider {
            config.provider = Some(provider);
        }
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
