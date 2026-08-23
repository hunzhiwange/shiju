//! 验证流式权限请求的目录选择、自动批准分组和选中状态同步。
//!
//! 权限夹具交叉组合活动会话、目录和全访问状态，避免请求被错误自动批准到其他会话。

#![allow(unused_must_use)]
#[test]
fn permissions_tests_module_is_wired() {
    assert!(module_path!().ends_with("permissions_tests"));
}

fn request(id: &str, session_id: &str) -> vw_gateway_client::PendingPermissionRequestDto {
    vw_gateway_client::PendingPermissionRequestDto {
        id: id.to_string(),
        session_id: session_id.to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: None,
    }
}

fn session_info(id: &str, directory: &str) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Session".to_string(),
        version: "0.0.0".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 10,
            updated: 20,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    }
}

#[test]
fn permission_directory_prefers_session_directory_and_filters_blank_fallbacks() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("s1".to_string());
    app.sessions = vec![session_info("s1", "/session")];
    app.project_path = Some("/project".to_string());

    assert_eq!(super::permission_directory(&app).as_deref(), Some("/session"));

    app.sessions[0].directory = "   ".to_string();
    assert_eq!(super::permission_directory(&app).as_deref(), Some("/project"));

    app.project_path = Some("\t".to_string());
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
fn split_auto_approved_permission_requests_keeps_other_sessions() {
    let requests = vec![request("b", "other"), request("a", "s1"), request("c", "s1")];

    let (remaining, auto_ids) =
        super::split_auto_approved_permission_requests(Some("s1"), requests);

    assert_eq!(remaining, vec![request("b", "other")]);
    assert_eq!(auto_ids, vec!["a".to_string(), "c".to_string()]);
}

#[test]
fn split_auto_approved_permission_requests_without_session_keeps_all() {
    let requests = vec![request("a", "s1")];

    let (remaining, auto_ids) = super::split_auto_approved_permission_requests(None, requests);

    assert_eq!(remaining, vec![request("a", "s1")]);
    assert!(auto_ids.is_empty());
}

#[test]
// 列表同步先稳定排序，再尽可能保留当前选择；失效选择才回退到首项。
fn sync_permission_requests_sorts_and_preserves_existing_selection() {
    let (requests, selected) = super::sync_permission_requests(
        Some("b"),
        vec![request("c", "s"), request("a", "s"), request("b", "s")],
    );

    assert_eq!(
        requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
    assert_eq!(selected.as_deref(), Some("b"));
}

#[test]
fn sync_permission_requests_selects_first_when_current_missing_or_empty() {
    let (_requests, selected) =
        super::sync_permission_requests(Some("z"), vec![request("b", "s"), request("a", "s")]);
    assert_eq!(selected.as_deref(), Some("a"));

    let (requests, selected) = super::sync_permission_requests(Some("a"), Vec::new());
    assert!(requests.is_empty());
    assert_eq!(selected, None);
}

#[test]
fn advance_permission_requests_after_reply_removes_selected_and_selects_next() {
    let (remaining, selected) = super::advance_permission_requests_after_reply(
        Some("b"),
        vec![request("a", "s"), request("b", "s"), request("c", "s")],
    );

    assert_eq!(
        remaining.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "c"]
    );
    assert_eq!(selected.as_deref(), Some("c"));
}

#[test]
fn advance_permission_requests_after_reply_selects_previous_slot_for_last_item() {
    let (remaining, selected) = super::advance_permission_requests_after_reply(
        Some("c"),
        vec![request("a", "s"), request("b", "s"), request("c", "s")],
    );

    assert_eq!(
        remaining.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert_eq!(selected.as_deref(), Some("b"));
}

#[test]
fn advance_permission_requests_after_reply_handles_empty_and_only_request() {
    let (remaining, selected) = super::advance_permission_requests_after_reply(None, Vec::new());
    assert!(remaining.is_empty());
    assert_eq!(selected, None);

    let (remaining, selected) =
        super::advance_permission_requests_after_reply(None, vec![request("a", "s")]);
    assert!(remaining.is_empty());
    assert_eq!(selected, None);
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
