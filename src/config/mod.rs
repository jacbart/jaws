use knuffel::Decode;
use std::path::{Path, PathBuf};

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
    /// Load configuration from jaws.kdl
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = PathBuf::from("jaws.kdl");

        if !path.exists() {
            // Return default config if file doesn't exist
            return Ok(Config {
                defaults: None,
                providers: Vec::new(),
            });
        }

        let content = std::fs::read_to_string(&path)?;
        let config = knuffel::parse::<Config>("jaws.kdl", &content)?;

        Ok(config)
    }

    /// Get the editor, defaulting to EDITOR env var or "vi"
    pub fn editor(&self) -> String {
        self.defaults
            .as_ref()
            .and_then(|d| d.editor.clone())
            .unwrap_or_else(|| std::env::var("EDITOR").unwrap_or_else(|_| "vi".into()))
    }

    /// Get the secrets path, defaulting to "./.secrets"
    pub fn secrets_path(&self) -> PathBuf {
        self.defaults
            .as_ref()
            .and_then(|d| d.secrets_path.clone())
            .map(PathBuf::from)
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
        let config_path = path.unwrap_or_else(|| PathBuf::from("./jaws.kdl"));

        // Check if file exists and overwrite flag
        if config_path.exists() && !overwrite {
            return Err(format!(
                "Config file already exists at: {}. Use --overwrite to replace it.",
                config_path.display()
            )
            .into());
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
