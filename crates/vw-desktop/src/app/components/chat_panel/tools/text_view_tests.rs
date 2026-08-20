use super::text_view::{
    tool_text_detail_message, tool_text_failure_title, tool_text_header_label, tool_text_view,
    workflow_preview_message,
};
use super::tool_detail_dialog::tool_detail_dialog_view;
use crate::app::{App, Message, message};
use iced::advanced::renderer::Headless;
use iced::advanced::text::highlighter::PlainText;
use iced::advanced::widget::{Tree, tree::Tag};
use iced::advanced::{Layout, layout, mouse, renderer};
use iced::widget::text_editor;
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};

fn app() -> App {
    App::new().0
}

fn tree_contains_tag(tree: &Tree, tag: Tag) -> bool {
    tree.tag == tag || tree.children.iter().any(|child| tree_contains_tag(child, tag))
}

fn assert_card_has_no_inline_editor(element: &iced::Element<'_, Message>) {
    let tree = Tree::new(element.as_widget());
    let editor_tag = Tag::of::<text_editor::State<PlainText>>();

    assert!(!tree_contains_tag(&tree, editor_tag));
}

#[test]
fn text_view_test_module_is_linked() {
    assert_eq!("text_view", "text_view");
}

#[test]
fn text_tool_header_includes_canonical_tool_name() {
    assert_eq!(tool_text_header_label("rustcodegraph", "工具"), "工具 rustcodegraph");
    assert_eq!(tool_text_header_label("shell", "运行"), "运行 bash");
    assert_eq!(tool_text_failure_title("工具 rustcodegraph"), "工具 rustcodegraph 失败");
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
fn text_view_builds_workflow_preview_card() {
    let app = app();
    let msg_idx = 3;
    let tool_idx = 4;

    let visible = r#"tool workflow_node
{"status":"completed","input":"{\"node\":\"alpha\"}","output":"workflow output","metadata":{"workflow_yaml":"nodes: []","node_id":"alpha"}}"#;

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

#[test]
fn text_tool_card_keeps_full_output_only_in_detail_action() {
    let mut app = app();
    let msg_idx = 7;
    let tool_idx = 8;
    app.chat_tool_hovered_idx = Some(((msg_idx as u64) << 32) | tool_idx as u64);
    let full_output = format!("FULL_RESULT_MARKER\n{}", "large result line\n".repeat(32_768));
    let visible = format!(
        "tool search\n{}",
        serde_json::json!({
            "status": "completed",
            "input": "{\"query\":\"needle\"}",
            "output": full_output,
        })
    );

    let element = tool_text_view(&app, msg_idx, tool_idx, &visible).expect("success card");
    assert_card_has_no_inline_editor(&element);
    assert!(matches!(
        tool_text_detail_message(msg_idx, tool_idx, &visible),
        Message::Chat(message::ChatMessage::OpenToolDetail(7, 8, ref raw))
            if raw == &visible && raw.contains("FULL_RESULT_MARKER")
    ));
}

#[test]
fn text_tool_error_card_keeps_detail_entry_without_inline_editor() {
    let mut app = app();
    let msg_idx = 9;
    let tool_idx = 10;
    app.chat_tool_hovered_idx = Some(((msg_idx as u64) << 32) | tool_idx as u64);
    let full_error = format!("权限检查失败：中文错误标题\n{}", "错误详情\n".repeat(32_768));
    let visible = format!(
        "tool search\n{}",
        serde_json::json!({
            "status": "error",
            "input": "{}",
            "error": full_error,
        })
    );

    let element = tool_text_view(&app, msg_idx, tool_idx, &visible).expect("error card");
    assert_card_has_no_inline_editor(&element);
    assert!(matches!(
        tool_text_detail_message(msg_idx, tool_idx, &visible),
        Message::Chat(message::ChatMessage::OpenToolDetail(9, 10, ref raw))
            if raw == &visible && raw.contains("中文错误标题")
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn text_tool_error_card_dark_chinese_detail_render_path_is_safe() {
    let mut app = app();
    let full_error = format!("权限检查失败：中文错误标题\n{}", "错误详情\n".repeat(4_096));
    let visible = format!(
        "tool search\n{}",
        serde_json::json!({
            "status": "error",
            "input": "{}",
            "error": full_error,
        })
    );
    let Message::Chat(open_detail) = tool_text_detail_message(11, 12, &visible) else {
        panic!("expected chat detail action");
    };

    let _task = crate::app::message::chat::update(&mut app, open_detail);
    let dialog = app.tool_detail_dialog.as_ref().expect("detail dialog opens");
    assert_eq!(dialog.content.trim_end(), full_error.trim_end());
    assert!(dialog.title.contains("失败"));

    let mut element = tool_detail_dialog_view(&app).expect("detail dialog view");
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let bounds = Size::new(960.0, 720.0);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, bounds),
    );
    let viewport = Rectangle::with_size(bounds);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
    let pixels = renderer.screenshot(Size::new(960, 720), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));

    if let Ok(path) = std::env::var("VW_TEXT_TOOL_DETAIL_SCREENSHOT") {
        image::save_buffer(path, &pixels, 960, 720, image::ColorType::Rgba8)
            .expect("save detail screenshot");
    }
}
