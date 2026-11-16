//! 原宿主权限队列断言，直接验证生产转换。
#[derive(Default)]
struct Fixture {
    permission_modal_request_id: Option<String>,
    permission_modal_request: Option<gateway_interaction::PendingPermissionRequestDto>,
    permission_modal_requests: Vec<gateway_interaction::PendingPermissionRequestDto>,
}
fn state(app: &mut Fixture) -> super::PermissionState<'_> {
    super::PermissionState {
        permission_modal_request_id: &mut app.permission_modal_request_id,
        permission_modal_request: &mut app.permission_modal_request,
        permission_modal_requests: &mut app.permission_modal_requests,
    }
}
fn handle_permission_list_loaded(
    app: &mut Fixture,
    result: Result<Vec<gateway_interaction::PendingPermissionRequestDto>, String>,
) {
    super::apply_permission_list_result(&mut state(app), result);
}
fn handle_permission_select_request(app: &mut Fixture, id: String) {
    super::apply_current_permission_request(&mut state(app), Some(id));
}
fn handle_permission_approve_all_always(app: &mut Fixture) {
    super::take_all_requests(&mut state(app));
}
fn handle_permission_approve_once(app: &mut Fixture) {
    super::take_selected_request(&mut state(app));
}
fn handle_permission_approve_always(app: &mut Fixture) {
    handle_permission_approve_once(app);
}
fn handle_permission_reject(app: &mut Fixture) {
    handle_permission_approve_once(app);
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
#[test]
fn handle_permission_list_loaded_applies_sorted_request_and_clears_on_error() {
    let mut app = Fixture::default();

    handle_permission_list_loaded(&mut app, Ok(vec![request("b", "s"), request("a", "s")]));

    assert_eq!(app.permission_modal_request_id.as_deref(), Some("a"));
    assert_eq!(
        app.permission_modal_requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );

    handle_permission_list_loaded(&mut app, Err("offline".to_string()));

    assert_eq!(app.permission_modal_request_id, None);
    assert!(app.permission_modal_request.is_none());
    assert!(app.permission_modal_requests.is_empty());
}

#[test]
fn handle_permission_list_loaded_clears_stale_modal_for_empty_success() {
    let mut app = Fixture::default();
    app.permission_modal_request_id = Some("stale".to_string());
    app.permission_modal_request = Some(request("stale", "s"));
    app.permission_modal_requests = vec![request("stale", "s")];

    handle_permission_list_loaded(&mut app, Ok(Vec::new()));

    assert_eq!(app.permission_modal_request_id, None);
    assert!(app.permission_modal_request.is_none());
    assert!(app.permission_modal_requests.is_empty());
}

#[test]
fn handle_permission_select_request_updates_current_when_found_and_clears_when_missing() {
    let mut app = Fixture::default();
    app.permission_modal_requests = vec![request("a", "s"), request("b", "s")];

    handle_permission_select_request(&mut app, "b".to_string());

    assert_eq!(app.permission_modal_request_id.as_deref(), Some("b"));
    assert_eq!(app.permission_modal_request.as_ref().map(|request| request.id.as_str()), Some("b"));

    handle_permission_select_request(&mut app, "missing".to_string());

    assert_eq!(app.permission_modal_request_id, None);
    assert!(app.permission_modal_request.is_none());
}

#[test]
fn handle_permission_approve_all_always_clears_modal_state() {
    let mut app = Fixture::default();
    app.permission_modal_request_id = Some("a".to_string());
    app.permission_modal_request = Some(request("a", "s"));
    app.permission_modal_requests = vec![request("a", "s"), request("b", "s")];

    handle_permission_approve_all_always(&mut app);

    assert_eq!(app.permission_modal_request_id, None);
    assert!(app.permission_modal_request.is_none());
    assert!(app.permission_modal_requests.is_empty());
}

#[test]
// 回复后选择相邻请求而不是清空整个队列，保证连续授权流程不会跳项。
fn handle_permission_approve_once_advances_modal_to_next_request() {
    let mut app = Fixture::default();
    app.permission_modal_request_id = Some("b".to_string());
    app.permission_modal_request = Some(request("b", "s"));
    app.permission_modal_requests = vec![request("c", "s"), request("a", "s"), request("b", "s")];

    handle_permission_approve_once(&mut app);

    assert_eq!(app.permission_modal_request_id.as_deref(), Some("c"));
    assert_eq!(app.permission_modal_request.as_ref().map(|request| request.id.as_str()), Some("c"));
    assert_eq!(
        app.permission_modal_requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "c"]
    );
}

#[test]
fn handle_permission_approve_once_without_selection_preserves_pending_requests() {
    let mut app = Fixture::default();
    app.permission_modal_requests = vec![request("a", "s")];

    handle_permission_approve_once(&mut app);

    assert_eq!(app.permission_modal_request_id, None);
    assert_eq!(app.permission_modal_requests, vec![request("a", "s")]);
}

#[test]
fn approve_always_advances_modal_to_remaining_request() {
    let mut app = Fixture::default();
    app.permission_modal_request_id = Some("a".to_string());
    app.permission_modal_request = Some(request("a", "s"));
    app.permission_modal_requests = vec![request("a", "s"), request("b", "s")];

    handle_permission_approve_always(&mut app);

    assert_eq!(app.permission_modal_request_id.as_deref(), Some("b"));
    assert_eq!(app.permission_modal_requests, vec![request("b", "s")]);
}

#[test]
fn reject_advances_modal_to_remaining_request() {
    let mut app = Fixture::default();
    app.permission_modal_request_id = Some("a".to_string());
    app.permission_modal_request = Some(request("a", "s"));
    app.permission_modal_requests = vec![request("a", "s"), request("b", "s")];

    handle_permission_reject(&mut app);

    assert_eq!(app.permission_modal_request_id.as_deref(), Some("b"));
    assert_eq!(app.permission_modal_requests, vec![request("b", "s")]);
}
