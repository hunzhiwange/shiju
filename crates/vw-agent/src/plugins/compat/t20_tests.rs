use super::mcp_connections::McpSessionConnections;
use crate::tools::{
    Tool,
    mcp_tool::{McpTool, mcp_tools_to_specs},
};
use agent_client_protocol::{McpServer, McpServerStdio};
use rmcp::{
    ServerHandler, ServiceExt,
    model::*,
    service::{Peer, RequestContext, RoleServer},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Default)]
struct Service {
    generation: Arc<AtomicUsize>,
    lists: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
    invalid: Arc<AtomicUsize>,
    resources: Arc<AtomicUsize>,
    prompts: Arc<AtomicUsize>,
    resource_lists: Arc<AtomicUsize>,
    template_lists: Arc<AtomicUsize>,
    prompt_lists: Arc<AtomicUsize>,
    changes_during_listing: Arc<AtomicUsize>,
    unadvertised: Arc<AtomicUsize>,
    call_started: Arc<tokio::sync::Notify>,
    finish_call: Arc<tokio::sync::Notify>,
    resource_started: Arc<tokio::sync::Notify>,
}
impl ServerHandler for Service {
    fn get_info(&self) -> ServerInfo {
        let mut capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_tool_list_changed()
            .enable_resources()
            .enable_resources_list_changed()
            .enable_prompts()
            .enable_prompts_list_changed()
            .build();
        if self.unadvertised.load(Ordering::SeqCst) != 0 {
            capabilities.tools.as_mut().unwrap().list_changed = None;
        }
        ServerInfo { capabilities, ..Default::default() }
    }
    async fn list_tools(
        &self,
        params: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        self.lists.fetch_add(1, Ordering::SeqCst);
        let generation = self.generation.load(Ordering::SeqCst);
        let cursor = self.invalid.load(Ordering::SeqCst) == 1;
        if self.changes_during_listing.load(Ordering::SeqCst) > 0 {
            self.changes_during_listing.fetch_sub(1, Ordering::SeqCst);
            self.generation.fetch_add(1, Ordering::SeqCst);
            context
                .peer
                .notify_tool_list_changed()
                .await
                .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let schema = serde_json::json!({"type":"object","properties":{"value":{"type":if generation == 0 {"string"} else {"integer"}}}});
        let tools = if params.and_then(|p| p.cursor).is_some() {
            vec![]
        } else {
            vec![rmcp::model::Tool::new(
                "read",
                format!("generation {generation}"),
                schema.as_object().unwrap().clone(),
            )]
        };
        Ok(ListToolsResult {
            tools,
            next_cursor: cursor.then(|| "loop".into()),
            ..Default::default()
        })
    }
    async fn call_tool(
        &self,
        params: CallToolRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if params
            .arguments
            .as_ref()
            .and_then(|p| p.get("wait"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            self.call_started.notify_one();
            self.finish_call.notified().await;
        }
        Ok(CallToolResult::success(vec![Content::text("called")]))
    }
    async fn list_resources(
        &self,
        params: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        self.resource_lists.fetch_add(1, Ordering::SeqCst);
        if self.invalid.load(Ordering::SeqCst) == 5 {
            self.resource_started.notify_one();
            std::future::pending::<()>().await;
        }
        let name = format!("resource-{}", self.resources.load(Ordering::SeqCst));
        let first = params.and_then(|p| p.cursor).is_none();
        Ok(ListResourcesResult {
            resources: if first {
                vec![RawResource::new(format!("test:///{name}"), name).no_annotation()]
            } else {
                vec![]
            },
            next_cursor: (first || self.invalid.load(Ordering::SeqCst) == 2).then(|| "next".into()),
            ..Default::default()
        })
    }
    async fn list_resource_templates(
        &self,
        params: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        self.template_lists.fetch_add(1, Ordering::SeqCst);
        let name = format!("template-{}", self.resources.load(Ordering::SeqCst));
        let first = params.and_then(|p| p.cursor).is_none();
        let template =
            serde_json::from_value(serde_json::json!({"uriTemplate":"test:///{name}","name":name}))
                .unwrap();
        Ok(ListResourceTemplatesResult {
            resource_templates: if first { vec![template] } else { vec![] },
            next_cursor: (first || self.invalid.load(Ordering::SeqCst) == 3).then(|| "next".into()),
            ..Default::default()
        })
    }
    async fn list_prompts(
        &self,
        params: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        self.prompt_lists.fetch_add(1, Ordering::SeqCst);
        let name = format!("prompt-{}", self.prompts.load(Ordering::SeqCst));
        let first = params.and_then(|p| p.cursor).is_none();
        Ok(ListPromptsResult {
            prompts: if first { vec![Prompt::new(name, Some("fixture"), None)] } else { vec![] },
            next_cursor: (first || self.invalid.load(Ordering::SeqCst) == 4).then(|| "next".into()),
            ..Default::default()
        })
    }
}

struct Fixture {
    service: Service,
    server: McpServer,
    peer: Arc<tokio::sync::Mutex<Option<Peer<RoleServer>>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Fixture {
    async fn start() -> Self {
        Self::with_service(Service::default()).await
    }
    async fn with_service(service: Service) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = Arc::new(tokio::sync::Mutex::new(None));
        let running_peer = peer.clone();
        let handler = service.clone();
        let task = tokio::spawn(async move {
            // Reconnection creates a distinct service connection with the same fixture state.
            while let Ok((socket, _)) = listener.accept().await {
                let handler = handler.clone();
                let running_peer = running_peer.clone();
                tokio::spawn(async move {
                    let running = handler.serve(socket).await.unwrap();
                    *running_peer.lock().await = Some(running.peer().clone());
                    let _ = running.waiting().await;
                });
            }
        });
        let server = McpServer::Stdio(McpServerStdio::new("catalog", "python3").args(vec![
            format!("{}/testdata/plugin_compat/t20/bridge.py", env!("CARGO_MANIFEST_DIR")),
            port.to_string(),
        ]));
        Self { service, server, peer, task }
    }
    async fn changed(&self, invalid: bool) {
        self.service.generation.fetch_add(1, Ordering::SeqCst);
        self.service.invalid.store(usize::from(invalid), Ordering::SeqCst);
        let peer = self.peer.lock().await.clone().unwrap();
        peer.notify_tool_list_changed().await.unwrap();
        // Let the actual protocol notification reach its client handler before the next access.
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn tools(pool: &McpSessionConnections, server: &McpServer) -> Vec<rmcp::model::Tool> {
    pool.list_tools(server, None).await.unwrap()
}
async fn bound(pool: &Arc<McpSessionConnections>, server: &McpServer) -> McpTool {
    let defs = tools(pool, server).await;
    let spec = mcp_tools_to_specs("catalog", None, &defs, &[]).unwrap().remove(0);
    McpTool::from_session_server(server.clone(), "read", spec, pool.clone(), None)
}

#[tokio::test]
async fn codex_plugin_compat_t20_invalidates_catalog_after_notification() {
    let fixture = Fixture::start().await;
    let pool = McpSessionConnections::default();
    let first = tools(&pool, &fixture.server).await;
    let before = fixture.service.lists.load(Ordering::SeqCst);
    assert_eq!(tools(&pool, &fixture.server).await, first);
    assert_eq!(
        fixture.service.lists.load(Ordering::SeqCst),
        before,
        "unchanged connection must reuse its cached catalog"
    );
    fixture.changed(false).await;
    let next = tools(&pool, &fixture.server).await;
    assert_ne!(next, first, "the notified connection must publish its new definitions");
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_rejects_invalid_input() {
    let fixture = Fixture::start().await;
    let pool = Arc::new(McpSessionConnections::default());
    let old = bound(&pool, &fixture.server).await;
    fixture.changed(true).await;
    let error = old
        .call(serde_json::json!({"value":"old"}))
        .await
        .expect_err("an invalid notified directory must not admit old calls");
    assert!(format!("{error:#}").contains("repeated"), "{error:#}");
    assert_eq!(fixture.service.calls.load(Ordering::SeqCst), 0);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_preserves_isolation() {
    let first = Fixture::start().await;
    let second = Fixture::start().await;
    let a = Arc::new(McpSessionConnections::default());
    let b = Arc::new(McpSessionConnections::default());
    let old_a = bound(&a, &first.server).await;
    let old_b = bound(&b, &second.server).await;
    first.changed(false).await;
    assert!(
        old_a.call(serde_json::json!({"value":"old"})).await.is_err(),
        "old model binding must not invoke changed definitions"
    );
    assert_eq!(
        old_b.call(serde_json::json!({"value":"unchanged"})).await.unwrap().model_text(),
        "called"
    );
    assert_eq!(first.service.calls.load(Ordering::SeqCst), 0);
    assert_eq!(second.service.calls.load(Ordering::SeqCst), 1);
    a.shutdown();
    assert!(old_a.call(serde_json::json!({})).await.is_err());
    b.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_inspection_refreshes_only_notified_resource_and_prompt_catalogs() {
    use crate::tools::{ToolUseContext, scope_tool_use_context};
    let fixture = Fixture::start().await;
    let root = tempfile::tempdir().unwrap();
    let mut config = vw_acp::load_resolved_config(root.path()).await.unwrap();
    config.mcp_servers = vec![fixture.server.clone()];
    config.mcp_server_management.clear();
    let context =
        Arc::new(ToolUseContext::new("t20-inspect", Some(root.path().to_string_lossy().into())));
    let inspect = || {
        scope_tool_use_context(
            context.clone(),
            crate::tools::mcp_inspect::inspect_mcp_servers(&config),
        )
    };
    let first = inspect().await;
    assert_eq!(first.servers[0].resources[0]["name"], "resource-0");
    assert_eq!(first.servers[0].resource_templates[0]["name"], "template-0");
    assert_eq!(first.servers[0].prompts[0]["name"], "prompt-0");
    let counts = || {
        (
            fixture.service.resource_lists.load(Ordering::SeqCst),
            fixture.service.template_lists.load(Ordering::SeqCst),
            fixture.service.prompt_lists.load(Ordering::SeqCst),
        )
    };
    assert_eq!(counts(), (2, 2, 2), "catalogs must consume both pages");
    inspect().await;
    assert_eq!(counts(), (2, 2, 2), "unchanged catalogs must reuse the connection-local cache");
    fixture.service.resources.store(1, Ordering::SeqCst);
    let peer = fixture.peer.lock().await.clone().unwrap();
    peer.notify_resource_list_changed().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    let changed = inspect().await;
    assert_eq!(changed.servers[0].resources[0]["name"], "resource-1");
    assert_eq!(changed.servers[0].resource_templates[0]["name"], "template-1");
    assert_eq!(counts(), (4, 4, 2), "resource notification must not invalidate prompt catalogs");
    fixture.service.prompts.store(1, Ordering::SeqCst);
    peer.notify_prompt_list_changed().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert_eq!(inspect().await.servers[0].prompts[0]["name"], "prompt-1");
    assert_eq!(counts(), (4, 4, 4));
    crate::plugins::close_session_mcp_connections("t20-inspect", root.path());
}

#[tokio::test]
async fn codex_plugin_compat_t20_retries_notification_during_initial_listing_without_mixed_publication()
 {
    let service = Service::default();
    service.changes_during_listing.store(1, Ordering::SeqCst);
    let fixture = Fixture::with_service(service).await;
    let pool = McpSessionConnections::default();
    let result = tools(&pool, &fixture.server).await;
    assert_eq!(result[0].description.as_deref(), Some("generation 1"));
    assert_eq!(fixture.service.lists.load(Ordering::SeqCst), 2);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_rejects_persistent_notification_churn() {
    let service = Service::default();
    service.changes_during_listing.store(2, Ordering::SeqCst);
    let fixture = Fixture::with_service(service).await;
    let pool = McpSessionConnections::default();
    let error = pool.list_tools(&fixture.server, None).await.unwrap_err();
    assert!(format!("{error:#}").contains("changed during pagination"), "{error:#}");
    assert_eq!(fixture.service.lists.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.service.calls.load(Ordering::SeqCst), 0);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_reconnect_rejects_old_binding_without_closing_current_binding() {
    let fixture = Fixture::start().await;
    let pool = Arc::new(McpSessionConnections::default());
    let old = bound(&pool, &fixture.server).await;
    pool.with_peer(&fixture.server, None, |_| {
        Box::pin(async { Err::<(), _>(anyhow::anyhow!("fixture connection failure")) })
    })
    .await
    .unwrap_err();
    let current = bound(&pool, &fixture.server).await;
    assert!(
        old.call(serde_json::json!({})).await.is_err(),
        "a new connection must not reuse old model bindings even with equal definitions"
    );
    assert_eq!(current.call(serde_json::json!({})).await.unwrap().model_text(), "called");
    assert_eq!(fixture.service.calls.load(Ordering::SeqCst), 1);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_notification_does_not_revoke_an_already_admitted_call() {
    let fixture = Fixture::start().await;
    let pool = Arc::new(McpSessionConnections::default());
    let tool = bound(&pool, &fixture.server).await;
    let running = tool.clone();
    let call = tokio::spawn(async move { running.call(serde_json::json!({"wait":true})).await });
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        fixture.service.call_started.notified(),
    )
    .await
    .unwrap();
    fixture.changed(false).await;
    fixture.service.finish_call.notify_one();
    assert_eq!(call.await.unwrap().unwrap().model_text(), "called");
    assert!(tool.call(serde_json::json!({})).await.is_err());
    assert_eq!(fixture.service.calls.load(Ordering::SeqCst), 1);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_equal_notification_preserves_bindings_and_shutdown_closes_catalogs()
 {
    let fixture = Fixture::start().await;
    let pool = Arc::new(McpSessionConnections::default());
    let tool = bound(&pool, &fixture.server).await;
    fixture.peer.lock().await.clone().unwrap().notify_tool_list_changed().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(tool.call(serde_json::json!({})).await.is_ok());
    let (catalogs, peer) = pool
        .with_client(&fixture.server, None, |client| {
            Box::pin(async move { Ok((client.service().catalogs.clone(), client.peer().clone())) })
        })
        .await
        .unwrap();
    let before = fixture.service.lists.load(Ordering::SeqCst);
    pool.shutdown();
    assert!(
        catalogs.tools(&peer).await.is_err(),
        "cancelled catalogs cannot publish updates or return a live cache"
    );
    assert_eq!(fixture.service.lists.load(Ordering::SeqCst), before);
}

#[tokio::test]
async fn codex_plugin_compat_t20_unadvertised_changes_do_not_enable_persistent_caching() {
    let service = Service::default();
    service.unadvertised.store(1, Ordering::SeqCst);
    let fixture = Fixture::with_service(service).await;
    let pool = McpSessionConnections::default();
    let first = tools(&pool, &fixture.server).await;
    let before = fixture.service.lists.load(Ordering::SeqCst);
    assert_eq!(tools(&pool, &fixture.server).await, first);
    assert!(
        fixture.service.lists.load(Ordering::SeqCst) > before,
        "servers that cannot invalidate must be listed anew"
    );
    pool.shutdown();
    let unsupported = crate::tools::mcp_common::unsupported_mcp_capabilities_json();
    assert!(
        unsupported
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["capability"] == "resources.subscribe")
    );
    assert!(
        unsupported
            .as_array()
            .unwrap()
            .iter()
            .all(|item| !item["capability"].as_str().unwrap().ends_with("listChanged"))
    );
}

#[tokio::test]
async fn codex_plugin_compat_t20_rejects_resource_template_and_prompt_cursor_cycles() {
    for mode in [2, 3, 4] {
        let service = Service::default();
        service.invalid.store(mode, Ordering::SeqCst);
        let fixture = Fixture::with_service(service).await;
        let pool = McpSessionConnections::default();
        let error = pool
            .with_client(&fixture.server, None, move |client| {
                Box::pin(async move {
                    let catalog = &client.service().catalogs;
                    let peer = client.peer();
                    match mode {
                        2 => {
                            catalog.resources(peer).await?;
                        }
                        3 => {
                            catalog.templates(peer).await?;
                        }
                        4 => {
                            catalog.prompts(peer).await?;
                        }
                        _ => unreachable!(),
                    }
                    Ok(())
                })
            })
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("repeated pagination cursor"), "{error:#}");
        pool.shutdown();
    }
}

#[tokio::test]
async fn codex_plugin_compat_t20_shutdown_cancels_inflight_resource_refresh() {
    let service = Service::default();
    service.invalid.store(5, Ordering::SeqCst);
    let fixture = Fixture::with_service(service).await;
    let pool = Arc::new(McpSessionConnections::default());
    let active = pool.clone();
    let server = fixture.server.clone();
    let refresh = tokio::spawn(async move {
        active
            .with_client(&server, None, |client| {
                Box::pin(async move { client.service().catalogs.resources(client.peer()).await })
            })
            .await
    });
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        fixture.service.resource_started.notified(),
    )
    .await
    .unwrap();
    pool.shutdown();
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(2), refresh).await.unwrap().unwrap();
    assert!(format!("{:#}", result.unwrap_err()).contains("closed"));
    assert_eq!(fixture.service.resource_lists.load(Ordering::SeqCst), 1);
    assert!(pool.list_tools(&fixture.server, None).await.is_err());
}
