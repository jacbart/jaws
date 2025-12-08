use std::process::Command;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::config::Region;
use clap::Parser;

use cli::Cli;
use config::Config;
use secrets::{AwsSecretManager, SecretManager};

mod cli;
mod config;
mod secrets;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = Config::load_from_cli(&cli)?;

    let region_provider = RegionProviderChain::first_try(config.region.map(Region::new))
        .or_default_provider()
        .or_else(Region::new("us-west-2"));
    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let client = aws_sdk_secretsmanager::Client::new(&shared_config);

    let secret_manager = AwsSecretManager::new(client);

    match &cli.command {
        cli::Commands::Pull { secret_name, edit } => {
            let secrets_to_pull = if let Some(name) = secret_name {
                // Pull specific secret immediately
                vec![name.clone()]
            } else {
                // Open TUI selector
                secret_manager.select_secrets(None).await?
            };

            let mut files: Vec<String> = vec![];
            for secret in secrets_to_pull {
                let file_path = secret_manager
                    .download(secret.as_str(), config.secrets_path.clone())
                    .await?;
                println!("{secret} -> {file_path}");
                files.push(file_path);
            }

            // Open in editor if requested
            if *edit && !files.is_empty() {
                let _ = Command::new(&config.editor)
                    .args(&files)
                    .status()
                    .expect("failed to launch editor");
            }
        }
    }

    Ok(())
}
