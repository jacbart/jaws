use jaws::commands::handle_pull_inject;
use jaws::config::{Config, Defaults};
use jaws::db::SecretRepository;
use jaws::secrets::Provider;
use jaws::secrets::providers::JawsSecretManager;
use std::fs;
use uuid::Uuid;

#[tokio::test]
async fn test_handle_pull_inject_or_logic() {
    // 1. Setup temporary directory
    let temp_dir = std::env::temp_dir().join(Uuid::new_v4().to_string());
    let secrets_dir = temp_dir.join("secrets");
    fs::create_dir_all(&secrets_dir).unwrap();

    // 2. Setup Config
    let config = Config {
        defaults: Some(Defaults {
            secrets_path: Some(secrets_dir.to_string_lossy().to_string()),
            ..Default::default()
        }),
        providers: vec![],
    };

    // 3. Setup Repository (Real DB file)
    let db_path = secrets_dir.join("jaws.db");
    let conn = jaws::db::init_db(&db_path).unwrap();
    let repo = SecretRepository::new(conn);

    // 4. Setup Provider (Jaws)
    let jaws_manager = JawsSecretManager::new(secrets_dir.clone(), "jaws".to_string());
    let providers: Vec<Provider> = vec![Box::new(jaws_manager)];

    // Ensure provider exists in DB (to satisfy FKs)
    repo.upsert_provider(&jaws::db::DbProvider {
        id: "jaws".to_string(),
        kind: "jaws".to_string(),
        last_sync_at: None,
        config_json: Some("{}".to_string()),
    })
    .unwrap();

    // 5. Create some secrets
    let secret1_name = "secret1";
    let secret1_val = "value1";
    providers[0]
        .create(secret1_name, secret1_val, None)
        .await
        .unwrap();

    let secret2_name = "secret2";
    let secret2_val = "value2";
    providers[0]
        .create(secret2_name, secret2_val, None)
        .await
        .unwrap();

    // 6. Create Template File
    let template_path = temp_dir.join("template.txt");
    let output_path = temp_dir.join("output.txt");

    let template_content = r#"
    Direct: {{ jaws://secret1 }}
    Or First: {{ jaws://secret1 || jaws://secret2 }}
    Or Second: {{ jaws://missing || jaws://secret2 }}
    Or Fallback: {{ jaws://missing || 'fallback_value' }}
    Double Fallback: {{ jaws://missing1 || jaws://missing2 || "double_fallback" }}
    quoted: {{ 'quoted_val' }}
    "#;

    fs::write(&template_path, template_content).unwrap();

    // 7. Run Injection
    handle_pull_inject(
        &config,
        &repo,
        &providers,
        &template_path,
        Some(&output_path),
    )
    .await
    .unwrap();

    // 8. Verify Output
    let output = fs::read_to_string(&output_path).unwrap();

    assert!(output.contains("Direct: value1"));
    assert!(output.contains("Or First: value1"));
    assert!(output.contains("Or Second: value2"));
    assert!(output.contains("Or Fallback: fallback_value"));
    assert!(output.contains("Double Fallback: double_fallback"));
    assert!(output.contains("quoted: quoted_val"));

    // Cleanup
    fs::remove_dir_all(&temp_dir).unwrap();
}
