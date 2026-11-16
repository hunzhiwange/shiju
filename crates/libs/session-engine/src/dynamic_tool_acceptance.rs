//! Dynamic-tool behavior contracts driven by the actual HTTP and executor fixture.
use crate::runtime::{SessionScope, dynamic_hosts::{SessionHost, HostTools}};
use gateway_http::GatewayTransport;
use tool_types::ToolSpec;
use serde_json::json;
use std::sync::Arc;
use vw_api_tools::{DynamicToolHostBinding, DynamicToolContent, DynamicToolRegistration, DynamicToolResponse, ReplaceDynamicToolsRequest, ToolSource};

pub trait Fixture: Sized {
    type Context: tool_context::host::ContextHost<ToolCatalogSnapshot = tool_catalog::catalog::ToolCatalogSnapshot>;
    type Tools: HostTools<Context = Self::Context>;
    fn new(trusted: bool) -> impl std::future::Future<Output = Self>;
    fn directory(&self) -> &str;
    fn session(&self) -> &str;
    fn scope(&self) -> &SessionScope;
    fn transport(&self) -> GatewayTransport;
    fn wrong_tenant_transport(&self) -> GatewayTransport;
    fn context(&self) -> Arc<tool_context::context::ToolUseContext<Self::Context>>;
    fn update_ui_title(&self) -> impl std::future::Future<Output = ()>;
    fn assert_ui_title(&self) -> impl std::future::Future<Output = ()>;
    fn saved_tools(&self) -> impl std::future::Future<Output = session_store::dynamic_tool_store::PersistedDynamicTools>;
    fn other_session(&self) -> impl std::future::Future<Output = String>;
    fn fork_session(&self) -> impl std::future::Future<Output = String>;
    fn register(&self) -> impl std::future::Future<Output = DynamicToolHostBinding> {
        async { gateway_host_tools::GatewayHostToolsClient::new(self.transport()).register_dynamic_tool_host(self.session(), Some(self.directory())).await.unwrap() }
    }
    fn replace(&self, binding: &DynamicToolHostBinding, name: &str, persist: bool) -> impl std::future::Future<Output = Result<u64, String>> {
        async move { gateway_host_tools::GatewayHostToolsClient::new(self.transport()).replace_dynamic_tools(self.session(), Some(self.directory()), binding, &replacement(name, persist)).await.map(|r| r.version) }
    }
}
fn entry<F: Fixture>(f: &F, binding: &DynamicToolHostBinding) -> Arc<SessionHost> {
    SessionHost::authenticate(f.scope(), f.session(), &binding.binding_token).unwrap()
}
pub fn replacement(name: &str, persist: bool) -> ReplaceDynamicToolsRequest {
    let tools = if name.is_empty() {
        vec![]
    } else {
        let mut spec = ToolSpec::new(name, name, json!({"type":"object","properties":{}}));
        spec.source = ToolSource::Client;
        spec.exposure = vw_api_tools::ToolExposure::Deferred;
        spec.read_only = true;
        vec![DynamicToolRegistration::Function(spec.to_dto())]
    };
    ReplaceDynamicToolsRequest { tools, persist }
}
pub fn response() -> DynamicToolResponse {
    DynamicToolResponse {
        success: true,
        content: vec![DynamicToolContent::Text { text: "host result".into() }],
    }
}

pub async fn dynamic_tools_acceptance_replacement_survives_resume<F: Fixture>() {
    let f = F::new(true).await;
    let binding = f.register().await;
    assert!(f.replace(&binding, "release", true).await.unwrap() > 0);
    f.update_ui_title().await;
    assert!(f.replace(&binding, "deployment", true).await.unwrap() > 1);
    f.assert_ui_title().await;
    let old_entry = entry(&f, &binding);
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
    assert!(old_entry.replace::<F::Tools>(replacement("forged-after-disconnect", true)).is_err());
    let saved = f.saved_tools().await;
    assert_eq!(saved.specs[0].id, "deployment");
    let rebound = f.register().await;
    assert_ne!(binding.binding_token, rebound.binding_token);
    assert!(
        entry(&f, &rebound).catalog.snapshot().specs().is_empty(),
        "saved metadata grants no execution"
    );
    f.replace(&rebound, "deployment", false).await.unwrap();
    assert!(entry(&f, &rebound).catalog.snapshot().bind("deployment").is_ok());
    assert!(entry(&f, &rebound).catalog.snapshot().bind("release").is_err());
    assert!(
        gateway_host_tools::GatewayHostToolsClient::new(f.transport())
            .replace_dynamic_tools(
                f.session(),
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
    gateway_host_tools::GatewayHostToolsClient::new(f.transport())
        .respond_dynamic_tool_call(
            f.session(),
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

pub async fn host_stream<F: Fixture>(
    f: &F,
    binding: &DynamicToolHostBinding,
) -> (
    tokio::task::JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<vw_api_tools::DynamicToolCallRequest>,
) {
    let (send, receive) = tokio::sync::mpsc::unbounded_channel();
    let client = f.transport();
    let session = f.session().to_owned();
    let directory = f.directory().to_owned();
    let binding = binding.clone();
    let task = tokio::spawn(async move {
        let _ = gateway_host_tools::GatewayHostToolsClient::new(client)
            .stream_dynamic_tool_calls(&session, Some(&directory), &binding, |call| {
                send.send(call).is_ok()
            })
            .await;
    });
    (task, receive)
}
pub async fn next_call(
    receive: &mut tokio::sync::mpsc::UnboundedReceiver<vw_api_tools::DynamicToolCallRequest>,
) -> vw_api_tools::DynamicToolCallRequest {
    tokio::time::timeout(std::time::Duration::from_secs(20), receive.recv()).await.unwrap().unwrap()
}
pub async fn execute<F: Fixture>(
    f: &F,
    binding: &DynamicToolHostBinding,
    name: &str,
) -> tokio::task::JoinHandle<Result<tool_context::ExecutedToolCall, tool_context::ToolCallError>> {
    let snapshot = entry(f, binding).catalog.snapshot();
    let call = snapshot.bind(name).unwrap();
    let context = f.context();
    tokio::spawn(async move { call.execute(&snapshot, json!({}), context).await })
}

pub async fn dynamic_tools_acceptance_default_policy_and_cross_session_are_denied<F: Fixture>() {
    let f = F::new(false).await;
    let binding = f.register().await;
    assert!(f.replace(&binding, "release", true).await.unwrap_err().contains("disabled"));
    assert!(entry(&f, &binding).catalog.snapshot().specs().is_empty());
    f.replace(&binding, "release", false).await.unwrap();
    let other = f.other_session().await;
    assert!(
        gateway_host_tools::GatewayHostToolsClient::new(f.transport())
            .replace_dynamic_tools(
                &other,
                Some(f.directory()),
                &binding,
                &replacement("release", false)
            )
            .await
            .unwrap_err()
            .contains("403")
    );
    let wrong_tenant = f.wrong_tenant_transport();
    assert!(
        gateway_host_tools::GatewayHostToolsClient::new(wrong_tenant)
            .register_dynamic_tool_host(f.session(), Some(f.directory()))
            .await
            .err()
            .unwrap()
            .contains("404")
    );
    drop(entry(&f, &binding).calls().unwrap());
}

pub async fn dynamic_tools_acceptance_replacement_cancellation_and_revocation<F: Fixture>() {
    let f = F::new(true).await;
    let binding = f.register().await;
    f.replace(&binding, "release", true).await.unwrap();
    let (stream, mut receive) = host_stream(&f, &binding).await;
    let call = execute(&f, &binding, "release").await;
    let request = next_call(&mut receive).await;
    assert_eq!(request.tool_id.as_ref(), "release");
    // Old pending execution remains answerable while the next request sees the replacement.
    f.replace(&binding, "deployment", true).await.unwrap();
    assert!(entry(&f, &binding).catalog.snapshot().bind("release").is_err());
    gateway_host_tools::GatewayHostToolsClient::new(f.transport())
        .respond_dynamic_tool_call(
            f.session(),
            Some(f.directory()),
            &binding,
            &request.call_id,
            &response(),
        )
        .await
        .unwrap();
    assert!(call.await.unwrap().unwrap().result.is_success());
    assert!(
        gateway_host_tools::GatewayHostToolsClient::new(f.transport())
            .respond_dynamic_tool_call(
                f.session(),
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
        gateway_host_tools::GatewayHostToolsClient::new(f.transport())
            .respond_dynamic_tool_call(
                f.session(),
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
    assert!(entry(&f, &binding).catalog.snapshot().bind("deployment").is_err());
    stream.abort();
    let _ = stream.await;
}

pub async fn dynamic_tools_acceptance_fork_requires_new_host_and_matching_specs<F: Fixture>() {
    let f = F::new(true).await;
    let binding = f.register().await;
    f.replace(&binding, "release", true).await.unwrap();
    let fork_id = f.fork_session().await;
    let fork = fork_id.as_str();
    assert!(
        gateway_host_tools::GatewayHostToolsClient::new(f.transport())
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
        gateway_host_tools::GatewayHostToolsClient::new(f.transport()).register_dynamic_tool_host(fork, Some(f.directory())).await.unwrap();
    let fork_entry = crate::runtime::dynamic_hosts::SessionHost::authenticate(
        f.scope(),
        fork,
        &fork_binding.binding_token,
    )
    .unwrap();
    assert!(fork_entry.catalog.snapshot().bind("release").is_err());
    assert!(
        gateway_host_tools::GatewayHostToolsClient::new(f.transport())
            .replace_dynamic_tools(
                fork,
                Some(f.directory()),
                &fork_binding,
                &replacement("changed", false)
            )
            .await
            .is_err()
    );
    gateway_host_tools::GatewayHostToolsClient::new(f.transport())
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
        gateway_host_tools::GatewayHostToolsClient::new(f.transport())
            .respond_dynamic_tool_call(fork, Some(f.directory()), &binding, "forged", &response())
            .await
            .unwrap_err()
            .contains("403")
    );
    let (stream, mut receive) = host_stream(&f, &binding).await;
    let call = execute(&f, &binding, "release").await;
    let request = next_call(&mut receive).await;
    assert!(
        gateway_host_tools::GatewayHostToolsClient::new(f.transport())
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
    gateway_host_tools::GatewayHostToolsClient::new(f.transport())
        .respond_dynamic_tool_call(
            f.session(),
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
