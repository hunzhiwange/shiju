use super::*;

pub async fn dynamic_tools_core_replacement_cannot_expand_child_authority<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "child forbidden ledger").unwrap();
    let catalog = Arc::new(tool_assembly::catalog::ToolCatalog::default());
    let tool =
        Arc::new(DiscoveryFileTool { id: DISCOVERY_LEDGER, path: workspace.path().join("ledger") });
    catalog.replace(vec![tool.clone()], [DISCOVERY_LEDGER.into()]).unwrap();
    let mut provider = DiscoveryProvider::new(
        true,
        vec![vec![("denied", DISCOVERY_LEDGER, serde_json::json!({}))]],
    );
    let replacement = catalog.clone();
    provider.on_request = Some(Arc::new(move |_| {
        replacement.replace(vec![tool.clone()], [DISCOVERY_LEDGER.into()]).unwrap();
    }));
    let context = Arc::new(
        ToolUseContext::<H::Context>::new("parent", None)
            .with_live_tool_catalog(catalog)
            .for_child("child".into(), Vec::new()),
    );
    let history = scope_tool_use_context(
        context,
        run_discovery_loop::<H>(&provider, Vec::new(), &[]),
    )
    .await;
    assert!(provider.requests().iter().all(|(_, tools)| tools.is_empty()));
    assert!(!history.iter().any(|message| message.content.contains("child forbidden ledger")));
    assert!(history.iter().any(|message| message.content.contains("not exposed")));
}

pub async fn native_tool_search_core_loop_core_calls_discovered_tool_next_request<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "native-ledger-content").unwrap();
    let provider = native_search_provider(vec![
        vec![("native-search", "tool_search", serde_json::json!({"query":"ledger"}))],
        vec![("read", DISCOVERY_LEDGER, serde_json::json!({}))],
    ]);
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
    assert_eq!(requests[1].1, [DISCOVERY_LEDGER, "tool_search"]);
    let native: Vec<serde_json::Value> = requests[1]
        .0
        .iter()
        .filter(|message| message.role == "native")
        .map(|message| serde_json::from_str(&message.content).unwrap())
        .collect();
    assert_eq!(native[0]["type"], "tool_search_call");
    assert_eq!(native[0]["call_id"], "native-search");
    assert_eq!(native[1]["type"], "tool_search_output");
    assert_eq!(native[1]["call_id"], "native-search");
    assert!(!native[1]["tools"].as_array().unwrap().is_empty());
    assert!(requests[2].0.iter().any(|message| message.content.contains("native-ledger-content")));
}

pub async fn native_tool_search_core_loop_same_batch_remains_denied<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "private-ledger").unwrap();
    let provider = native_search_provider(vec![vec![
        ("search", "tool_search", serde_json::json!({"query":"ledger"})),
        ("read", DISCOVERY_LEDGER, serde_json::json!({})),
    ]]);
    let history = run_discovery_loop::<H>(&provider, discovery_tools(workspace.path()), &[]).await;
    assert!(
        history.iter().any(|message| message.content.contains("not exposed in this model request"))
    );
    assert!(!history.iter().any(|message| message.content.contains("private-ledger")));
    assert!(history.iter().any(|message| message.role == "native"));
}

pub async fn native_tool_search_core_loop_empty_and_excluded_results_do_not_load_tools<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    let provider = native_search_provider(vec![vec![(
        "search",
        "tool_search",
        serde_json::json!({"query":"ledger"}),
    )]]);
    let history = run_discovery_loop::<H>(
        &provider,
        discovery_tools(workspace.path()),
        &[DISCOVERY_LEDGER.into()],
    )
    .await;
    let output = history
        .iter()
        .filter(|message| message.role == "native")
        .map(|message| serde_json::from_str::<serde_json::Value>(&message.content).unwrap())
        .find(|message| message["type"] == "tool_search_output")
        .unwrap();
    assert!(output["tools"].as_array().unwrap().is_empty());
    assert_eq!(provider.requests()[1].1, ["tool_search"]);
}

pub async fn native_tool_search_core_loop_rejects_unadvertised_and_invalid_search<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    for (enabled, args) in [
        (false, serde_json::json!({"query":"ledger"})),
        (true, serde_json::json!({"query":"ledger", "tools":[DISCOVERY_LEDGER]})),
        (true, serde_json::json!({"query":""})),
    ] {
        let workspace = tempfile::tempdir().unwrap();
        let mut provider = native_search_provider(vec![vec![("search", "tool_search", args)]]);
        provider.search = enabled;
        let mut agent = Agent::<H>::builder()
            .provider(Box::new(provider.clone()))
            .tools(discovery_tools(workspace.path()))
            .memory(Arc::new(agent_memory::NoneMemory))
            .observer(Arc::new(agent_observability::NoopObserver))
            .workspace_dir(workspace.path().to_path_buf())
            .model_name("fixture/model".into())
            .build()
            .unwrap();
        assert!(agent.turn("inspect ledger").await.is_err());
        assert_eq!(provider.requests().len(), 1);
    }
}

pub async fn native_tool_search_core_loop_ordinary_discovery_still_executes<H>()
where
    H: AgentHost,
    H::Context: tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot, ToolDiscoverySession = ToolDiscoverySession>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "ordinary-ledger").unwrap();
    let provider = DiscoveryProvider::new(
        true,
        vec![
            vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![("read", DISCOVERY_LEDGER, serde_json::json!({}))],
        ],
    );
    let history = run_discovery_loop::<H>(&provider, discovery_tools(workspace.path()), &[]).await;
    assert!(history.iter().any(|message| message.content.contains("ordinary-ledger")));
    assert!(history.iter().all(|message| message.role != "native"));
    assert_eq!(provider.requests()[1].1, [DISCOVERY_LEDGER, "tool_search"]);
}
