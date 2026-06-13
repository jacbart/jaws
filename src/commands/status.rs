//! Status command handler — git-like view of working dir vs SQLite.

use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::sync::{status, StatusEntry};

/// Handle `jaws status`. Read-only.
pub fn handle_status(
    config: &Config,
    repo: &SecretRepository,
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = status(repo, &config.secrets_path())?;

    let mut news = Vec::new();
    let mut modified = Vec::new();
    let mut orphans = Vec::new();
    let mut unpushed = Vec::new();
    let mut unchanged_count = 0;

    for e in entries {
        match e {
            StatusEntry::New { provider_id, name } => news.push(format!("{}://{}", provider_id, name)),
            StatusEntry::Modified { provider_id, name } => {
                modified.push(format!("{}://{}", provider_id, name))
            }
            StatusEntry::Orphan { provider_id, name } => {
                orphans.push(format!("{}://{}", provider_id, name))
            }
            StatusEntry::Unpushed {
                provider_id,
                name,
                version,
            } => unpushed.push(format!("{}://{} (v{})", provider_id, name, version)),
            StatusEntry::Unchanged { .. } => unchanged_count += 1,
        }
    }

    println!("Secrets directory: {}", config.secrets_path().display());
    println!();

    if !news.is_empty() {
        println!("New (not yet saved — run `jaws save`):");
        for s in &news {
            println!("  + {}", s);
        }
        println!();
    }
    if !modified.is_empty() {
        println!("Modified (working file differs from latest version):");
        for s in &modified {
            println!("  M {}", s);
        }
        println!();
    }
    if !unpushed.is_empty() {
        println!("Unpushed (saved locally, not yet uploaded — run `jaws push`):");
        for s in &unpushed {
            println!("  ↑ {}", s);
        }
        println!();
    }
    if !orphans.is_empty() {
        println!("Orphan (DB row exists, working file missing — `jaws pull` to restore):");
        for s in &orphans {
            println!("  ? {}", s);
        }
        println!();
    }
    if news.is_empty() && modified.is_empty() && unpushed.is_empty() && orphans.is_empty() {
        println!("Clean — {} secret(s) unchanged.", unchanged_count);
    } else {
        println!("({} unchanged)", unchanged_count);
    }

    Ok(())
}
