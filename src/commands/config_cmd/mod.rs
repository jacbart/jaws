//! Config command handlers - managing configuration.
//!
//! Organized into submodules:
//! - `helpers`: Shared prompt/confirm functions and credential storage
//! - `discovery`: Provider auto-discovery (AWS, 1Password, Bitwarden)
//! - `handlers`: Public command handlers (init, add-provider, remove-provider, clear-cache)

mod discovery;
mod handlers;
mod helpers;

pub use handlers::{
    handle_add_provider, handle_clear_cache, handle_interactive_generate, handle_remove_provider,
};
