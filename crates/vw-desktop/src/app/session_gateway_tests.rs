//! 桌面活动端点与作用域装配测试。

#[test]
fn gateway_client_binds_configured_endpoint_without_network_access() {
    let client =
        super::gateway_client().expect("configured gateway endpoint should build a client");
    let configured = crate::app::config::gateway_client_endpoint();

    assert_eq!(client.endpoint().host, configured.host);
    assert_eq!(client.endpoint().port, configured.port);
    assert_eq!(client.endpoint().tenant_scope(), configured.tenant_scope());
}

#[test]
fn desktop_project_calls_use_active_tenant_scope() {
    let scope = super::active_tenant_scope();
    let endpoint = crate::app::config::gateway_client_endpoint();

    assert_eq!(&scope, endpoint.tenant_scope());
}

#[test]
fn active_project_scope_preserves_active_tenant_and_project_id() {
    let scope = desktop_session::session_gateway::active_project_scope(crate::app::config::gateway_client_endpoint(), "project-stable".into())
        .expect("non-empty project id should build a project scope");
    let endpoint = crate::app::config::gateway_client_endpoint();

    assert_eq!(scope.tenant_scope(), endpoint.tenant_scope());
    assert_eq!(scope.project_id().as_ref(), "project-stable");
}

#[test]
fn active_project_scope_rejects_empty_project_id() {
    let error =
        desktop_session::session_gateway::active_project_scope(crate::app::config::gateway_client_endpoint(), "".into()).expect_err("empty project id should be rejected");

    assert_eq!(error, "project id must not be empty");
}

#[test]
fn active_project_scope_rejects_whitespace_project_id() {
    let error = desktop_session::session_gateway::active_project_scope(crate::app::config::gateway_client_endpoint(), " \t ".into())
        .expect_err("whitespace-only project id should be rejected");

    assert_eq!(error, "project id must not be empty");
}

#[test]
fn desktop_agent_scheduler_calls_use_resolved_active_scope() {
    let scope = desktop_session::session_gateway::active_agent_scheduler_scope(crate::app::config::gateway_client_endpoint(), "project-stable".into())
        .expect("resolved project id should build a scheduler scope");
    let project_scope = scope.project_scope();
    let endpoint = crate::app::config::gateway_client_endpoint();

    assert_eq!(project_scope.tenant_scope(), endpoint.tenant_scope());
    assert_eq!(project_scope.project_id().as_ref(), "project-stable");
    assert_eq!(
        gateway_scheduler::GatewaySchedulerClient::agent_scheduler_events_stream_path(&scope, None),
        "/v1/agent-scheduler/projects/project-stable/events/stream"
    );
}

#[test]
fn active_agent_scheduler_scope_propagates_invalid_project_id() {
    let error = desktop_session::session_gateway::active_agent_scheduler_scope(crate::app::config::gateway_client_endpoint(), " ".into())
        .expect_err("invalid project id should be rejected before scheduler scope construction");

    assert_eq!(error, "project id must not be empty");
}
