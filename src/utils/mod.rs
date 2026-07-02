//! Utility functions shared across the application.

mod editor;
mod parsing;
pub(crate) mod permissions;

pub use editor::edit_secret_value;
pub use parsing::parse_secret_ref;
pub use permissions::restrict_file_permissions;

/// Macro for debug-only output. Only prints when JAWS_DEBUG environment variable is set.
#[macro_export]
macro_rules! debug_eprintln {
    ($($arg:tt)*) => {
        if std::env::var("JAWS_DEBUG").is_ok() {
            eprintln!($($arg)*);
        }
    };
}
