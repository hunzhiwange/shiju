use super::preset_catalog::omp_presets;
use serde_json::{Value, json, to_value};
use vw_api_types::{agent_apps::AppId, app_providers::omp::OmpProfile};

#[test]
fn catalog_presets_omp_lists_complete_native_catalog() {
    let rows: Vec<_> = omp_presets(AppId::Omp).collect();
    assert_eq!(rows.len(), 115);
    assert_eq!(rows[0].provider_key, "cc-switch-kimi");
    assert_eq!(rows.iter().map(|row| row.settings_config.models.len()).sum::<usize>(), 416);
    for row in rows {
        let mut declaration = to_value(&row.settings_config).unwrap();
        assert!(declaration.get("name").is_none(), "{}", row.provider_key);
        for model in declaration["models"].as_array().unwrap() {
            assert!(model.get("thinkingLevelMap").is_none(), "{}", row.provider_key);
            if let Some(compat) = model.get("compat").and_then(Value::as_object) {
                assert!(!compat.is_empty(), "{}", row.provider_key);
                for key in [
                    "forceAdaptiveThinking",
                    "requiresReasoningContentOnAssistantMessages",
                    "deferredToolsMode",
                ] {
                    assert!(!compat.contains_key(key), "{} {key}", row.provider_key);
                }
                assert_ne!(compat.get("thinkingFormat"), Some(&json!("deepseek")));
            }
        }
        declaration["auth"] = json!("none");
        crate::providers::omp::validate(&OmpProfile { name: row.name.into(), declaration })
            .unwrap_or_else(|error| {
                panic!("invalid native preset {}: {error:?}", row.provider_key)
            });
    }
}
