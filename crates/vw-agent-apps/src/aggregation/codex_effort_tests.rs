use serde_json::json;
use toml_edit::{DocumentMut, Item};
use vw_api_types::{
    ReasoningEffortDto::{High, Low},
    model_catalog::SavedCodexCatalog,
};

use super::plan_codex_subagent;
use crate::aggregation::codex_catalog::{
    CodexCatalogDefault, CodexCatalogMember, CodexCatalogRow, plan_codex_stack_catalog,
};
use crate::providers::live::patch_toml::TomlPatch;

#[test]
fn aggregation_t22_codex_effort_preserves_contract() {
    let first = "ccs-relay/first";
    let second = "ccs-relay/second";
    let row = |slug, efforts: &[&str]| {
        json!({
            "slug": slug,
            "display_name": slug,
            "description": slug,
            "base_instructions": "fixture",
            "supports_reasoning_summaries": true,
            "supports_parallel_tool_calls": false,
            "supported_reasoning_levels": efforts.iter().map(|effort| {
                json!({"effort": effort, "description": effort})
            }).collect::<Vec<_>>(),
        })
    };
    let mut legacy = row("legacy", &[]);
    legacy.as_object_mut().unwrap().remove("supported_reasoning_levels");
    let saved: SavedCodexCatalog = serde_json::from_value(json!({"models": [
        row("first", &["low", "high"]),
        row("second", &["low"]),
        legacy,
    ]}))
    .unwrap();
    let config = "model = 'primary'".parse().unwrap();
    let catalog = plan_codex_stack_catalog(
        CodexCatalogDefault::ThirdParty(CodexCatalogRow { config: &config, catalog: None }),
        &[CodexCatalogMember {
            key: "relay",
            provider_name: "Relay",
            row: CodexCatalogRow { config: &config, catalog: Some(&saved) },
        }],
    )
    .unwrap();
    let choices = plan_codex_subagent(Some(&catalog), None).unwrap().choices;
    assert_eq!(choices[0].efforts, [Low, High]);
    assert_eq!(choices[1].efforts, [Low]);
    assert!(choices[2].efforts.is_empty());

    let mut bytes = b"[agents]\ndefault_subagent_model = 'old'\ndefault_subagent_reasoning_effort = 'medium'\nmax_threads = 3\n".to_vec();
    for (model, requested, expected) in [
        (Some(first), Some(High), Some("high")),
        (Some(second), Some(Low), Some("low")),
        (Some(first), Some(High), Some("high")),
        (Some(second), Some(High), None),
        (Some(second), None, None),
        (Some("ccs-relay/legacy"), Some(High), None),
        (Some(first), Some(High), Some("high")),
        (None, Some(High), None),
    ] {
        let plan = plan_codex_subagent(Some(&catalog), model).unwrap().with_effort(requested);
        let mut nested = Vec::new();
        plan.apply_to_nested(&mut nested);
        bytes = TomlPatch {
            set: nested.into_iter().map(|(path, value)| (path, Item::Value(value))).collect(),
            remove: ["default_subagent_model", "default_subagent_reasoning_effort"]
                .map(|key| vec!["agents".into(), key.into()])
                .into(),
            ..Default::default()
        }
        .apply(Some(&bytes))
        .unwrap();
        let config: DocumentMut = std::str::from_utf8(&bytes).unwrap().parse().unwrap();
        let agents = config["agents"].as_table().unwrap();
        assert_eq!(agents.get("default_subagent_model").and_then(Item::as_str), model);
        assert_eq!(
            agents.get("default_subagent_reasoning_effort").and_then(Item::as_str),
            expected
        );
        assert_eq!(agents["max_threads"].as_integer(), Some(3));
    }
}
