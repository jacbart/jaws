use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "jaws")]
#[command(about = "A CLI tool for managing secrets", long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Default, Args)]
pub struct ConfigArgs {
    /// Editor to use for opening secrets
    #[arg(long, global = true)]
    pub editor: Option<String>,

    /// Path where secrets will be downloaded
    #[arg(long, global = true)]
    pub secrets_path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Pull secrets from your secrets manager
    Pull {
        /// Name of the secret to pull (optional - if not provided, opens TUI selector)
        secret_name: Option<String>,

        /// Open secrets in editor after downloading
        #[arg(short, long)]
        edit: bool,
    },
    /// Push secrets to your secrets manager
    Push {
        /// Name of the secret to push (optional - if not provided, pushes all changed secrets)
        secret_name: Option<String>,

        /// Open secrets in editor before pushing
        #[arg(short, long)]
        edit: bool,
    },
    /// Delete a local secret and all its versions
    Delete {
        /// Name of the secret to delete (optional - if not provided, opens TUI selector)
        secret_name: Option<String>,
    },
    /// Refresh the local cache of remote secrets
    Sync,
    /// View version history of downloaded secrets
    History {
        /// Name of the secret to show history for (optional - if not provided, opens TUI selector)
        secret_name: Option<String>,

        /// Show full details including file hashes
        #[arg(short, long)]
        verbose: bool,

        /// Maximum number of versions to show (default: all)
        #[arg(short = 'n', long)]
        limit: Option<usize>,
    },
    /// Restore a previous version of a secret
    Restore {
        /// Name of the secret to restore
        secret_name: Option<String>,

        /// Version number to restore (optional - if not provided, shows version selector)
        #[arg(short, long)]
        version: Option<i32>,

        /// Open the restored secret in editor
        #[arg(short, long)]
        edit: bool,
    },
    /// Export and encrypt the secrets directory to a .barrel file
    Export {
        /// Encrypt to an SSH public key file instead of passphrase
        #[arg(short = 'K', long, value_name = "PATH")]
        ssh_key: Option<PathBuf>,

        /// Output path for the archive (default: ./jaws.barrel)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Delete secrets directory after successful export
        #[arg(long)]
        delete: bool,
    },
    /// Import and decrypt a .barrel archive
    Import {
        /// Path to the .barrel archive file
        archive: PathBuf,

        /// Decrypt with an SSH private key file instead of passphrase
        #[arg(short = 'K', long, value_name = "PATH")]
        ssh_key: Option<PathBuf>,

        /// Delete archive after successful import
        #[arg(long)]
        delete: bool,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Undo the last VCS operation
    Undo,
    /// Show operation log (jj-style history)
    Log {
        /// Maximum number of operations to show
        #[arg(short = 'n', long)]
        limit: Option<usize>,
    },
    /// Show diff between current state and a previous operation
    Diff {
        /// Operation ID to diff against (defaults to previous operation)
        #[arg(short, long)]
        operation: Option<String>,
    },
    /// Remote provider operations
    Remote {
        #[command(subcommand)]
        command: RemoteCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum RemoteCommands {
    /// Delete a secret from the provider
    Delete {
        /// Name of the secret to delete (optional - if not provided, opens TUI selector)
        secret_name: Option<String>,

        /// Force delete without recovery period
        #[arg(short, long)]
        force: bool,
    },
    /// Rollback a secret to a previous version on the provider
    Rollback {
        /// Name of the secret to rollback (optional - if not provided, opens TUI selector)
        secret_name: Option<String>,

        /// Version ID to rollback to (optional - if not provided, uses previous version)
        #[arg(long)]
        version_id: Option<String>,
    },
    /// View version history from the provider (not yet implemented)
    History {
        /// Name of the secret to show history for
        secret_name: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Generate a new config file
    Generate {
        /// Path where to create the config file (default: ./jaws.kdl)
        #[arg(long)]
        path: Option<std::path::PathBuf>,

        /// Overwrite existing config file if it exists
        #[arg(long)]
        overwrite: bool,

        /// Interactive mode - prompts for settings and discovers providers
        #[arg(short, long)]
        interactive: bool,
    },
    /// List all configuration settings
    List,
    /// Get a specific configuration value
    Get {
        /// Setting key (e.g., "editor", "secrets_path", "cache_ttl")
        key: String,
    },
    /// Set a configuration value
    Set {
        /// Setting key
        key: String,
        /// New value
        value: String,
    },
    /// List configured providers
    Providers,
}
