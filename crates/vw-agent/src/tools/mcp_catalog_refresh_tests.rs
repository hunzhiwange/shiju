use super::{Tool, catalog::ToolCatalog};
use crate::plugins::McpSessionConnections;
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

type RevisionProbe = Arc<std::sync::Mutex<Option<Arc<dyn Fn() -> u64 + Send + Sync>>>>;

#[derive(Clone, Default)]
pub(crate) struct Service {
    pub(crate) generation: Arc<AtomicUsize>,
    pub(crate) calls: Arc<AtomicUsize>,
    invalid: Arc<AtomicUsize>,
    pagination_changes: Arc<AtomicUsize>,
    revision: RevisionProbe,
}
impl ServerHandler for Service {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
            ..Default::default()
        }
    }
    async fn call_tool(
        &self,
        _: CallToolRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(CallToolResult::success(vec![Content::text("refreshed MCP call")]))
    }
    async fn list_tools(
        &self,
        params: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let generation = self.generation.load(Ordering::SeqCst);
        let second_page = params.and_then(|params| params.cursor).is_some();
        if second_page && self.pagination_changes.load(Ordering::SeqCst) > 0 {
            self.pagination_changes.fetch_sub(1, Ordering::SeqCst);
            let revision = self.revision.lock().unwrap().as_ref().unwrap().clone();
            let before = revision();
            context.peer.notify_tool_list_changed().await.unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                while revision() == before {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        }
        if self.invalid.load(Ordering::SeqCst) != 0 {
            return Err(ErrorData::internal_error("fixture list failure", None));
        }
        let schema = serde_json::json!({"type":"object","properties":{"value":{"type":if generation == 0 {"string"} else {"integer"}}}});
        let names = match generation {
            _ if !second_page => vec!["read"],
            0 => vec!["removed"],
            _ => vec!["added"],
        };
        Ok(ListToolsResult {
            tools: names
                .into_iter()
                .map(|name| {
                    rmcp::model::Tool::new(
                        name,
                        format!("generation {generation}"),
                        schema.as_object().unwrap().clone(),
                    )
                })
                .collect(),
            next_cursor: (!second_page).then(|| "second".into()),
            ..Default::default()
        })
    }
}
pub(crate) struct Fixture {
    pub(crate) service: Service,
    server: McpServer,
    pub(crate) pool: Arc<McpSessionConnections>,
    peer: Arc<tokio::sync::Mutex<Option<Peer<RoleServer>>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Fixture {
    pub(crate) async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let service = Service::default();
        let peer = Arc::new(tokio::sync::Mutex::new(None));
        let handler = service.clone();
        let saved = peer.clone();
        let task = tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let running = handler.clone().serve(socket).await.unwrap();
                *saved.lock().await = Some(running.peer().clone());
                tokio::spawn(async move {
                    let _ = running.waiting().await;
                });
            }
        });
        let server = McpServer::Stdio(McpServerStdio::new("catalog", "python3").args(vec![
            format!("{}/testdata/plugin_compat/t20/bridge.py", env!("CARGO_MANIFEST_DIR")),
            port.to_string(),
        ]));
        let pool = Arc::new(McpSessionConnections::default());
        pool.synchronize(std::slice::from_ref(&server)).unwrap();
        let weak = Arc::downgrade(&pool);
        let observed_server = server.clone();
        *service.revision.lock().unwrap() = Some(Arc::new(move || {
            weak.upgrade().and_then(|pool| pool.tool_revision(&observed_server)).unwrap_or(u64::MAX)
        }));
        Self { service, server, pool, peer, task }
    }
    pub(crate) async fn catalog(&self) -> Arc<ToolCatalog> {
        let tools = self
            .pool
            .discover_catalog_tools(
                &self.server,
                vw_acp::McpServerManagement::default_for_server("catalog"),
                &[],
            )
            .await
            .unwrap();
        let ids = tools.iter().map(|tool| tool.spec().id).collect::<Vec<_>>();
        let catalog = Arc::new(ToolCatalog::default());
        catalog.replace(tools, ids).unwrap();
        catalog
    }
    pub(crate) async fn changed(&self) {
        let revision = self.pool.tool_revision(&self.server).unwrap();
        self.peer.lock().await.as_ref().unwrap().notify_tool_list_changed().await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while self.pool.tool_revision(&self.server) == Some(revision) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.pool.shutdown();
        self.task.abort();
    }
}

#[tokio::test]
async fn dynamic_tools_mcp_refresh_notification_updates_next_request() {
    let fixture = Fixture::start().await;
    let catalog = fixture.catalog().await;
    let old = catalog.snapshot();
    fixture.service.generation.store(1, Ordering::SeqCst);
    fixture.changed().await;
    assert!(Arc::ptr_eq(&old, &catalog.snapshot()), "notification only invalidates");
    fixture.pool.refresh_tool_catalog(&catalog).await.unwrap();
    let next = catalog.snapshot();
    assert!(next.version() > old.version());
    assert_eq!(old.specs()[0].input_schema["properties"]["value"]["type"], "string");
    assert_eq!(next.specs()[0].input_schema["properties"]["value"]["type"], "integer");
    assert!(!next.authorizes("mcp__catalog__removed"));
    assert!(!next.authorizes("mcp__catalog__added"), "notifications cannot grant a new stable ID");
    assert!(next.bind("mcp__catalog__read").is_ok());
}

#[tokio::test]
async fn dynamic_tools_mcp_refresh_failure_retains_old_catalog_and_retries() {
    let fixture = Fixture::start().await;
    let catalog = fixture.catalog().await;
    let old = catalog.snapshot();
    fixture.service.invalid.store(1, Ordering::SeqCst);
    fixture.changed().await;
    assert!(fixture.pool.refresh_tool_catalog(&catalog).await.is_err());
    assert!(Arc::ptr_eq(&old, &catalog.snapshot()));
    fixture.service.invalid.store(0, Ordering::SeqCst);
    fixture.service.generation.store(1, Ordering::SeqCst);
    fixture.pool.refresh_tool_catalog(&catalog).await.unwrap();
    assert!(catalog.snapshot().version() > old.version());
}

#[tokio::test]
async fn dynamic_tools_mcp_refresh_closed_connection_never_revives_tools() {
    let fixture = Fixture::start().await;
    let catalog = fixture.catalog().await;
    fixture.pool.shutdown();
    assert!(fixture.pool.refresh_tool_catalog(&catalog).await.is_err());
}

#[tokio::test]
async fn dynamic_tools_mcp_refresh_pagination_changes_never_publish_mixed_catalog() {
    let fixture = Fixture::start().await;
    let catalog = fixture.catalog().await;
    let old = catalog.snapshot();
    fixture.service.pagination_changes.store(2, Ordering::SeqCst);
    fixture.changed().await;
    assert!(fixture.pool.refresh_tool_catalog(&catalog).await.is_err());
    assert!(Arc::ptr_eq(&old, &catalog.snapshot()));
    fixture.pool.refresh_tool_catalog(&catalog).await.unwrap();
    assert!(catalog.snapshot().version() > old.version());
}

#[tokio::test]
async fn dynamic_tools_mcp_refresh_unchanged_catalog_keeps_version() {
    let fixture = Fixture::start().await;
    let catalog = fixture.catalog().await;
    let old = catalog.snapshot();
    fixture.pool.refresh_tool_catalog(&catalog).await.unwrap();
    assert!(Arc::ptr_eq(&old, &catalog.snapshot()));
}

#[tokio::test]
async fn dynamic_tools_mcp_refresh_applies_current_server_authorization() {
    let fixture = Fixture::start().await;
    let catalog = fixture.catalog().await;
    let mut management = vw_acp::McpServerManagement::default_for_server("catalog");
    management.denied_tools.push("removed".into());
    fixture.pool.register_tool_source(fixture.server.clone(), management);
    fixture.pool.refresh_tool_catalog(&catalog).await.unwrap();
    assert!(catalog.snapshot().authorizes("mcp__catalog__read"));
    assert!(!catalog.snapshot().authorizes("mcp__catalog__removed"));
}

#[tokio::test]
async fn dynamic_tools_mcp_refresh_addition_requires_explicit_authorization() {
    let fixture = Fixture::start().await;
    let catalog = fixture.catalog().await;
    let mut management = vw_acp::McpServerManagement::default_for_server("catalog");
    management.allowed_tools = vec!["read".into(), "removed".into(), "added".into()];
    fixture.pool.register_tool_source(fixture.server.clone(), management);
    fixture.service.generation.store(1, Ordering::SeqCst);
    fixture.changed().await;
    fixture.pool.refresh_tool_catalog(&catalog).await.unwrap();
    assert!(catalog.snapshot().authorizes("mcp__catalog__added"));
    assert!(!catalog.snapshot().authorizes("mcp__catalog__removed"));
    let snapshot = catalog.snapshot();
    let tool = snapshot.tools().iter().find(|tool| tool.name() == "mcp__catalog__added").unwrap();
    assert_eq!(
        tool.call(serde_json::json!({"value":1})).await.unwrap().model_text(),
        "refreshed MCP call"
    );
}

#[tokio::test]
async fn dynamic_tools_mcp_refresh_disabled_source_does_not_reconnect() {
    let fixture = Fixture::start().await;
    let catalog = fixture.catalog().await;
    fixture.pool.synchronize(&[]).unwrap();
    fixture.pool.refresh_tool_catalog(&catalog).await.unwrap();
    assert!(catalog.snapshot().specs().is_empty());
    assert!(fixture.pool.tool_revision(&fixture.server).is_none());
    fixture.pool.refresh_tool_catalog(&catalog).await.unwrap();
    assert!(catalog.snapshot().specs().is_empty());
}
