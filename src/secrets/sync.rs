//! Folder-first reconciliation between the user-editable working dir, SQLite,
//! and remote providers.
//!
//! Two commands, layered:
//! - [`save_one`] / [`save_all`]: **local only**. Hash each working file under
//!   `secrets/{provider}/{name}`, diff against the latest `downloads.file_hash`,
//!   and insert a new pending (`pushed_at = NULL`) row when content changed.
//!   The local `jaws` provider auto-stamps `pushed_at` since it has no remote.
//! - [`push_one`] / [`push_all`]: upload any rows with `pushed_at IS NULL` to
//!   their provider, stamping `pushed_at` on success. Runs [`save_all`] first
//!   so the DB is always current before the upload step.

use chrono::Utc;
use std::path::{Path, PathBuf};

use crate::db::{DbSecret, SecretInput, SecretRepository};
use crate::error::JawsError;
use crate::secrets::manager::SecretManager;
use crate::secrets::providers::JawsSecretManager;
use crate::secrets::storage::{
    compute_content_hash, hash_api_ref, read_working_file, scan_working_dir, working_file_path,
    write_secret_version,
};

/// Outcome of a local [`save_one`] step.
#[derive(Debug, Clone)]
pub enum SaveOutcome {
    Created {
        provider_id: String,
        name: String,
        version: i32,
    },
    Updated {
        provider_id: String,
        name: String,
        from_version: i32,
        to_version: i32,
    },
    Unchanged {
        provider_id: String,
        name: String,
    },
}

/// Outcome of a remote [`push_one`] step.
#[derive(Debug, Clone)]
pub enum PushOutcome {
    Pushed {
        provider_id: String,
        name: String,
        version: i32,
        api_ref: String,
    },
    UpToDate {
        provider_id: String,
        name: String,
    },
    Conflict {
        provider_id: String,
        name: String,
        reason: String,
    },
    Failed {
        provider_id: String,
        name: String,
        error: String,
    },
}

/// Entry in the "what's in my working dir vs DB" diff produced by [`status`].
#[derive(Debug, Clone)]
pub enum StatusEntry {
    /// File exists in working dir, no DB row.
    New { provider_id: String, name: String },
    /// File exists, DB row exists, hashes differ.
    Modified { provider_id: String, name: String },
    /// File exists, hash matches latest DB row.
    Unchanged { provider_id: String, name: String },
    /// DB row exists, working file missing.
    Orphan { provider_id: String, name: String },
    /// DB row exists with `pushed_at IS NULL`.
    Unpushed {
        provider_id: String,
        name: String,
        version: i32,
    },
}

// ----------------------------------------------------------------------------
// Save (local-only)
// ----------------------------------------------------------------------------

/// Reconcile one working file with the DB and `.versions/` archive. Pure local;
/// never calls a remote provider.
///
/// For the local `jaws` provider, new download rows are stamped `pushed_at = now`
/// (no remote exists). For every other provider, `pushed_at` is left NULL so
/// that a subsequent [`push_one`] picks them up.
pub fn save_one(
    repo: &SecretRepository,
    secrets_root: &Path,
    provider_id: &str,
    name: &str,
) -> Result<SaveOutcome, JawsError> {
    let content = read_working_file(secrets_root, provider_id, name)?;
    let working_hash = compute_content_hash(&content);

    let existing = repo.find_secret_by_provider_and_name(provider_id, name)?;
    let is_local_jaws = provider_id == "jaws";
    let pushed_stamp = if is_local_jaws { Some(Utc::now()) } else { None };

    match existing {
        None => {
            // No DB row — register the secret. For remote providers we use a
            // placeholder api_ref; the real one will be filled in by push_one
            // once the provider returns it.
            let placeholder_ref = if is_local_jaws {
                JawsSecretManager::generate_api_ref()
            } else {
                format!("pending://{}/{}", provider_id, name)
            };
            let hash = hash_api_ref(&placeholder_ref);
            let secret_id = repo.upsert_secret(&SecretInput {
                provider_id: provider_id.to_string(),
                api_ref: placeholder_ref,
                display_name: name.to_string(),
                hash,
                description: None,
                remote_updated_at: None,
            })?;
            // Mirror working file → archive v1.
            let (relpath, content_hash) =
                write_secret_version(secrets_root, provider_id, name, 1, &content)?;
            repo.create_download(secret_id, &relpath, &content_hash, pushed_stamp)?;
            repo.log_operation("save_create", provider_id, name, None)?;
            Ok(SaveOutcome::Created {
                provider_id: provider_id.to_string(),
                name: name.to_string(),
                version: 1,
            })
        }
        Some(secret) => {
            let latest = repo.get_latest_download(secret.id)?;
            if let Some(ref d) = latest
                && d.file_hash.as_deref() == Some(working_hash.as_str())
            {
                return Ok(SaveOutcome::Unchanged {
                    provider_id: provider_id.to_string(),
                    name: name.to_string(),
                });
            }
            let prev_version = latest.as_ref().map(|d| d.version).unwrap_or(0);
            let new_version = prev_version + 1;
            let (relpath, content_hash) =
                write_secret_version(secrets_root, provider_id, name, new_version, &content)?;
            repo.create_download(secret.id, &relpath, &content_hash, pushed_stamp)?;
            repo.log_operation(
                "save_update",
                provider_id,
                name,
                Some(&format!("{{\"version\": {}}}", new_version)),
            )?;
            Ok(SaveOutcome::Updated {
                provider_id: provider_id.to_string(),
                name: name.to_string(),
                from_version: prev_version,
                to_version: new_version,
            })
        }
    }
}

/// Reconcile every working file under `secrets_root/secrets/`. Returns one
/// [`SaveOutcome`] per file scanned.
pub fn save_all(
    repo: &SecretRepository,
    secrets_root: &Path,
) -> Result<Vec<SaveOutcome>, JawsError> {
    let mut out = Vec::new();
    for wf in scan_working_dir(secrets_root)? {
        match save_one(repo, secrets_root, &wf.provider_id, &wf.display_name) {
            Ok(o) => out.push(o),
            Err(e) => {
                eprintln!(
                    "Warning: failed to save {}://{}: {}",
                    wf.provider_id, wf.display_name, e
                );
            }
        }
    }
    Ok(out)
}

// ----------------------------------------------------------------------------
// Push (remote sync)
// ----------------------------------------------------------------------------

/// Push every unpushed download row for one secret to its provider. Returns the
/// most recent outcome (callers typically only show the final state per secret).
pub async fn push_one(
    provider: &dyn SecretManager,
    repo: &SecretRepository,
    secrets_root: &Path,
    secret: &DbSecret,
) -> PushOutcome {
    // Local jaws provider has no remote to push to.
    if provider.kind() == "jaws" {
        return PushOutcome::UpToDate {
            provider_id: secret.provider_id.clone(),
            name: secret.display_name.clone(),
        };
    }

    // Find the latest unpushed download.
    let unpushed = match repo.list_downloads(secret.id) {
        Ok(rows) => rows.into_iter().find(|d| d.pushed_at.is_none()),
        Err(e) => {
            return PushOutcome::Failed {
                provider_id: secret.provider_id.clone(),
                name: secret.display_name.clone(),
                error: e.to_string(),
            };
        }
    };
    let Some(download) = unpushed else {
        return PushOutcome::UpToDate {
            provider_id: secret.provider_id.clone(),
            name: secret.display_name.clone(),
        };
    };

    // Has the remote drifted since our last successful push?
    let placeholder = secret.api_ref.starts_with("pending://");
    if !placeholder {
        match provider.get_secret(&secret.api_ref).await {
            Ok(remote_content) => {
                let remote_hash = compute_content_hash(&remote_content);
                let last_pushed_hash = match repo.get_last_pushed_download(secret.id) {
                    Ok(Some(d)) => d.file_hash,
                    Ok(None) => None,
                    Err(e) => {
                        return PushOutcome::Failed {
                            provider_id: secret.provider_id.clone(),
                            name: secret.display_name.clone(),
                            error: e.to_string(),
                        };
                    }
                };
                if let Some(prev) = last_pushed_hash
                    && prev != remote_hash
                {
                    return PushOutcome::Conflict {
                        provider_id: secret.provider_id.clone(),
                        name: secret.display_name.clone(),
                        reason: format!(
                            "Remote content has changed since the last push (remote hash {} ≠ last pushed hash {}). \
                             Resolve with `jaws pull {}://{}` (overwrites your local copy with remote) \
                             or `jaws push --force {}://{}` (overwrites the remote).",
                            &remote_hash[..8.min(remote_hash.len())],
                            &prev[..8.min(prev.len())],
                            secret.provider_id, secret.display_name,
                            secret.provider_id, secret.display_name,
                        ),
                    };
                }
            }
            Err(e) => {
                // Not found is fine — we'll create. Anything else: surface it.
                let msg = e.to_string();
                if !msg.contains("ResourceNotFound") && !msg.contains("not found") {
                    return PushOutcome::Failed {
                        provider_id: secret.provider_id.clone(),
                        name: secret.display_name.clone(),
                        error: msg,
                    };
                }
            }
        }
    }

    // Read the version's archived content (canonical) — equivalent to the
    // working file for the latest version, but stable.
    let content = match crate::secrets::storage::load_secret_file(secrets_root, &download.filename)
    {
        Ok(c) => c,
        Err(_) => match read_working_file(secrets_root, &secret.provider_id, &secret.display_name) {
            Ok(c) => c,
            Err(e) => {
                return PushOutcome::Failed {
                    provider_id: secret.provider_id.clone(),
                    name: secret.display_name.clone(),
                    error: e.to_string(),
                };
            }
        },
    };

    // Either create (placeholder) or update.
    let result = if placeholder {
        match provider
            .create(&secret.display_name, &content, secret.description.as_deref())
            .await
        {
            Ok(new_ref) => {
                let new_hash = hash_api_ref(&new_ref);
                if let Err(e) = repo.update_api_ref(secret.id, &new_ref, &new_hash) {
                    return PushOutcome::Failed {
                        provider_id: secret.provider_id.clone(),
                        name: secret.display_name.clone(),
                        error: e.to_string(),
                    };
                }
                Ok(new_ref)
            }
            Err(e) => Err(e.to_string()),
        }
    } else {
        match provider.update(&secret.api_ref, &content).await {
            Ok(_) => Ok(secret.api_ref.clone()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("ResourceNotFound") || msg.contains("not found") {
                    // Remote was deleted out-of-band — fall back to create.
                    match provider
                        .create(&secret.display_name, &content, secret.description.as_deref())
                        .await
                    {
                        Ok(new_ref) => {
                            let new_hash = hash_api_ref(&new_ref);
                            let _ = repo.update_api_ref(secret.id, &new_ref, &new_hash);
                            Ok(new_ref)
                        }
                        Err(e2) => Err(e2.to_string()),
                    }
                } else {
                    Err(msg)
                }
            }
        }
    };

    match result {
        Ok(api_ref) => {
            if let Err(e) = repo.mark_pushed(download.id, Utc::now()) {
                return PushOutcome::Failed {
                    provider_id: secret.provider_id.clone(),
                    name: secret.display_name.clone(),
                    error: e.to_string(),
                };
            }
            let _ = repo.log_operation(
                "push",
                &secret.provider_id,
                &secret.display_name,
                Some(&format!("{{\"version\": {}}}", download.version)),
            );
            PushOutcome::Pushed {
                provider_id: secret.provider_id.clone(),
                name: secret.display_name.clone(),
                version: download.version,
                api_ref,
            }
        }
        Err(e) => PushOutcome::Failed {
            provider_id: secret.provider_id.clone(),
            name: secret.display_name.clone(),
            error: e,
        },
    }
}

/// Push every secret with at least one unpushed download row. Runs [`save_all`]
/// first to make sure all local edits are recorded before uploading.
pub async fn push_all(
    providers: &[crate::secrets::providers::Provider],
    repo: &SecretRepository,
    secrets_root: &Path,
    provider_filter: Option<&str>,
    name_filter: Option<&str>,
) -> Result<(Vec<SaveOutcome>, Vec<PushOutcome>), JawsError> {
    let saves = save_all(repo, secrets_root)?;
    let mut pushes = Vec::new();

    let unpushed = repo.list_unpushed_downloads(provider_filter, name_filter)?;
    // Deduplicate by secret_id — one push per secret reconciles all its rows
    // (we only push the latest unpushed; older unpushed rows are subsumed).
    let mut seen = std::collections::HashSet::new();
    for (secret, _download) in unpushed {
        if !seen.insert(secret.id) {
            continue;
        }
        let Some(provider) = providers.iter().find(|p| p.id() == secret.provider_id) else {
            pushes.push(PushOutcome::Failed {
                provider_id: secret.provider_id.clone(),
                name: secret.display_name.clone(),
                error: format!("Provider '{}' not configured", secret.provider_id),
            });
            continue;
        };
        let outcome = push_one(provider.as_ref(), repo, secrets_root, &secret).await;
        pushes.push(outcome);
    }
    Ok((saves, pushes))
}

// ----------------------------------------------------------------------------
// Status
// ----------------------------------------------------------------------------

/// Compare the working dir against the DB and return per-secret status entries
/// (new / modified / unchanged / orphan / unpushed).
pub fn status(
    repo: &SecretRepository,
    secrets_root: &Path,
) -> Result<Vec<StatusEntry>, JawsError> {
    let mut out = Vec::new();
    let working = scan_working_dir(secrets_root)?;

    // Track what we've seen so we can flag DB-only orphans afterwards.
    let mut seen_keys: std::collections::HashSet<(String, String)> = Default::default();

    for wf in working {
        seen_keys.insert((wf.provider_id.clone(), wf.display_name.clone()));
        let content = match read_working_file(secrets_root, &wf.provider_id, &wf.display_name) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let h = compute_content_hash(&content);
        let secret = repo.find_secret_by_provider_and_name(&wf.provider_id, &wf.display_name)?;
        match secret {
            None => out.push(StatusEntry::New {
                provider_id: wf.provider_id,
                name: wf.display_name,
            }),
            Some(s) => {
                let latest = repo.get_latest_download(s.id)?;
                let same = latest
                    .as_ref()
                    .and_then(|d| d.file_hash.as_deref())
                    .is_some_and(|stored| stored == h);
                if same {
                    out.push(StatusEntry::Unchanged {
                        provider_id: wf.provider_id,
                        name: wf.display_name,
                    });
                } else {
                    out.push(StatusEntry::Modified {
                        provider_id: wf.provider_id,
                        name: wf.display_name,
                    });
                }
            }
        }
    }

    // Orphans: DB rows whose working file does not exist.
    for s in repo.list_all_secrets(None)? {
        let key = (s.provider_id.clone(), s.display_name.clone());
        if seen_keys.contains(&key) {
            continue;
        }
        let working = working_file_path(secrets_root, &s.provider_id, &s.display_name);
        if !working.exists()
            && let Ok(Some(_)) = repo.get_latest_download(s.id)
        {
            out.push(StatusEntry::Orphan {
                provider_id: s.provider_id,
                name: s.display_name,
            });
        }
    }

    // Unpushed rows: anything with pushed_at IS NULL.
    for (s, d) in repo.list_unpushed_downloads(None, None)? {
        out.push(StatusEntry::Unpushed {
            provider_id: s.provider_id,
            name: s.display_name,
            version: d.version,
        });
    }

    Ok(out)
}

// ----------------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------------

/// Materialize a secret's value to its working file path. Returns the path.
pub fn write_working_only(
    secrets_root: &Path,
    provider_id: &str,
    name: &str,
    content: &str,
) -> Result<PathBuf, JawsError> {
    let path = working_file_path(secrets_root, provider_id, name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, content)?;
    crate::utils::restrict_file_permissions(&path)?;
    Ok(path)
}
