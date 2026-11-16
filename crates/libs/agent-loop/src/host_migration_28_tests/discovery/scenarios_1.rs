use super::*;

pub async fn codex_discovery_core_agent_discovers_and_calls_only_on_next_request<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "ledger-file-content").unwrap();
    let provider = DiscoveryProvider::new(
        true,
        vec![
            vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![("read", DISCOVERY_LEDGER, serde_json::json!({}))],
        ],
    );
    let mut agent = Agent::<H>::builder()
        .provider(Box::new(provider.clone()))
        .tools(discovery_tools(workspace.path()))
        .memory(Arc::new(agent_memory::NoneMemory))
        .observer(Arc::new(agent_observability::NoopObserver))
        .workspace_dir(workspace.path().to_path_buf())
        .model_name("fixture/model".into())
        .build()
        .unwrap();
    assert_eq!(agent.turn("inspect ledger").await.unwrap(), "final answer");
    let requests = provider.requests();
    assert_eq!(requests[0].1, ["tool_search"]);
    assert!(
        !requests[0].0.iter().any(|message| message.content.contains("\"mcp__fixture__ledger\"")),
        "initial prompt must not leak deferred parameter schemas"
    );
    assert_eq!(requests[1].1, [DISCOVERY_LEDGER, "tool_search"]);
    let results: Vec<_> = requests[2]
        .0
        .iter()
        .filter(|message| message.role == "tool")
        .map(|message| serde_json::from_str::<serde_json::Value>(&message.content).unwrap())
        .collect();
    assert_eq!(results[0]["tool_call_id"], "search");
    assert_eq!(results[1]["tool_call_id"], "read");
    assert!(results[1]["content"].as_str().unwrap().contains("ledger-file-content"));
}

pub async fn codex_discovery_core_text_protocol_exposes_only_current_specs_without_persisting_them<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "text-ledger-content").unwrap();
    let provider = DiscoveryProvider::new(
        false,
        vec![
            vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![("read", DISCOVERY_LEDGER, serde_json::json!({}))],
        ],
    );
    let history = run_discovery_loop::<H>(&provider, discovery_tools(workspace.path()), &[]).await;
    let requests = provider.requests();
    assert!(requests.iter().all(|(_, tools)| tools.is_empty()));
    let prompt = |index: usize| {
        requests[index]
            .0
            .iter()
            .filter(|message| message.content.contains("## Tool Use Protocol"))
            .map(|message| message.content.as_str())
            .collect::<String>()
    };
    assert!(prompt(0).contains("**tool_search**"));
    assert!(!prompt(0).contains(DISCOVERY_LEDGER));
    assert!(prompt(1).contains(&format!("**{DISCOVERY_LEDGER}**")));
    assert!(!prompt(1).contains(DISCOVERY_QUEUE));
    assert!(history.iter().any(|message| message.content.contains("text-ledger-content")));
    assert!(history.iter().all(|message| !message.content.contains("## Tool Use Protocol")));
}

pub async fn codex_discovery_core_same_round_search_does_not_allow_deferred_call<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "must-not-read-ledger").unwrap();
    let provider = DiscoveryProvider::new(
        true,
        vec![vec![
            ("search", "tool_search", serde_json::json!({"query":"ledger"})),
            ("read", DISCOVERY_LEDGER, serde_json::json!({})),
        ]],
    );
    let history = run_discovery_loop::<H>(&provider, discovery_tools(workspace.path()), &[]).await;
    let result = history
        .iter()
        .find(|message| {
            message.role == "tool" && message.content.contains("\"tool_call_id\":\"read\"")
        })
        .unwrap();
    assert!(result.content.contains("not exposed in this model request"));
    assert!(!result.content.contains("must-not-read-ledger"));
}

pub async fn codex_discovery_core_failed_and_repeated_searches_keep_visibility_consistent<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    let provider = DiscoveryProvider::new(
        true,
        vec![
            vec![("bad", "tool_search", serde_json::json!({"query":""}))],
            vec![("first", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![("second", "tool_search", serde_json::json!({"query":"ledger"}))],
        ],
    );
    run_discovery_loop::<H>(&provider, discovery_tools(workspace.path()), &[]).await;
    let requests = provider.requests();
    assert_eq!(requests[0].1, ["tool_search"]);
    assert_eq!(requests[1].1, ["tool_search"]);
    assert_eq!(requests[2].1, [DISCOVERY_LEDGER, "tool_search"]);
    assert_eq!(requests[3].1, requests[2].1);
}

pub async fn codex_discovery_core_excluded_tool_cannot_be_searched_or_called<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "excluded-ledger-content").unwrap();
    let provider = DiscoveryProvider::new(
        true,
        vec![
            vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![("read", DISCOVERY_LEDGER, serde_json::json!({}))],
        ],
    );
    let history = run_discovery_loop::<H>(
        &provider,
        discovery_tools(workspace.path()),
        &[DISCOVERY_LEDGER.into()],
    )
    .await;
    assert!(
        provider.requests().iter().all(|(_, tools)| !tools.contains(&DISCOVERY_LEDGER.to_string()))
    );
    let results: Vec<_> = history.iter().filter(|message| message.role == "tool").collect();
    assert!(results[0].content.contains("\\\"tools\\\":[]"));
    assert!(results[1].content.contains("not available in this channel"));
}

pub async fn codex_discovery_core_denied_search_keeps_authorized_mcp_directly_visible<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    let provider = DiscoveryProvider::new(true, Vec::new());
    run_discovery_loop::<H>(&provider, discovery_tools(workspace.path()), &["tool_search".into()]).await;
    assert_eq!(provider.requests()[0].1, [DISCOVERY_LEDGER, DISCOVERY_QUEUE]);
}
