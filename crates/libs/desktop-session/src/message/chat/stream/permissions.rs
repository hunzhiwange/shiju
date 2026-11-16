use gateway_interaction::PendingPermissionRequestDto;
pub fn split_auto_approved_permission_requests(
    session_id: Option<&str>,
    requests: Vec<PendingPermissionRequestDto>,
) -> (Vec<PendingPermissionRequestDto>, Vec<String>) {
    let Some(session_id) = session_id else {
        return (requests, Vec::new());
    };

    let mut remaining_requests = Vec::new();
    let mut auto_approve_request_ids = Vec::new();
    for request in requests {
        if request.session_id == session_id
            && request.metadata.get("requested_channel").and_then(serde_json::Value::as_str)
                != Some("desktop-acp")
        {
            auto_approve_request_ids.push(request.id);
        } else {
            remaining_requests.push(request);
        }
    }

    (remaining_requests, auto_approve_request_ids)
}
pub fn sync_permission_requests(
    current_request_id: Option<&str>,
    mut requests: Vec<PendingPermissionRequestDto>,
) -> (Vec<PendingPermissionRequestDto>, Option<String>) {
    requests.sort_by(|a, b| a.id.cmp(&b.id));
    if requests.is_empty() {
        return (requests, None);
    }

    let selected_request_id = current_request_id
        .filter(|current| requests.iter().any(|request| request.id == *current))
        .map(ToOwned::to_owned)
        .or_else(|| requests.first().map(|request| request.id.clone()));

    (requests, selected_request_id)
}
pub fn advance_permission_requests_after_reply(
    current_request_id: Option<&str>,
    requests: Vec<PendingPermissionRequestDto>,
) -> (Vec<PendingPermissionRequestDto>, Option<String>) {
    let (mut requests, selected_request_id) =
        sync_permission_requests(current_request_id, requests);
    let Some(selected_request_id) = selected_request_id else {
        return (requests, None);
    };
    let Some(selected_idx) = requests.iter().position(|request| request.id == selected_request_id)
    else {
        return (requests, None);
    };

    requests.remove(selected_idx);
    if requests.is_empty() {
        return (requests, None);
    }

    let next_idx = selected_idx.min(requests.len().saturating_sub(1));
    let next_request_id = requests.get(next_idx).map(|request| request.id.clone());
    (requests, next_request_id)
}

#[cfg(test)]
#[path = "permissions_tests.rs"]
mod permissions_tests;

pub struct PermissionState<'a> {
    pub permission_modal_request_id: &'a mut Option<String>,
    pub permission_modal_request: &'a mut Option<PendingPermissionRequestDto>,
    pub permission_modal_requests: &'a mut Vec<PendingPermissionRequestDto>,
}
pub fn clear_permission_modal(app: &mut PermissionState<'_>) {
    *app.permission_modal_request_id = None;
    *app.permission_modal_request = None;
    app.permission_modal_requests.clear();
}
pub fn apply_current_permission_request(app: &mut PermissionState<'_>, request_id: Option<String>) {
    let Some(request_id) = request_id else {
        *app.permission_modal_request_id = None;
        *app.permission_modal_request = None;
        return;
    };

    let Some(request) =
        app.permission_modal_requests.iter().find(|candidate| candidate.id == request_id).cloned()
    else {
        *app.permission_modal_request_id = None;
        *app.permission_modal_request = None;
        return;
    };

    *app.permission_modal_request_id = Some(request.id.clone());
    *app.permission_modal_request = Some(request);
}
pub fn apply_permission_request(app: &mut PermissionState<'_>, requests: Vec<PendingPermissionRequestDto>) {
    let (pending_requests, selected_request_id) =
        sync_permission_requests(app.permission_modal_request_id.as_deref(), requests);
    if pending_requests.is_empty() {
        clear_permission_modal(app);
        return;
    }

    *app.permission_modal_requests = pending_requests;
    apply_current_permission_request(app, selected_request_id);
}
