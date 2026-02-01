//! Editor utilities for editing secret values.

use std::io::Write;
use std::process::Command;

use crate::config::Config;

/// Open an editor to edit a secret value.
/// If initial_content is provided, it's pre-populated in the temp file.
/// Returns the content after the editor is closed.
pub fn edit_secret_value(
    config: &Config,
    initial_content: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create a temp file
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("jaws_secret_{}.txt", std::process::id()));

    // Write initial content if provided
    {
        let mut file = std::fs::File::create(&temp_path)?;
        if let Some(content) = initial_content {
            file.write_all(content.as_bytes())?;
        }
    }

    // Open editor
    let status = Command::new(config.editor()).arg(&temp_path).status()?;

    if !status.success() {
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);
        return Err("Editor exited with non-zero status".into());
    }

    // Read the result
    let content = std::fs::read_to_string(&temp_path)?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    Ok(content)
}
