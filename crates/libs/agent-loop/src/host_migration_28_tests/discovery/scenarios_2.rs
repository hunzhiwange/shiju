use super::*;

pub async fn codex_discovery_core_batch_rejects_same_round_and_excluded_inner_calls<H>()
where
    H: AgentHost,
    H::Context: tool_assembly::tool_search::SearchHost + tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "batch-ledger-content").unwrap();
    for excluded in [Vec::new(), vec![DISCOVERY_LEDGER.to_string()]] {
        let mut tools = discovery_tools::<H::Context>(workspace.path());
        let inner: Vec<Arc<dyn Tool>> = vec![Arc::new(DiscoveryFileTool {
            id: DISCOVERY_LEDGER,
            path: workspace.path().join("ledger"),
        })];
        tools.push(Box::new(tool_services::batch::BatchTool::<H::Context>::new(Arc::new(inner))));
        let provider = DiscoveryProvider::new(
            true,
            vec![vec![
                ("search", "tool_search", serde_json::json!({"query":"ledger"})),
                (
                    "batch",
                    "batch",
                    serde_json::json!({"tool_calls":[{"tool":DISCOVERY_LEDGER,"args":{}}]}),
                ),
            ]],
        );
        let history = run_discovery_loop::<H>(&provider, tools, &excluded).await;
        let result = history
            .iter()
            .find(|message| {
                message.role == "tool" && message.content.contains("\"tool_call_id\":\"batch\"")
            })
            .unwrap();
        assert!(result.content.contains("not exposed in this model request"));
        assert!(!result.content.contains("batch-ledger-content"));
    }
}

pub async fn codex_discovery_core_empty_registry_provides_no_tool_protocol<H>()
where
    H: AgentHost,
    H::Context: tool_assembly::tool_search::SearchHost + tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot>,
{
    let provider = DiscoveryProvider::new(false, Vec::new());
    run_discovery_loop::<H>(&provider, Vec::new(), &[]).await;
    let requests = provider.requests();
    assert!(requests[0].1.is_empty());
    assert!(requests[0].0.iter().all(|message| !message.content.contains("## Tool Use Protocol")));
}

pub async fn codex_discovery_core_loaded_batch_calls_preserve_inner_permission_checks<H>()
where
    H: AgentHost,
    H::Context: tool_assembly::tool_search::SearchHost + tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "permitted-batch-content").unwrap();
    let mut tools = discovery_tools::<H::Context>(workspace.path());
    let inner: Vec<Arc<dyn Tool>> = vec![Arc::new(DiscoveryFileTool {
        id: DISCOVERY_LEDGER,
        path: workspace.path().join("ledger"),
    })];
    tools.push(Box::new(tool_services::batch::BatchTool::<H::Context>::new(Arc::new(inner))));
    let provider = DiscoveryProvider::new(
        true,
        vec![
            vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![(
                "permitted",
                "batch",
                serde_json::json!({"tool_calls":[{"tool":DISCOVERY_LEDGER,"args":{}}]}),
            )],
            vec![(
                "denied",
                "batch",
                serde_json::json!({"tool_calls":[{"tool":DISCOVERY_LEDGER,"args":{"deny":true}}]}),
            )],
        ],
    );
    let history = run_discovery_loop::<H>(&provider, tools, &[]).await;
    let results: Vec<_> = history
        .iter()
        .filter(|message| message.role == "tool")
        .map(|message| serde_json::from_str::<serde_json::Value>(&message.content).unwrap())
        .collect();
    assert!(results[1]["content"].as_str().unwrap().contains("permitted-batch-content"));
    assert!(results[2]["content"].as_str().unwrap().contains("ledger access denied"));
    assert!(!results[2]["content"].as_str().unwrap().contains("permitted-batch-content"));
}

pub async fn dynamic_tools_core_replacement_visible_next_request<H>()
where
    H: AgentHost,
    H::Context: tool_assembly::tool_search::SearchHost + tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "new executor completed").unwrap();
    let catalog = Arc::new(tool_assembly::catalog::ToolCatalog::default());
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    catalog
        .replace(
            vec![Arc::new(PendingCatalogTool {
                started: started.clone(),
                release: release.clone(),
            })],
            ["old_tool".into()],
        )
        .unwrap();
    let provider = DiscoveryProvider::new(
        true,
        vec![
            vec![("old", "old_tool", serde_json::json!({}))],
            vec![("new", DISCOVERY_LEDGER, serde_json::json!({}))],
        ],
    );
    let context = Arc::new(
        ToolUseContext::<H::Context>::new("dynamic-core", Some(workspace.path().to_string_lossy().into_owned()))
            .with_live_tool_catalog(catalog.clone()),
    );
    let mut agent = Agent::<H>::builder()
        .provider(Box::new(provider.clone()))
        .tools(Vec::new())
        .memory(Arc::new(agent_memory::NoneMemory))
        .observer(Arc::new(agent_observability::NoopObserver))
        .workspace_dir(workspace.path().to_path_buf())
        .model_name("fixture/model".into())
        .build()
        .unwrap();
    let run = scope_tool_use_context(context, async {
        assert_eq!(agent.turn("inspect tools").await.unwrap(), "final answer");
    });
    let replace = async {
        started.notified().await;
        catalog
            .replace(
                vec![Arc::new(DiscoveryFileTool {
                    id: DISCOVERY_LEDGER,
                    path: workspace.path().join("ledger"),
                })],
                [DISCOVERY_LEDGER.into()],
            )
            .unwrap();
        release.notify_one();
    };
    let ((), ()) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(run, replace)
    })
    .await
    .unwrap();
    assert_eq!(provider.requests()[0].1, ["old_tool"]);
    assert_eq!(provider.requests()[1].1, [DISCOVERY_LEDGER]);
    assert!(
        provider.requests()[1]
            .0
            .iter()
            .any(|message| message.content.contains("old executor completed"))
    );
    assert!(
        provider.requests()[2]
            .0
            .iter()
            .any(|message| message.content.contains("new executor completed"))
    );
}

pub async fn dynamic_tools_core_replacement_preserves_same_batch_rejection<H>()
where
    H: AgentHost,
    H::Context: tool_assembly::tool_search::SearchHost + tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "private ledger").unwrap();
    let catalog = Arc::new(tool_assembly::catalog::ToolCatalog::default());
    let tools = discovery_tools::<H::Context>(workspace.path());
    let specs = tools.iter().map(|tool| tool.spec().id).collect::<Vec<_>>();
    catalog.replace(tools.into_iter().map(Arc::from).collect(), specs).unwrap();
    let provider = DiscoveryProvider::new(
        true,
        vec![vec![
            ("search", "tool_search", serde_json::json!({"query":"ledger"})),
            ("read", DISCOVERY_LEDGER, serde_json::json!({})),
        ]],
    );
    let context =
        Arc::new(ToolUseContext::<H::Context>::new("dynamic-core", None).with_live_tool_catalog(catalog));
    let history = scope_tool_use_context(
        context,
        run_discovery_loop::<H>(&provider, Vec::new(), &[]),
    )
    .await;
    assert!(
        history.iter().any(|message| message.content.contains("not exposed in this model request"))
    );
    assert!(!history.iter().any(|message| message.content.contains("private ledger")));
}

pub async fn dynamic_tools_core_replacement_intersects_loaded_authorization<H>()
where
    H: AgentHost,
    H::Context: tool_assembly::tool_search::SearchHost + tool_services::BatchHost + ContextHost<ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot>,
{
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("ledger"), "authorized old ledger").unwrap();
    let catalog = Arc::new(tool_assembly::catalog::ToolCatalog::default());
    let tools = discovery_tools::<H::Context>(workspace.path());
    let ids = tools.iter().map(|tool| tool.spec().id).collect::<Vec<_>>();
    catalog.replace(tools.into_iter().map(Arc::from).collect(), ids).unwrap();
    let mut provider = DiscoveryProvider::new(
        true,
        vec![
            vec![("search", "tool_search", serde_json::json!({"query":"ledger"}))],
            vec![("old", DISCOVERY_LEDGER, serde_json::json!({}))],
            vec![("denied", DISCOVERY_LEDGER, serde_json::json!({}))],
        ],
    );
    let replacement = catalog.clone();
    provider.on_request = Some(Arc::new(move |index| {
        if index == 2 {
            replacement.replace(Vec::new(), Vec::new()).unwrap();
        }
    }));
    let context =
        Arc::new(ToolUseContext::<H::Context>::new("dynamic-core", None).with_live_tool_catalog(catalog));
    let history = scope_tool_use_context(
        context,
        run_discovery_loop::<H>(&provider, Vec::new(), &[]),
    )
    .await;
    assert!(provider.requests()[1].1.contains(&DISCOVERY_LEDGER.into()));
    assert!(provider.requests()[2].1.is_empty());
    assert!(
        provider.requests()[2]
            .0
            .iter()
            .any(|message| message.content.contains("authorized old ledger"))
    );
    let denied = history
        .iter()
        .find(|message| message.role == "tool" && message.content.contains("denied"))
        .unwrap();
    assert!(!denied.content.contains("authorized old ledger"));
    assert!(denied.content.contains("Unknown tool: mcp__fixture__ledger"));
}
