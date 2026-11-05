//! Real session storage, HTTP/SSE and ordinary tool execution; only the model is scripted.
use crate::session::runtime::{
    CreateSessionInput, SessionScope, create_session, dynamic_hosts::SessionHost,
};
use crate::tools::{ToolSpec, ToolUseContext, catalog::ToolCatalog};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;
use vw_api_types::{
    id::TenantScope,
    tools::{DynamicToolHostBinding, DynamicToolResponse, ReplaceDynamicToolsRequest, ToolSource},
};

struct Fixture {
    directory: tempfile::TempDir,
    scope: SessionScope,
    session: String,
    router: Router,
    _config: crate::session::llm::recovery_test_support::RecoveryConfig,
    _env: tokio::sync::MutexGuard<'static, ()>,
}
impl Fixture {
    async fn new() -> Self {
        let env = crate::test_support::global_async_env_lock().lock().await;
        let config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        )
        .await;
        let directory = tempfile::tempdir().unwrap();
        let tenant = TenantScope::new("dynamic-gateway", uuid::Uuid::new_v4().to_string()).unwrap();
        crate::config::update_global_for_tenant(&tenant, json!({"providers":{"model-recovery":{"api":"https://example.test/v1","models":{"original":{"status":"active","tool_call":true}}}}})).await.unwrap();
        let scope = SessionScope::new(tenant, directory.path()).unwrap();
        let created = create_session(CreateSessionInput {
            scope: scope.clone(),
            parent_id: None,
            title: None,
            permission: None,
        })
        .await
        .unwrap();
        let router = crate::gateway::router::build_router(Vec::new());
        Self { directory, scope, session: created.info.id, router, _config: config, _env: env }
    }
    fn request(
        &self,
        method: &str,
        session: &str,
        suffix: &str,
        token: Option<&str>,
        payload: Value,
    ) -> Request<Body> {
        let mut request = Request::builder()
            .method(method)
            .uri(format!("/session/{session}/dynamic-tools{suffix}"))
            .header("X-Platform-ID", self.scope.tenant().platform_id().as_ref())
            .header("X-Company-ID", self.scope.tenant().company_id().as_ref())
            .header("x-vibewindow-directory", self.directory.path().to_str().unwrap())
            .header("content-type", "application/json");
        if let Some(token) = token {
            request = request.header("x-vibewindow-tool-host", token);
        }
        request.body(Body::from(payload.to_string())).unwrap()
    }
    async fn send(
        &self,
        method: &str,
        session: &str,
        suffix: &str,
        token: Option<&str>,
        payload: Value,
    ) -> axum::response::Response {
        self.router
            .clone()
            .oneshot(self.request(method, session, suffix, token, payload))
            .await
            .unwrap()
    }
    async fn register(&self) -> DynamicToolHostBinding {
        let response = self.send("POST", &self.session, "/hosts", None, json!({})).await;
        assert_eq!(response.status(), StatusCode::OK);
        serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 4096).await.unwrap())
            .unwrap()
    }
    fn context(&self) -> Arc<ToolUseContext> {
        crate::session::ui_store::with_tenant_scope_sync(self.scope.tenant().clone(), || {
            Arc::new(
                ToolUseContext::new(
                    self.session.clone(),
                    Some(self.directory.path().to_string_lossy().into()),
                )
                .with_message_id("turn"),
            )
        })
    }
    fn host(&self, binding: &DynamicToolHostBinding) -> Arc<SessionHost> {
        SessionHost::authenticate(&self.scope, &self.session, &binding.binding_token).unwrap()
    }
}
fn registration() -> Value {
    let mut spec = ToolSpec::new(
        "host_echo",
        "Echo",
        json!({"type":"object","properties":{},"additionalProperties":true}),
    );
    spec.source = ToolSource::Client;
    spec.read_only = true;
    serde_json::to_value(vw_api_types::tools::DynamicToolRegistration::Function(spec.to_dto()))
        .unwrap()
}
fn replacement() -> Value {
    json!({"tools":[registration()]})
}

#[tokio::test]
async fn dynamic_tools_gateway_cross_tenant_registration_is_denied() {
    let f = Fixture::new().await;
    let mut request = f.request("POST", &f.session, "/hosts", None, json!({}));
    request.headers_mut().insert("X-Company-ID", "other".parse().unwrap());
    assert_eq!(f.router.clone().oneshot(request).await.unwrap().status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn dynamic_tools_gateway_scope_and_host_updates_are_denied() {
    let f = Fixture::new().await;
    let mut missing_tenant = f.request("POST", &f.session, "/hosts", None, json!({}));
    missing_tenant.headers_mut().remove("X-Company-ID");
    assert_eq!(
        f.router.clone().oneshot(missing_tenant).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    let binding = f.register().await;
    assert_eq!(
        f.send("POST", &f.session, "/hosts", None, json!({})).await.status(),
        StatusCode::CONFLICT
    );
    for token in [None, Some("forged-host")] {
        assert_eq!(
            f.send("PUT", &f.session, "", token, replacement()).await.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            f.send("GET", &f.session, "/calls", token, json!({})).await.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            f.send(
                "POST",
                &f.session,
                "/calls/fake/response",
                token,
                json!({"success":true,"content":[]})
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
    }
    let directory = tempfile::tempdir().unwrap();
    let mut wrong_directory =
        f.request("PUT", &f.session, "", Some(&binding.binding_token), replacement());
    wrong_directory
        .headers_mut()
        .insert("x-vibewindow-directory", directory.path().to_str().unwrap().parse().unwrap());
    assert_eq!(
        f.router.clone().oneshot(wrong_directory).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    let other = create_session(CreateSessionInput {
        scope: f.scope.clone(),
        parent_id: None,
        title: None,
        permission: None,
    })
    .await
    .unwrap();
    assert_eq!(
        f.send("PUT", &other.info.id, "", Some(&binding.binding_token), replacement())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let mut wrong_tenant =
        f.request("PUT", &f.session, "", Some(&binding.binding_token), replacement());
    wrong_tenant.headers_mut().insert("X-Platform-ID", "other".parse().unwrap());
    assert_eq!(
        f.router.clone().oneshot(wrong_tenant).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    assert!(f.host(&binding).catalog.snapshot().specs().is_empty());
    drop(f.host(&binding).calls().unwrap());
}

#[tokio::test]
async fn dynamic_tools_gateway_replacement_is_atomic_and_policy_cannot_be_elevated() {
    let f = Fixture::new().await;
    let binding = f.register().await;
    let token = Some(binding.binding_token.as_str());
    assert_eq!(f.send("PUT", &f.session, "", token, replacement()).await.status(), StatusCode::OK);
    let old = f.host(&binding).catalog.snapshot();
    let mut spoofed = registration();
    spoofed["source"] = json!("builtin");
    for (malicious, expected) in [
        (json!({"tools":[],"persist":true}), StatusCode::BAD_REQUEST),
        (json!({"tools":[],"allowed":true}), StatusCode::UNPROCESSABLE_ENTITY),
        (json!({"tools":[registration(),registration()]}), StatusCode::BAD_REQUEST),
        (json!({"tools":[spoofed]}), StatusCode::BAD_REQUEST),
        (
            json!({"tools":[],"persistent_tool_policy":{"allowed":true}}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        assert_eq!(f.send("PUT", &f.session, "", token, malicious).await.status(), expected);
        assert!(Arc::ptr_eq(&old, &f.host(&binding).catalog.snapshot()));
    }
    assert_eq!(
        f.send("PUT", &f.session, "", token, json!({"tools":[]})).await.status(),
        StatusCode::OK
    );
    assert!(f.host(&binding).catalog.snapshot().specs().is_empty());
    drop(f.host(&binding).calls().unwrap());
}

#[tokio::test]
async fn dynamic_tools_gateway_disconnect_cancels_pending_and_expires_binding() {
    let f = Fixture::new().await;
    let binding = f.register().await;
    assert_eq!(
        f.send("PUT", &f.session, "", Some(&binding.binding_token), replacement()).await.status(),
        StatusCode::OK
    );
    let response =
        f.send("GET", &f.session, "/calls", Some(&binding.binding_token), json!({})).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        f.send("GET", &f.session, "/calls", Some(&binding.binding_token), json!({})).await.status(),
        StatusCode::FORBIDDEN
    );
    let snapshot = f.host(&binding).catalog.snapshot();
    let context = f.context();
    let call = tokio::spawn(async move {
        snapshot.bind("host_echo").unwrap().execute(&snapshot, json!({}), context).await
    });
    let mut stream = response.into_body().into_data_stream();
    let data = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(String::from_utf8_lossy(&data).contains("host_echo"));
    drop(stream);
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(5), call).await.unwrap().unwrap();
    assert!(result.is_err());
    assert_eq!(
        f.send("PUT", &f.session, "", Some(&binding.binding_token), replacement()).await.status(),
        StatusCode::FORBIDDEN
    );
    let rebound = f.register().await;
    assert_ne!(rebound.host_id, binding.host_id);
    drop(f.host(&rebound).calls().unwrap());
}

#[tokio::test]
async fn dynamic_tools_gateway_http_host_receives_real_call_and_responds_once() {
    let f = Fixture::new().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = f.router.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = reqwest::Client::new();
    let url = format!("http://{address}/session/{}/dynamic-tools", f.session);
    let request = |method, suffix: &str, token: Option<&str>| {
        let mut request = client
            .request(method, format!("{url}{suffix}"))
            .header("X-Platform-ID", f.scope.tenant().platform_id().as_ref())
            .header("X-Company-ID", f.scope.tenant().company_id().as_ref())
            .header("x-vibewindow-directory", f.directory.path().to_str().unwrap());
        if let Some(token) = token {
            request = request.header("x-vibewindow-tool-host", token);
        }
        request
    };
    let binding: DynamicToolHostBinding = request(reqwest::Method::POST, "/hosts", None)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let token = Some(binding.binding_token.as_str());
    request(reqwest::Method::PUT, "", token)
        .json(&replacement())
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let response = request(reqwest::Method::GET, "/calls", token)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    let context = f.context();
    let snapshot = crate::session::runtime::dynamic_hosts::request_catalog(
        &context,
        ToolCatalog::default().snapshot(),
    )
    .unwrap();
    let call = tokio::spawn(async move {
        snapshot.bind("host_echo").unwrap().execute(&snapshot, json!({"echo":"hi"}), context).await
    });
    let mut stream = response.bytes_stream();
    let mut data = String::new();
    while !data.contains("\n\n") {
        data.push_str(&String::from_utf8_lossy(
            &tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
        ));
    }
    let payload: Value =
        serde_json::from_str(data.lines().find_map(|line| line.strip_prefix("data: ")).unwrap())
            .unwrap();
    assert_eq!(payload["arguments"]["echo"], "hi");
    assert_eq!(payload["turn_id"], "turn");
    let suffix = format!("/calls/{}/response", payload["call_id"].as_str().unwrap());
    let result = json!({"success":true,"content":[{"type":"text","text":"done"}]});
    assert_eq!(
        request(reqwest::Method::POST, &suffix, token).json(&result).send().await.unwrap().status(),
        StatusCode::NO_CONTENT
    );
    assert!(call.await.unwrap().unwrap().result.is_success());
    assert_eq!(
        request(reqwest::Method::POST, &suffix, token).json(&result).send().await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    drop(stream);
    server.abort();
    let _ = server.await;
}

#[test]
fn dynamic_tools_gateway_production_chat_emits_tool_events_and_keeps_runtime_tools() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let f = runtime.block_on(Fixture::new());
    let _streams = crate::session::processor::test_llm_stream_guard();
    use crate::session::llm::{StreamEvent, ToolCall};
    crate::session::processor::enqueue_test_llm_stream(vec![
        StreamEvent::ToolCalls(vec![ToolCall {
            id: "model-call".into(),
            name: "host_echo".into(),
            arguments: "{}".into(),
        }]),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("tool_calls".into()) },
    ]);
    crate::session::processor::enqueue_test_llm_stream(vec![
        StreamEvent::Delta("finished".into()),
        StreamEvent::Done { usage: Default::default(), finish_reason: Some("stop".into()) },
    ]);
    runtime.block_on(async {
        let binding = f.register().await;
        assert_eq!(f.send("PUT", &f.session, "", Some(&binding.binding_token), replacement()).await.status(), StatusCode::OK);
        let calls = f.send("GET", &f.session, "/calls", Some(&binding.binding_token), json!({})).await;
        let router = f.router.clone();
        let mut response_request = f.request("POST", &f.session, "/calls/placeholder/response", Some(&binding.binding_token), json!({"success":true,"content":[{"type":"text","text":"host result"}]}));
        let replace_request = f.request("PUT", &f.session, "", Some(&binding.binding_token), json!({"tools":[]}));
        let session = f.session.clone();
        let host = tokio::spawn(async move {
            let mut stream = calls.into_body().into_data_stream();
            let bytes = stream.next().await.unwrap().unwrap();
            let data = String::from_utf8(bytes.to_vec()).unwrap();
            let call: Value = serde_json::from_str(data.lines().find_map(|line| line.strip_prefix("data: ")).unwrap()).unwrap();
            *response_request.uri_mut() = format!("/session/{session}/dynamic-tools/calls/{}/response", call["call_id"].as_str().unwrap()).parse().unwrap();
            assert_eq!(router.clone().oneshot(replace_request).await.unwrap().status(), StatusCode::OK);
            assert_eq!(router.oneshot(response_request).await.unwrap().status(), StatusCode::NO_CONTENT);
            stream
        });
        let body = serde_json::from_value(json!({"session_id":f.session,"model":"model-recovery/original","allowed_tools":["host_echo","read"],"messages":[{"role":"user","content":"perform action"}],"options":{"allowed_tools":["host_echo","read"],"full_access":true}})).unwrap();
        let mut events = Vec::new();
        let result = tokio::time::timeout(std::time::Duration::from_secs(30), crate::session::ui_store::with_tenant_scope(f.scope.tenant().clone(), super::session::stream::collect_gateway_chat_stream_with_observer(f.directory.path().to_string_lossy().into(), body, |event| events.push(event.clone())))).await.unwrap().unwrap();
        drop(host.await.unwrap());
        assert!(result.output.ends_with("finished"));
        assert!(events.iter().any(|event| event["type"] == "chat.tool" && event["name"] == "host_echo" && event["kind"] == "complete" && event["status"] == "completed" && event["call_id"] == "model-call"), "missing existing completion ToolEvent: {events:?}");
        let requests = crate::session::processor::take_test_llm_messages(&f.session);
        assert!(requests[0].tool_specs.iter().any(|spec| spec.id == "host_echo"));
        assert!(requests[0].tool_specs.iter().any(|spec| spec.id != "host_echo"), "runtime tools must survive host overlay");
        assert!(requests[1].messages.iter().any(|message| message.to_string().contains("host result")));
        assert!(!requests[1].tool_specs.iter().any(|spec| spec.id == "host_echo"), "replacement applies at the next model request");
    });
}

#[tokio::test]
async fn dynamic_tools_gateway_scope_validation_does_not_interrupt_active_history() {
    let f = Fixture::new().await;
    let tenant = f.scope.tenant().clone();
    let session = f.session.clone();
    crate::project::instance::provide(f.scope.directory().to_path_buf(), None, move || Box::pin(async move {
        crate::session::ui_store::with_tenant_scope(tenant, async move {
            use crate::session::message;
            let info: message::Info = serde_json::from_value(json!({"role":"user","id":"msg-active","sessionID":session,"time":{"created":1},"agent":"test","model":{"providerID":"test","modelID":"test"}})).unwrap();
            message::update_message(&info).await.unwrap();
            let part: message::Part = serde_json::from_value(json!({"type":"tool","id":"part-active","sessionID":session,"messageID":"msg-active","callID":"call-active","tool":"host_echo","state":{"status":"pending","input":{},"raw":""}})).unwrap();
            message::update_part(&part).await.unwrap();
        }).await
    })).await.unwrap();
    let binding = f.register().await;
    assert_eq!(
        f.send("PUT", &f.session, "", Some(&binding.binding_token), replacement()).await.status(),
        StatusCode::OK
    );
    let tenant = f.scope.tenant().clone();
    let session = f.session.clone();
    let history =
        crate::project::instance::provide(f.scope.directory().to_path_buf(), None, move || {
            Box::pin(async move {
                crate::session::ui_store::with_tenant_scope(tenant, async move {
                    crate::session::message::messages(&session, None).await.unwrap()
                })
                .await
            })
        })
        .await
        .unwrap();
    assert!(
        matches!(&history[0].parts[0], crate::session::message::Part::Tool(tool) if matches!(tool.state, crate::session::message::ToolState::Pending(_)))
    );
    drop(f.host(&binding).calls().unwrap());
}
