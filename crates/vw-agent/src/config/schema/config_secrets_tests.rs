use super::config_secrets::{
    decrypt_config_secrets, decrypt_map_secrets, decrypt_vec_secrets, encrypt_config_secrets,
    encrypt_map_secrets, encrypt_vec_secrets,
};
use super::{Config, DelegateAgentConfig, EmbeddingRouteConfig, TelegramConfig};
use crate::app::agent::security::SecretStore;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn vec_and_map_secrets_noop_when_encryption_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), false);
    let mut vec_values = vec!["alpha".to_string()];
    let mut map_values = HashMap::from([("one".to_string(), "bravo".to_string())]);

    encrypt_vec_secrets(&store, &mut vec_values, "vec").unwrap();
    encrypt_map_secrets(&store, &mut map_values, "map").unwrap();
    decrypt_vec_secrets(&store, &mut vec_values[..], "vec").unwrap();
    decrypt_map_secrets(&store, &mut map_values, "map").unwrap();

    assert_eq!(vec_values, vec!["alpha".to_string()]);
    assert_eq!(map_values.get("one").map(String::as_str), Some("bravo"));
}

#[test]
fn vec_and_map_secrets_encrypt_once_and_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut vec_values = vec!["alpha".to_string(), "bravo".to_string()];
    let mut map_values = HashMap::from([
        ("one".to_string(), "charlie".to_string()),
        ("two".to_string(), "delta".to_string()),
    ]);

    encrypt_vec_secrets(&store, &mut vec_values, "vec").unwrap();
    encrypt_map_secrets(&store, &mut map_values, "map").unwrap();

    assert!(vec_values.iter().all(|value| SecretStore::is_encrypted(value)));
    assert!(map_values.values().all(|value| SecretStore::is_encrypted(value)));
    let encrypted_vec = vec_values.clone();
    let encrypted_map = map_values.clone();

    encrypt_vec_secrets(&store, &mut vec_values, "vec").unwrap();
    encrypt_map_secrets(&store, &mut map_values, "map").unwrap();

    assert_eq!(vec_values, encrypted_vec);
    assert_eq!(map_values, encrypted_map);

    decrypt_vec_secrets(&store, &mut vec_values, "vec").unwrap();
    decrypt_map_secrets(&store, &mut map_values, "map").unwrap();

    assert_eq!(vec_values, vec!["alpha".to_string(), "bravo".to_string()]);
    assert_eq!(map_values.get("one").map(String::as_str), Some("charlie"));
    assert_eq!(map_values.get("two").map(String::as_str), Some("delta"));
}

#[test]
fn vec_and_map_decrypt_errors_include_position() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut vec_values = vec!["plain".to_string(), "enc2:not-hex".to_string()];
    let mut map_values = HashMap::from([("broken".to_string(), "enc2:not-hex".to_string())]);

    let vec_err = decrypt_vec_secrets(&store, &mut vec_values, "vec").unwrap_err().to_string();
    let map_err = decrypt_map_secrets(&store, &mut map_values, "map").unwrap_err().to_string();

    assert!(vec_err.contains("Failed to decrypt vec[1]"));
    assert!(map_err.contains("Failed to decrypt map.broken"));
}

#[test]
fn vec_and_map_encrypt_errors_include_position() {
    let tmp = tempfile::tempdir().unwrap();
    let blocked_path = tmp.path().join("blocked");
    std::fs::write(&blocked_path, "not a directory").unwrap();
    let store = SecretStore::new(&blocked_path, true);
    let mut vec_values = vec!["alpha".to_string()];
    let mut map_values = HashMap::from([("broken".to_string(), "bravo".to_string())]);

    let vec_err = encrypt_vec_secrets(&store, &mut vec_values, "vec").unwrap_err().to_string();
    let map_err = encrypt_map_secrets(&store, &mut map_values, "map").unwrap_err().to_string();

    assert!(vec_err.contains("Failed to encrypt vec[0]"));
    assert!(map_err.contains("Failed to encrypt map.broken"));
}

#[test]
fn config_secrets_encrypt_and_decrypt_representative_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.config_path = tmp.path().join("vibewindow.json");
    config.api_key = Some("api-key".to_string());
    config.proxy.http_proxy = Some("http://user:pass@proxy.example:8080".to_string());
    config.proxy.https_proxy = Some("https://user:pass@proxy.example:8443".to_string());
    config.proxy.all_proxy = Some("socks5://user:pass@proxy.example:1080".to_string());
    config.browser.computer_use.api_key = Some("computer-use-key".to_string());
    config.web_search.brave_api_key = Some("brave-key".to_string());
    config.storage.provider.config.db_url = Some("postgres://user:pass@localhost/db".to_string());
    config.reliability.api_keys = vec!["fallback-key".to_string()];
    config.reliability.fallback_api_keys.insert("openai".to_string(), "openai-key".to_string());
    config.gateway.paired_tokens = vec!["paired-token".to_string()];
    config.embedding_routes = vec![EmbeddingRouteConfig {
        hint: "semantic".to_string(),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimensions: Some(1536),
        api_key: Some("embedding-key".to_string()),
    }];
    config.agents.insert(
        "writer".to_string(),
        DelegateAgentConfig {
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: Some("agent-key".to_string()),
            ..DelegateAgentConfig::default()
        },
    );
    config.channels_config.telegram = Some(TelegramConfig {
        bot_token: "telegram-token".to_string(),
        allowed_users: Vec::new(),
        stream_mode: Default::default(),
        draft_update_interval_ms: 1000,
        interrupt_on_new_message: false,
        mention_only: false,
        group_reply: None,
        base_url: None,
    });

    encrypt_config_secrets(&mut config).unwrap();

    assert!(crate::app::agent::security::SecretStore::is_encrypted(
        config.api_key.as_deref().unwrap()
    ));
    assert!(SecretStore::is_encrypted(config.proxy.http_proxy.as_deref().unwrap()));
    assert!(SecretStore::is_encrypted(config.proxy.https_proxy.as_deref().unwrap()));
    assert!(SecretStore::is_encrypted(config.proxy.all_proxy.as_deref().unwrap()));
    assert!(SecretStore::is_encrypted(config.browser.computer_use.api_key.as_deref().unwrap()));
    assert!(SecretStore::is_encrypted(config.web_search.brave_api_key.as_deref().unwrap()));
    assert!(SecretStore::is_encrypted(config.storage.provider.config.db_url.as_deref().unwrap()));
    assert!(SecretStore::is_encrypted(&config.reliability.api_keys[0]));
    assert!(SecretStore::is_encrypted(&config.reliability.fallback_api_keys["openai"]));
    assert!(SecretStore::is_encrypted(&config.gateway.paired_tokens[0]));
    assert!(SecretStore::is_encrypted(config.embedding_routes[0].api_key.as_deref().unwrap()));
    assert!(SecretStore::is_encrypted(config.agents["writer"].api_key.as_deref().unwrap()));
    assert!(SecretStore::is_encrypted(
        &config.channels_config.telegram.as_ref().unwrap().bot_token
    ));

    decrypt_config_secrets(&mut config, tmp.path()).unwrap();

    assert_eq!(config.api_key.as_deref(), Some("api-key"));
    assert_eq!(config.proxy.http_proxy.as_deref(), Some("http://user:pass@proxy.example:8080"));
    assert_eq!(config.proxy.https_proxy.as_deref(), Some("https://user:pass@proxy.example:8443"));
    assert_eq!(config.proxy.all_proxy.as_deref(), Some("socks5://user:pass@proxy.example:1080"));
    assert_eq!(config.browser.computer_use.api_key.as_deref(), Some("computer-use-key"));
    assert_eq!(config.web_search.brave_api_key.as_deref(), Some("brave-key"));
    assert_eq!(
        config.storage.provider.config.db_url.as_deref(),
        Some("postgres://user:pass@localhost/db")
    );
    assert_eq!(config.reliability.api_keys, vec!["fallback-key".to_string()]);
    assert_eq!(
        config.reliability.fallback_api_keys.get("openai").map(String::as_str),
        Some("openai-key")
    );
    assert_eq!(config.gateway.paired_tokens, vec!["paired-token".to_string()]);
    assert_eq!(config.embedding_routes[0].api_key.as_deref(), Some("embedding-key"));
    assert_eq!(config.agents["writer"].api_key.as_deref(), Some("agent-key"));
    assert_eq!(config.channels_config.telegram.as_ref().unwrap().bot_token, "telegram-token");
}

#[test]
fn encrypt_config_secrets_rejects_paths_without_parent_directory() {
    let mut config = Config::default();
    config.config_path = PathBuf::new();

    let err = encrypt_config_secrets(&mut config).unwrap_err().to_string();

    assert!(err.contains("Config path must have a parent directory"));
}

#[test]
fn decrypt_config_secrets_reports_embedding_route_error_context() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.embedding_routes = vec![EmbeddingRouteConfig {
        hint: "semantic".to_string(),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimensions: None,
        api_key: Some("enc2:not-hex".to_string()),
    }];

    let error = decrypt_config_secrets(&mut config, tmp.path())
        .expect_err("invalid embedding route secret");

    assert!(error.to_string().contains("config.embedding_routes.*.api_key"));
}

#[test]
fn encrypt_config_secrets_reports_embedding_route_storage_error() {
    let tmp = tempfile::tempdir().unwrap();
    let blocked_path = tmp.path().join("blocked");
    std::fs::write(&blocked_path, "not a directory").unwrap();
    let mut config = Config::default();
    config.config_path = blocked_path.join("vibewindow.json");
    config.embedding_routes = vec![EmbeddingRouteConfig {
        hint: "semantic".to_string(),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimensions: None,
        api_key: Some("embedding-key".to_string()),
    }];

    let error =
        encrypt_config_secrets(&mut config).expect_err("unwritable embedding route secret store");

    assert!(error.to_string().contains("config.embedding_routes.*.api_key"));
}
