//! Parsing utilities for secret references.

use crate::error::JawsError;

/// Parse a secret reference like "jaws://my-secret" or "aws-dev://path/to/secret"
/// Returns (provider_id, secret_name)
///
/// # Examples
/// ```
/// use jaws::utils::parse_secret_ref;
///
/// let (provider, name) = parse_secret_ref("jaws://my-secret", None).unwrap();
/// assert_eq!(provider, "jaws");
/// assert_eq!(name, "my-secret");
/// ```
pub fn parse_secret_ref(
    input: &str,
    default_provider: Option<&str>,
) -> Result<(String, String), JawsError> {
    if let Some((provider, name)) = input.split_once("://") {
        if name.is_empty() {
            return Err(JawsError::validation(format!(
                "Invalid secret reference '{}': secret name cannot be empty",
                input
            )));
        }
        Ok((provider.to_string(), name.to_string()))
    } else if let Some(default) = default_provider {
        Ok((default.to_string(), input.to_string()))
    } else {
        Err(JawsError::validation(format!(
            "Invalid secret reference '{}'. Use format: PROVIDER://SECRET_NAME (e.g., jaws://my-secret)\n\
             Or set default_provider in config to omit the prefix.",
            input
        )))
    }
}
