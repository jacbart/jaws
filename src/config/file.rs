use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileConfig {
    pub region: Option<String>,
    pub editor: Option<String>,
    pub secrets_path: Option<PathBuf>,
}

pub fn load_config_file() -> Result<Option<FileConfig>, Box<dyn std::error::Error>> {
    let mut config_paths = vec![
        PathBuf::from("./jaws.toml"),
        PathBuf::from("./.jaws.toml"),
    ];
    
    if let Some(home) = dirs::home_dir() {
        config_paths.push(home.join(".config/jaws.toml"));
    }

    for path in config_paths.iter() {
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            let config: FileConfig = toml::from_str(&contents)?;
            return Ok(Some(config));
        }
    }

    Ok(None)
}
