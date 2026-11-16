//! Selected record/spec projection driven by the actual host registry.
use crate::mcp::McpCatalogStore;
use vw_api_transport::ProjectScope;
use vw_api_mcp_management::{McpCatalogScope, McpRecordRef, McpRuntimeSelection};
use tool_types::{ToolSpec, ToolCallResult};
pub fn selected_record(
    catalog: &McpCatalogStore, scope: &McpCatalogScope, project: &ProjectScope, record: McpRecordRef,
    specs_for_context: impl Fn() -> Vec<ToolSpec>,
    execute: impl Fn(&str, &str) -> anyhow::Result<ToolCallResult>,
) -> anyhow::Result<()> {
    assert!(
        specs_for_context()
            .iter()
            .all(|spec| !spec.id.starts_with("mcp__selected__")),
        "an active CLI binding must not enable the native runtime",
    );

    catalog
        .select_for_runtime(
            &project,
            &scope,
            &McpRuntimeSelection { record: record, project: project.clone() },
        )
        .unwrap();

    let specs = specs_for_context();
    let tool = specs
        .iter()
        .find(|spec| spec.id.starts_with("mcp__selected__"))
        .expect("the selected catalog record must expose the real MCP tool");
    let result = execute(&tool.id, r#"{"message":"catalog-discovery-fixture"}"#)?;
    assert!(result.model_text().contains("catalog-discovery-fixture|server-cwd"));
    Ok(())
}
