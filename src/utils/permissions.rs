//! File permission utilities for restricting access to sensitive files.

use std::path::Path;

use crate::error::JawsError;

/// Set restrictive permissions (owner-only read/write) on a file.
///
/// On Unix systems this sets mode 0o600. On other platforms this is a no-op
/// since the permission model differs.
pub fn restrict_file_permissions(path: &Path) -> Result<(), JawsError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)
            .map_err(|e| format!("Failed to set permissions on {}: {}", path.display(), e))?;
    }

    #[cfg(not(unix))]
    {
        let _ = path; // suppress unused warning
    }

    Ok(())
}
