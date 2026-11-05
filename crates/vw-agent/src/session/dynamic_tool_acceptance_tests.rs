//! Real HTTP gateway, SDK, SQLite and client executor. Model output alone is scripted.
use super::dynamic_tool_store::PersistentToolPolicy;
use super::runtime::{CreateSessionInput, SessionScope, create_session};
use crate::tools::{ToolSpec, ToolUseContext};
use serde_json::json;
use std::sync::Arc;
use vw_api_types::{
    id::TenantScope,
    tools::{
        DynamicToolContent, DynamicToolHostBinding, DynamicToolRegistration, DynamicToolResponse,
        ReplaceDynamicToolsRequest, ToolSource,
    },
};
use vw_gateway_client::GatewayClient;
use vw_gateway_client::GatewayEndpoint;

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
                vw_gateway_client::GatewayAuth { skey: Some("dynamic-acceptance-skey".into()) },
            ),
        )
        .unwrap();
        Self { directory, scope, session, client, server, _config: config, _env: env }
    }
    fn directory(&self) -> &str {
        self.directory.path().to_str().unwrap()
    }
    async fn register(&self) -> DynamicToolHostBinding {
        self.client.register_dynamic_tool_host(&self.session, Some(self.directory())).await.unwrap()
    }
    async fn replace(
        &self,
        binding: &DynamicToolHostBinding,
        name: &str,
        persist: bool,
    ) -> Result<u64, String> {
        self.client
            .replace_dynamic_tools(
                &self.session,
                Some(self.directory()),
                binding,
                &replacement(name, persist),
            )
            .await
            .map(|r| r.version)
    }
    fn entry(
        &self,
        binding: &DynamicToolHostBinding,
    ) -> Arc<super::runtime::dynamic_hosts::SessionHost> {
        super::runtime::dynamic_hosts::SessionHost::authenticate(
            &self.scope,
            &self.session,
            &binding.binding_token,
        )
        .unwrap()
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
fn replacement(name: &str, persist: bool) -> ReplaceDynamicToolsRequest {
    let tools = if name.is_empty() {
        vec![]
    } else {
        let mut spec = ToolSpec::new(name, name, json!({"type":"object","properties":{}}));
        spec.source = ToolSource::Client;
        spec.exposure = vw_api_types::tools::ToolExposure::Deferred;
        spec.read_only = true;
        vec![DynamicToolRegistration::Function(spec.to_dto())]
    };
    ReplaceDynamicToolsRequest { tools, persist }
}
fn response() -> DynamicToolResponse {
    DynamicToolResponse {
        success: true,
        content: vec![DynamicToolContent::Text { text: "host result".into() }],
    }
}

#[tokio::test]
async fn dynamic_tools_acceptance_replacement_survives_resume() {
    let f = Fixture::new(true).await;
    let binding = f.register().await;
    assert!(f.replace(&binding, "release", true).await.unwrap() > 0);
    let info = super::runtime::validate_session_scope(f.scope.clone(), &f.session).await.unwrap();
    super::ui_store::with_tenant_scope_sync(f.scope.tenant().clone(), || {
        super::ui_store::with_dynamic_tool_store(&info.project_id, |store, scope| {
            let mut ui = store.load_session(scope, &f.session)?.unwrap();
            ui.title = "UI title".into();
            store.save_session(scope, &ui)
        })
        .unwrap()
    });
    assert!(f.replace(&binding, "deployment", true).await.unwrap() > 1);
    super::ui_store::with_tenant_scope_sync(f.scope.tenant().clone(), || {
        super::ui_store::with_dynamic_tool_store(&info.project_id, |store, scope| {
            assert_eq!(store.load_session_meta(scope, &f.session)?.unwrap().title, "UI title");
            Ok(())
        })
        .unwrap()
    });
    let old_entry = f.entry(&binding);
    let (stream, mut receive) = host_stream(&f, &binding).await;
    let cancelled = execute(&f, &binding, "deployment").await;
    let _request = next_call(&mut receive).await;
    stream.abort();
    let _ = stream.await;
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(20), cancelled)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert!(old_entry.replace(replacement("forged-after-disconnect", true)).is_err());
    let loaded = super::runtime::load_session(f.scope.clone(), &f.session).await.unwrap();
    let saved = loaded.dynamic_tools.unwrap();
    assert_eq!(saved.specs[0].id, "deployment");
    let rebound = f.register().await;
    assert_ne!(binding.binding_token, rebound.binding_token);
    assert!(
        f.entry(&rebound).catalog.snapshot().specs().is_empty(),
        "saved metadata grants no execution"
    );
    f.replace(&rebound, "deployment", false).await.unwrap();
    assert!(f.entry(&rebound).catalog.snapshot().bind("deployment").is_ok());
    assert!(f.entry(&rebound).catalog.snapshot().bind("release").is_err());
    assert!(
        f.client
            .replace_dynamic_tools(
                &f.session,
                Some(f.directory()),
                &binding,
                &replacement("release", false)
            )
            .await
            .unwrap_err()
            .contains("403")
    );
    let (stream, mut receive) = host_stream(&f, &rebound).await;
    let call = execute(&f, &rebound, "deployment").await;
    let request = next_call(&mut receive).await;
    f.client
        .respond_dynamic_tool_call(
            &f.session,
            Some(f.directory()),
            &rebound,
            &request.call_id,
            &response(),
        )
        .await
        .unwrap();
    assert!(call.await.unwrap().unwrap().result.is_success());
    stream.abort();
    let _ = stream.await;
}

async fn host_stream(
    f: &Fixture,
    binding: &DynamicToolHostBinding,
) -> (
    tokio::task::JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<vw_api_types::tools::DynamicToolCallRequest>,
) {
    let (send, receive) = tokio::sync::mpsc::unbounded_channel();
    let client = f.client.clone();
    let session = f.session.clone();
    let directory = f.directory().to_owned();
    let binding = binding.clone();
    let task = tokio::spawn(async move {
        let _ = client
            .stream_dynamic_tool_calls(&session, Some(&directory), &binding, |call| {
                send.send(call).is_ok()
            })
            .await;
    });
    (task, receive)
}
async fn next_call(
    receive: &mut tokio::sync::mpsc::UnboundedReceiver<vw_api_types::tools::DynamicToolCallRequest>,
) -> vw_api_types::tools::DynamicToolCallRequest {
    tokio::time::timeout(std::time::Duration::from_secs(20), receive.recv()).await.unwrap().unwrap()
}
async fn execute(
    f: &Fixture,
    binding: &DynamicToolHostBinding,
    name: &str,
) -> tokio::task::JoinHandle<Result<crate::tools::ExecutedToolCall, crate::tools::ToolCallError>> {
    let snapshot = f.entry(binding).catalog.snapshot();
    let call = snapshot.bind(name).unwrap();
    let context = f.context();
    tokio::spawn(async move { call.execute(&snapshot, json!({}), context).await })
}

#[tokio::test]
async fn dynamic_tools_acceptance_default_policy_and_cross_session_are_denied() {
    let f = Fixture::new(false).await;
    let binding = f.register().await;
    assert!(f.replace(&binding, "release", true).await.unwrap_err().contains("disabled"));
    assert!(f.entry(&binding).catalog.snapshot().specs().is_empty());
    f.replace(&binding, "release", false).await.unwrap();
    let other = create_session(CreateSessionInput {
        scope: f.scope.clone(),
        parent_id: None,
        title: None,
        permission: None,
    })
    .await
    .unwrap();
    assert!(
        f.client
            .replace_dynamic_tools(
                &other.info.id,
                Some(f.directory()),
                &binding,
                &replacement("release", false)
            )
            .await
            .unwrap_err()
            .contains("403")
    );
    let wrong_tenant = GatewayClient::new(
        f.client
            .endpoint()
            .clone()
            .with_tenant_scope(TenantScope::new("acceptance", "other").unwrap()),
    )
    .unwrap();
    assert!(
        wrong_tenant
            .register_dynamic_tool_host(&f.session, Some(f.directory()))
            .await
            .err()
            .unwrap()
            .contains("404")
    );
    drop(f.entry(&binding).calls().unwrap());
}

#[tokio::test]
async fn dynamic_tools_acceptance_replacement_cancellation_and_revocation() {
    let f = Fixture::new(true).await;
    let binding = f.register().await;
    f.replace(&binding, "release", true).await.unwrap();
    let (stream, mut receive) = host_stream(&f, &binding).await;
    let call = execute(&f, &binding, "release").await;
    let request = next_call(&mut receive).await;
    assert_eq!(request.tool_id.as_ref(), "release");
    // Old pending execution remains answerable while the next request sees the replacement.
    f.replace(&binding, "deployment", true).await.unwrap();
    assert!(f.entry(&binding).catalog.snapshot().bind("release").is_err());
    f.client
        .respond_dynamic_tool_call(
            &f.session,
            Some(f.directory()),
            &binding,
            &request.call_id,
            &response(),
        )
        .await
        .unwrap();
    assert!(call.await.unwrap().unwrap().result.is_success());
    assert!(
        f.client
            .respond_dynamic_tool_call(
                &f.session,
                Some(f.directory()),
                &binding,
                &request.call_id,
                &response()
            )
            .await
            .unwrap_err()
            .contains("403")
    );
    let cancelled = execute(&f, &binding, "deployment").await;
    let request = next_call(&mut receive).await;
    cancelled.abort();
    assert!(cancelled.await.unwrap_err().is_cancelled());
    assert!(
        f.client
            .respond_dynamic_tool_call(
                &f.session,
                Some(f.directory()),
                &binding,
                &request.call_id,
                &response()
            )
            .await
            .unwrap_err()
            .contains("403")
    );
    f.replace(&binding, "", true).await.unwrap();
    assert!(f.entry(&binding).catalog.snapshot().bind("deployment").is_err());
    stream.abort();
    let _ = stream.await;
}

#[tokio::test]
async fn dynamic_tools_acceptance_fork_requires_new_host_and_matching_specs() {
    let f = Fixture::new(true).await;
    let binding = f.register().await;
    f.replace(&binding, "release", true).await.unwrap();
    let fork: serde_json::Value =
        f.client.session_fork(&f.session, Some(f.directory()), &None).await.unwrap();
    let fork = fork["id"].as_str().unwrap();
    assert!(
        f.client
            .replace_dynamic_tools(
                fork,
                Some(f.directory()),
                &binding,
                &replacement("release", false)
            )
            .await
            .unwrap_err()
            .contains("403")
    );
    let fork_binding =
        f.client.register_dynamic_tool_host(fork, Some(f.directory())).await.unwrap();
    let fork_entry = super::runtime::dynamic_hosts::SessionHost::authenticate(
        &f.scope,
        fork,
        &fork_binding.binding_token,
    )
    .unwrap();
    assert!(fork_entry.catalog.snapshot().bind("release").is_err());
    assert!(
        f.client
            .replace_dynamic_tools(
                fork,
                Some(f.directory()),
                &fork_binding,
                &replacement("changed", false)
            )
            .await
            .is_err()
    );
    f.client
        .replace_dynamic_tools(
            fork,
            Some(f.directory()),
            &fork_binding,
            &replacement("release", false),
        )
        .await
        .unwrap();
    assert!(fork_entry.catalog.snapshot().bind("release").is_ok());
    assert!(
        f.client
            .respond_dynamic_tool_call(fork, Some(f.directory()), &binding, "forged", &response())
            .await
            .unwrap_err()
            .contains("403")
    );
    let (stream, mut receive) = host_stream(&f, &binding).await;
    let call = execute(&f, &binding, "release").await;
    let request = next_call(&mut receive).await;
    assert!(
        f.client
            .respond_dynamic_tool_call(
                fork,
                Some(f.directory()),
                &fork_binding,
                &request.call_id,
                &response()
            )
            .await
            .unwrap_err()
            .contains("403")
    );
    f.client
        .respond_dynamic_tool_call(
            &f.session,
            Some(f.directory()),
            &binding,
            &request.call_id,
            &response(),
        )
        .await
        .unwrap();
    assert!(call.await.unwrap().unwrap().result.is_success());
    stream.abort();
    let _ = stream.await;
    drop(fork_entry.calls().unwrap());
}

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
            f.client.respond_dynamic_tool_call(&f.session, Some(f.directory()), &binding, &request.call_id, &response()).await.unwrap();
        };
        let body = serde_json::from_value(json!({"session_id":f.session,"model":"model-recovery/original","allowed_tools":["tool_search","release","deployment","read"],"messages":[{"role":"user","content":"perform action"}],"options":{"allowed_tools":["tool_search","release","deployment","read"],"full_access":true}})).unwrap();
        let mut events = Vec::new();
        let chat = GatewayClient::stream_chat(f.client.endpoint(), Some(f.directory()), &body, |event| { events.push(event); true });
        let (_, result) = tokio::time::timeout(std::time::Duration::from_secs(30), async { tokio::join!(host, chat) }).await.unwrap();
        result.unwrap();
        assert!(events.iter().any(|event| matches!(event, vw_api_types::chat::GatewayChatStreamEvent::Delta(text) if text.contains("finished"))), "chat events: {events:?}");
        assert!(!events.iter().any(|event| matches!(event, vw_api_types::chat::GatewayChatStreamEvent::Error(_))), "chat errors: {events:?}");
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
