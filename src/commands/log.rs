//! Log command handlers - viewing operation history.

use crate::db::SecretRepository;

/// Handle the log command - show all secret operations from DB
pub fn handle_log(
    repo: &SecretRepository,
    limit: Option<usize>,
    provider: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    use chrono_humanize::HumanTime;

    let operations = repo.list_operations(limit, provider.as_deref())?;

    if operations.is_empty() {
        println!("No operations recorded yet.");
        return Ok(());
    }

    println!("Operation log:");
    for op in operations {
        let age = HumanTime::from(op.created_at);
        let details = op.details.as_deref().unwrap_or("");
        println!(
            "  {} | {:8} | {:12} | {} {}",
            age, op.operation_type, op.provider_id, op.secret_name, details
        );
    }

    Ok(())
}
