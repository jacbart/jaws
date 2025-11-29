use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "jaws")]
pub struct Flags {
    /// AWS region
    #[arg(long)]
    pub region: Option<String>,

    /// Editor to use for opening secrets
    #[arg(long)]
    pub editor: Option<String>,

    /// Path where secrets will be downloaded
    #[arg(long)]
    pub secrets_path: Option<PathBuf>,
}

pub fn parse_flags() -> Flags {
    Flags::parse()
}
