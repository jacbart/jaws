//! Command-line interface definitions.

mod args;
mod commands;

pub use args::Cli;
pub use commands::{Commands, ConfigCommands, DeleteScope};
