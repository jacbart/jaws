//! Configuration file loading and saving.

use std::path::{Path, PathBuf};

use super::types::Config;

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

        let kdl_content = r#"// Global defaults
// cache_ttl is in seconds (default: 900 = 15 minutes)
// default_provider allows omitting the provider:// prefix in commands (e.g., jaws pull my-secret -p)
defaults editor="vim" secrets_path="./.secrets" cache_ttl=900
// default_provider="jaws"

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
            if let Some(default_provider) = &d.default_provider {
                output.push_str(&format!(" default_provider=\"{}\"", default_provider));
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
}
