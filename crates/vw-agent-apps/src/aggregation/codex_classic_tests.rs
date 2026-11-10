use super::codex_classic::{CodexClassicTools, project_classic_tools};
use serde_json::{Map, Value, json};
use toml_edit::DocumentMut;

fn catalog_rows() -> [Map<String, Value>; 3] {
    [
        json!({"slug": "gpt-5.5", "multi_agent_version": "v2", "priority": 1}),
        json!({"slug": "ccs-kimi/kimi-k3", "priority": 2}),
        json!({"slug": "ccs-other/model", "multi_agent_version": "v1", "priority": 3}),
    ]
    .map(|row| row.as_object().unwrap().clone())
}

#[test]
fn aggregation_t24_codex_classic_preserves_contract() {
    let config = DocumentMut::new();
    let mut rows = catalog_rows();

    let status = project_classic_tools(None, &config, true, &mut rows);

    assert_eq!(
        status,
        CodexClassicTools {
            requested: true,
            effective: true,
            multi_agent_v2_forced: false,
            encrypted_task_cross_provider: false,
        }
    );
    let expected = [
        json!({"slug": "gpt-5.5", "multi_agent_version": "v1", "priority": 1}),
        json!({"slug": "ccs-kimi/kimi-k3", "multi_agent_version": "v1", "priority": 2}),
        json!({"slug": "ccs-other/model", "multi_agent_version": "v1", "priority": 3}),
    ];
    assert_eq!(rows.map(Value::Object), expected);
}

#[test]
fn aggregation_t24_disabled_classic_keeps_native_rows_and_warns_across_providers() {
    for cross_provider in [true, false] {
        let config = "[features]\nmulti_agent_v2 = false\n".parse::<DocumentMut>().unwrap();
        let mut rows = catalog_rows();

        let status = project_classic_tools(Some(false), &config, cross_provider, &mut rows);

        assert_eq!(
            status,
            CodexClassicTools {
                requested: false,
                effective: false,
                multi_agent_v2_forced: false,
                encrypted_task_cross_provider: cross_provider,
            }
        );
        assert_eq!(rows, catalog_rows());
    }
}

#[test]
fn aggregation_t24_forced_v2_reports_override_and_keeps_user_config() {
    for config_text in [
        "[features]\nmulti_agent_v2 = true # user override\n",
        "[features.multi_agent_v2]\nenabled = true\nmax_threads = 4\n",
        "features = { multi_agent_v2 = { enabled = true } }\n",
    ] {
        let config = config_text.parse::<DocumentMut>().unwrap();
        let mut rows = catalog_rows();

        let status = project_classic_tools(Some(true), &config, true, &mut rows);

        assert_eq!(
            status,
            CodexClassicTools {
                requested: true,
                effective: false,
                multi_agent_v2_forced: true,
                encrypted_task_cross_provider: true,
            }
        );
        assert_eq!(config.to_string(), config_text);
        assert!(rows.iter().all(|row| row["multi_agent_version"] == "v1"));
    }
}

#[test]
fn aggregation_t24_v2_table_without_enabled_keeps_classic_effective() {
    for config_text in [
        "[features.multi_agent_v2]\nmax_threads = 4\n",
        "[features.multi_agent_v2]\nenabled = false\n",
    ] {
        let config = config_text.parse::<DocumentMut>().unwrap();

        let status = project_classic_tools(Some(true), &config, true, &mut catalog_rows());

        assert!(status.effective);
        assert!(!status.multi_agent_v2_forced);
        assert!(!status.encrypted_task_cross_provider);
    }
}
