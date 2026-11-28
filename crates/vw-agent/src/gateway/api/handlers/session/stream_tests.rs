// 覆盖会话流事件转换、历史整理及代理选项合并。

use super::*;


#[tokio::test(flavor = "multi_thread")]
async fn gateway_chat_persists_and_reports_session_git_line_counts() {
    gateway_session::stream_runtime_acceptance::gateway_chat_git_line_counts::<Host>(|directory| {

        let repo = git2::Repository::init(&directory).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("example.txt")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let author = git2::Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &author, &author, "initial", &tree, &[]).unwrap();

    }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn codex_skills_defaults_gateway_main_and_child_preload_before_first_request() {
    gateway_session::stream_runtime_acceptance::codex_skills_defaults_gateway_main_and_child_preload_before_first_request::<Host>().await;
}

#[tokio::test]
async fn resolve_delegate_request_overrides_without_agent_returns_explicit_values() {
    gateway_session::stream_acceptance::resolve_delegate_request_overrides_without_agent_returns_explicit_values::<Host>().await;
}

#[tokio::test]
async fn resolve_delegate_request_overrides_rejects_unknown_agent() {
    gateway_session::stream_acceptance::resolve_delegate_request_overrides_rejects_unknown_agent::<Host>().await;
}

#[tokio::test]
async fn chat_stream_rejects_missing_workspace_without_retryable_status() {
    gateway_session::stream_acceptance::chat_stream_rejects_missing_workspace_without_retryable_status::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn native_workflow_gateway_persists_bound_graph_without_llm_artifacts() {
    gateway_session::stream_runtime_acceptance::native_workflow_gateway_persists_bound_graph_without_llm_artifacts::<Host>().await;
}

#[tokio::test]
async fn collect_gateway_chat_stream_runs_docs_request_and_persists_done_message_ids() {
    gateway_session::stream_acceptance::collect_gateway_chat_stream_runs_docs_request_and_persists_done_message_ids::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gateway_first_turn_registers_parent_before_thread_tool_use() {
    gateway_session::stream_acceptance::gateway_first_turn_registers_parent_before_thread_tool_use::<Host>().await;
}

#[tokio::test]
async fn chat_stream_returns_sse_response_for_docs_request() {
    gateway_session::stream_acceptance::chat_stream_returns_sse_response_for_docs_request::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn agcollab_delivery_reaches_next_model_request() {
    gateway_session::stream_runtime_acceptance::agcollab_delivery_reaches_next_model_request::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_native_defaults_reach_registered_tools_and_prompt() {
    gateway_session::stream_runtime_acceptance::agent_native_defaults_reach_registered_tools_and_prompt::<Host>(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/session/native_defaults_mcp_fixture.py"))).await;
}

#[tokio::test]
async fn agent_native_defaults_do_not_grant_disabled_mcp() {
    gateway_session::stream_runtime_acceptance::agent_native_defaults_do_not_grant_disabled_mcp::<Host>().await;
}

#[tokio::test]
async fn agent_session_identity_rejects_forged_ui_binding() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    gateway_session::stream_acceptance::agent_session_identity_rejects_forged_ui_binding::<Host>().await;
}

#[test]
fn agent_session_identity_survives_profile_edits() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    let _streams = agent_session::processor::test_llm_stream_guard();
    let workspace = tempfile::tempdir().unwrap();
    let session_id = format!("ses_binding_{}", uuid::Uuid::new_v4());
    gateway_session::stream_runtime_acceptance::profile_edit_stream::<Host>(&runtime, workspace.path(), &session_id);
    let tenant = crate::session::ui_store::current_tenant_scope();
    let directory = workspace.path().to_string_lossy().into_owned();
    runtime
        .block_on(with_tenant_instance(tenant.clone(), directory, move || {
            Box::pin(async move {
                let fork = agent_session::session::fork(&session_id, None).await.unwrap();
                assert!(fork.parent_id.is_none(), "fork must retain existing parent semantics");
                let fork_binding =
                    crate::session::agent_binding::load(&fork.id).await.unwrap().unwrap();
                assert_eq!(fork_binding.system_prompt.as_deref(), Some("ORIGINAL_BINDING_RULE"));
                crate::session::ui_config::set_config_field_result(
                    &tenant,
                    "permission",
                    serde_json::json!({"*":"deny"}),
                )
                .unwrap();
                crate::config::update(serde_json::json!({
                    "agents":{"main":{"enabled":false}}
                }))
                .await
                .unwrap();
                gateway_session::stream_acceptance::resumed_profile_selection::<Host>(&session_id).await;
                let mut config = crate::config::get_current().await.unwrap();
                crate::session::agent_binding::apply_to_config(&session_id, &mut config)
                    .await
                    .unwrap();
                let defaults = crate::session::capability_policy::native_agent_defaults(
                    &config,
                    "main",
                    None,
                    workspace.path(),
                )
                .await
                .unwrap()
                .unwrap();
                assert_eq!(
                    defaults.effective.permission_action("shell", "*"),
                    crate::permission::next::Action::Deny,
                    "the latest security revocation must still apply to the stored snapshot"
                );
                let mut new_session =
                    GatewayChatStreamRequest { agent: Some("main".into()), ..Default::default() };
                assert!(
                    gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut new_session, None).await.is_err(),
                    "disabled agents cannot start new sessions"
                );
                Ok(())
            })
        }))
        .unwrap();
}

#[tokio::test]
async fn agent_session_identity_derives_backend_and_rejects_conflicts() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    gateway_session::stream_acceptance::agent_session_identity_derives_backend_and_rejects_conflicts::<Host>().await;
}

#[tokio::test]
async fn acp_background_approval_gateway_uses_turn_grant_after_agent_resolution() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    with_tenant_instance(tenant, workspace.path().to_string_lossy().into_owned(), || {
        Box::pin(async {
            let (project, session) = gateway_session::stream_acceptance::background_approval_options::<Host>().await;
            assert!(
                !crate::session::agent_binding::confirmed_full_access(&project, &session)
                    .await
                    .unwrap()
            );
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn agent_config_cleanup_full_access_gateway_freezes_defaults_and_session_choice() {
    gateway_session::stream_runtime_acceptance::agent_config_cleanup_full_access_gateway_freezes_defaults_and_session_choice::<Host>().await;
}

#[test]
fn native_tool_search_core_loop_gateway_history_reaches_model_as_native_item() {
    let item = serde_json::json!({"type":"tool_search_output", "call_id":"search-id", "status":"completed", "execution":"client", "tools":[]});
    let history = prepare_stream_history(
        vec![serde_json::json!({"role":"native", "content":item.to_string()})],
        true,
        true,
    );
    assert_eq!(history.history.len(), 1);
    assert!(matches!(history.history[0].role, ui_models::ChatRole::Tool));
    let message = crate::session::session::Message {
        role: crate::session::session::Role::Tool,
        content: history.history[0].content.clone(),
    };
    let converted =
        crate::session::processor::llm_messages::session_message_to_llm_message(&message).unwrap();
    assert_eq!(converted["role"], "native");
    let restored: crate::session::llm::native_tools::NativeHistoryItem =
        serde_json::from_str(converted["content"].as_str().unwrap()).unwrap();
    assert!(
        matches!(restored.into_message(), aisdk::core::Message::ToolSearchOutput(output) if output.call_id.as_deref() == Some("search-id"))
    );
    let user = crate::session::session::Message {
        role: crate::session::session::Role::User,
        content: item.to_string(),
    };
    assert_eq!(
        crate::session::processor::llm_messages::session_message_to_llm_message(&user).unwrap()["role"],
        "user"
    );
}

#[tokio::test]
async fn agent_model_inheritance_acp_new_and_reopened_sessions_preserve_priority() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"agents":{
        "custom-model": {"runtime":{"kind":"acp","backend":"codex"},
            "defaults":{"model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}}}},
        "custom-inherit": {"runtime":{"kind":"acp","backend":"codex"}},
        "custom-forced": {"runtime":{"kind":"acp","backend":"codex"},
            "provider":"model-recovery","model":"original","force_model_selection":true}
    }})).await.unwrap();
    gateway_session::stream_acceptance::agent_model_inheritance_acp_new_and_reopened_sessions_preserve_priority::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gateway_first_response_replays_the_same_session_and_rejects_other_directories() {

    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "http://127.0.0.1:1/v1",
        None,
    )
    .await;
    gateway_session::stream_acceptance::gateway_first_response_replays_the_same_session_and_rejects_other_directories::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gateway_first_response_failure_preserves_history_until_same_session_retries() {
    gateway_session::stream_runtime_acceptance::gateway_first_response_failure_preserves_history_until_same_session_retries::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cron_stream_first_runtime_error_leaves_a_navigable_session() {
    use gateway_session::stream_runtime_acceptance_host::RuntimeAcceptanceHost;

    let _env = Host::env_lock().lock().await;
    let _fixture = Host::recovery("http://127.0.0.1:1/v1", None).await;
    let _streams = Host::stream_guard().await;
    let workspace = tempfile::tempdir().unwrap();
    let session_id = format!("cron-{}", uuid::Uuid::new_v4());
    let request = GatewayChatStreamRequest {
        session_id: Some(session_id.clone().into()),
        messages: vec![serde_json::json!({"role": "user", "content": "prepare the report"})],
        model: Some("model-recovery/original".into()),
        ..Default::default()
    };
    Host::enqueue(vec![llm_runtime::StreamEvent::Error(
        session_engine::message::AssistantError::Unknown { message: "fixture failure".into() },
    )]);
    let error = collect_cron_gateway_chat_stream(
        workspace.path().to_string_lossy().into_owned(), request, false,
    )
    .await
    .expect_err("the cron run still reports failure");
    assert!(error.message.contains("fixture failure"));
    let info = crate::session::ui_store::load_agent_session_any(&session_id).unwrap();
    assert!(info.is_user_navigable());
}

#[tokio::test]
async fn gateway_first_response_missing_replay_does_not_start_a_new_turn() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    gateway_session::stream_acceptance::gateway_first_response_missing_replay_does_not_start_a_new_turn::<Host>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gateway_first_response_disconnect_after_tool_start_replays_cancellation_without_publishing() {
    gateway_session::stream_runtime_acceptance::gateway_first_response_disconnect_after_tool_start_replays_cancellation_without_publishing::<Host>().await;
}

#[tokio::test]
async fn task_pool_agent_uses_existing_definition_and_defaults() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"agents":{"main":{"defaults":{"full_access":false},"allowed_tools":["read"],"permission":{"*":"deny"}},"task-reviewer":{"enabled":true,"runtime":{"kind":"native"},"provider":"model-recovery","model":"original","allowed_tools":["read"],"permission":{"*":"deny"},"defaults":{"full_access":false}}}})).await.unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    with_tenant_instance(tenant, workspace.path().to_string_lossy().into_owned(), move || {
        Box::pin(async move {
            for agent in ["main", "task-reviewer", "acp-codex"] {
                let mut request = GatewayChatStreamRequest {
                    agent: Some(agent.into()),
                    session_id: Some(format!("agent-scheduler-task-{agent}").into()),
                    ..Default::default()
                };
                let resolved = gateway_session::stream::prepare_gateway_agent_request::<Host>(&mut request, None).await.unwrap();
                let binding = crate::session::agent_binding::load(
                    request.session_id.as_ref().unwrap().as_ref(),
                )
                .await
                .unwrap()
                .unwrap();
                gateway_session::stream_acceptance::task_agent_defaults(agent, &resolved, &request, &binding);
                if agent != "acp-codex" {
                    let rules = crate::agent::permission_rules(agent).await.unwrap();
                    assert_eq!(
                        crate::permission::next::evaluate("shell", "*", &[rules]).action,
                        crate::permission::next::Action::Deny
                    );
                }
            }
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn task_pool_agent_rejects_unknown_or_disabled_definition() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update(serde_json::json!({"agents":{"main":{"enabled":false}}})).await.unwrap();
    gateway_session::stream_acceptance::task_pool_agent_rejects_unknown_or_disabled_definition::<Host>().await;
}
