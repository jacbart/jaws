//! Save command handler — reconcile working files with the local DB and
//! `.versions/` archive. Never touches a remote provider.

use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::sync::{save_all, save_one, SaveOutcome};
use crate::utils::parse_secret_ref;

/// Handle `jaws save`.
///
/// - No args: scan the entire working dir.
/// - `provider://name` or bare `name` (with default_provider): save just that one.
pub fn handle_save(
    config: &Config,
    repo: &SecretRepository,
    secret_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let outcomes = match secret_name {
        None => save_all(repo, &config.secrets_path())?,
        Some(s) => {
            let (provider_id, name) = parse_secret_ref(&s, config.default_provider().as_deref())?;
            vec![save_one(repo, &config.secrets_path(), &provider_id, &name)?]
        }
    };

    print_outcomes(&outcomes);
    Ok(())
}

fn print_outcomes(outcomes: &[SaveOutcome]) {
    let mut created = 0;
    let mut updated = 0;
    let mut unchanged = 0;
    for o in outcomes {
        match o {
            SaveOutcome::Created { provider_id, name, version } => {
                println!("  created {}://{} v{}", provider_id, name, version);
                created += 1;
            }
            SaveOutcome::Updated {
                provider_id,
                name,
                from_version,
                to_version,
            } => {
                println!(
                    "  updated {}://{} v{} → v{}",
                    provider_id, name, from_version, to_version
                );
                updated += 1;
            }
            SaveOutcome::Unchanged { provider_id, name } => {
                println!("  unchanged {}://{}", provider_id, name);
                unchanged += 1;
            }
        }
    }
    if outcomes.is_empty() {
        println!("No secrets found under secrets/. Create one with `mkdir -p .secrets/secrets/jaws && $EDITOR .secrets/secrets/jaws/my-secret`, then run `jaws save`.");
    } else {
        println!(
            "\nSave complete: {} created, {} updated, {} unchanged",
            created, updated, unchanged
        );
    }
}
