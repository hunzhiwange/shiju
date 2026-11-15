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
