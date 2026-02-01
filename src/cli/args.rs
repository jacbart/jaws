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
    /// Path to config file (overrides default search paths)
    #[arg(short = 'c', long = "config", global = true, value_name = "PATH")]
    pub config_path: Option<PathBuf>,
}
