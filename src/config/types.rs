//! Configuration type definitions.

use knuffel::Decode;
use std::path::PathBuf;

/// Expand tilde (~) prefix to the user's home directory.
/// Handles both "~" alone and "~/path/to/something" patterns.
pub(crate) fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    } else if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(path)
}

/// Main configuration structure parsed from jaws.kdl.
#[derive(Debug, Decode, Clone)]
pub struct Config {
    #[knuffel(child)]
    pub defaults: Option<Defaults>,

    #[knuffel(children(name = "provider"))]
    pub providers: Vec<ProviderConfig>,
}

/// Default settings for jaws.
#[derive(Debug, Decode, Clone, Default)]
pub struct Defaults {
    #[knuffel(property)]
    pub editor: Option<String>,

    #[knuffel(property(name = "secrets_path"))]
    pub secrets_path: Option<String>,

    #[knuffel(property(name = "cache_ttl"))]
    pub cache_ttl: Option<u64>,

    /// Default provider for commands that accept a secret reference.
    /// When set, allows omitting the provider:// prefix.
    #[knuffel(property(name = "default_provider"))]
    pub default_provider: Option<String>,
}

/// Configuration for a secrets provider.
#[derive(Debug, Decode, Clone)]
pub struct ProviderConfig {
    #[knuffel(argument)]
    pub id: String,

    #[knuffel(property)]
    pub kind: String, // "aws" or "onepassword"

    #[knuffel(child, unwrap(argument))]
    pub profile: Option<String>,

    #[knuffel(child, unwrap(argument))]
    pub region: Option<String>,

    #[knuffel(child, unwrap(argument))]
    pub vault: Option<String>,

    #[knuffel(child, unwrap(argument))]
    pub token_env: Option<String>,
}

impl ProviderConfig {
    /// Create a new AWS provider config
    pub fn new_aws(id: String, profile: Option<String>, region: Option<String>) -> Self {
        Self {
            id,
            kind: "aws".to_string(),
            profile,
            region,
            vault: None,
            token_env: None,
        }
    }

    /// Create a new 1Password provider config
    pub fn new_onepassword(id: String, vault: Option<String>, token_env: Option<String>) -> Self {
        Self {
            id,
            kind: "onepassword".to_string(),
            profile: None,
            region: None,
            vault,
            token_env,
        }
    }
}

impl Config {
    /// Get the editor, defaulting to EDITOR env var or "vi"
    pub fn editor(&self) -> String {
        self.defaults
            .as_ref()
            .and_then(|d| d.editor.clone())
            .unwrap_or_else(|| std::env::var("EDITOR").unwrap_or_else(|_| "vi".into()))
    }

    /// Get the secrets path, defaulting to "./.secrets"
    /// Expands ~ to the user's home directory if present.
    pub fn secrets_path(&self) -> PathBuf {
        self.defaults
            .as_ref()
            .and_then(|d| d.secrets_path.clone())
            .map(|p| expand_tilde(&p))
            .unwrap_or_else(|| PathBuf::from("./.secrets"))
    }

    /// Get the cache TTL in seconds, defaulting to 900 (15 minutes)
    pub fn cache_ttl(&self) -> u64 {
        self.defaults
            .as_ref()
            .and_then(|d| d.cache_ttl)
            .unwrap_or(900)
    }

    /// Get the default provider (if set).
    /// When set, allows omitting the provider:// prefix in commands.
    pub fn default_provider(&self) -> Option<String> {
        self.defaults
            .as_ref()
            .and_then(|d| d.default_provider.clone())
    }

    /// Get the path to the database file
    pub fn db_path(&self) -> PathBuf {
        self.secrets_path().join("jaws.db")
    }

    /// Update a default setting
    pub fn set_default(&mut self, key: &str, value: &str) -> Result<(), String> {
        let defaults = self.defaults.get_or_insert(Defaults::default());
        match key {
            "editor" => defaults.editor = Some(value.to_string()),
            "secrets_path" => defaults.secrets_path = Some(value.to_string()),
            "cache_ttl" => {
                defaults.cache_ttl =
                    Some(value.parse().map_err(|_| "Invalid number for cache_ttl")?)
            }
            "default_provider" => defaults.default_provider = Some(value.to_string()),
            _ => {
                return Err(format!(
                    "Unknown setting: {}. Valid settings: editor, secrets_path, cache_ttl, default_provider",
                    key
                ))
            }
        }
        Ok(())
    }

    /// Get a default setting value as string
    pub fn get_default(&self, key: &str) -> Result<String, String> {
        match key {
            "editor" => Ok(self.editor()),
            "secrets_path" => Ok(self.secrets_path().to_string_lossy().to_string()),
            "cache_ttl" => Ok(self.cache_ttl().to_string()),
            "default_provider" => {
                Ok(self.default_provider().unwrap_or_else(|| "(not set)".to_string()))
            }
            _ => Err(format!(
                "Unknown setting: {}. Valid settings: editor, secrets_path, cache_ttl, default_provider",
                key
            )),
        }
    }

    /// Add a provider to the config
    #[allow(dead_code)]
    pub fn add_provider(&mut self, provider: ProviderConfig) {
        // Remove existing provider with same id if present
        self.providers.retain(|p| p.id != provider.id);
        self.providers.push(provider);
    }

    /// Remove a provider by id
    #[allow(dead_code)]
    pub fn remove_provider(&mut self, id: &str) -> bool {
        let len_before = self.providers.len();
        self.providers.retain(|p| p.id != id);
        self.providers.len() < len_before
    }
}
