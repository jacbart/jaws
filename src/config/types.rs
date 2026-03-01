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

    /// Maximum number of versions to keep per secret.
    /// Older versions are automatically pruned when this limit is exceeded.
    /// Default: 10
    #[knuffel(property(name = "max_versions"))]
    pub max_versions: Option<u32>,

    /// Whether to cache decrypted credentials in the OS keychain.
    /// When enabled (the default), the first successful decryption stores the
    /// plaintext in the native credential store (e.g. macOS Keychain) so that
    /// subsequent invocations don't prompt for a passphrase until the cache_ttl
    /// expires.  Set to `false` to disable.
    /// Default: true
    #[knuffel(property(name = "keychain_cache"))]
    pub keychain_cache: Option<bool>,
}

/// Configuration for a secrets provider.
#[derive(Debug, Decode, Clone)]
pub struct ProviderConfig {
    #[knuffel(argument)]
    pub id: String,

    #[knuffel(property)]
    pub kind: String, // "aws", "onepassword", "bw", or "gcp"

    #[knuffel(child, unwrap(argument))]
    pub profile: Option<String>,

    #[knuffel(child, unwrap(argument))]
    pub region: Option<String>,

    /// Vault ID for 1Password, or Project ID for Bitwarden
    #[knuffel(child, unwrap(argument))]
    pub vault: Option<String>,

    /// Organization ID for Bitwarden (and potentially others)
    #[knuffel(child, unwrap(argument))]
    pub organization: Option<String>,

    #[knuffel(child, unwrap(argument))]
    pub token_env: Option<String>,

    /// GCP project ID for Google Cloud Secret Manager
    #[knuffel(child, unwrap(argument))]
    pub project: Option<String>,
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
            organization: None,
            token_env: None,
            project: None,
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
            organization: None,
            token_env,
            project: None,
        }
    }

    /// Create a new Bitwarden provider config
    pub fn new_bitwarden(
        id: String,
        project_id: Option<String>,
        organization_id: Option<String>,
        token_env: Option<String>,
    ) -> Self {
        Self {
            id,
            kind: "bw".to_string(),
            profile: None,
            region: None,
            vault: project_id, // We map project_id to the 'vault' field
            organization: organization_id,
            token_env,
            project: None,
        }
    }

    /// Create a new GCP Secret Manager provider config
    pub fn new_gcp(id: String, project_id: Option<String>) -> Self {
        Self {
            id,
            kind: "gcp".to_string(),
            profile: None,
            region: None,
            vault: None,
            organization: None,
            token_env: None,
            project: project_id,
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

    /// Get the maximum number of versions to keep per secret.
    /// Returns None if unlimited, or Some(n) to keep last n versions.
    /// Default: 10
    pub fn max_versions(&self) -> Option<u32> {
        self.defaults
            .as_ref()
            .and_then(|d| d.max_versions)
            .or(Some(10)) // Default to 10 versions
    }

    /// Whether OS keychain caching is enabled (default: true).
    pub fn keychain_cache(&self) -> bool {
        self.defaults
            .as_ref()
            .and_then(|d| d.keychain_cache)
            .unwrap_or(true)
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
            "max_versions" => {
                let v: u32 = value
                    .parse()
                    .map_err(|_| "Invalid number for max_versions")?;
                if v == 0 {
                    return Err("max_versions must be at least 1".to_string());
                }
                defaults.max_versions = Some(v);
            }
            "keychain_cache" => {
                defaults.keychain_cache = Some(
                    value
                        .parse()
                        .map_err(|_| "Invalid boolean for keychain_cache (use true/false)")?,
                );
            }
            _ => {
                return Err(format!(
                    "Unknown setting: {}. Valid settings: editor, secrets_path, cache_ttl, default_provider, max_versions, keychain_cache",
                    key
                ));
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
            "default_provider" => Ok(self
                .default_provider()
                .unwrap_or_else(|| "(not set)".to_string())),
            "max_versions" => Ok(self
                .max_versions()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unlimited".to_string())),
            "keychain_cache" => Ok(self.keychain_cache().to_string()),
            _ => Err(format!(
                "Unknown setting: {}. Valid settings: editor, secrets_path, cache_ttl, default_provider, max_versions, keychain_cache",
                key
            )),
        }
    }

    /// Add a provider to the config
    pub fn add_provider(&mut self, provider: ProviderConfig) {
        // Remove existing provider with same id if present
        self.providers.retain(|p| p.id != provider.id);
        self.providers.push(provider);
    }

    /// Remove a provider by id
    pub fn remove_provider(&mut self, id: &str) -> bool {
        let len_before = self.providers.len();
        self.providers.retain(|p| p.id != id);
        self.providers.len() < len_before
    }
}
