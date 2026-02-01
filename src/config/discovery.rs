//! AWS profile and provider discovery utilities.

use super::types::Config;

impl Config {
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
            } else if in_profile_section && line.starts_with("region")
                && let Some((_key, value)) = line.split_once('=')
            {
                return Some(value.trim().to_string());
            }
        }

        None
    }
}
