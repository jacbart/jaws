//! Command and subcommand definitions.

use clap::Subcommand;
use std::path::PathBuf;

/// Top-level commands available in jaws.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Pull secrets from your secrets manager
    Pull {
        /// Secret reference: PROVIDER://SECRET_NAME (e.g., jaws://my-secret, aws-dev://db-pass)
        /// If default_provider is set in config, the prefix can be omitted.
        /// If not provided, opens TUI selector.
        secret_name: Option<String>,

        /// Open secrets in editor after downloading
        #[arg(short, long)]
        edit: bool,

        /// Print secret value to stdout (for use in scripts). Requires secret_name.
        #[arg(short, long)]
        print: bool,

        /// Inject secrets into a template file. Replaces {{PROVIDER://SECRET}} patterns.
        #[arg(short, long, value_name = "FILE")]
        inject: Option<PathBuf>,

        /// Output file for inject mode (default: stdout)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
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
    /// List all known secrets (one per line, for scripting)
    List {
        /// Filter by provider (e.g., "jaws", "aws-dev")
        #[arg(short, long)]
        provider: Option<String>,
    },
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
    /// Rollback a secret to a previous version
    Rollback {
        /// Name of the secret to rollback
        secret_name: Option<String>,

        /// Version number to rollback to (optional - if not provided, shows version selector)
        #[arg(short, long)]
        version: Option<i32>,

        /// Open the rolled back secret in editor
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
    /// Create a new secret (defaults to local jaws provider if not specified)
    Create {
        /// Name for the secret (e.g. "my-secret" or "aws://my-secret")
        name: String,

        /// Optional description
        #[arg(short, long)]
        description: Option<String>,

        /// Read value from file instead of editor
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Show operation log (all secret operations)
    Log {
        /// Maximum number of operations to show
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Filter by provider
        #[arg(short, long)]
        provider: Option<String>,
    },
    /// Remote provider operations
    Remote {
        #[command(subcommand)]
        command: RemoteCommands,
    },
    /// Clear local cache and secrets
    Clean {
        /// Delete without confirmation (dangerous for local jaws secrets)
        #[arg(short, long)]
        force: bool,

        /// Show what would be deleted without actually deleting
        #[arg(long)]
        dry_run: bool,

        /// Keep local jaws secrets, only delete cached remote secrets
        #[arg(long)]
        keep_local: bool,
    },
}

/// Subcommands for remote provider operations.
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

/// Subcommands for configuration management.
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
