//! Command and subcommand definitions.

use clap::{Subcommand, ValueEnum};
use std::path::PathBuf;

/// Scope for delete operations
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DeleteScope {
    /// Delete only local cached files
    Local,
    /// Delete only from the remote provider
    Remote,
    /// Delete from both local cache and remote provider
    Both,
}

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
        /// Name of the secret to push (optional - if not provided, shows TUI with modified secrets)
        secret_name: Option<String>,

        /// Open secrets in editor before pushing
        #[arg(short, long)]
        edit: bool,
    },
    /// Delete a secret (prompts for scope: local, remote, or both)
    Delete {
        /// Name of the secret to delete (optional - if not provided, opens TUI selector)
        secret_name: Option<String>,

        /// Delete scope: local, remote, or both (if not provided, prompts interactively)
        #[arg(short, long, value_enum)]
        scope: Option<DeleteScope>,

        /// Force delete without recovery period (for remote deletions)
        #[arg(short, long)]
        force: bool,
    },
    /// Refresh the local cache of remote secrets
    Sync,
    /// List all known secrets (one per line, for scripting)
    List {
        /// Filter by provider (e.g., "jaws", "aws-dev")
        #[arg(short, long)]
        provider: Option<String>,

        /// Show only locally downloaded secrets
        #[arg(short, long)]
        local: bool,
    },
    /// View version history (local and remote)
    History {
        /// Name of the secret to show history for (optional - if not provided, opens TUI selector)
        secret_name: Option<String>,

        /// Show full details including file hashes
        #[arg(short, long)]
        verbose: bool,

        /// Maximum number of versions to show (default: all)
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Show remote provider version history instead of local
        #[arg(short, long)]
        remote: bool,
    },
    /// Rollback a secret to a previous version (local or remote)
    Rollback {
        /// Name of the secret to rollback
        secret_name: Option<String>,

        /// Version number to rollback to for local rollback (optional - shows version selector)
        #[arg(short, long)]
        version: Option<i32>,

        /// Open the rolled back secret in editor (local rollback only)
        #[arg(short, long)]
        edit: bool,

        /// Rollback on the remote provider instead of locally
        #[arg(short, long)]
        remote: bool,

        /// Version ID for remote rollback (provider-specific, e.g., AWS version ID)
        #[arg(long)]
        version_id: Option<String>,
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
    /// Manage configuration (shows current config if no subcommand provided)
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommands>,
    },
    /// Create a new secret (uses default_provider from config, or prompts for provider)
    Create {
        /// Name for the secret (optional - if not provided, prompts interactively)
        name: Option<String>,

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
    /// Print version information
    Version,
}

/// Subcommands for configuration management.
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Initialize a new config file (interactive by default)
    Init {
        /// Path where to create the config file (default: ./jaws.kdl)
        #[arg(long)]
        path: Option<PathBuf>,

        /// Overwrite existing config file if it exists
        #[arg(long)]
        overwrite: bool,

        /// Generate minimal config without interactive prompts
        #[arg(short, long)]
        minimal: bool,
    },
    /// Get a specific configuration value
    Get {
        /// Setting key (e.g., "editor", "secrets_path", "cache_ttl")
        key: String,
    },
    /// Clear cached credentials from the OS keychain
    ClearCache,
    /// Set a configuration value
    Set {
        /// Setting key
        key: String,
        /// New value
        value: String,
    },
}
