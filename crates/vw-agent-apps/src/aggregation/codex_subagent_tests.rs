use serde_json::json;
use toml_edit::Value as TomlValue;

use super::codex_catalog::{
    CodexCatalogDefault, CodexCatalogError, CodexCatalogMember, CodexCatalogRow,
    plan_codex_stack_catalog,
};
use super::codex_subagent::{CodexSubagentChoice, plan_codex_subagent};

#[test]
fn aggregation_t21_codex_subagent_preserves_contract() {
    let primary = "model = 'vendor/primary'".parse().unwrap();
    let member = "model = 'vendor/child'".parse().unwrap();
    let default = CodexCatalogRow { config: &primary, catalog: None };
    let member = CodexCatalogMember {
        key: "relay",
        provider_name: "Relay",
        row: CodexCatalogRow { config: &member, catalog: None },
    };
    let catalog =
        plan_codex_stack_catalog(CodexCatalogDefault::ThirdParty(default), &[member]).unwrap();
    let removed = plan_codex_stack_catalog(CodexCatalogDefault::ThirdParty(default), &[]).unwrap();
    let model_id = "ccs-relay/vendor/child";
    let requested = Some(model_id);

    let plan = plan_codex_subagent(Some(&catalog), requested).unwrap();
    assert_eq!(
        plan.choices,
        [CodexSubagentChoice {
            model_id: "ccs-relay/vendor/child",
            label: "vendor/child（Relay）"
        }]
    );
    assert_eq!(plan.effective_model_id, requested);

    let route_nested = |table: &str, key: &str, value: &str| {
        (vec![table.to_owned(), key.to_owned()], TomlValue::from(value))
    };
    let memories = route_nested("memories", "extract_model", "memory-model");
    let mut nested = vec![
        route_nested("agents", "default_subagent_model", "card-model"),
        route_nested("agents", "default_subagent_reasoning_effort", "high"),
        memories.clone(),
    ];
    plan.apply_to_nested(&mut nested);
    let as_strings = |nested: &[(Vec<String>, TomlValue)]| {
        nested
            .iter()
            .map(|(path, value)| (path.join("."), value.as_str().unwrap().to_owned()))
            .collect::<Vec<_>>()
    };
    let expected = [
        ("memories.extract_model".to_owned(), "memory-model".to_owned()),
        ("agents.default_subagent_model".to_owned(), model_id.to_owned()),
    ];
    assert_eq!(as_strings(&nested), expected);

    for unavailable in [
        plan_codex_subagent(Some(&removed), requested).unwrap(),
        plan_codex_subagent(None, requested).unwrap(),
        plan_codex_subagent(Some(&catalog), Some("vendor/primary")).unwrap(),
        plan_codex_subagent(Some(&catalog), Some("ccs-relay/vendor/unpublished")).unwrap(),
        plan_codex_subagent(Some(&catalog), None).unwrap(),
    ] {
        assert_eq!(unavailable.effective_model_id, None);
        unavailable.apply_to_nested(&mut nested);
        assert_eq!(as_strings(&nested), as_strings(std::slice::from_ref(&memories)));
    }

    plan_codex_subagent(Some(&catalog), requested).unwrap().apply_to_nested(&mut nested);
    assert_eq!(as_strings(&nested), expected);
}

#[test]
fn aggregation_t21_codex_subagent_rejects_disabled_and_ambiguous_rows() {
    let disabled = json!({"models": [
        {"slug": "ccs-relay/child", "multi_agent_version": "disabled"}
    ]});
    let plan = plan_codex_subagent(Some(&disabled), Some("ccs-relay/child")).unwrap();
    assert!(plan.choices.is_empty());
    assert_eq!(plan.effective_model_id, None);

    for malformed in [
        json!({}),
        json!({"models": [{"slug": "ccs-invalid"}]}),
        json!({"models": [{"slug": "ccs-relay/child"}, {"slug": "ccs-relay/child"}]}),
    ] {
        assert_eq!(
            plan_codex_subagent(Some(&malformed), Some("ccs-relay/child")),
            Err(CodexCatalogError::InvalidCatalog)
        );
    }
}
