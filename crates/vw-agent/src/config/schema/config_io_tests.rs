//! 配置 I/O 兼容性归一化测试。
//!
//! 这些测试覆盖旧配置别名到当前字段名的迁移行为，确保用户升级后不会因为历史键名而丢失 ACP 配置。

use super::config_io::{
    extract_config_payload, merge_config_payload, normalize_legacy_alias_conflicts,
    normalize_top_level_table_aliases, parse_config_contents, read_config_root_json,
    read_project_config_override, upsert_config_payload,
};

#[test]
/// JSON payload 缺少 `acp` 时，应将旧 `acp2` 字段迁移过去。
fn normalize_json_acp2_moves_to_acp_when_acp_missing() {
    let mut payload = serde_json::json!({
        "acp2": {
            "demo": {
                "command": "codex-acp"
            }
        }
    });

    normalize_legacy_alias_conflicts(&mut payload);

    assert!(payload.get("acp2").is_none());
    assert_eq!(payload["acp"]["demo"]["command"], "codex-acp");
}

#[test]
/// JSON payload 同时存在新旧字段时，应丢弃旧 `acp2`，避免覆盖新配置。
fn normalize_json_acp2_is_dropped_when_acp_exists() {
    let mut payload = serde_json::json!({
        "acp": {},
        "acp2": {
            "demo": {
                "command": "codex-acp"
            }
        }
    });

    normalize_legacy_alias_conflicts(&mut payload);

    assert!(payload.get("acp2").is_none());
    assert_eq!(payload["acp"], serde_json::json!({}));
}

#[test]
fn normalize_json_acp_is_unchanged_when_legacy_key_is_absent() {
    let mut payload = serde_json::json!({
        "acp": {
            "demo": {
                "command": "current"
            }
        }
    });

    normalize_legacy_alias_conflicts(&mut payload);

    assert_eq!(payload["acp"]["demo"]["command"], "current");
    assert!(payload.get("acp2").is_none());
}

#[test]
/// TOML 顶层 `[acp2]` 表在没有 `[acp]` 时应迁移为 `[acp]`。
fn normalize_toml_acp2_moves_to_acp_when_acp_missing() {
    let mut raw = toml::from_str::<toml::Value>(
        r#"
        [acp2.demo]
        command = "codex-acp"
        "#,
    )
    .expect("valid toml");

    normalize_top_level_table_aliases(&mut raw);

    let root = raw.as_table().expect("root table");
    assert!(!root.contains_key("acp2"));
    assert_eq!(root["acp"]["demo"]["command"].as_str(), Some("codex-acp"));
}

#[test]
fn normalize_toml_gateway_moves_to_gateway_when_gateway_missing() {
    let mut raw = toml::from_str::<toml::Value>(
        r#"
        [Gateway]
        enabled = true
        "#,
    )
    .expect("valid toml");

    normalize_top_level_table_aliases(&mut raw);

    let root = raw.as_table().expect("root table");
    assert!(!root.contains_key("Gateway"));
    assert_eq!(root["gateway"]["enabled"].as_bool(), Some(true));
}

#[test]
fn normalize_toml_legacy_tables_are_dropped_when_current_tables_exist() {
    let mut raw = toml::from_str::<toml::Value>(
        r#"
        [gateway]
        enabled = true

        [Gateway]
        enabled = false

        [acp.demo]
        command = "current"

        [acp2.demo]
        command = "legacy"
        "#,
    )
    .expect("valid toml");

    normalize_top_level_table_aliases(&mut raw);

    let root = raw.as_table().expect("root table");
    assert!(!root.contains_key("Gateway"));
    assert!(!root.contains_key("acp2"));
    assert_eq!(root["gateway"]["enabled"].as_bool(), Some(true));
    assert_eq!(root["acp"]["demo"]["command"].as_str(), Some("current"));
}

#[test]
fn normalize_toml_non_table_root_is_unchanged() {
    let mut raw = toml::Value::String("plain".to_string());

    normalize_top_level_table_aliases(&mut raw);

    assert_eq!(raw.as_str(), Some("plain"));
}

#[test]
fn normalize_json_model_aliases_prefer_current_keys() {
    let mut payload = serde_json::json!({
        "default_model": "new",
        "model": "old",
        "default_provider": "provider-new",
        "model_provider": "provider-old"
    });

    normalize_legacy_alias_conflicts(&mut payload);

    assert!(payload.get("model").is_none());
    assert!(payload.get("model_provider").is_none());
    assert_eq!(payload["default_model"], "new");
}

#[test]
fn upsert_config_payload_removes_legacy_keys_and_merges_payload() {
    let mut root = serde_json::json!({
        "agent": {"legacy": true},
        "model": "old",
        "model_provider": "old-provider",
        "retained": true
    });

    upsert_config_payload(
        &mut root,
        serde_json::json!({"default_model": "new", "default_provider": "new-provider"}),
    );

    assert!(root.get("agent").is_none());
    assert!(root.get("model").is_none());
    assert!(root.get("model_provider").is_none());
    assert_eq!(root["retained"], true);
    assert_eq!(root["default_model"], "new");
}

#[test]
fn upsert_config_payload_initializes_non_object_root_and_ignores_agent_payload() {
    let mut root = serde_json::json!(false);

    upsert_config_payload(
        &mut root,
        serde_json::json!({"agent": {"ignored": true}, "default_model": "new"}),
    );

    assert_eq!(root, serde_json::json!({"default_model": "new"}));
}

#[test]
fn upsert_config_payload_ignores_non_object_payload() {
    let mut root = serde_json::json!({"retained": true});

    upsert_config_payload(&mut root, serde_json::json!("not an object"));

    assert_eq!(root, serde_json::json!({"retained": true}));
}

#[test]
fn extract_config_payload_treats_empty_or_non_object_roots_as_absent() {
    assert!(extract_config_payload(&serde_json::json!({})).is_none());
    assert!(extract_config_payload(&serde_json::json!(null)).is_none());
    assert_eq!(
        extract_config_payload(&serde_json::json!({"default_model": "m"})).unwrap()["default_model"],
        "m"
    );
}

#[test]
fn parse_config_contents_rejects_empty_source() {
    let error = parse_config_contents(" \n\t ").unwrap_err().to_string();

    assert!(error.contains("config source is empty"));
}

#[test]
fn parse_config_contents_rejects_non_object_json_root() {
    let error = parse_config_contents("[]").unwrap_err().to_string();

    assert!(error.contains("config JSON root must be an object"));
}

#[tokio::test]
async fn read_config_root_json_handles_missing_empty_and_invalid_files() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("missing.json");
    assert_eq!(read_config_root_json(&missing).await.unwrap(), serde_json::json!({}));

    let empty = tmp.path().join("empty.json");
    tokio::fs::write(&empty, "   ").await.unwrap();
    assert_eq!(read_config_root_json(&empty).await.unwrap(), serde_json::json!({}));

    let invalid = tmp.path().join("invalid.json");
    tokio::fs::write(&invalid, "{").await.unwrap();
    assert!(read_config_root_json(&invalid).await.unwrap_err().to_string().contains("parse"));
}

#[test]
fn normalize_json_non_object_payload_is_unchanged() {
    let mut payload = serde_json::json!("plain");

    normalize_legacy_alias_conflicts(&mut payload);

    assert_eq!(payload, serde_json::json!("plain"));
}

#[tokio::test]
async fn read_config_root_json_reads_valid_root_object() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.json");
    tokio::fs::write(&path, r#"{"default_model":"demo"}"#).await.unwrap();

    assert_eq!(
        read_config_root_json(&path).await.unwrap(),
        serde_json::json!({"default_model": "demo"})
    );
}

#[tokio::test]
async fn read_config_root_json_reports_read_errors() {
    let tmp = tempfile::tempdir().unwrap();

    assert!(read_config_root_json(tmp.path()).await.unwrap_err().to_string().contains("read"));
}

#[test]
fn merge_config_payload_recurses_and_treats_null_as_deletion() {
    let mut base = serde_json::json!({
        "default_model": "base",
        "gateway": { "host": "127.0.0.1", "port": 8080 },
        "retained": true
    });

    merge_config_payload(
        &mut base,
        serde_json::json!({
            "default_model": "project",
            "gateway": { "host": null, "port": 9090 }
        }),
    );

    assert_eq!(
        base,
        serde_json::json!({
            "default_model": "project",
            "gateway": { "port": 9090 },
            "retained": true
        })
    );
}

#[tokio::test]
async fn project_config_override_requires_an_explicit_object_file() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join(".vibewindow.json");
    assert!(read_project_config_override(&missing).await.unwrap().is_none());

    tokio::fs::write(&missing, r#"{"default_model":"project"}"#).await.unwrap();
    assert_eq!(
        read_project_config_override(&missing).await.unwrap().unwrap()["default_model"],
        "project"
    );

    tokio::fs::write(&missing, "null").await.unwrap();
    assert!(
        read_project_config_override(&missing)
            .await
            .unwrap_err()
            .to_string()
            .contains("root must be an object")
    );
}
