//! Configuration file loading and saving.

use std::path::{Path, PathBuf};

use super::types::{Config, Defaults, ProviderConfig, ServerConnection};

/// Raw deserialization structs mirroring hcl-rs's labeled-block shape.
/// `provider "<id>" { ... }` deserializes as a map keyed by the block label.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    defaults: Option<Defaults>,
    #[serde(default)]
    provider: hcl::Map<String, RawProvider>,
    #[serde(default)]
    server: hcl::Map<String, RawServer>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProvider {
    kind: String,
    profile: Option<String>,
    region: Option<String>,
    vault: Option<String>,
    organization: Option<String>,
    token_env: Option<String>,
    project: Option<String>,
    force_cli: Option<bool>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawServer {
    url: String,
    ca_cert: Option<String>,
    client_cert: Option<String>,
    client_key: Option<String>,
}

impl Config {
    /// Parse configuration from an HCL string.
    pub fn from_hcl(content: &str) -> Result<Self, hcl::Error> {
        let raw: RawConfig = hcl::from_str(content)?;
        Ok(Self {
            defaults: raw.defaults,
            providers: raw
                .provider
                .into_iter()
                .map(|(id, p)| ProviderConfig {
                    id,
                    kind: p.kind,
                    profile: p.profile,
                    region: p.region,
                    vault: p.vault,
                    organization: p.organization,
                    token_env: p.token_env,
                    project: p.project,
                    force_cli: p.force_cli,
                })
                .collect(),
            servers: raw
                .server
                .into_iter()
                .map(|(name, s)| ServerConnection {
                    name,
                    url: s.url,
                    ca_cert: s.ca_cert,
                    client_cert: s.client_cert,
                    client_key: s.client_key,
                })
                .collect(),
        })
    }

    /// Get the explicit ~/.config/jaws/jaws.hcl path (XDG-style, cross-platform)
    fn xdg_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".config/jaws/jaws.hcl"))
    }

    /// Get the list of config file search paths in priority order
    fn get_config_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. ./jaws.hcl (current directory - highest priority for project-local config)
        paths.push(PathBuf::from("jaws.hcl"));

        // 2. ~/.config/jaws/jaws.hcl (XDG-style, explicit cross-platform support)
        if let Some(xdg_path) = Self::xdg_config_path() {
            paths.push(xdg_path);
        }

        // 3. Platform-native config directory (~/Library/Application Support/ on macOS)
        // Skip if it's the same as the XDG path (e.g., on Linux where they're identical)
        if let Some(config_dir) = dirs::config_dir() {
            let native_path = config_dir.join("jaws/jaws.hcl");
            if Self::xdg_config_path().as_ref() != Some(&native_path) {
                paths.push(native_path);
            }
        }

        // 4. ~/.local/share/jaws/jaws.hcl (XDG data directory)
        if let Some(data_dir) = dirs::data_dir() {
            paths.push(data_dir.join("jaws/jaws.hcl"));
        }

        // 5. ~/jaws/jaws.hcl (home directory)
        if let Some(home_dir) = dirs::home_dir() {
            paths.push(home_dir.join("jaws/jaws.hcl"));
        }

        paths
    }

    /// Get config location options with human-readable descriptions for interactive selection
    /// Returns tuples of (PathBuf, description) - ordered with recommended first
    pub fn get_config_location_options() -> Vec<(PathBuf, &'static str)> {
        let mut options = Vec::new();

        // ~/.config/jaws/ (XDG-style, cross-platform - recommended)
        if let Some(xdg_path) = Self::xdg_config_path() {
            options.push((xdg_path, "~/.config/jaws/ (Recommended)"));
        }

        // Platform-native config directory (e.g., ~/Library/Application Support/ on macOS)
        // Skip if it's the same as the XDG path
        if let Some(config_dir) = dirs::config_dir() {
            let native_path = config_dir.join("jaws/jaws.hcl");
            if Self::xdg_config_path().as_ref() != Some(&native_path) {
                options.push((native_path, "Platform config directory"));
            }
        }

        // XDG data directory
        if let Some(data_dir) = dirs::data_dir() {
            options.push((data_dir.join("jaws/jaws.hcl"), "XDG data directory"));
        }

        // Home directory
        if let Some(home_dir) = dirs::home_dir() {
            options.push((home_dir.join("jaws/jaws.hcl"), "Home directory"));
        }

        // Current directory
        options.push((PathBuf::from("jaws.hcl"), "Current directory"));

        options
    }

    /// Find existing config file by searching all standard locations
    /// Returns the path to the first existing config file found, or None
    pub fn find_existing_config() -> Option<PathBuf> {
        Self::get_config_search_paths()
            .into_iter()
            .find(|path| path.exists())
    }

    /// Get the default config path (~/.config/jaws/jaws.hcl)
    pub fn default_config_path() -> PathBuf {
        Self::xdg_config_path().unwrap_or_else(|| PathBuf::from("jaws.hcl"))
    }

    /// Load configuration from a specific path
    fn load_from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Self::from_hcl(&content)
            .map_err(|e| format!("failed to parse {}: {}", path.display(), e).into())
    }

    /// Load configuration from jaws.hcl, searching multiple locations
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let search_paths = Self::get_config_search_paths();

        // Try each path in priority order
        for path in &search_paths {
            if path.exists() {
                return Self::load_from_path(path);
            }
        }

        // Return default config if no file found
        Ok(Config {
            defaults: None,
            providers: Vec::new(),
            servers: Vec::new(),
        })
    }

    /// Load configuration, optionally from a specific path
    /// If path is provided but doesn't exist, returns an error
    pub fn load_from(path: Option<&Path>) -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(p) = path {
            if !p.exists() {
                return Err(format!("Config file not found: {}", p.display()).into());
            }
            return Self::load_from_path(p);
        }
        if let Ok(env_path) = std::env::var("JAWS_CONFIG_PATH") {
            let p = std::path::PathBuf::from(env_path);
            if p.exists() {
                return Self::load_from_path(&p);
            }
        }
        Self::load()
    }

    /// Generate a config file with default values
    pub fn generate_config_file(
        path: Option<PathBuf>,
        overwrite: bool,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_path = path.unwrap_or_else(Self::default_config_path);

        // Check if file exists and overwrite flag
        if config_path.exists() && !overwrite {
            return Err(format!(
                "Config file already exists at: {}. Use --overwrite to replace it.",
                config_path.display()
            )
            .into());
        }

        // Create parent directories if they don't exist
        if let Some(parent) = config_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }

        let hcl_content = r#"# Global defaults
# cache_ttl is in seconds (default: 900 = 15 minutes)
# default_provider allows omitting the provider:// prefix in commands (e.g., jaws pull my-secret -p)
# keychain_cache caches decrypted credentials in the OS keychain (default: true)
defaults {
  editor       = "vim"
  secrets_path = "./.secrets"
  cache_ttl    = 900
  # default_provider = "jaws"
  # keychain_cache   = false
}

# Example AWS Provider
# provider "aws-dev" {
#   kind    = "aws"
#   profile = "default"
#   region  = "us-east-1"
# }

# Use profile "all" to auto-discover all AWS profiles from ~/.aws/credentials
# provider "aws" {
#   kind    = "aws"
#   profile = "all"
# }

# Example 1Password Provider
# provider "op-team" {
#   kind  = "onepassword"
#   vault = "Engineering"
# }

# Use vault "all" to auto-discover all 1Password vaults
# provider "op" {
#   kind  = "onepassword"
#   vault = "all"
# }

# Example GCP Secret Manager Provider
# Uses Application Default Credentials (gcloud auth application-default login)
# provider "gcp-prod" {
#   kind    = "gcp"
#   project = "my-gcp-project-id"
# }
"#;

        std::fs::write(&config_path, hcl_content)?;
        Ok(config_path)
    }

    /// Serialize config to HCL format
    pub fn to_hcl(&self) -> String {
        use hcl::{Block, Body};

        let mut body = Body::builder();

        if let Some(d) = &self.defaults {
            let mut b = Block::builder("defaults");
            if let Some(v) = &d.editor {
                b = b.add_attribute(("editor", v.as_str()));
            }
            if let Some(v) = &d.secrets_path {
                b = b.add_attribute(("secrets_path", v.as_str()));
            }
            if let Some(v) = d.cache_ttl {
                b = b.add_attribute(("cache_ttl", v));
            }
            if let Some(v) = &d.default_provider {
                b = b.add_attribute(("default_provider", v.as_str()));
            }
            if let Some(v) = d.max_versions {
                b = b.add_attribute(("max_versions", v as u64));
            }
            if let Some(v) = d.keychain_cache {
                b = b.add_attribute(("keychain_cache", v));
            }
            body = body.add_block(b.build());
        }

        for provider in &self.providers {
            let mut b = Block::builder("provider")
                .add_label(provider.id.as_str())
                .add_attribute(("kind", provider.kind.as_str()));
            if let Some(v) = &provider.profile {
                b = b.add_attribute(("profile", v.as_str()));
            }
            if let Some(v) = &provider.region {
                b = b.add_attribute(("region", v.as_str()));
            }
            if let Some(v) = &provider.vault {
                b = b.add_attribute(("vault", v.as_str()));
            }
            if let Some(v) = &provider.organization {
                b = b.add_attribute(("organization", v.as_str()));
            }
            if let Some(v) = &provider.token_env {
                b = b.add_attribute(("token_env", v.as_str()));
            }
            if let Some(v) = &provider.project {
                b = b.add_attribute(("project", v.as_str()));
            }
            if let Some(v) = provider.force_cli {
                b = b.add_attribute(("force_cli", v));
            }
            body = body.add_block(b.build());
        }

        for server in &self.servers {
            let mut b = Block::builder("server")
                .add_label(server.name.as_str())
                .add_attribute(("url", server.url.as_str()));
            if let Some(v) = &server.ca_cert {
                b = b.add_attribute(("ca_cert", v.as_str()));
            }
            if let Some(v) = &server.client_cert {
                b = b.add_attribute(("client_cert", v.as_str()));
            }
            if let Some(v) = &server.client_key {
                b = b.add_attribute(("client_key", v.as_str()));
            }
            body = body.add_block(b.build());
        }

        let rendered = hcl::to_string(&body.build())
            .expect("serializing config to HCL cannot fail");

        format!(
            "# jaws configuration file\n# cache_ttl is in seconds (default: 900 = 15 minutes)\n\n{rendered}"
        )
    }

    /// Save config to file
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(path, self.to_hcl())?;
        // Restrict config file permissions (may contain provider details)
        crate::utils::restrict_file_permissions(path)?;
        Ok(())
    }
}
