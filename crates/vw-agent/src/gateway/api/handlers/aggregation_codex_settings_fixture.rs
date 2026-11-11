//! Published rows produced by the real Codex catalog owner for client validation.
use serde_json::Value;
use vw_agent_apps::aggregation::codex_catalog::{
    CodexCatalogDefault, CodexCatalogMember, CodexCatalogRow, plan_codex_stack_catalog,
};
use vw_api_types::{
    ReasoningEffortDto,
    model_catalog::{CodexNativeModel, CodexReasoningLevel, SavedCodexCatalog},
};

pub(super) fn catalog() -> Value {
    let primary = "model='primary'".parse().unwrap();
    let child = "model='child'".parse().unwrap();
    let saved = SavedCodexCatalog {
        models: vec![CodexNativeModel {
            slug: "child".into(),
            display_name: "Child".into(),
            description: "Fixture child".into(),
            base_instructions: "Complete the fixture task.".into(),
            supports_reasoning_summaries: true,
            supports_parallel_tool_calls: true,
            context_window: None,
            supported_reasoning_levels: vec![CodexReasoningLevel {
                effort: ReasoningEffortDto::High,
                description: "High".into(),
            }],
        }],
    };
    plan_codex_stack_catalog(
        CodexCatalogDefault::ThirdParty(CodexCatalogRow { config: &primary, catalog: None }),
        &[CodexCatalogMember {
            key: "beta",
            provider_name: "Beta",
            row: CodexCatalogRow { config: &child, catalog: Some(&saved) },
        }],
    )
    .unwrap()
}
