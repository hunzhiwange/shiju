use super::permissions::{advance_permission_requests_after_reply, split_auto_approved_permission_requests, sync_permission_requests};

fn request(id: &str) -> gateway_interaction::PendingPermissionRequestDto {
    gateway_interaction::PendingPermissionRequestDto {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: None,
    }
}

#[test]
fn sync_permission_requests_preserves_existing_selection_after_sort() {
    let (requests, selected_request_id) = sync_permission_requests(
        Some("perm-2"),
        vec![request("perm-3"), request("perm-2"), request("perm-1")],
    );

    assert_eq!(
        requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["perm-1", "perm-2", "perm-3"]
    );
    assert_eq!(selected_request_id.as_deref(), Some("perm-2"));
}

#[test]
fn advance_permission_requests_after_reply_selects_next_request() {
    let (requests, selected_request_id) = advance_permission_requests_after_reply(
        Some("perm-2"),
        vec![request("perm-1"), request("perm-2"), request("perm-3")],
    );

    assert_eq!(
        requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["perm-1", "perm-3"]
    );
    assert_eq!(selected_request_id.as_deref(), Some("perm-3"));
}

#[test]
fn advance_permission_requests_after_reply_falls_back_to_previous_request() {
    let (requests, selected_request_id) = advance_permission_requests_after_reply(
        Some("perm-3"),
        vec![request("perm-1"), request("perm-2"), request("perm-3")],
    );

    assert_eq!(
        requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["perm-1", "perm-2"]
    );
    assert_eq!(selected_request_id.as_deref(), Some("perm-2"));
}

#[test]
fn split_auto_approved_permission_requests_separates_active_session_requests() {
    let mut other_request = request("perm-2");
    other_request.session_id = "session-2".to_string();

    let (remaining_requests, auto_approve_request_ids) = split_auto_approved_permission_requests(
        Some("session-1"),
        vec![request("perm-1"), other_request, request("perm-3")],
    );

    assert_eq!(
        remaining_requests.iter().map(|request| request.id.as_str()).collect::<Vec<_>>(),
        vec!["perm-2"]
    );
    assert_eq!(auto_approve_request_ids, vec!["perm-1", "perm-3"]);
}
