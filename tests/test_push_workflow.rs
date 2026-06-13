//! Save vs push split — verifies that `save` never touches a remote, and
//! `push` only uploads rows with `pushed_at IS NULL`. Uses an in-memory mock
//! provider so there's no network dependency.

use async_trait::async_trait;
use futures::stream::{self, Stream};
use jaws::config::{Config, Defaults};
use jaws::db::{DbProvider, SecretRepository};
use jaws::error::JawsError;
use jaws::secrets::manager::SecretManager;
use jaws::secrets::providers::Provider;
use jaws::secrets::storage::working_file_path;
use jaws::secrets::sync::{push_all, save_all, PushOutcome, SaveOutcome};
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone, Default)]
struct MockRemote {
    inner: Arc<Mutex<HashMap<String, String>>>,
    id: String,
    create_returns: Arc<Mutex<Option<String>>>,
}

impl MockRemote {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            inner: Arc::default(),
            create_returns: Arc::default(),
        }
    }
    fn snapshot(&self) -> HashMap<String, String> {
        self.inner.lock().unwrap().clone()
    }
    fn set_remote(&self, name: &str, value: &str) {
        self.inner.lock().unwrap().insert(name.to_string(), value.to_string());
    }
}

#[async_trait]
impl SecretManager for MockRemote {
    fn id(&self) -> &str {
        &self.id
    }
    fn kind(&self) -> &str {
        "mock"
    }
    async fn get_secret(&self, name: &str) -> Result<String, JawsError> {
        // Mimic AWS: resolve either a bare name or an ARN-like api_ref.
        let key = name.rsplit('/').next().unwrap_or(name).to_string();
        self.inner
            .lock()
            .unwrap()
            .get(&key)
            .cloned()
            .ok_or_else(|| JawsError::not_found(format!("not found: {}", key)))
    }
    async fn list_all(&self) -> Result<Vec<String>, JawsError> {
        Ok(self.inner.lock().unwrap().keys().cloned().collect())
    }
    fn list_secrets_stream(
        &self,
    ) -> Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send>>> + Send + Unpin>
    {
        let items: Vec<_> = self
            .inner
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .map(Ok::<_, Box<dyn std::error::Error + Send>>)
            .collect();
        Box::new(stream::iter(items))
    }
    async fn create(
        &self,
        name: &str,
        value: &str,
        _description: Option<&str>,
    ) -> Result<String, JawsError> {
        self.inner.lock().unwrap().insert(name.to_string(), value.to_string());
        let ref_to_return = self
            .create_returns
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| format!("mock://{}/{}", self.id, name));
        Ok(ref_to_return)
    }
    async fn update(&self, api_ref: &str, value: &str) -> Result<String, JawsError> {
        // api_ref for our mock = the secret name.
        let name = api_ref
            .rsplit('/')
            .next()
            .map(|s| s.to_string())
            .unwrap_or(api_ref.to_string());
        let exists = self.inner.lock().unwrap().contains_key(&name);
        if !exists {
            return Err(JawsError::not_found(format!("not found: {}", name)));
        }
        self.inner.lock().unwrap().insert(name.clone(), value.to_string());
        Ok(api_ref.to_string())
    }
    async fn delete(&self, _name: &str, _force: bool) -> Result<(), JawsError> {
        Ok(())
    }
    async fn rollback(&self, _: &str, _: Option<&str>) -> Result<String, JawsError> {
        Ok("ok".to_string())
    }
}

fn temp_root() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("jaws-push-test-{}", Uuid::new_v4()));
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
        id: "mock-prod".to_string(),
        kind: "mock".to_string(),
        last_sync_at: None,
        config_json: None,
    })
    .unwrap();
    (config, repo)
}

#[tokio::test]
async fn save_does_not_touch_remote_push_does() {
    let root = temp_root();
    let (config, repo) = boot(&root);

    let mock = MockRemote::new("mock-prod");
    let providers: Vec<Provider> = vec![Box::new(mock.clone())];

    // Drop a working file.
    let path = working_file_path(config.secrets_path().as_path(), "mock-prod", "api-key");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "first-content").unwrap();

    // Step 1: save_all — local only.
    let saves = save_all(&repo, &config.secrets_path()).unwrap();
    assert_eq!(saves.len(), 1);
    assert!(matches!(saves[0], SaveOutcome::Created { .. }));
    // Remote untouched.
    assert!(
        mock.snapshot().is_empty(),
        "save must NOT touch the remote backend"
    );

    // Step 2: push_all — uploads.
    let (_saves, pushes) = push_all(&providers, &repo, &config.secrets_path(), None, None)
        .await
        .unwrap();
    assert_eq!(pushes.len(), 1);
    assert!(matches!(pushes[0], PushOutcome::Pushed { .. }));
    // Remote now has it.
    assert_eq!(
        mock.snapshot().get("api-key").map(String::as_str),
        Some("first-content")
    );

    // pushed_at stamped.
    let secret = repo
        .find_secret_by_provider_and_name("mock-prod", "api-key")
        .unwrap()
        .unwrap();
    let latest = repo.get_latest_download(secret.id).unwrap().unwrap();
    assert!(latest.pushed_at.is_some());
    // api_ref replaced from placeholder to provider-assigned.
    assert!(secret.api_ref.starts_with("mock://"));

    // Step 3: edit + save → unpushed again. push again uploads.
    fs::write(&path, "second-content").unwrap();
    save_all(&repo, &config.secrets_path()).unwrap();
    assert_eq!(
        mock.snapshot().get("api-key").map(String::as_str),
        Some("first-content"),
        "save must not push edits"
    );
    let (_saves, pushes) = push_all(&providers, &repo, &config.secrets_path(), None, None)
        .await
        .unwrap();
    assert!(pushes.iter().any(|p| matches!(p, PushOutcome::Pushed { .. })));
    assert_eq!(
        mock.snapshot().get("api-key").map(String::as_str),
        Some("second-content")
    );

    fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn push_detects_remote_drift_conflict() {
    let root = temp_root();
    let (config, repo) = boot(&root);
    let mock = MockRemote::new("mock-prod");
    let providers: Vec<Provider> = vec![Box::new(mock.clone())];

    // Initial push: local "v1" → remote.
    let path = working_file_path(config.secrets_path().as_path(), "mock-prod", "drift");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "v1").unwrap();
    save_all(&repo, &config.secrets_path()).unwrap();
    push_all(&providers, &repo, &config.secrets_path(), None, None)
        .await
        .unwrap();

    // Local edit (creates an unpushed v2).
    fs::write(&path, "v2-local").unwrap();
    save_all(&repo, &config.secrets_path()).unwrap();

    // Someone else changed the remote out-of-band.
    mock.set_remote("drift", "v2-remote-other-team");

    let (_saves, pushes) = push_all(&providers, &repo, &config.secrets_path(), None, None)
        .await
        .unwrap();
    assert_eq!(pushes.len(), 1);
    match &pushes[0] {
        PushOutcome::Conflict { reason, .. } => {
            assert!(reason.contains("drifted") || reason.contains("changed"), "conflict reason should hint at remote drift: {}", reason);
        }
        other => panic!("expected Conflict, got {:?}", other),
    }
    // Remote should NOT have been overwritten.
    assert_eq!(
        mock.snapshot().get("drift").map(String::as_str),
        Some("v2-remote-other-team")
    );

    fs::remove_dir_all(&root).ok();
}
