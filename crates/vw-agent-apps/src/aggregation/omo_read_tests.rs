#![cfg(target_os = "macos")]

use super::omo_read::{OmoReadError, read};
use serde_json::json;
use std::{fs, path::Path};

fn write(home: &Path, relative: &str, contents: &str) {
    let path = home.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn aggregation_t32_omo_read_preserves_contract() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    write(home, ".config/opencode/oh-my-openagent.jsonc", r#"{"agents":{"legacy":{}}}"#);
    write(home, ".omo/omo.json", r#"{"[opencode]":{"agents":{"secondary":{}}}}"#);
    let unified = r#"{
        // The Codex section belongs to a different client.
        "[codex]": {"agents":{"codex":{}}},
        "[opencode]": {
            "agents": {"builder":{"model":"provider/model","apiKey":"fixture-secret"}},
            "categories": {"quick":{"model":"provider/fast"}},
            "settings": {"endpoint":"https://example.test/api", "enabled":true, "hooks":[{"token":"fixture-secret","mode":"normal"}],},
        },
    }"#;
    write(home, ".omo/omo.jsonc", unified);

    let result = read(home, None, Some("omo-current")).unwrap();
    assert_eq!(json!(result.agents), json!({"builder":{"model":"provider/model"}}));
    assert_eq!(json!(result.categories), json!({"quick":{"model":"provider/fast"}}));
    assert_eq!(
        json!(result.other_fields),
        json!({"settings":{"endpoint":"https://example.test/api","enabled":true,"hooks":[{"mode":"normal"}]}})
    );
    assert_eq!(result.current_provider_id.as_deref(), Some("omo-current"));
    assert_eq!(fs::read_to_string(home.join(".omo/omo.jsonc")).unwrap(), unified);
}

#[test]
fn aggregation_t32_legacy_order_respects_the_saved_directory() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    write(home, ".config/opencode/oh-my-openagent.jsonc", r#"{"agents":{"default":{}}}"#);
    let names = [
        "oh-my-openagent.jsonc",
        "oh-my-openagent.json",
        "oh-my-opencode.jsonc",
        "oh-my-opencode.json",
    ];
    for name in names {
        write(home, &format!("alternate/{name}"), &json!({"selected":name}).to_string());
    }
    write(home, ".omo/omo.json", r#"{"[opencode]":{"selected":"unified"}}"#);
    let result = read(home, Some("~/alternate"), None).unwrap();
    assert_eq!(result.other_fields.unwrap()["selected"], "unified");
    fs::remove_file(home.join(".omo/omo.json")).unwrap();

    for name in names {
        let result = read(home, Some("~/alternate"), None).unwrap();
        assert_eq!(result.other_fields.unwrap()["selected"], name);
        fs::remove_file(home.join("alternate").join(name)).unwrap();
    }
    let result = read(home, Some("~/alternate"), None).unwrap();
    assert!(result.agents.is_none());
    assert!(result.current_provider_id.is_none());
}

#[test]
fn aggregation_t32_unified_without_opencode_does_not_read_legacy() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    write(home, ".config/opencode/oh-my-openagent.jsonc", r#"{"agents":{"legacy":{}}}"#);
    write(home, ".omo/omo.json", r#"{"[opencode]":{"agents":{"secondary":{}}}}"#);
    write(home, ".omo/omo.jsonc", r#"{"[codex]":{"agents":{"codex":{}}}}"#);
    let result = read(home, None, Some("omo-current")).unwrap();
    assert!(result.agents.is_none());
    assert!(result.categories.is_none());
    assert!(result.other_fields.is_none());
    assert_eq!(result.current_provider_id.as_deref(), Some("omo-current"));
}

#[test]
fn aggregation_t32_damaged_configuration_does_not_fall_back() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    write(home, ".config/opencode/oh-my-openagent.jsonc", "{}");
    write(home, ".omo/omo.json", r#"{"[opencode]":{}}"#);
    for damaged in [
        r#"{"[opencode]":{/*broken"#,
        r#"{"[opencode]":[]}"#,
        r#"{"[opencode]":{"agents":[]}}"#,
        r#"{"[opencode]":{},"[opencode]":{}}"#,
    ] {
        write(home, ".omo/omo.jsonc", damaged);
        assert_eq!(read(home, None, None), Err(OmoReadError::InvalidConfiguration));
    }
}

#[test]
fn aggregation_t32_inaccessible_preferred_file_does_not_fall_back() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    write(home, ".omo/omo.json", r#"{"[opencode]":{}}"#);
    std::os::unix::fs::symlink("omo.json", home.join(".omo/omo.jsonc")).unwrap();
    assert_eq!(read(home, None, None), Err(OmoReadError::TargetFailed));
    assert_eq!(read(home, Some("~/../outside"), None), Err(OmoReadError::Forbidden));
}
