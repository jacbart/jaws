use crate::cli::Cli;
use std::path::PathBuf;

pub struct Flags {
    pub region: Option<String>,
    pub editor: Option<String>,
    pub secrets_path: Option<PathBuf>,
}

impl From<&Cli> for Flags {
    fn from(cli: &Cli) -> Self {
        Self {
            region: cli.region.clone(),
            editor: cli.editor.clone(),
            secrets_path: cli.secrets_path.clone(),
        }
    }
}
