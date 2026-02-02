//! Remote command handlers - operations on remote providers.

use crate::cli::RemoteCommands;
use crate::config::Config;
use crate::secrets::Provider;

use super::delete::handle_remote_delete;
use super::rollback::handle_remote_rollback;

/// Handle remote subcommands
pub async fn handle_remote(
    config: &Config,
    providers: &[Provider],
    command: RemoteCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        RemoteCommands::Delete { secret_name, force } => {
            handle_remote_delete(config, providers, secret_name, force).await?;
        }
        RemoteCommands::Rollback {
            secret_name,
            version_id,
        } => {
            handle_remote_rollback(config, providers, secret_name, version_id).await?;
        }
        RemoteCommands::History { secret_name: _ } => {
            handle_remote_history().await?;
        }
    }
    Ok(())
}

/// Handle the remote history command - placeholder
pub async fn handle_remote_history() -> Result<(), Box<dyn std::error::Error>> {
    println!("Remote history is not yet implemented.");
    println!("Use 'jaws history' to view local version history.");
    Ok(())
}
