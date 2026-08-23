// 覆盖格式化器状态序列化、启用缓存与项目实例复用。

use super::state::{EnabledCheck, FormatterInfo, FormatterStatus, LOGGER, State};
use std::collections::HashMap;
use std::sync::Mutex;

// 临时项目和可执行文件夹具用于验证真实探测顺序、缓存命中与失败回退。
#[test]
fn formatter_status_serializes_all_fields() {
    let status = FormatterStatus {
        name: "demo".to_string(),
        extensions: vec![".demo".to_string(), ".example".to_string()],
        enabled: true,
    };

    assert_eq!(
        serde_json::to_value(status).expect("serialize status"),
        serde_json::json!({
            "name": "demo",
            "extensions": [".demo", ".example"],
            "enabled": true,
        })
    );
}

#[test]
fn formatter_status_rejects_missing_or_invalid_required_fields() {
    let missing_enabled = serde_json::json!({
        "name": "rustfmt",
        "extensions": [".rs"],
    });
    let invalid_extensions = serde_json::json!({
        "name": "rustfmt",
        "extensions": ".rs",
        "enabled": true,
    });

    assert!(serde_json::from_value::<FormatterStatus>(missing_enabled).is_err());
    assert!(serde_json::from_value::<FormatterStatus>(invalid_extensions).is_err());
}

#[test]
fn formatter_info_clone_preserves_configuration() {
    let formatter = FormatterInfo {
        name: "demo".to_string(),
        command: vec!["demo".to_string(), "--write".to_string(), "$FILE".to_string()],
        environment: HashMap::from([("DEMO_MODE".to_string(), "safe".to_string())]),
        extensions: vec![".demo".to_string(), ".example".to_string()],
        enabled: EnabledCheck::Which("demo"),
    };
    let cloned = formatter.clone();

    assert_eq!(cloned.name, "demo");
    assert_eq!(cloned.command, ["demo", "--write", "$FILE"]);
    assert_eq!(cloned.environment.get("DEMO_MODE").map(String::as_str), Some("safe"));
    assert_eq!(cloned.extensions, [".demo", ".example"]);
    assert!(matches!(cloned.enabled, EnabledCheck::Which("demo")));
}

#[test]
fn formatter_status_round_trips_through_json() {
    let status = FormatterStatus {
        name: "rustfmt".to_string(),
        extensions: vec![".rs".to_string()],
        enabled: false,
    };

    let json = serde_json::to_string(&status).expect("serialize status");
    let parsed: FormatterStatus = serde_json::from_str(&json).expect("deserialize status");

    assert_eq!(parsed.name, "rustfmt");
    assert_eq!(parsed.extensions, vec![".rs"]);
    assert!(!parsed.enabled);
}

#[test]
fn state_default_starts_empty() {
    let state = State::default();

    assert!(state.enabled.lock().unwrap().is_empty());
    assert!(state.formatters.is_empty());
}

#[test]
fn state_enabled_cache_records_true_and_false_results() {
    let state = State { enabled: Mutex::new(HashMap::new()), formatters: HashMap::new() };

    {
        let mut enabled = state.enabled.lock().expect("enabled cache lock");
        enabled.insert("available".to_string(), true);
        enabled.insert("unavailable".to_string(), false);
    }

    let enabled = state.enabled.lock().expect("enabled cache lock");
    assert_eq!(enabled.get("available"), Some(&true));
    assert_eq!(enabled.get("unavailable"), Some(&false));
    assert!(state.formatters.is_empty());
}

#[test]
fn enabled_check_unit_variants_clone_with_same_debug_representation() {
    let variants = vec![
        EnabledCheck::Always,
        EnabledCheck::Prettier,
        EnabledCheck::Oxfmt,
        EnabledCheck::Biome,
        EnabledCheck::ClangFormat,
        EnabledCheck::Ruff,
        EnabledCheck::UvFormat,
        EnabledCheck::Pint,
        EnabledCheck::Ocamlformat,
        EnabledCheck::RLangAir,
    ];

    for variant in variants {
        let cloned = variant.clone();
        assert_eq!(format!("{cloned:?}"), format!("{variant:?}"));
    }
}

#[test]
fn enabled_check_data_variants_preserve_payloads_when_cloned() {
    let executable = EnabledCheck::Which("demo").clone();
    let marker_files = EnabledCheck::FileUpAny(&["demo.json", ".demorc"]).clone();

    assert!(matches!(executable, EnabledCheck::Which("demo")));
    match marker_files {
        EnabledCheck::FileUpAny(files) => assert_eq!(files, ["demo.json", ".demorc"]),
        other => panic!("expected file marker check, got {other:?}"),
    }
}

#[test]
fn logger_initializes_once_and_reuses_the_same_instance() {
    let first = &*LOGGER as *const _;
    let second = &*LOGGER as *const _;

    assert_eq!(first, second);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn instance_state_loads_builtins_and_reuses_project_state() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let (same_state, cache_is_empty, rustfmt_command, rustfmt_extensions) =
        crate::app::agent::project::instance::provide(temp.path(), None, || {
            Box::pin(async move {
                let first = super::state::instance_state()().await;
                let second = super::state::instance_state()().await;
                let rustfmt = first.formatters.get("rustfmt").expect("rustfmt formatter");

                match &rustfmt.enabled {
                    EnabledCheck::Which(executable) => assert_eq!(*executable, "rustfmt"),
                    other => panic!("expected executable check, got {other:?}"),
                }
                (
                    std::sync::Arc::ptr_eq(&first, &second),
                    first.enabled.lock().expect("enabled cache lock").is_empty(),
                    rustfmt.command.clone(),
                    rustfmt.extensions.clone(),
                )
            })
        })
        .await
        .expect("project context");

    assert!(same_state);
    assert!(cache_is_empty);
    assert_eq!(rustfmt_command, ["rustfmt", "$FILE"]);
    assert_eq!(rustfmt_extensions, [".rs"]);
}
