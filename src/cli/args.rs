//! CLI argument parsing structures.

use clap::{Args, Parser};
use std::path::PathBuf;

use super::commands::Commands;

/// Main CLI structure for jaws.
#[derive(Parser, Debug)]
#[command(name = "jaws")]
#[command(about = "A CLI tool for managing secrets", long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Global configuration arguments available to all commands.
#[derive(Debug, Default, Args)]
pub struct ConfigArgs {
    /// Editor to use for opening secrets
    #[arg(long, global = true)]
    pub editor: Option<String>,

    /// Path where secrets will be downloaded
    #[arg(long, global = true)]
    pub secrets_path: Option<PathBuf>,
}
