#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("session_gateway_tests"));
}

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
    let scope = super::active_project_scope("project-stable".into())
        .expect("non-empty project id should build a project scope");
    let endpoint = crate::app::config::gateway_client_endpoint();

    assert_eq!(scope.tenant_scope(), endpoint.tenant_scope());
    assert_eq!(scope.project_id().as_ref(), "project-stable");
}

#[test]
fn active_project_scope_rejects_empty_project_id() {
    let error =
        super::active_project_scope("".into()).expect_err("empty project id should be rejected");

    assert_eq!(error, "project id must not be empty");
}

#[test]
fn active_project_scope_rejects_whitespace_project_id() {
    let error = super::active_project_scope(" \t ".into())
        .expect_err("whitespace-only project id should be rejected");

    assert_eq!(error, "project id must not be empty");
}

#[test]
fn desktop_agent_scheduler_calls_use_resolved_active_scope() {
    let scope = super::active_agent_scheduler_scope("project-stable".into())
        .expect("resolved project id should build a scheduler scope");
    let project_scope = scope.project_scope();
    let endpoint = crate::app::config::gateway_client_endpoint();

    assert_eq!(project_scope.tenant_scope(), endpoint.tenant_scope());
    assert_eq!(project_scope.project_id().as_ref(), "project-stable");
    assert_eq!(
        vw_gateway_client::GatewayClient::agent_scheduler_events_stream_path(&scope, None),
        "/v1/agent-scheduler/projects/project-stable/events/stream"
    );
}

#[test]
fn active_agent_scheduler_scope_propagates_invalid_project_id() {
    let error = super::active_agent_scheduler_scope(" ".into())
        .expect_err("invalid project id should be rejected before scheduler scope construction");

    assert_eq!(error, "project id must not be empty");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn block_on_gateway_runs_future_without_existing_runtime() {
    let result = super::block_on_gateway(async { Ok::<_, String>("completed".to_string()) });

    assert_eq!(result.as_deref(), Ok("completed"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn block_on_gateway_propagates_future_error() {
    let result = super::block_on_gateway(async { Err::<String, _>("gateway failed".to_string()) });

    assert_eq!(result, Err("gateway failed".to_string()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn block_on_gateway_runs_future_inside_multithread_runtime() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("multi-thread runtime should build");

    let result = runtime.block_on(async {
        super::block_on_gateway(async { Ok::<_, String>("inside-runtime".to_string()) })
    });

    assert_eq!(result.as_deref(), Ok("inside-runtime"));
}

#[test]
fn desktop_session_persistence_surface_is_gateway_backed() {
    let _ = super::gateway_load_session_async;
    let _ = super::gateway_load_session_any_async;
    let _ = super::gateway_load_sessions_scoped_async;
    let _ = super::gateway_save_session_async;
    let _ = super::gateway_save_agent_session_scoped_async;
    let _ = super::gateway_session_file_path_async;
    let _ = super::gateway_session_todo_list_async;
}

#[test]
fn desktop_gateway_async_surfaces_are_exposed_without_network_access() {
    let _ = super::gateway_load_archived_session_ids_async;
    let _ = super::gateway_session_preview_meta_async;
    let _ = super::gateway_session_step_llm_raw_async;
    let _ = super::gateway_resolve_session_scope_id_async;
    let _ = super::gateway_permission_list_owned_async;
    let _ = super::gateway_set_session_scope_async;
    let _ = super::gateway_current_session_scope_async;
    let _ = super::gateway_question_list_async;
    let _ = super::gateway_question_reply_async;
    let _ = super::gateway_question_reject_async;
    let _ = super::gateway_permission_list_async;
    let _ = super::gateway_permission_reply_async;
    let _ = super::gateway_external_apps_async;
    let _ = super::gateway_external_open_async;
    let _ = super::gateway_external_reveal_async;
    let _ = super::gateway_skills_async;
    let _ = super::gateway_skill_detail_async;
    let _ = super::gateway_skill_create_async;
    let _ = super::gateway_skill_install_builtin_async;
    let _ = super::gateway_skill_set_enabled_async;
    let _ = super::gateway_skill_delete_async;
    let _ = super::gateway_plugins_async;
    let _ = super::gateway_plugin_set_enabled_async;
    let _ = super::gateway_plugin_delete_async;
    let _ = super::gateway_project_change_records_async;
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn desktop_gateway_blocking_surfaces_are_exposed_without_invoking_gateway() {
    let _ = super::gateway_load_session;
    let _ = super::gateway_load_session_any;
    let _ = super::gateway_load_sessions_scoped;
    let _ = super::gateway_load_archived_session_ids;
    let _ = super::gateway_session_preview_meta;
    let _ = super::gateway_session_file_path;
    let _ = super::gateway_resolve_session_scope_id;
    let _ = super::gateway_save_session;
    let _ = super::gateway_set_session_scope;
    let _ = super::gateway_current_session_scope;
    let _ = super::gateway_question_list;
    let _ = super::gateway_question_reply;
    let _ = super::gateway_question_reject;
    let _ = super::gateway_permission_list;
    let _ = super::gateway_permission_reply;
    let _ = super::gateway_save_agent_session_scoped;
    let _ = super::gateway_project_change_records;
}
