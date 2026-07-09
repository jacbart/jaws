//! Tests for HCL config parsing and serialization.

use jaws::config::{Config, Defaults, ProviderConfig, ServerConnection};

#[test]
fn test_full_coverage_parse() {
    let hcl = r#"
defaults {
  editor           = "hx"
  secrets_path     = "~/.secrets"
  cache_ttl        = 600
  default_provider = "op"
  max_versions     = 5
  keychain_cache   = false
}

provider "aws-dev" {
  kind    = "aws"
  profile = "default"
  region  = "us-east-1"
}

provider "op-team" {
  kind         = "onepassword"
  vault        = "Engineering"
  organization = "my-org"
  token_env    = "OP_TOKEN"
  project      = "proj-id"
  force_cli    = true
}

server "myserver" {
  url         = "https://10.0.0.5:9643"
  ca_cert     = "/certs/ca.pem"
  client_cert = "/certs/client.pem"
  client_key  = "/certs/client.key"
}
"#;

    let config = Config::from_hcl(hcl).expect("full config should parse");

    let d = config.defaults.as_ref().expect("defaults present");
    assert_eq!(d.editor.as_deref(), Some("hx"));
    assert_eq!(d.secrets_path.as_deref(), Some("~/.secrets"));
    assert_eq!(d.cache_ttl, Some(600));
    assert_eq!(d.default_provider.as_deref(), Some("op"));
    assert_eq!(d.max_versions, Some(5));
    assert_eq!(d.keychain_cache, Some(false));

    assert_eq!(config.providers.len(), 2);
    let aws = config
        .providers
        .iter()
        .find(|p| p.id == "aws-dev")
        .expect("aws-dev provider");
    assert_eq!(aws.kind, "aws");
    assert_eq!(aws.profile.as_deref(), Some("default"));
    assert_eq!(aws.region.as_deref(), Some("us-east-1"));

    let op = config
        .providers
        .iter()
        .find(|p| p.id == "op-team")
        .expect("op-team provider");
    assert_eq!(op.kind, "onepassword");
    assert_eq!(op.vault.as_deref(), Some("Engineering"));
    assert_eq!(op.organization.as_deref(), Some("my-org"));
    assert_eq!(op.token_env.as_deref(), Some("OP_TOKEN"));
    assert_eq!(op.project.as_deref(), Some("proj-id"));
    assert_eq!(op.force_cli, Some(true));

    assert_eq!(config.servers.len(), 1);
    let s = &config.servers[0];
    assert_eq!(s.name, "myserver");
    assert_eq!(s.url, "https://10.0.0.5:9643");
    assert_eq!(s.ca_cert.as_deref(), Some("/certs/ca.pem"));
    assert_eq!(s.client_cert.as_deref(), Some("/certs/client.pem"));
    assert_eq!(s.client_key.as_deref(), Some("/certs/client.key"));
}

#[test]
fn test_empty_config_parses() {
    let config = Config::from_hcl("").expect("empty config should parse");
    assert!(config.defaults.is_none());
    assert!(config.providers.is_empty());
    assert!(config.servers.is_empty());
}

#[test]
fn test_defaults_only() {
    let config = Config::from_hcl("defaults {\n  editor = \"vim\"\n}\n").unwrap();
    let d = config.defaults.expect("defaults present");
    assert_eq!(d.editor.as_deref(), Some("vim"));
    assert!(d.cache_ttl.is_none());
    assert!(config.providers.is_empty());
}

#[test]
fn test_round_trip_all_fields() {
    let original = Config {
        defaults: Some(Defaults {
            editor: Some("hx".into()),
            secrets_path: Some("./.secrets".into()),
            cache_ttl: Some(900),
            default_provider: Some("op".into()),
            // Regression: the old KDL serializer silently dropped these two.
            max_versions: Some(7),
            keychain_cache: Some(false),
        }),
        providers: vec![ProviderConfig {
            id: "op-service-account".into(),
            kind: "onepassword".into(),
            profile: None,
            region: None,
            vault: Some("vault-id".into()),
            organization: Some("org-id".into()),
            token_env: Some("OP_TOKEN".into()),
            project: None,
            force_cli: Some(true),
        }],
        servers: vec![ServerConnection {
            name: "remote".into(),
            url: "https://example.com:9643".into(),
            ca_cert: Some("/ca.pem".into()),
            client_cert: Some("/cert.pem".into()),
            client_key: Some("/key.pem".into()),
        }],
    };

    let rendered = original.to_hcl();
    let reparsed = Config::from_hcl(&rendered).expect("to_hcl output should reparse");

    let d = reparsed.defaults.expect("defaults survive round-trip");
    assert_eq!(d.editor.as_deref(), Some("hx"));
    assert_eq!(d.secrets_path.as_deref(), Some("./.secrets"));
    assert_eq!(d.cache_ttl, Some(900));
    assert_eq!(d.default_provider.as_deref(), Some("op"));
    assert_eq!(d.max_versions, Some(7));
    assert_eq!(d.keychain_cache, Some(false));

    assert_eq!(reparsed.providers.len(), 1);
    let p = &reparsed.providers[0];
    assert_eq!(p.id, "op-service-account");
    assert_eq!(p.kind, "onepassword");
    assert_eq!(p.vault.as_deref(), Some("vault-id"));
    assert_eq!(p.organization.as_deref(), Some("org-id"));
    assert_eq!(p.token_env.as_deref(), Some("OP_TOKEN"));
    assert_eq!(p.force_cli, Some(true));

    assert_eq!(reparsed.servers.len(), 1);
    let s = &reparsed.servers[0];
    assert_eq!(s.name, "remote");
    assert_eq!(s.url, "https://example.com:9643");
    assert_eq!(s.ca_cert.as_deref(), Some("/ca.pem"));
    assert_eq!(s.client_cert.as_deref(), Some("/cert.pem"));
    assert_eq!(s.client_key.as_deref(), Some("/key.pem"));
}

#[test]
fn test_round_trip_escaping() {
    // Regression: the old KDL serializer did no escaping at all.
    let original = Config {
        defaults: Some(Defaults {
            editor: Some(r#"vi "m" \ edit"#.into()),
            ..Defaults::default()
        }),
        providers: vec![],
        servers: vec![],
    };

    let rendered = original.to_hcl();
    let reparsed = Config::from_hcl(&rendered).expect("escaped value should reparse");
    assert_eq!(
        reparsed.defaults.unwrap().editor.as_deref(),
        Some(r#"vi "m" \ edit"#)
    );
}

#[test]
fn test_missing_kind_errors() {
    let hcl = "provider \"x\" {\n  vault = \"v\"\n}\n";
    assert!(Config::from_hcl(hcl).is_err());
}

#[test]
fn test_missing_url_errors() {
    let hcl = "server \"x\" {\n  ca_cert = \"/ca.pem\"\n}\n";
    assert!(Config::from_hcl(hcl).is_err());
}

#[test]
fn test_wrong_type_errors() {
    let hcl = "defaults {\n  cache_ttl = \"abc\"\n}\n";
    assert!(Config::from_hcl(hcl).is_err());
}

#[test]
fn test_unknown_field_errors() {
    // Typos must hard-error, not be silently ignored (secrets tool).
    let hcl = "defaults {\n  secert_path = \"./.secrets\"\n}\n";
    assert!(Config::from_hcl(hcl).is_err());
}

#[test]
fn test_provider_order_preserved() {
    let hcl = r#"
provider "zeta" {
  kind = "aws"
}
provider "alpha" {
  kind = "gcp"
}
"#;
    let config = Config::from_hcl(hcl).unwrap();
    let ids: Vec<&str> = config.providers.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["zeta", "alpha"]);
}
