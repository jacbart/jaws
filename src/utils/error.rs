//! Error formatting utilities.

/// Format an error in a user-friendly way.
/// Translates common AWS Secrets Manager errors into readable messages.
pub fn format_error(e: &dyn std::error::Error) -> String {
    let msg = e.to_string();

    // Check for common AWS Secrets Manager errors
    if msg.contains("ResourceNotFoundException") {
        return "Secret not found (may have been deleted)".to_string();
    }
    if msg.contains("AccessDeniedException") {
        return "Access denied (check IAM permissions)".to_string();
    }
    if msg.contains("InvalidParameterException") {
        return "Invalid parameter".to_string();
    }
    if msg.contains("InvalidRequestException") {
        return "Invalid request".to_string();
    }
    if msg.contains("DecryptionFailure") {
        return "Decryption failed (KMS key issue)".to_string();
    }
    if msg.contains("InternalServiceError") {
        return "AWS internal error (try again later)".to_string();
    }

    // Default: return the Display version (cleaner than Debug)
    msg
}
