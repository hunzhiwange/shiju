//! Real HTTP gateway, SDK, SQLite and client executor. Model output alone is scripted.
use super::dynamic_tool_store::PersistentToolPolicy;
use super::runtime::{CreateSessionInput, SessionScope, create_session};
use crate::tools::ToolUseContext;
use serde_json::json;
use std::sync::Arc;
use {
    vw_api_transport::id::TenantScope,
};
use vw_gateway_client::GatewayClient;
use gateway_endpoint::GatewayEndpoint;

struct Fixture {
    directory: tempfile::TempDir,
    scope: SessionScope,
    session: String,
    client: GatewayClient,
    server: tokio::task::JoinHandle<()>,
    _config: super::llm::recovery_test_support::RecoveryConfig,
    _env: tokio::sync::MutexGuard<'static, ()>,
}
impl Fixture {
    async fn new(trusted: bool) -> Self {
        let env = crate::test_support::global_async_env_lock().lock().await;
        let config =
            super::llm::recovery_test_support::RecoveryConfig::new("https://example.test/v1", None)
                .await;
        let directory = tempfile::tempdir().unwrap();
        let tenant = TenantScope::new("acceptance", uuid::Uuid::new_v4().to_string()).unwrap();
        crate::config::update_global_for_tenant(&tenant, json!({"providers":{"model-recovery":{"api":"https://example.test/v1","models":{"original":{"status":"active","tool_call":true}}}}})).await.unwrap();
        let scope = SessionScope::new(tenant.clone(), directory.path()).unwrap();
        let session = create_session(CreateSessionInput {
            scope: scope.clone(),
            parent_id: None,
            title: None,
            permission: None,
        })
        .await
        .unwrap()
        .info
        .id;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut router = crate::gateway::dynamic_tools_acceptance_router();
        if trusted {
            router = router.layer(axum::Extension(PersistentToolPolicy::trusted_host()));
        }
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let client = GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(tenant).with_auth(
                gateway_endpoint::GatewayAuth { skey: Some("dynamic-acceptance-skey".into()) },
            ),
        )
        .unwrap();
        Self { directory, scope, session, client, server, _config: config, _env: env }
    }
    fn directory(&self) -> &str {
        self.directory.path().to_str().unwrap()
    }
    fn context(&self) -> Arc<ToolUseContext> {
        super::ui_store::with_tenant_scope_sync(self.scope.tenant().clone(), || {
            Arc::new(
                ToolUseContext::new(self.session.clone(), Some(self.directory().into()))
                    .with_message_id("turn"),
            )
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
use session_engine::dynamic_tool_acceptance::{self as contracts, Fixture as _, replacement, response, host_stream, execute, next_call};
impl contracts::Fixture for Fixture {
    type Context = crate::tools::context::AgentContextHost;
    type Tools = super::runtime::dynamic_hosts::HostTools;
    async fn new(trusted: bool) -> Self { Fixture::new(trusted).await }
    fn directory(&self) -> &str { self.directory() }
    fn session(&self) -> &str { &self.session }
    fn scope(&self) -> &SessionScope { &self.scope }
    fn transport(&self) -> gateway_http::GatewayTransport { self.client.transport().clone() }
    fn context(&self) -> Arc<ToolUseContext> { self.context() }
    fn wrong_tenant_transport(&self) -> gateway_http::GatewayTransport {
        GatewayClient::new(self.client.endpoint().clone().with_tenant_scope(TenantScope::new("acceptance", "other").unwrap())).unwrap().transport().clone()
    }
    async fn update_ui_title(&self) {
        let info = super::runtime::validate_session_scope(self.scope.clone(), &self.session).await.unwrap();
        super::ui_store::with_tenant_scope_sync(self.scope.tenant().clone(), || {
            super::ui_store::with_dynamic_tool_store(&info.project_id, |store, scope| {
                let mut ui = store.load_session(scope, &self.session)?.unwrap();
                ui.title = "UI title".into(); store.save_session(scope, &ui)
            }).unwrap()
        });
    }
    async fn assert_ui_title(&self) {
        let info = super::runtime::validate_session_scope(self.scope.clone(), &self.session).await.unwrap();
        super::ui_store::with_tenant_scope_sync(self.scope.tenant().clone(), || {
            super::ui_store::with_dynamic_tool_store(&info.project_id, |store, scope| {
                assert_eq!(store.load_session_meta(scope, &self.session)?.unwrap().title, "UI title"); Ok(())
            }).unwrap()
        });
    }
    async fn saved_tools(&self) -> session_store::dynamic_tool_store::PersistedDynamicTools {
        super::runtime::load_session(self.scope.clone(), &self.session).await.unwrap().dynamic_tools.unwrap()
    }
    async fn other_session(&self) -> String {
        create_session(CreateSessionInput { scope: self.scope.clone(), parent_id: None, title: None, permission: None }).await.unwrap().info.id
    }
    async fn fork_session(&self) -> String {
        let fork: serde_json::Value = gateway_session::GatewaySessionClient::new(self.client.transport().clone()).session_fork(&self.session, Some(self.directory()), &None).await.unwrap();
        fork["id"].as_str().unwrap().to_owned()
    }
}
#[tokio::test]
async fn dynamic_tools_acceptance_replacement_survives_resume() { contracts::dynamic_tools_acceptance_replacement_survives_resume::<Fixture>().await; }
#[tokio::test]
async fn dynamic_tools_acceptance_default_policy_and_cross_session_are_denied() { contracts::dynamic_tools_acceptance_default_policy_and_cross_session_are_denied::<Fixture>().await; }
#[tokio::test]
async fn dynamic_tools_acceptance_replacement_cancellation_and_revocation() { contracts::dynamic_tools_acceptance_replacement_cancellation_and_revocation::<Fixture>().await; }
#[tokio::test]
async fn dynamic_tools_acceptance_fork_requires_new_host_and_matching_specs() { contracts::dynamic_tools_acceptance_fork_requires_new_host_and_matching_specs::<Fixture>().await; }
#[test]
fn dynamic_tools_acceptance_search_then_call_uses_gateway_model_boundary() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let f = runtime.block_on(Fixture::new(true));
    let _streams = super::processor::test_llm_stream_guard();
    use super::llm::{StreamEvent, ToolCall};
    for (id, name, arguments) in
        [("search", "tool_search", json!({"query":"release"})), ("invoke", "release", json!({}))]
    {
        super::processor::enqueue_test_llm_stream(vec![
            StreamEvent::ToolCalls(vec![ToolCall {
                kind: Default::default(),
                namespace: None,
                id: id.into(),
                name: name.into(),
                arguments: arguments.to_string(),
            }]),
            StreamEvent::Done {
                usage: Default::default(),
                finish_reason: Some("tool_calls".into()),
            },
        ]);
    }
    super::processor::enqueue_test_llm_stream(vec![
        StreamEvent::Delta("finished".into()),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("stop".into()) },
    ]);
    runtime.block_on(async {
        let binding = f.register().await;
        f.replace(&binding, "release", true).await.unwrap();
        let (stream, mut receive) = host_stream(&f, &binding).await;
        let host = async {
            let request = next_call(&mut receive).await;
            assert_eq!(request.tool_id.as_ref(), "release");
            f.replace(&binding, "deployment", true).await.unwrap();
            gateway_host_tools::GatewayHostToolsClient::new(f.client.transport().clone()).respond_dynamic_tool_call(&f.session, Some(f.directory()), &binding, &request.call_id, &response()).await.unwrap();
        };
        let body = serde_json::from_value(json!({"session_id":f.session,"model":"model-recovery/original","allowed_tools":["tool_search","release","deployment","read"],"messages":[{"role":"user","content":"perform action"}],"options":{"allowed_tools":["tool_search","release","deployment","read"],"full_access":true}})).unwrap();
        let mut events = Vec::new();
        let chat = GatewayClient::stream_chat(f.client.endpoint(), Some(f.directory()), &body, |event| { events.push(event); true });
        let (_, result) = tokio::time::timeout(std::time::Duration::from_secs(30), async { tokio::join!(host, chat) }).await.unwrap();
        result.unwrap();
        assert!(events.iter().any(|event| matches!(event, vw_api_chat::chat::GatewayChatStreamEvent::Delta(text) if text.contains("finished"))), "chat events: {events:?}");
        assert!(!events.iter().any(|event| matches!(event, vw_api_chat::chat::GatewayChatStreamEvent::Error(_))), "chat errors: {events:?}");
        let requests = super::processor::take_test_llm_messages(&f.session);
        assert_eq!(requests.len(), 3);
        assert!(!requests[0].tool_specs.iter().any(|s| s.id == "release"));
        assert!(requests[0].tool_specs.iter().any(|s| s.id == "tool_search"));
        assert!(requests[1].tool_specs.iter().any(|s| s.id == "release"));
        assert!(requests[1].messages.iter().any(|m| m["tool_call_id"] == "search"));
        assert!(requests[2].messages.iter().any(|m| m.to_string().contains("host result")));
        assert!(!requests[2].tool_specs.iter().any(|s| s.id == "release"));
        assert!(requests[0].tool_specs.iter().any(|s| s.id != "tool_search"), "existing runtime tools survive overlay");
        stream.abort();
        let _ = stream.await;
    });
}
