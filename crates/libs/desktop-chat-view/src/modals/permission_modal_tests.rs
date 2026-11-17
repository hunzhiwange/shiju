use super::*;
use serde_json::{Map, json};
fn request(id: &str) -> gateway_interaction::PendingPermissionRequestDto {
    gateway_interaction::PendingPermissionRequestDto {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: Map::new(),
        always: Vec::new(),
        tool: None,
    }
}

#[test]
fn permission_modal_title_prefers_trimmed_metadata_title() {
    let mut req = request("perm-1");
    req.metadata.insert("title".to_string(), json!("  Approve shell  "));

    assert_eq!(
        permission_modal_title_localized(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            &req
        ),
        "Approve shell"
    );
}

#[test]
fn permission_modal_title_falls_back_to_argument_summary_or_permission() {
    let mut req = request("perm-1");
    req.metadata.insert("arguments".to_string(), json!("echo hello"));

    assert!(
        permission_modal_title_localized(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            &req
        )
        .contains("echo hello")
    );

    req.metadata.clear();
    assert_eq!(
        permission_modal_title_localized(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            &req
        ),
        "需要批准 shell 操作"
    );
}

#[test]
fn permission_modal_title_ignores_blank_and_non_string_titles() {
    let mut req = request("perm-1");
    req.metadata.insert("arguments".to_string(), json!({"cmd": "pwd"}));
    req.metadata.insert("title".to_string(), json!("   "));

    assert_eq!(
        permission_modal_title_localized(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            &req
        ),
        "批准运行：pwd"
    );

    req.metadata.insert("title".to_string(), json!(42));
    assert_eq!(
        permission_modal_title_localized(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            &req
        ),
        "批准运行：pwd"
    );
}

#[test]
fn permission_metadata_text_trims_empty_values() {
    let mut req = request("perm-1");
    req.metadata.insert("requested_by".to_string(), json!("  codex  "));
    req.metadata.insert("requested_channel".to_string(), json!("   "));

    assert_eq!(permission_metadata_text(&req, "requested_by").as_deref(), Some("codex"));
    assert_eq!(permission_metadata_text(&req, "requested_channel"), None);
    assert_eq!(permission_metadata_text(&req, "missing"), None);
}

#[test]
fn permission_metadata_text_rejects_non_string_values() {
    let mut req = request("perm-1");
    req.metadata.insert("requested_by".to_string(), json!({"name": "codex"}));

    assert_eq!(permission_metadata_text(&req, "requested_by"), None);
}

#[test]
fn permission_argument_summary_handles_strings_objects_and_nulls() {
    let mut req = request("perm-1");
    req.metadata.insert("arguments".to_string(), json!("  long raw argument  "));
    assert_eq!(permission_argument_summary(&req).as_deref(), Some("long raw argument"));

    req.metadata.insert("arguments".to_string(), json!({"cmd": "pwd"}));
    assert!(permission_argument_summary(&req).is_some());

    req.metadata.insert("arguments".to_string(), json!(null));
    assert_eq!(permission_argument_summary(&req), None);
}

#[test]
fn permission_argument_summary_rejects_missing_empty_and_unsupported_arguments() {
    let mut req = request("perm-1");
    assert_eq!(permission_argument_summary(&req), None);

    req.metadata.insert("arguments".to_string(), json!("   "));
    assert_eq!(permission_argument_summary(&req), None);

    req.permission = "custom_tool".to_string();
    req.metadata.insert("arguments".to_string(), json!({"opaque": true}));
    assert_eq!(permission_argument_summary(&req), None);
}

#[test]
fn permission_arguments_preview_skips_empty_values_and_preserves_large_json() {
    let mut req = request("perm-1");
    req.metadata.insert("arguments".to_string(), json!({}));
    assert_eq!(permission_arguments_preview(&req), None);

    req.metadata.insert("arguments".to_string(), json!({"cmd": "pwd"}));
    assert!(permission_arguments_preview(&req).expect("preview").contains("\"cmd\""));

    req.metadata.insert("arguments".to_string(), json!({"text": "x".repeat(500)}));
    let preview = permission_arguments_preview(&req).expect("complete preview");
    assert_eq!(
        preview,
        serde_json::to_string_pretty(req.metadata.get("arguments").expect("arguments")).unwrap()
    );
    let arguments: serde_json::Value = serde_json::from_str(&preview).expect("valid JSON");
    assert_eq!(arguments["text"], "x".repeat(500));
}

#[test]
fn permission_arguments_preview_handles_missing_null_and_scalar_values() {
    let mut req = request("perm-1");
    assert_eq!(permission_arguments_preview(&req), None);

    req.metadata.insert("arguments".to_string(), json!(null));
    assert_eq!(permission_arguments_preview(&req), None);

    req.metadata.insert("arguments".to_string(), json!("value"));
    assert_eq!(permission_arguments_preview(&req).as_deref(), Some("\"value\""));
}

#[test]
fn permission_request_selector_label_uses_permission_and_summary() {
    let mut req = request("perm-1");
    req.metadata.insert("arguments".to_string(), json!("echo hello"));

    let label = permission_request_selector_label_localized(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &req,
    );

    assert!(label.contains("echo hello"));
    assert!(label.contains("·"));
}

#[test]
fn permission_request_selector_label_falls_back_and_truncates_by_character() {
    let mut req = request("perm-1");
    assert_eq!(
        permission_request_selector_label_localized(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            &req
        ),
        "运行 · shell"
    );

    req.metadata.insert("arguments".to_string(), json!("命令".repeat(50)));
    let label = permission_request_selector_label_localized(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &req,
    );

    assert_eq!(label.chars().count(), 81);
    assert!(label.starts_with("运行 · 命令"));
    assert!(label.ends_with('…'));
}

#[test]
fn permission_detail_helpers_build_rows_and_blocks() {
    let _line = permission_meta_line::<()>("权限", "shell");
    let _block = permission_detail_block::<()>("参数", "{\"cmd\":\"pwd\"}".to_string());
}

#[test]
fn permission_modal_queue_inserts_current_request_when_queue_is_empty() {
    let current = request("perm-current");

    let queue = permission_modal_queue(&current, &[]);

    assert_eq!(queue.requests.len(), 1);
    assert_eq!(queue.requests[0].id, "perm-current");
    assert_eq!(queue.current_index, 0);
    assert_eq!(queue.previous_request_id, None);
    assert_eq!(queue.next_request_id, None);
}

#[test]
fn permission_modal_queue_tracks_previous_and_next_ids_for_matching_request() {
    let requests = vec![request("perm-1"), request("perm-2"), request("perm-3")];

    let queue = permission_modal_queue(&requests[1], &requests);

    assert_eq!(queue.requests.len(), 3);
    assert_eq!(queue.current_index, 1);
    assert_eq!(queue.previous_request_id.as_deref(), Some("perm-1"));
    assert_eq!(queue.next_request_id.as_deref(), Some("perm-3"));
}

#[test]
fn permission_modal_queue_uses_first_position_when_current_request_is_missing() {
    let requests = vec![request("perm-1"), request("perm-2")];

    let queue = permission_modal_queue(&request("perm-current"), &requests);

    assert_eq!(queue.requests.len(), 2);
    assert_eq!(queue.current_index, 0);
    assert_eq!(queue.previous_request_id, None);
    assert_eq!(queue.next_request_id.as_deref(), Some("perm-2"));
}
