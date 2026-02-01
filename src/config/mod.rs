use knuffel::Decode;
use std::path::{Path, PathBuf};

/// Expand tilde (~) prefix to the user's home directory.
/// Handles both "~" alone and "~/path/to/something" patterns.
fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    } else if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

#[derive(Debug, Decode, Clone)]
pub struct Config {
    #[knuffel(child)]
    pub defaults: Option<Defaults>,

    #[knuffel(children(name = "provider"))]
    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Decode, Clone, Default)]
pub struct Defaults {
    #[knuffel(property)]
    pub editor: Option<String>,

    #[knuffel(property(name = "secrets_path"))]
    pub secrets_path: Option<String>,

    #[knuffel(property(name = "cache_ttl"))]
    pub cache_ttl: Option<u64>,
}

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
    /// Get the explicit ~/.config/jaws/jaws.kdl path (XDG-style, cross-platform)
    fn xdg_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".config/jaws/jaws.kdl"))
    }

    /// Get the list of config file search paths in priority order
    fn get_config_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. ./jaws.kdl (current directory - highest priority for project-local config)
        paths.push(PathBuf::from("jaws.kdl"));

        // 2. ~/.config/jaws/jaws.kdl (XDG-style, explicit cross-platform support)
        if let Some(xdg_path) = Self::xdg_config_path() {
            paths.push(xdg_path);
        }

        // 3. Platform-native config directory (~/Library/Application Support/ on macOS)
        // Skip if it's the same as the XDG path (e.g., on Linux where they're identical)
        if let Some(config_dir) = dirs::config_dir() {
            let native_path = config_dir.join("jaws/jaws.kdl");
            if Self::xdg_config_path().as_ref() != Some(&native_path) {
                paths.push(native_path);
            }
        }

        // 4. ~/.local/share/jaws/jaws.kdl (XDG data directory)
        if let Some(data_dir) = dirs::data_dir() {
            paths.push(data_dir.join("jaws/jaws.kdl"));
        }

        // 5. ~/jaws/jaws.kdl (home directory)
        if let Some(home_dir) = dirs::home_dir() {
            paths.push(home_dir.join("jaws/jaws.kdl"));
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
            let native_path = config_dir.join("jaws/jaws.kdl");
            if Self::xdg_config_path().as_ref() != Some(&native_path) {
                options.push((native_path, "Platform config directory"));
            }
        }

        // XDG data directory
        if let Some(data_dir) = dirs::data_dir() {
            options.push((data_dir.join("jaws/jaws.kdl"), "XDG data directory"));
        }

        // Home directory
        if let Some(home_dir) = dirs::home_dir() {
            options.push((home_dir.join("jaws/jaws.kdl"), "Home directory"));
        }

        // Current directory
        options.push((PathBuf::from("jaws.kdl"), "Current directory"));

        options
    }

    /// Find existing config file by searching all standard locations
    /// Returns the path to the first existing config file found, or None
    pub fn find_existing_config() -> Option<PathBuf> {
        Self::get_config_search_paths()
            .into_iter()
            .find(|path| path.exists())
    }

    /// Get the default config path (~/.config/jaws/jaws.kdl)
    pub fn default_config_path() -> PathBuf {
        Self::xdg_config_path().unwrap_or_else(|| PathBuf::from("jaws.kdl"))
    }

    /// Load configuration from a specific path
    fn load_from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config = knuffel::parse::<Config>("jaws.kdl", &content)?;
        Ok(config)
    }

    /// Load configuration from jaws.kdl, searching multiple locations
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
        })
    }

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

    /// Get the path to the database file
    pub fn db_path(&self) -> PathBuf {
        self.secrets_path().join("jaws.db")
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
        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let kdl_content = r#"// Global defaults
// cache_ttl is in seconds (default: 900 = 15 minutes)
defaults editor="vim" secrets_path="./.secrets" cache_ttl=900

// Example AWS Provider
// provider "aws-dev" kind="aws" {
//     profile "default"
//     region "us-east-1"
// }

// Use profile "all" to auto-discover all AWS profiles from ~/.aws/credentials
// provider "aws" kind="aws" {
//     profile "all"
// }

// Example 1Password Provider
// provider "op-team" kind="onepassword" {
//     vault "Engineering"
// }

// Use vault "all" to auto-discover all 1Password vaults
// provider "op" kind="onepassword" {
//     vault "all"
// }
"#;

        std::fs::write(&config_path, kdl_content)?;
        Ok(config_path)
    }

    /// Serialize config to KDL format
    pub fn to_kdl(&self) -> String {
        let mut output = String::new();

        // Write header comment
        output.push_str("// jaws configuration file\n");
        output.push_str("// cache_ttl is in seconds (default: 900 = 15 minutes)\n\n");

        // Write defaults
        let defaults = self.defaults.as_ref();
        output.push_str("defaults");

        if let Some(d) = defaults {
            if let Some(editor) = &d.editor {
                output.push_str(&format!(" editor=\"{}\"", editor));
            }
            if let Some(secrets_path) = &d.secrets_path {
                output.push_str(&format!(" secrets_path=\"{}\"", secrets_path));
            }
            if let Some(cache_ttl) = d.cache_ttl {
                output.push_str(&format!(" cache_ttl={}", cache_ttl));
            }
        }
        output.push('\n');

        // Write providers
        for provider in &self.providers {
            output.push_str(&format!(
                "\nprovider \"{}\" kind=\"{}\" {{\n",
                provider.id, provider.kind
            ));

            if let Some(profile) = &provider.profile {
                output.push_str(&format!("    profile \"{}\"\n", profile));
            }
            if let Some(region) = &provider.region {
                output.push_str(&format!("    region \"{}\"\n", region));
            }
            if let Some(vault) = &provider.vault {
                output.push_str(&format!("    vault \"{}\"\n", vault));
            }
            if let Some(token_env) = &provider.token_env {
                output.push_str(&format!("    token_env \"{}\"\n", token_env));
            }

            output.push_str("}\n");
        }

        output
    }

    /// Save config to file
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(path, self.to_kdl())?;
        Ok(())
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
            _ => {
                return Err(format!(
                    "Unknown setting: {}. Valid settings: editor, secrets_path, cache_ttl",
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
            _ => Err(format!(
                "Unknown setting: {}. Valid settings: editor, secrets_path, cache_ttl",
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

    /// Discover AWS profiles from ~/.aws/credentials
    pub fn discover_aws_profiles() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let credentials_path = dirs::home_dir()
            .ok_or("Could not find home directory")?
            .join(".aws/credentials");

        if !credentials_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(credentials_path)?;
        let profiles: Vec<String> = content
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.starts_with('[') && line.ends_with(']') {
                    Some(line[1..line.len() - 1].to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(profiles)
    }

    /// Get AWS region for a profile from ~/.aws/config
    pub fn get_aws_profile_region(profile: &str) -> Option<String> {
        let config_path = dirs::home_dir()?.join(".aws/config");

        if !config_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(config_path).ok()?;
        let mut in_profile_section = false;
        let profile_header = if profile == "default" {
            "[default]".to_string()
        } else {
            format!("[profile {}]", profile)
        };

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_profile_section = line == profile_header;
            } else if in_profile_section && line.starts_with("region") {
                if let Some((_key, value)) = line.split_once('=') {
                    return Some(value.trim().to_string());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde_with_path() {
        let expanded = expand_tilde("~/some/path");
        // Should not start with ~ anymore
        assert!(!expanded.to_string_lossy().starts_with('~'));
        // Should end with the rest of the path
        assert!(expanded.to_string_lossy().ends_with("some/path"));
    }

    #[test]
    fn test_expand_tilde_alone() {
        let expanded = expand_tilde("~");
        // Should be the home directory (not ~)
        assert!(!expanded.to_string_lossy().starts_with('~'));
        // Should be an absolute path
        assert!(expanded.is_absolute());
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        // Paths without ~ should be unchanged
        let expanded = expand_tilde("/some/absolute/path");
        assert_eq!(expanded, PathBuf::from("/some/absolute/path"));

        let expanded = expand_tilde("relative/path");
        assert_eq!(expanded, PathBuf::from("relative/path"));

        let expanded = expand_tilde("./local/path");
        assert_eq!(expanded, PathBuf::from("./local/path"));
    }

    #[test]
    fn test_expand_tilde_not_at_start() {
        // ~ in the middle of a path should not be expanded
        let expanded = expand_tilde("/home/~user/path");
        assert_eq!(expanded, PathBuf::from("/home/~user/path"));
    }
}
