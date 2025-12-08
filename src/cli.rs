use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "jaws")]
#[command(about = "A CLI tool for managing secrets", long_about = None)]
pub struct Cli {
    /// AWS region
    #[arg(long, global = true)]
    pub region: Option<String>,

    /// Editor to use for opening secrets
    #[arg(long, global = true)]
    pub editor: Option<String>,

    /// Path where secrets will be downloaded
    #[arg(long, global = true)]
    pub secrets_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Pull secrets from the secrets manager
    Pull {
        /// Name of the secret to pull (optional - if not provided, opens TUI selector)
        secret_name: Option<String>,

        /// Open secrets in editor after downloading
        #[arg(short, long)]
        edit: bool,
    },
}
