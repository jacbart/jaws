//! Push command handler — runs `save_all` then uploads every unpushed download
//! row to its remote provider.

use std::process::Command;

use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::storage::working_file_path;
use crate::secrets::sync::{push_all, save_all, PushOutcome, SaveOutcome};
use crate::secrets::Provider;
use crate::utils::parse_secret_ref;

/// Handle the push command.
///
/// `secret_name` may be a `provider://name` reference (filters to that secret)
/// or a substring match against unpushed secrets. `edit` opens the user's
/// editor against the matching working files first.
pub async fn handle_push(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    secret_name: Option<String>,
    edit: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: optionally open editor on the matching working file(s) so the
    // user can edit before save+push runs.
    if edit {
        let files = collect_editor_targets(config, repo, secret_name.as_deref())?;
        if files.is_empty() {
            eprintln!("Nothing to edit — no unpushed local secrets matched.");
        } else {
            Command::new(config.editor())
                .args(&files)
                .status()
                .map_err(|e| {
                    format!(
                        "Failed to launch editor '{}': {}. Set a valid editor with 'jaws config set editor <path>'.",
                        config.editor(), e
                    )
                })?;
        }
    }

    // Step 2: local save (records any edits in DB + .versions/).
    let saves = save_all(repo, &config.secrets_path())?;
    print_save_summary(&saves);

    // Step 3: filter by name/provider if requested and push.
    let (provider_filter, name_filter) = parse_filters(secret_name.as_deref(), config);

    let (_saves, pushes) = push_all(
        providers,
        repo,
        &config.secrets_path(),
        provider_filter.as_deref(),
        name_filter.as_deref(),
    )
    .await?;

    print_push_summary(&pushes);
    Ok(())
}

fn parse_filters(name: Option<&str>, config: &Config) -> (Option<String>, Option<String>) {
    match name {
        None => (None, None),
        Some(s) => match parse_secret_ref(s, config.default_provider().as_deref()) {
            Ok((p, n)) => (Some(p), Some(n)),
            Err(_) => (None, Some(s.to_string())),
        },
    }
}

fn collect_editor_targets(
    config: &Config,
    repo: &SecretRepository,
    name: Option<&str>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let (pf, nf) = parse_filters(name, config);
    let unpushed = repo.list_unpushed_downloads(pf.as_deref(), nf.as_deref())?;
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (secret, _) in unpushed {
        if !seen.insert(secret.id) {
            continue;
        }
        let p = working_file_path(&config.secrets_path(), &secret.provider_id, &secret.display_name);
        out.push(p.to_string_lossy().to_string());
    }
    Ok(out)
}

fn print_save_summary(saves: &[SaveOutcome]) {
    let mut new_count = 0;
    let mut upd_count = 0;
    for s in saves {
        match s {
            SaveOutcome::Created { provider_id, name, version } => {
                println!("  saved {}://{} v{} (new)", provider_id, name, version);
                new_count += 1;
            }
            SaveOutcome::Updated {
                provider_id,
                name,
                from_version,
                to_version,
            } => {
                println!(
                    "  saved {}://{} v{} → v{}",
                    provider_id, name, from_version, to_version
                );
                upd_count += 1;
            }
            SaveOutcome::Unchanged { .. } => {}
        }
    }
    if new_count == 0 && upd_count == 0 && !saves.is_empty() {
        // All unchanged — quiet.
    } else if new_count + upd_count > 0 {
        println!();
    }
}

fn print_push_summary(pushes: &[PushOutcome]) {
    if pushes.is_empty() {
        println!("Nothing to push — all local edits already synced.");
        return;
    }
    let mut pushed = 0;
    let mut conflicts = 0;
    let mut failed = 0;
    for p in pushes {
        match p {
            PushOutcome::Pushed { provider_id, name, version, .. } => {
                println!("  pushed {}://{} v{} -> remote", provider_id, name, version);
                pushed += 1;
            }
            PushOutcome::UpToDate { .. } => {}
            PushOutcome::Conflict { provider_id, name, reason } => {
                eprintln!("  CONFLICT {}://{}: {}", provider_id, name, reason);
                conflicts += 1;
            }
            PushOutcome::Failed { provider_id, name, error } => {
                eprintln!("  FAILED {}://{}: {}", provider_id, name, error);
                failed += 1;
            }
        }
    }
    println!(
        "\nPush complete: {} pushed, {} conflicts, {} failed",
        pushed, conflicts, failed
    );
}
