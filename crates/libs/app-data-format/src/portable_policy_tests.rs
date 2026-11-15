use super::portable_policy::{ExclusionReason, MigrationLocation, filter_portable_item};
use serde_json::json;

#[test]
fn cc_data_06_policy_keeps_portable_fields() {
    let source = json!({
        "name": "Example", "app_type": "claude", "content": "匿名规则正文",
        "settings_config": {"env": {
            "ANTHROPIC_MODEL": "example-model",
            "ANTHROPIC_BASE_URL": "https://example.invalid/v1"
        }},
        "tags": ["sample"], "cwd": "/anonymous/workspace",
        "launchOnStartup": true, "permissions": {"allow": ["all"]}
    });
    let portable = filter_portable_item(&source, MigrationLocation::CrossDevice);
    assert_eq!(portable.value, json!({
        "name": "Example", "app_type": "claude", "content": "匿名规则正文",
        "settings_config": {"env": {
            "ANTHROPIC_MODEL": "example-model",
            "ANTHROPIC_BASE_URL": "https://example.invalid/v1"
        }}, "tags": ["sample"]
    }));
    assert_eq!(portable.excluded.len(), 3);
    let local = filter_portable_item(&source, MigrationLocation::SameHost);
    assert_eq!(local.value["cwd"], "/anonymous/workspace");
    assert_eq!(local.excluded.len(), 2);
    assert!(source["launchOnStartup"].as_bool().unwrap());
}

#[test]
fn cc_data_06_policy_excludes_nested_credentials() {
    let source = json!({
        "providers": [{"settings_config": {"env": {
            "ANTHROPIC_AUTH_TOKEN": "fixture-secret", "UNKNOWN_ENV": "fixture-secret",
            "NAME": "fixture-secret", "ANTHROPIC_MODEL": "example-model"
        }, "apiKey": "fixture-secret"}}],
        "mcp": {"server_config": {
            "headers": {"Authorization": "fixture-secret"},
            "url": "https://user:fixture-secret@example.invalid/mcp",
            "command": "runner", "args": ["--token=fixture-secret"]
        }},
        "meta": {"nestedFutureField": "fixture-secret"},
        "settings": {"env": ["fixture-secret"]}
    });
    for location in [MigrationLocation::CrossDevice, MigrationLocation::SameHost] {
        let filtered = filter_portable_item(&source, location);
        assert!(!filtered.value.to_string().contains("fixture-secret"));
        assert_eq!(filtered.excluded.len(), 10);
        assert!(filtered.excluded.iter().any(|field| {
            field.pointer == "/providers/0/settings_config/env/ANTHROPIC_AUTH_TOKEN"
                && field.reason == ExclusionReason::CredentialRequiresAccountAuthorization
        }));
        assert!(!format!("{:?}", filtered.excluded).contains("fixture-secret"));
    }
}

#[test]
fn cc_data_06_policy_rejects_opaque_config_and_unsafe_urls() {
    for value in [
        json!({"settings_config": "api_key = 'fixture-secret'"}),
        json!({"url": "https://example.invalid/?key=fixture-secret"}),
        json!({"url": "https://example.invalid/#fixture-secret"}),
        json!({"server_config": ["fixture-secret"]}),
    ] {
        let filtered = filter_portable_item(&value, MigrationLocation::CrossDevice);
        assert_eq!(filtered.value, json!({}));
        assert_eq!(filtered.excluded.len(), 1);
    }
    let filtered = filter_portable_item(&json!(["fixture-secret"]), MigrationLocation::SameHost);
    assert!(filtered.value.is_null());
    assert_eq!(filtered.excluded[0].reason, ExclusionReason::UnsupportedShape);
}

#[test]
fn cc_data_06_policy_filters_foreign_paths_and_bounds_depth() {
    for path in ["/anonymous/work", r"C:\anonymous\work", r"\\server\share", "~/work"] {
        let source = json!({"name": path});
        assert_eq!(filter_portable_item(&source, MigrationLocation::CrossDevice).value, json!({}));
        assert_eq!(filter_portable_item(&source, MigrationLocation::SameHost).value, source);
    }
    let mut source = json!({"name": "example"});
    for _ in 0..70 {
        source = json!({"meta": source});
    }
    let filtered = filter_portable_item(&source, MigrationLocation::CrossDevice);
    assert_eq!(filtered.excluded.len(), 1);
    assert_eq!(filtered.excluded[0].reason, ExclusionReason::DepthLimit);
}
