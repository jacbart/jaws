//! Version control integration using jj-lib (Jujutsu).
//!
//! This module provides a wrapper around jj-lib to track secret file changes
//! with full version control capabilities including history, undo, and diff.

use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use jj_lib::config::StackedConfig;
use jj_lib::gitignore::GitIgnoreFile;
use jj_lib::matchers::EverythingMatcher;
use jj_lib::object_id::ObjectId as _;
use jj_lib::op_store::OperationId;
use jj_lib::operation::Operation;
use jj_lib::repo::StoreFactories;
use jj_lib::repo::{ReadonlyRepo, Repo as _};
use jj_lib::settings::UserSettings;
use jj_lib::working_copy::SnapshotOptions;
use jj_lib::workspace::{default_working_copy_factories, Workspace, WorkspaceLoadError};
use pollster::FutureExt as _;
use thiserror::Error;

/// Errors that can occur during VCS operations.
#[derive(Debug, Error)]
pub enum VcsError {
    #[error("Failed to initialize VCS repository: {0}")]
    Init(String),

    #[error("Failed to load VCS repository: {0}")]
    Load(String),

    #[error("Failed to commit changes: {0}")]
    Commit(String),

    #[error("Failed to get history: {0}")]
    History(String),

    #[error("Failed to restore: {0}")]
    Restore(String),

    #[error("Failed to undo: {0}")]
    Undo(String),

    #[error("Operation not found: {0}")]
    OperationNotFound(String),

    #[error("VCS error: {0}")]
    Other(String),
}

/// A single entry in the operation history.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// Short operation ID (first 12 chars)
    pub id_short: String,
    /// Full operation ID
    pub id_full: String,
    /// When the operation was performed
    pub timestamp: DateTime<Utc>,
    /// Description of the operation
    pub description: String,
    /// Username who performed the operation
    pub user: String,
    /// Hostname where operation was performed
    pub hostname: String,
}

/// Wrapper around jj-lib for secret version control.
pub struct SecretsVcs {
    workspace: Workspace,
    settings: UserSettings,
}

impl SecretsVcs {
    /// Check if a jj repository exists in the given path.
    pub fn exists(secrets_path: &Path) -> bool {
        secrets_path.join(".jj").is_dir()
    }

    /// Initialize a new jj repository in the secrets directory.
    pub fn init(secrets_path: &Path) -> Result<Self, VcsError> {
        let settings = create_user_settings()?;

        // Use SimpleBackend (pure jj, no git)
        let (workspace, _repo) = Workspace::init_simple(&settings, secrets_path)
            .map_err(|e| VcsError::Init(e.to_string()))?;

        Ok(Self {
            workspace,
            settings,
        })
    }

    /// Load an existing jj repository from the secrets directory.
    pub fn load(secrets_path: &Path) -> Result<Self, VcsError> {
        let settings = create_user_settings()?;
        let store_factories = StoreFactories::default();
        let working_copy_factories = default_working_copy_factories();

        let workspace = Workspace::load(
            &settings,
            secrets_path,
            &store_factories,
            &working_copy_factories,
        )
        .map_err(|e| match e {
            WorkspaceLoadError::NoWorkspaceHere(p) => {
                VcsError::Load(format!("No jj repository found at {}", p.display()))
            }
            _ => VcsError::Load(e.to_string()),
        })?;

        Ok(Self {
            workspace,
            settings,
        })
    }

    /// Get the secrets directory path.
    #[allow(dead_code)]
    pub fn workspace_root(&self) -> &Path {
        self.workspace.workspace_root()
    }

    /// Load the repository at HEAD.
    fn load_repo(&self) -> Result<Arc<ReadonlyRepo>, VcsError> {
        self.workspace
            .repo_loader()
            .load_at_head()
            .map_err(|e| VcsError::Other(e.to_string()))
    }

    /// Snapshot working copy changes and commit with a message.
    ///
    /// This captures any file changes in the working copy and records them
    /// as a new operation in the jj operation log.
    pub fn commit(&mut self, message: &str) -> Result<(), VcsError> {
        // First, snapshot the working copy to capture file changes
        self.snapshot_working_copy()?;

        // Load repo after snapshot
        let repo = self.load_repo()?;

        // Create a transaction to record this as an operation
        let tx = repo.start_transaction();

        // Commit the transaction with the description
        let _new_repo = tx
            .commit(message)
            .map_err(|e| VcsError::Commit(e.to_string()))?;

        Ok(())
    }

    /// Snapshot the working copy to detect and record file changes.
    fn snapshot_working_copy(&mut self) -> Result<(), VcsError> {
        let repo = self.load_repo()?;

        // Start a working copy mutation
        let mut locked_ws = self
            .workspace
            .start_working_copy_mutation()
            .map_err(|e| VcsError::Commit(e.to_string()))?;

        // Create snapshot options - track all files
        // Use empty gitignore - don't ignore anything in secrets directory
        let base_ignores = GitIgnoreFile::empty()
            .chain("", Path::new(".gitignore"), b"")
            .unwrap_or_else(|_| GitIgnoreFile::empty());
        let options = SnapshotOptions {
            base_ignores,
            progress: None,
            start_tracking_matcher: &EverythingMatcher,
            force_tracking_matcher: &EverythingMatcher,
            max_new_file_size: u64::MAX, // Allow any file size for secrets
        };

        // Perform the snapshot (async, use block_on)
        let _result = locked_ws
            .locked_wc()
            .snapshot(&options)
            .block_on()
            .map_err(|e| VcsError::Commit(format!("Failed to snapshot: {}", e)))?;

        // Finish the working copy mutation
        locked_ws
            .finish(repo.op_id().clone())
            .map_err(|e| VcsError::Commit(e.to_string()))?;

        Ok(())
    }

    /// Get the operation history (all operations, not just commits).
    pub fn history(&self) -> Result<Vec<HistoryEntry>, VcsError> {
        let repo = self.load_repo()?;
        let head_op = repo.operation();

        let mut entries = Vec::new();
        let mut current_op = Some(head_op.clone());

        while let Some(op) = current_op {
            let metadata = op.metadata();
            let id = op.id();

            // Convert timestamp from jj's TimestampRange
            let timestamp = DateTime::from_timestamp(metadata.time.start.timestamp.0, 0)
                .unwrap_or_else(Utc::now);

            entries.push(HistoryEntry {
                id_short: short_operation_id(id),
                id_full: id.hex(),
                timestamp,
                description: metadata.description.clone(),
                user: metadata.username.clone(),
                hostname: metadata.hostname.clone(),
            });

            // Get parent operation(s) - for simplicity, just follow first parent
            let parent_ids = op.parent_ids();
            if parent_ids.is_empty() {
                current_op = None;
            } else {
                current_op = self.load_operation(&parent_ids[0]).ok();
            }
        }

        Ok(entries)
    }

    /// Load an operation by its ID.
    fn load_operation(&self, op_id: &OperationId) -> Result<Operation, VcsError> {
        let repo = self.load_repo()?;
        let op_store = repo.op_store();
        let op_data = op_store
            .read_operation(op_id)
            .block_on()
            .map_err(|e| VcsError::History(e.to_string()))?;
        Ok(Operation::new(op_store.clone(), op_id.clone(), op_data))
    }

    /// Find an operation by short ID prefix.
    pub fn find_operation(&self, short_id: &str) -> Result<Operation, VcsError> {
        let history = self.history()?;

        let matches: Vec<_> = history
            .iter()
            .filter(|e| e.id_short.starts_with(short_id) || e.id_full.starts_with(short_id))
            .collect();

        match matches.len() {
            0 => Err(VcsError::OperationNotFound(short_id.to_string())),
            1 => {
                let op_id = OperationId::try_from_hex(&matches[0].id_full).ok_or_else(|| {
                    VcsError::Other(format!("Invalid operation ID: {}", &matches[0].id_full))
                })?;
                self.load_operation(&op_id)
            }
            _ => Err(VcsError::OperationNotFound(format!(
                "Ambiguous operation ID '{}' matches {} operations",
                short_id,
                matches.len()
            ))),
        }
    }

    /// Undo the last operation (restore to parent operation state).
    pub fn undo(&mut self) -> Result<String, VcsError> {
        let repo = self.load_repo()?;
        let head_op = repo.operation();

        let parent_ids = head_op.parent_ids();
        if parent_ids.is_empty() {
            return Err(VcsError::Undo("Cannot undo: at root operation".to_string()));
        }

        let parent_op = self.load_operation(&parent_ids[0])?;
        let undone_desc = head_op.metadata().description.clone();

        // Restore to parent operation
        self.restore_to_operation(&parent_op)?;

        Ok(undone_desc)
    }

    /// Restore the repository to a specific operation state.
    pub fn restore_to_operation(&mut self, target_op: &Operation) -> Result<(), VcsError> {
        let current_repo = self.load_repo()?;

        // Create a transaction that represents the restore
        let mut tx = current_repo.start_transaction();

        // Merge the target operation into current state
        tx.merge_operation(target_op.clone())
            .map_err(|e| VcsError::Restore(e.to_string()))?;

        // Commit with a description
        let desc = format!(
            "restore: to operation {}",
            short_operation_id(target_op.id())
        );
        tx.commit(&desc)
            .map_err(|e| VcsError::Restore(e.to_string()))?;

        // Update working copy to match the restored state
        let repo = self.load_repo()?;
        let wc_commit = repo
            .view()
            .get_wc_commit_id(self.workspace.workspace_name())
            .and_then(|id| repo.store().get_commit(id).ok());

        if let Some(commit) = wc_commit {
            let tree = commit.tree();

            self.workspace
                .check_out(repo.op_id().clone(), Some(&tree), &commit)
                .map_err(|e| VcsError::Restore(e.to_string()))?;
        }

        Ok(())
    }

    /// Restore to a specific operation by short ID.
    pub fn restore(&mut self, op_short_id: &str) -> Result<(), VcsError> {
        let target_op = self.find_operation(op_short_id)?;
        self.restore_to_operation(&target_op)
    }
}

/// Create UserSettings with inferred user identity.
fn create_user_settings() -> Result<UserSettings, VcsError> {
    // Start with jj-lib's default config (includes signing.behavior, etc.)
    let mut config = StackedConfig::with_defaults();

    // Try to infer user name and email
    let user_name = infer_user_name();
    let user_email = infer_user_email();
    let hostname = infer_hostname();

    // Override user-specific settings
    let user_toml = format!(
        r#"
[user]
name = "{user_name}"
email = "{user_email}"

[operation]
hostname = "{hostname}"
username = "{user_name}"
"#
    );

    // Parse the TOML and add it as a config layer
    let layer = jj_lib::config::ConfigLayer::parse(jj_lib::config::ConfigSource::User, &user_toml)
        .map_err(|e| VcsError::Init(format!("Failed to parse user config: {}", e)))?;

    config.add_layer(layer);

    UserSettings::from_config(config).map_err(|e| VcsError::Init(e.to_string()))
}

/// Infer the user's name from system/environment.
fn infer_user_name() -> String {
    // Try git config first
    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "--get", "user.name"])
        .output()
    {
        if output.status.success() {
            if let Ok(name) = String::from_utf8(output.stdout) {
                let name = name.trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }

    // Fall back to system username
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "jaws".to_string())
}

/// Infer the user's email from system/environment.
fn infer_user_email() -> String {
    // Try git config first
    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "--get", "user.email"])
        .output()
    {
        if output.status.success() {
            if let Ok(email) = String::from_utf8(output.stdout) {
                let email = email.trim();
                if !email.is_empty() {
                    return email.to_string();
                }
            }
        }
    }

    // Fall back to username@hostname
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "jaws".to_string());

    format!("{}@{}", user, infer_hostname())
}

/// Infer the hostname from system.
fn infer_hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "localhost".to_string())
}

/// Get a short version of an operation ID (first 12 hex chars).
fn short_operation_id(id: &OperationId) -> String {
    let hex = id.hex();
    hex.chars().take(12).collect()
}
