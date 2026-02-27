//! Command handlers for the jaws CLI.
//!
//! This module contains all the command handler functions, organized by command type.

mod clean;
mod config_cmd;
mod create;
mod default;
mod delete;
mod export_import;
mod history;
mod list;
mod log;
mod pull;
mod push;
mod rollback;
pub mod snapshot;
mod sync;

pub use clean::handle_clean;
pub use config_cmd::{
    handle_add_provider, handle_clear_cache, handle_interactive_generate, handle_remove_provider,
};
pub use create::handle_create;
pub use default::handle_default_command;
pub use delete::handle_delete;
pub use export_import::{handle_export, handle_import};
pub use history::handle_history;
pub use list::handle_list;
pub use log::handle_log;
pub use pull::{handle_pull, handle_pull_inject};
pub use push::handle_push;
pub use rollback::handle_rollback;
pub use snapshot::{
    check_and_snapshot, is_dirty, print_snapshot_summary, snapshot_all_modified, snapshot_secrets,
};
pub use sync::handle_sync;
