use super::text_view::{copy_content_hash, tool_text_view, workflow_preview_message};
use super::tool_text_key;
use crate::app::{App, Message};
use iced::widget::text_editor;
use web_time::{Duration as WebDuration, SystemTime as WebSystemTime};

fn app() -> App {
    App::new().0
}

#[test]
fn text_view_test_module_is_linked() {
    assert_eq!("text_view", "text_view");
}

#[test]
fn copy_hash_is_stable_and_content_sensitive() {
    assert_eq!(copy_content_hash("same"), copy_content_hash("same"));
    assert_ne!(copy_content_hash("same"), copy_content_hash("different"));
}

#[test]
fn workflow_preview_message_requires_workflow_metadata() {
    let value = serde_json::json!({
        "metadata": {
            "workflow_yaml": "nodes: []",
            "node_id": "node-1"
        }
    });

    assert!(matches!(
        workflow_preview_message("workflow_node", &value),
        Some(Message::WorkflowTool(_))
    ));

    assert!(workflow_preview_message("bash", &value).is_none());
    assert!(
        workflow_preview_message("workflow_node", &serde_json::json!({"metadata": {}})).is_none()
    );
}

#[test]
fn workflow_preview_message_trims_yaml_and_ignores_blank_node_ids() {
    let value = serde_json::json!({
        "metadata": {
            "workflow_yaml": "  nodes:\n    - id: a  ",
            "node_id": "   "
        }
    });

    assert!(matches!(
        workflow_preview_message("WORKFLOW_NODE", &value),
        Some(Message::WorkflowTool(_))
    ));

    assert!(
        workflow_preview_message(
            "workflow_node",
            &serde_json::json!({"metadata": {"workflow_yaml": "   "}})
        )
        .is_none()
    );
    assert!(
        workflow_preview_message("workflow_node", &serde_json::json!({"metadata": []})).is_none()
    );
}

#[test]
fn workflow_preview_message_normalizes_payload_fields() {
    let value = serde_json::json!({
        "metadata": {
            "workflow_yaml": "  nodes:\n    - id: alpha  ",
            "node_id": "  alpha  "
        }
    });

    let Some(Message::WorkflowTool(crate::apps::workflow::WorkflowMessage::OpenInlineYaml {
        workflow_yaml,
        focus_node_id,
    })) = workflow_preview_message("workflow_node", &value)
    else {
        panic!("expected inline workflow preview message");
    };

    assert_eq!(workflow_yaml, "nodes:\n    - id: alpha");
    assert_eq!(focus_node_id.as_deref(), Some("alpha"));
}

#[test]
fn text_view_filters_special_tools_and_bad_blocks() {
    let app = app();

    assert!(tool_text_view(&app, 0, 0, "tool bash\n{}").is_none());
    assert!(tool_text_view(&app, 0, 0, "tool read\n{}").is_none());
    assert!(tool_text_view(&app, 0, 0, "tool custom\nnot-json").is_none());
    assert!(tool_text_view(&app, 0, 0, "custom\n{}").is_none());
    assert!(tool_text_view(&app, 0, 0, "tool custom").is_none());
    assert!(tool_text_view(&app, 0, 0, "tool   \n{}").is_none());
}

#[test]
fn text_view_builds_success_error_permission_and_summary_only_cards() {
    let mut app = app();
    app.chat_tool_hovered_idx = Some((1_u64 << 32) | 2);

    let success = r#"tool grep
{"status":"completed","input":"{\"pattern\":\"App\"}","output":"match line"}"#;
    let error = r#"tool grep
{"status":"error","input":"{}","error":"failed hard"}"#;
    let denied = r#"tool file_write
{"status":"denied","input":"{\"path\":\"src/lib.rs\"}","error":"permission denied"}"#;
    let summary_only = r#"tool question
{"status":"completed","input":"What next?","summary":"Need answer"}"#;
    let empty = r#"tool question
{"status":"completed","input":"","output":""}"#;

    assert!(tool_text_view(&app, 1, 2, success).is_some());
    assert!(tool_text_view(&app, 1, 3, error).is_some());
    assert!(tool_text_view(&app, 1, 4, denied).is_some());
    assert!(tool_text_view(&app, 1, 5, summary_only).is_some());
    assert!(tool_text_view(&app, 1, 6, empty).is_none());
}

#[test]
fn text_view_still_renders_excluded_tools_when_they_fail() {
    let app = app();
    let bash_error = r#"tool bash
{"status":"error","input":"{\"cmd\":\"false\"}","error":"command failed"}"#;
    let read_denied = r#"tool read
{"status":"denied","input":"{\"path\":\"secret.txt\"}","error":"permission denied"}"#;

    assert!(tool_text_view(&app, 6, 0, bash_error).is_some());
    assert!(tool_text_view(&app, 6, 1, read_denied).is_some());
}

#[test]
fn text_view_builds_cards_from_result_content_and_render_hint_summary() {
    let app = app();
    let result_text = r#"tool search
{"result":{"success":true,"content":[{"type":"text","text":" result body "}]},"renderHint":{"summary":" hinted summary "}}"#;
    let result_json = r#"tool search
{"result":{"success":true,"content":[{"type":"json","value":{"message":"json body"}}]}}"#;
    let model_result = r#"tool task
{"result":{"success":true,"model_result":{"content":"model body"}}}"#;
    let result_data = r#"tool task
{"result":{"success":true,"data":{"value":"data body"}}}"#;
    let failed_output = r#"tool search
{"result":{"success":false,"content":[{"type":"text","text":"result error"}]}}"#;

    assert!(tool_text_view(&app, 2, 0, result_text).is_some());
    assert!(tool_text_view(&app, 2, 1, result_json).is_some());
    assert!(tool_text_view(&app, 2, 2, model_result).is_some());
    assert!(tool_text_view(&app, 2, 3, result_data).is_some());
    assert!(tool_text_view(&app, 2, 4, failed_output).is_some());
}

#[test]
fn text_view_handles_context_menu_workflow_preview_and_text_editors() {
    let mut app = app();
    let msg_idx = 3;
    let tool_idx = 4;
    app.chat_context_menu_target = Some((msg_idx as u64) << 32 | ((tool_idx as u64) + 1));
    app.chat_context_menu_pos = Some((24.0, 48.0));
    app.chat_tool_text_editors.insert(
        tool_text_key(msg_idx, tool_idx, 0),
        text_editor::Content::with_text("editor output"),
    );

    let visible = r#"tool workflow_node
{"status":"completed","input":"{\"node\":\"alpha\"}","output":"workflow output","metadata":{"workflow_yaml":"nodes: []","node_id":"alpha"}}"#;

    assert!(tool_text_view(&app, msg_idx, tool_idx, visible).is_some());
}

#[test]
fn text_view_context_menu_prefers_selected_tool_editor_text() {
    let mut app = app();
    let msg_idx = 7;
    let tool_idx = 8;
    let context_key = (msg_idx as u64) << 32 | ((tool_idx as u64) + 1);
    let editor_key = tool_text_key(msg_idx, tool_idx, 0);
    app.chat_context_menu_target = Some(context_key);
    app.chat_tool_text_editors
        .insert(editor_key, text_editor::Content::with_text("selected output text"));
    app.chat_tool_text_editors
        .get_mut(&editor_key)
        .expect("editor")
        .perform(text_editor::Action::SelectAll);

    let visible = r#"tool grep
{"status":"completed","input":"{}","output":"fallback output"}"#;

    assert!(tool_text_view(&app, msg_idx, tool_idx, visible).is_some());
}

#[test]
fn text_view_error_card_uses_recent_copy_and_editor_paths() {
    let mut app = app();
    let msg_idx = 4;
    let tool_idx = 5;
    let error_text = "failed hard";
    app.last_copied_code_hash = Some(copy_content_hash(error_text));
    app.last_copy_time = Some(WebSystemTime::now() - WebDuration::from_millis(500));
    app.chat_tool_text_editors
        .insert(tool_text_key(msg_idx, tool_idx, 0), text_editor::Content::with_text(error_text));

    let visible = r#"tool grep
{"status":"error","input":"{}","error":"failed hard"}"#;

    assert!(tool_text_view(&app, msg_idx, tool_idx, visible).is_some());
}

#[test]
fn text_view_summarizes_from_input_and_truncated_output() {
    let app = app();
    let input_summary = r#"tool grep
{"status":"completed","input":"{\"pattern\":\"needle\",\"path\":\"src\"}","output":"match"}"#;
    let output_summary = format!(
        "tool custom\n{}",
        serde_json::json!({
            "status": "completed",
            "input": "",
            "output": "x".repeat(140)
        })
    );

    assert!(tool_text_view(&app, 5, 0, input_summary).is_some());
    assert!(tool_text_view(&app, 5, 1, &output_summary).is_some());
}
