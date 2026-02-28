//! Editor utilities for editing secret values.

use std::io::Write;
use std::process::Command;

use super::permissions::restrict_file_permissions;
use crate::config::Config;

/// Open an editor to edit a secret value.
/// If initial_content is provided, it's pre-populated in the temp file.
/// Returns the content after the editor is closed.
pub fn edit_secret_value(
    config: &Config,
    initial_content: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create a temp file with a random suffix to avoid predictable names
    let temp_dir = std::env::temp_dir();
    let random_suffix: u64 = std::hash::Hasher::finish(&std::hash::BuildHasher::build_hasher(
        &std::collections::hash_map::RandomState::new(),
    ));
    let temp_path = temp_dir.join(format!(".jaws_{}_{}", std::process::id(), random_suffix));

    // Write initial content if provided, with restrictive permissions
    {
        let mut file = std::fs::File::create(&temp_path)?;
        if let Some(content) = initial_content {
            file.write_all(content.as_bytes())?;
        }
    }
    // Set restrictive permissions before opening editor (contains secret data)
    restrict_file_permissions(&temp_path)?;

    // Open editor
    let status = Command::new(config.editor())
        .arg(&temp_path)
        .status()
        .map_err(|e| {
            // Clean up temp file on editor launch failure
            let _ = std::fs::remove_file(&temp_path);
            format!(
                "Failed to launch editor '{}': {}. Set a valid editor with 'jaws config set editor <path>'.",
                config.editor(), e
            )
        })?;

    if !status.success() {
        let _ = std::fs::remove_file(&temp_path);
        return Err("Editor exited with non-zero status".into());
    }

    // Read the result
    let content = std::fs::read_to_string(&temp_path)?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    Ok(content)
}
