use super::codex_catalog::*;
use serde_json::{Map, Value, json};
use toml_edit::DocumentMut;
use vw_api_types::model_catalog::{CodexNativeModel, SavedCodexCatalog};

fn native_rows() -> Vec<Map<String, Value>> {
    serde_json::from_value(json!([
        {
            "slug": "gpt-sol",
            "display_name": "Sol",
            "priority": 8,
            "visibility": "hide",
            "comp_hash": "official-sol",
            "use_responses_lite": true,
            "model_messages": { "instructions_template": "Official instructions" },
            "supports_reasoning_summaries": true,
            "supports_parallel_tool_calls": false
        },
        {
            "slug": "gpt-astra",
            "display_name": "Astra",
            "priority": 1,
            "tool_mode": "code_mode_only",
            "base_instructions": "Native",
            "supports_reasoning_summaries": true,
            "supports_parallel_tool_calls": true
        },
        {
            "slug": "gpt-tied",
            "display_name": "Tied",
            "priority": 8,
            "base_instructions": "Native",
            "supports_reasoning_summaries": true,
            "supports_parallel_tool_calls": true
        }
    ]))
    .unwrap()
}

fn saved_model(slug: &str, label: &str) -> CodexNativeModel {
    CodexNativeModel {
        slug: slug.into(),
        display_name: label.into(),
        description: "Saved description".into(),
        base_instructions: "Saved instructions".into(),
        supports_reasoning_summaries: true,
        supports_parallel_tool_calls: true,
        context_window: Some(256_000),
    }
}

#[test]
fn aggregation_t09_codex_catalog_preserves_contract() {
    let native = native_rows();
    let official_config = DocumentMut::new();
    let member_config = "model = 'not-in-the-saved-list'".parse().unwrap();
    let saved = SavedCodexCatalog {
        models: vec![saved_model("vendor/z", "Z"), saved_model("vendor/a", "A")],
    };
    let fallback_config =
        "model = 'fallback'\nmodel_context_window = 400000\n[profiles.alt]\nmodel = 'nested'"
            .parse()
            .unwrap();
    let catalog = plan_codex_stack_catalog(
        CodexCatalogDefault::Official { native: &native, config: &official_config },
        &[
            CodexCatalogMember {
                key: "relay",
                provider_name: "Relay",
                row: CodexCatalogRow { catalog: Some(&saved), config: &member_config },
            },
            CodexCatalogMember {
                key: "backup",
                provider_name: "Backup",
                row: CodexCatalogRow { catalog: None, config: &fallback_config },
            },
        ],
    )
    .unwrap();
    let rows = catalog["models"].as_array().unwrap();
    assert_eq!(
        rows.iter().map(|row| row["slug"].as_str().unwrap()).collect::<Vec<_>>(),
        [
            "gpt-astra",
            "gpt-sol",
            "gpt-tied",
            "ccs-relay/vendor/z",
            "ccs-relay/vendor/a",
            "ccs-backup/fallback"
        ]
    );
    for (index, original) in [1, 0, 2].into_iter().enumerate() {
        let mut expected = native[original].clone();
        expected.insert("priority".into(), json!(index + 1));
        assert_eq!(rows[index], json!(expected));
    }
    assert_eq!(rows[3]["display_name"], "Z（Relay）");
    assert_eq!(rows[3]["base_instructions"], "Saved instructions");
    assert_eq!(rows[3]["supports_parallel_tool_calls"], true);
    assert_eq!(rows[5]["description"], "fallback · 400K");
    assert_eq!(rows[5]["auto_compact_token_limit"], 360_000);
    assert_ne!(rows[5]["use_responses_lite"], true);
    assert_eq!(
        rows.iter().map(|row| row["priority"].as_u64().unwrap()).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 6]
    );
}

#[test]
fn aggregation_t09_third_party_default_uses_configured_model_for_empty_catalog() {
    let config =
        "model = 'vendor/primary'\nmodel_auto_compact_token_limit = 16000".parse().unwrap();
    let empty = SavedCodexCatalog { models: vec![] };
    let catalog = plan_codex_stack_catalog(
        CodexCatalogDefault::ThirdParty(CodexCatalogRow { catalog: Some(&empty), config: &config }),
        &[],
    )
    .unwrap();
    assert_eq!(catalog["models"].as_array().unwrap().len(), 1);
    assert_eq!(catalog["models"][0]["slug"], "vendor/primary");
    assert_eq!(catalog["models"][0]["auto_compact_token_limit"], 16_000);
}

#[test]
fn aggregation_t09_missing_official_catalog_refuses_member_only_output() {
    let config = "model = 'configured-but-not-native'".parse().unwrap();
    let members = [CodexCatalogMember {
        key: "relay",
        provider_name: "Relay",
        row: CodexCatalogRow { catalog: None, config: &config },
    }];
    assert_eq!(
        plan_codex_stack_catalog(
            CodexCatalogDefault::Official { native: &[], config: &config },
            &members
        ),
        Err(CodexCatalogError::NativeCatalogUnavailable)
    );
}

#[test]
fn aggregation_t09_duplicate_published_ids_refuse_ambiguous_dispatch() {
    let config = "model = 'duplicate'".parse().unwrap();
    let row = CodexCatalogRow { catalog: None, config: &config };
    let members = [
        CodexCatalogMember { key: "same", provider_name: "First", row },
        CodexCatalogMember { key: "same", provider_name: "Second", row },
    ];
    assert_eq!(
        plan_codex_stack_catalog(CodexCatalogDefault::ThirdParty(row), &members),
        Err(CodexCatalogError::InvalidCatalog)
    );
}
