use super::*;

pub async fn native_tool_search_core_loop_replayed_search_is_not_completed_again<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    let provider = native_search_provider(vec![
        vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
        vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
    ]);
    let mut history = vec![ChatMessage::user("inspect ledger")];
    let tools = discovery_tools(workspace.path());
    let result = run_tool_call_loop::<H>(
        &provider,
        &mut history,
        &tools,
        &agent_observability::NoopObserver,
        "fixture",
        "fixture/model",
        0.0,
        true,
        None,
        "cli",
        &config_tools::MultimodalConfig::default(),
        8,
        None,
        None,
        None,
        None,
        &[],
    )
    .await;
    assert!(result.unwrap_err().to_string().contains("Skipped duplicate tool call"));
    assert_eq!(
        history
            .iter()
            .filter(|message| message.role == "native"
                && message.content.contains("tool_search_output"))
            .count(),
        1
    );
}
