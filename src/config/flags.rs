use crate::cli::Cli;
use std::path::PathBuf;

pub struct Flags {
    pub provider: Option<String>,
    pub region: Option<String>,
    pub editor: Option<String>,
    pub secrets_path: Option<PathBuf>,
}

impl From<&Cli> for Flags {
    fn from(cli: &Cli) -> Self {
        Self {
            provider: cli.provider.clone(),
            region: cli.region.clone(),
            editor: cli.editor.clone(),
            secrets_path: cli.secrets_path.clone(),
        }
    }
}
