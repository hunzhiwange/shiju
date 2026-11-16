// 宿主契约：核对权限消息分支到弹窗字段、ACP 重建及持久化任务的接线。
//! 验证流式权限请求的目录选择、自动批准分组和选中状态同步。
//!
//! 权限夹具交叉组合活动会话、目录和全访问状态，避免请求被错误自动批准到其他会话。

#![allow(unused_must_use)]
#[test]
fn permissions_tests_module_is_wired() {
    assert!(module_path!().ends_with("permissions_tests"));
}

fn request(id: &str, session_id: &str) -> gateway_interaction::PendingPermissionRequestDto {
    gateway_interaction::PendingPermissionRequestDto {
        id: id.to_string(),
        session_id: session_id.to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: None,
    }
}

fn session_info(id: &str, directory: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Session".to_string(),
        version: "0.0.0".to_string(),
        time: session_info::TimeInfo {
            created: 10,
            updated: 20,
            compacting: None,
            archived: None,
        },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn permission_directory_prefers_session_directory_and_filters_blank_fallbacks() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    app.sessions = vec![session_info("s1", "/session")];
    app.navigation_tabs.project_path = Some("/project".to_string());

    assert_eq!(super::permission_directory(&app).as_deref(), Some("/session"));

    app.sessions[0].directory = "   ".to_string();
    assert_eq!(super::permission_directory(&app).as_deref(), Some("/project"));

    app.navigation_tabs.project_path = Some("\t".to_string());
    assert_eq!(super::permission_directory(&app), None);
}

#[test]
fn current_full_access_session_id_requires_active_enabled_runtime() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());

    assert_eq!(super::current_full_access_session_id(&app), None);

    app.current_session_runtime_mut().full_access_enabled = false;
    assert_eq!(super::current_full_access_session_id(&app), None);

    app.current_session_runtime_mut().full_access_enabled = true;
    assert_eq!(super::current_full_access_session_id(&app), Some("s1"));
}

#[test]
fn handle_permission_list_loaded_applies_sorted_request_and_clears_on_error() {
    let (mut app, _task) = crate::app::App::new();

    super::handle_permission_list_loaded(&mut app, Ok(vec![request("b", "s"), request("a", "s")]));

    assert_eq!(app.permission_modal_request_id.as_deref(), Some("a"));
    assert_eq!(
        app.permission_modal_requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );

    super::handle_permission_list_loaded(&mut app, Err("offline".to_string()));

    assert_eq!(app.permission_modal_request_id, None);
    assert!(app.permission_modal_request.is_none());
    assert!(app.permission_modal_requests.is_empty());
}

#[test]
fn handle_permission_list_loaded_clears_stale_modal_for_empty_success() {
    let (mut app, _task) = crate::app::App::new();
    app.permission_modal_request_id = Some("stale".to_string());
    app.permission_modal_request = Some(request("stale", "s"));
    app.permission_modal_requests = vec![request("stale", "s")];

    super::handle_permission_list_loaded(&mut app, Ok(Vec::new()));

    assert_eq!(app.permission_modal_request_id, None);
    assert!(app.permission_modal_request.is_none());
    assert!(app.permission_modal_requests.is_empty());
}

#[test]
// 全访问自动批准严格限制在活动且已启用的会话，其他会话请求必须继续排队。
fn handle_permission_list_loaded_auto_approves_only_active_full_access_session() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("active".to_string());
    app.current_session_runtime_mut().full_access_enabled = true;

    super::handle_permission_list_loaded(
        &mut app,
        Ok(vec![request("active-request", "active"), request("other-request", "other")]),
    );

    assert_eq!(app.permission_modal_request_id.as_deref(), Some("other-request"));
    assert_eq!(
        app.permission_modal_requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["other-request"]
    );
}

#[test]
fn handle_permission_select_request_updates_current_when_found_and_clears_when_missing() {
    let (mut app, _task) = crate::app::App::new();
    app.permission_modal_requests = vec![request("a", "s"), request("b", "s")];

    super::handle_permission_select_request(&mut app, "b".to_string());

    assert_eq!(app.permission_modal_request_id.as_deref(), Some("b"));
    assert_eq!(app.permission_modal_request.as_ref().map(|request| request.id.as_str()), Some("b"));

    super::handle_permission_select_request(&mut app, "missing".to_string());

    assert_eq!(app.permission_modal_request_id, None);
    assert!(app.permission_modal_request.is_none());
}

#[test]
fn handle_permission_approve_all_always_clears_modal_state() {
    let (mut app, _task) = crate::app::App::new();
    app.permission_modal_request_id = Some("a".to_string());
    app.permission_modal_request = Some(request("a", "s"));
    app.permission_modal_requests = vec![request("a", "s"), request("b", "s")];

    super::handle_permission_approve_all_always(&mut app);

    assert_eq!(app.permission_modal_request_id, None);
    assert!(app.permission_modal_request.is_none());
    assert!(app.permission_modal_requests.is_empty());
}

#[test]
fn handle_toggle_full_access_permission_enables_and_removes_active_session_requests() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("active".to_string());
    app.current_session_runtime_mut().full_access_enabled = false;
    app.permission_modal_requests =
        vec![request("active-request", "active"), request("other-request", "other")];

    super::handle_toggle_full_access_permission(&mut app);

    assert!(app.current_session_runtime_ref().expect("active runtime").full_access_enabled);
    assert_eq!(app.permission_modal_request_id.as_deref(), Some("other-request"));
    assert_eq!(
        app.permission_modal_requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["other-request"]
    );
}

#[test]
// 切换全访问除了更新界面状态，还需标记 ACP 运行时重建以应用新的权限边界。
fn handle_toggle_full_access_permission_marks_acp_runtime_for_rebuild() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("active".to_string());
    {
        let runtime = app.current_session_runtime_mut();
        runtime.full_access_enabled = true;
        runtime.acp_agent = Some("agent".to_string());
        runtime.last_effective_acp_agent = Some("agent".to_string());
    }

    super::handle_toggle_full_access_permission(&mut app);

    let runtime = app.current_session_runtime_ref().expect("active runtime");
    assert!(!runtime.full_access_enabled);
    assert!(runtime.acp_rebuild_required);
    assert_eq!(runtime.last_effective_acp_agent, None);
}

#[test]
fn disabling_full_access_without_acp_preserves_pending_modal() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("active".to_string());
    app.current_session_runtime_mut().full_access_enabled = true;
    app.permission_modal_request_id = Some("a".to_string());
    app.permission_modal_request = Some(request("a", "active"));
    app.permission_modal_requests = vec![request("a", "active")];

    super::handle_toggle_full_access_permission(&mut app);

    let runtime = app.current_session_runtime_ref().expect("active runtime");
    assert!(!runtime.full_access_enabled);
    assert!(!runtime.acp_rebuild_required);
    assert_eq!(app.permission_modal_request_id.as_deref(), Some("a"));
    assert_eq!(app.permission_modal_requests, vec![request("a", "active")]);
}

#[test]
// 回复后选择相邻请求而不是清空整个队列，保证连续授权流程不会跳项。
fn handle_permission_approve_once_advances_modal_to_next_request() {
    let (mut app, _task) = crate::app::App::new();
    app.permission_modal_request_id = Some("b".to_string());
    app.permission_modal_request = Some(request("b", "s"));
    app.permission_modal_requests = vec![request("c", "s"), request("a", "s"), request("b", "s")];

    super::handle_permission_approve_once(&mut app);

    assert_eq!(app.permission_modal_request_id.as_deref(), Some("c"));
    assert_eq!(app.permission_modal_request.as_ref().map(|request| request.id.as_str()), Some("c"));
    assert_eq!(
        app.permission_modal_requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "c"]
    );
}

#[test]
fn handle_permission_approve_once_without_selection_preserves_pending_requests() {
    let (mut app, _task) = crate::app::App::new();
    app.permission_modal_requests = vec![request("a", "s")];

    super::handle_permission_approve_once(&mut app);

    assert_eq!(app.permission_modal_request_id, None);
    assert_eq!(app.permission_modal_requests, vec![request("a", "s")]);
}

#[test]
fn approve_always_advances_modal_to_remaining_request() {
    let (mut app, _task) = crate::app::App::new();
    app.permission_modal_request_id = Some("a".to_string());
    app.permission_modal_request = Some(request("a", "s"));
    app.permission_modal_requests = vec![request("a", "s"), request("b", "s")];

    super::handle_permission_approve_always(&mut app);

    assert_eq!(app.permission_modal_request_id.as_deref(), Some("b"));
    assert_eq!(app.permission_modal_requests, vec![request("b", "s")]);
}

#[test]
fn reject_advances_modal_to_remaining_request() {
    let (mut app, _task) = crate::app::App::new();
    app.permission_modal_request_id = Some("a".to_string());
    app.permission_modal_request = Some(request("a", "s"));
    app.permission_modal_requests = vec![request("a", "s"), request("b", "s")];

    super::handle_permission_reject(&mut app);

    assert_eq!(app.permission_modal_request_id.as_deref(), Some("b"));
    assert_eq!(app.permission_modal_requests, vec![request("b", "s")]);
}

#[test]
fn reply_result_schedules_permission_refresh_for_success_and_error() {
    let (app, _task) = crate::app::App::new();

    let success = super::handle_permission_reply_submitted(&app, Ok(()));
    let error = super::handle_permission_reply_submitted(&app, Err("offline".to_string()));

    assert!(iced_runtime::task::into_stream(success.collect()).is_some());
    assert!(iced_runtime::task::into_stream(error.collect()).is_some());
}

#[test]
fn agent_config_cleanup_full_access_disabled_keeps_normal_approval() {
    let mut app = crate::app::App::new().0;
    app.agents_settings.config_ready = true;
    app.last_llm_full_access = Some(false);
    app.active_session_id = Some("normal-approval".into());
    app.current_session_runtime_mut();
    let pending = request("pending", "normal-approval");
    let _ = super::handle_permission_list_loaded(&mut app, Ok(vec![pending.clone()]));
    assert!(!app.current_session_runtime().full_access_enabled);
    assert_eq!(app.permission_modal_requests, vec![pending]);
    assert_eq!(app.permission_modal_request_id.as_deref(), Some("pending"));
}

#[test]
fn full_access_save_failure_restores_previous_mode_and_allows_retry() {
    let (mut app, _) = crate::app::App::new();
    let id = "stored-session";
    app.active_session_id = Some(id.into());
    let runtime = app.current_session_runtime_mut();
    runtime.full_access_enabled = false;
    runtime.full_access_saving = true;

    let _ = super::handle_full_access_saved(&mut app, id, true, Err("offline".into()));
    let runtime = app.current_session_runtime();
    assert!(runtime.full_access_enabled);
    assert!(!runtime.full_access_saving);
    assert!(runtime.full_access_save_failed);

    let _ = super::handle_toggle_full_access_permission(&mut app);
    let _ = super::handle_full_access_saved(&mut app, id, true, Ok(false));
    let runtime = app.current_session_runtime();
    assert!(!runtime.full_access_enabled);
    assert!(!runtime.full_access_save_failed);
}
