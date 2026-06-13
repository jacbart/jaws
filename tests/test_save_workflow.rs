//! Folder-first workflow: edit a working file, `jaws save`, repeat — verifies
//! the local DB + `.versions/` archive stay in sync with what's on disk.

use jaws::config::{Config, Defaults};
use jaws::db::{DbProvider, SecretRepository};
use jaws::secrets::storage::{
    compute_content_hash, version_archive_path, working_file_path,
};
use jaws::secrets::sync::{save_all, save_one, SaveOutcome};
use std::fs;
use uuid::Uuid;

fn temp_root() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("jaws-save-test-{}", Uuid::new_v4()));
    fs::create_dir_all(&p).unwrap();
    p
}

fn boot(root: &std::path::Path) -> (Config, SecretRepository) {
    let config = Config {
        defaults: Some(Defaults {
            secrets_path: Some(root.to_string_lossy().to_string()),
            ..Default::default()
        }),
        providers: vec![],
        servers: vec![],
    };
    let conn = jaws::db::init_db(&root.join("jaws.db")).unwrap();
    let repo = SecretRepository::new(conn);
    repo.upsert_provider(&DbProvider {
        id: "jaws".to_string(),
        kind: "jaws".to_string(),
        last_sync_at: None,
        config_json: None,
    })
    .unwrap();
    (config, repo)
}

#[test]
fn drop_file_then_save_creates_secret() {
    let root = temp_root();
    let (config, repo) = boot(&root);

    // User drops a file in the working dir.
    let path = working_file_path(config.secrets_path().as_path(), "jaws", "db-password");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "first-value").unwrap();

    let outcome = save_one(&repo, &config.secrets_path(), "jaws", "db-password").unwrap();
    assert!(matches!(outcome, SaveOutcome::Created { version: 1, .. }));

    // DB row recorded.
    let secret = repo
        .find_secret_by_provider_and_name("jaws", "db-password")
        .unwrap()
        .expect("secrets row");
    let latest = repo
        .get_latest_download(secret.id)
        .unwrap()
        .expect("download row");
    assert_eq!(latest.version, 1);
    assert_eq!(latest.file_hash.as_deref(), Some(compute_content_hash("first-value").as_str()));
    // jaws provider auto-stamps pushed_at.
    assert!(latest.pushed_at.is_some());

    // Archive exists with the right content.
    let archive = version_archive_path(config.secrets_path().as_path(), "jaws", "db-password", 1);
    assert_eq!(fs::read_to_string(&archive).unwrap(), "first-value");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn edit_then_save_archives_old_keeps_new() {
    let root = temp_root();
    let (config, repo) = boot(&root);

    let path = working_file_path(config.secrets_path().as_path(), "jaws", "api-key");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "v1-content").unwrap();
    save_one(&repo, &config.secrets_path(), "jaws", "api-key").unwrap();

    // User edits.
    fs::write(&path, "v2-content").unwrap();

    let outcome = save_one(&repo, &config.secrets_path(), "jaws", "api-key").unwrap();
    match outcome {
        SaveOutcome::Updated { from_version, to_version, .. } => {
            assert_eq!(from_version, 1);
            assert_eq!(to_version, 2);
        }
        other => panic!("expected Updated, got {:?}", other),
    }

    // v1 archive preserved verbatim.
    let v1 = version_archive_path(config.secrets_path().as_path(), "jaws", "api-key", 1);
    assert_eq!(fs::read_to_string(&v1).unwrap(), "v1-content");
    // v2 archive matches user edit.
    let v2 = version_archive_path(config.secrets_path().as_path(), "jaws", "api-key", 2);
    assert_eq!(fs::read_to_string(&v2).unwrap(), "v2-content");
    // Working file still has user edit.
    assert_eq!(fs::read_to_string(&path).unwrap(), "v2-content");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn save_unchanged_is_noop() {
    let root = temp_root();
    let (config, repo) = boot(&root);

    let path = working_file_path(config.secrets_path().as_path(), "jaws", "stable");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "same").unwrap();
    save_one(&repo, &config.secrets_path(), "jaws", "stable").unwrap();

    let outcome = save_one(&repo, &config.secrets_path(), "jaws", "stable").unwrap();
    assert!(matches!(outcome, SaveOutcome::Unchanged { .. }));

    // Still exactly one download row.
    let secret = repo
        .find_secret_by_provider_and_name("jaws", "stable")
        .unwrap()
        .unwrap();
    let downloads = repo.list_downloads(secret.id).unwrap();
    assert_eq!(downloads.len(), 1);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn save_all_picks_up_many_files() {
    let root = temp_root();
    let (config, repo) = boot(&root);

    for name in ["alpha", "beta", "gamma"] {
        let p = working_file_path(config.secrets_path().as_path(), "jaws", name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, format!("content-{}", name)).unwrap();
    }

    let outcomes = save_all(&repo, &config.secrets_path()).unwrap();
    assert_eq!(outcomes.len(), 3);
    assert!(outcomes.iter().all(|o| matches!(o, SaveOutcome::Created { .. })));

    fs::remove_dir_all(&root).ok();
}

#[test]
fn remote_provider_save_leaves_pushed_at_null() {
    let root = temp_root();
    let (config, repo) = boot(&root);
    repo.upsert_provider(&DbProvider {
        id: "aws-prod".to_string(),
        kind: "aws".to_string(),
        last_sync_at: None,
        config_json: None,
    })
    .unwrap();

    let p = working_file_path(config.secrets_path().as_path(), "aws-prod", "remote-key");
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, "needs-pushing").unwrap();

    let outcome = save_one(&repo, &config.secrets_path(), "aws-prod", "remote-key").unwrap();
    assert!(matches!(outcome, SaveOutcome::Created { version: 1, .. }));

    let secret = repo
        .find_secret_by_provider_and_name("aws-prod", "remote-key")
        .unwrap()
        .unwrap();
    // Placeholder api_ref until push runs.
    assert!(secret.api_ref.starts_with("pending://"));
    let latest = repo.get_latest_download(secret.id).unwrap().unwrap();
    assert!(latest.pushed_at.is_none(), "remote-provider save must leave pushed_at NULL");

    // list_unpushed_downloads surfaces it.
    let unpushed = repo.list_unpushed_downloads(None, None).unwrap();
    assert!(unpushed.iter().any(|(s, _)| s.display_name == "remote-key"));

    fs::remove_dir_all(&root).ok();
}
