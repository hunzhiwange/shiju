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
#[tokio::test]
async fn codex_plugin_compat_t20_inspection_refreshes_only_notified_resource_and_prompt_catalogs() {
    use crate::tools::{ToolUseContext, scope_tool_use_context};
    let fixture = Fixture::start().await;
    let root = tempfile::tempdir().unwrap();
    let mut config = acp_config::load_resolved_config(root.path()).await.unwrap();
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
