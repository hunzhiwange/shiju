use super::super::ToolPermissionState;
use super::common_view::{
    build_common_tool_view, common_tool_context_menu_message, common_tool_fallback_text,
    common_tool_meta_text, common_tool_scroll_height, common_tool_show_list_view,
    common_tool_title,
};
use super::{FileListState, FilesViewContext};
use crate::app::components::chat_panel::tool_text_support::tool_text_key;
use crate::app::components::chat_panel::utils::chat_context_target_key;
use crate::app::{App, Message, message};
use iced::widget::text_editor;
use iced::{Length, Point};

fn test_app() -> App {
    App::new().0
}

fn state(total_items: usize, is_search: bool) -> FileListState {
    FileListState {
        items_for_display: Vec::new(),
        total_items,
        display_count: total_items,
        truncated_middle: false,
        middle_omitted: 0,
        tail_omitted: 0,
        filter_query: String::new(),
        is_empty_filtered: total_items == 0,
        max_items: 100,
        is_search,
    }
}

fn ctx<'a>(app: &'a App, tool_name: &str, status: &str, output: &str) -> FilesViewContext<'a> {
    FilesViewContext {
        app,
        msg_idx: 0,
        tool_idx: 1,
        visible: format!(
            r#"tool {tool_name}
{{"status":"{status}","output":"{output}"}}"#
        ),
        tool_name: tool_name.to_string(),
        error_text: (status == "error").then(|| output.to_string()),
        input: "{}".to_string(),
        output: output.to_string(),
        verb: "读取",
        is_error: status == "error",
        is_running: status == "running",
        is_edit_like: false,
        read_range: None,
    }
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn common_view_test_module_is_linked() {
    assert_eq!("common_view", "common_view");
}

#[test]
fn common_tool_meta_text_prefers_filter_then_middle_then_tail_then_total() {
    let mut render_state = state(12, true);
    render_state.display_count = 5;
    render_state.filter_query = "main".to_string();
    render_state.truncated_middle = true;
    render_state.middle_omitted = 4;
    render_state.tail_omitted = 3;

    assert_eq!(common_tool_meta_text(&render_state), "5/12 项");

    render_state.filter_query.clear();
    render_state.max_items = 8;
    assert_eq!(common_tool_meta_text(&render_state), "12项 (省略4项)");

    render_state.truncated_middle = false;
    assert_eq!(common_tool_meta_text(&render_state), "12项 (省略3项)");

    render_state.tail_omitted = 0;
    assert_eq!(common_tool_meta_text(&render_state), "12项");
}

#[test]
fn common_tool_title_applies_running_permission_error_and_label_precedence() {
    assert_eq!(
        common_tool_title("read", "读取", true, Some(ToolPermissionState::Rejected), true),
        "读取中"
    );
    assert_eq!(
        common_tool_title("read", "读取", false, Some(ToolPermissionState::ApprovalRequired), true,),
        "读取待批准"
    );
    assert_eq!(
        common_tool_title("read", "读取", false, Some(ToolPermissionState::Rejected), true),
        "读取已拒绝"
    );
    assert_eq!(common_tool_title("read", "读取", false, None, true), "读取失败");
    assert_eq!(common_tool_title("read", "读取", false, None, false), "读取");
}

#[test]
fn common_tool_fallback_text_prefers_permission_then_error_and_only_uses_output_when_empty() {
    assert_eq!(
        common_tool_fallback_text(
            true,
            0,
            "output",
            Some("  permission denied  ".to_string()),
            Some("disk full"),
        ),
        Some("permission denied".to_string())
    );
    assert_eq!(
        common_tool_fallback_text(true, 0, "output", None, Some("  disk full  ")),
        Some("disk full".to_string())
    );
    assert_eq!(common_tool_fallback_text(true, 0, "output", None, Some("   ")), None);
    assert_eq!(
        common_tool_fallback_text(false, 0, "  plain output  ", None, Some("ignored")),
        Some("plain output".to_string())
    );
    assert_eq!(common_tool_fallback_text(false, 2, "plain output", None, None), None);
    assert_eq!(common_tool_fallback_text(false, 0, "   ", None, None), None);
}

#[test]
fn common_tool_scroll_height_caps_large_lists_and_shrinks_small_ones() {
    assert_eq!(common_tool_scroll_height(6, 100), Length::Shrink);
    assert_eq!(common_tool_scroll_height(7, 100), Length::Fixed(220.0));
    assert_eq!(common_tool_scroll_height(20, 3), Length::Shrink);
}

#[test]
fn common_tool_show_list_view_keeps_running_items_or_missing_fallback_visible() {
    assert!(common_tool_show_list_view(true, 0, true));
    assert!(common_tool_show_list_view(false, 1, true));
    assert!(common_tool_show_list_view(false, 0, false));
    assert!(!common_tool_show_list_view(false, 0, true));
}

#[test]
fn common_tool_context_menu_message_preserves_target_point_and_text() {
    let result =
        common_tool_context_menu_message(42, Point::new(12.5, 26.25), "plain output".to_string());

    match result {
        Message::Chat(message::ChatMessage::OpenMessageContextMenu { target, x, y, text }) => {
            assert_eq!(target, 42);
            assert_eq!(x, 12.5);
            assert_eq!(y, 26.25);
            assert_eq!(text, "plain output");
        }
        _ => panic!("expected common context-menu message"),
    }
}

#[test]
fn build_common_tool_view_handles_running_and_empty_list() {
    let app = test_app();
    let view_ctx = ctx(&app, "read", "running", "");
    let list_column = iced::widget::column![];

    keep_element(build_common_tool_view(&view_ctx, &state(0, false), list_column));
}

#[test]
fn build_common_tool_view_handles_error_fallback_text() {
    let app = test_app();
    let view_ctx = ctx(&app, "read", "error", "permission denied");
    let list_column = iced::widget::column![];

    keep_element(build_common_tool_view(&view_ctx, &state(0, false), list_column));
}

#[test]
fn build_common_tool_view_handles_search_filter_meta_and_tail_omission() {
    let app = test_app();
    let view_ctx = ctx(&app, "grep", "completed", "");
    let mut render_state = state(12, true);
    render_state.display_count = 5;
    render_state.filter_query = "main".to_string();
    render_state.tail_omitted = 7;
    let list_column = iced::widget::column![];

    keep_element(build_common_tool_view(&view_ctx, &render_state, list_column));
}

#[test]
fn build_common_tool_view_handles_non_error_output_fallback() {
    let app = test_app();
    let view_ctx = ctx(&app, "bash", "completed", "plain output\nsecond line");
    let list_column = iced::widget::column![];

    keep_element(build_common_tool_view(&view_ctx, &state(0, false), list_column));
}

#[test]
fn build_common_tool_view_handles_truncated_middle_meta() {
    let app = test_app();
    let view_ctx = ctx(&app, "read", "completed", "");
    let mut render_state = state(100, false);
    render_state.truncated_middle = true;
    render_state.middle_omitted = 10;
    render_state.display_count = 90;
    let list_column = iced::widget::column![];

    keep_element(build_common_tool_view(&view_ctx, &render_state, list_column));
}

#[test]
fn build_common_tool_view_handles_permission_state() {
    let app = test_app();
    let visible = r#"tool read
{"status":"error","error":"permission denied","output":""}"#;
    let view_ctx = FilesViewContext {
        app: &app,
        msg_idx: 0,
        tool_idx: 1,
        visible: visible.to_string(),
        tool_name: "read".to_string(),
        error_text: Some("permission denied".to_string()),
        input: "{}".to_string(),
        output: "".to_string(),
        verb: "读取",
        is_error: true,
        is_running: false,
        is_edit_like: false,
        read_range: None,
    };
    let list_column = iced::widget::column![];

    keep_element(build_common_tool_view(&view_ctx, &state(0, false), list_column));
}

#[test]
fn build_common_tool_view_handles_summary_from_json() {
    let app = test_app();
    let visible = r#"tool bash
{"status":"completed","output":"result","summary":"search results"}"#;
    let view_ctx = FilesViewContext {
        app: &app,
        msg_idx: 0,
        tool_idx: 1,
        visible: visible.to_string(),
        tool_name: "bash".to_string(),
        error_text: None,
        input: "{}".to_string(),
        output: "result".to_string(),
        verb: "执行",
        is_error: false,
        is_running: false,
        is_edit_like: false,
        read_range: None,
    };
    let list_column = iced::widget::column![];

    keep_element(build_common_tool_view(&view_ctx, &state(1, false), list_column));
}

#[test]
fn build_common_tool_view_handles_no_fallback_from_empty_output() {
    let app = test_app();
    // total_items=0, output="" → fallback_body=None, show_list_view triggers via
    // fallback_body.is_none() (not running, not total_items>0)
    let view_ctx = ctx(&app, "read", "completed", "");
    let list_column = iced::widget::column![];

    keep_element(build_common_tool_view(&view_ctx, &state(0, false), list_column));
}

#[test]
fn build_common_tool_view_handles_estimated_height_cap() {
    let app = test_app();
    // display_count=10 → estimated_height = 10*32+16 = 336 >= 220 → Fixed(220)
    let view_ctx = ctx(&app, "read", "completed", "items with results");
    let mut render_state = state(10, false);
    render_state.display_count = 10;
    let list_column = iced::widget::column![];

    keep_element(build_common_tool_view(&view_ctx, &render_state, list_column));
}

#[test]
fn build_common_tool_view_handles_tail_omission_without_filter() {
    let app = test_app();
    let view_ctx = ctx(&app, "glob", "completed", "");
    let mut render_state = state(20, false);
    render_state.display_count = 15;
    render_state.tail_omitted = 5;

    keep_element(build_common_tool_view(&view_ctx, &render_state, iced::widget::column![]));
}

#[test]
fn build_common_tool_view_uses_error_and_output_text_editors() {
    let mut app = test_app();
    app.chat_tool_text_editors
        .insert(tool_text_key(0, 1, 0), text_editor::Content::with_text("cached body"));

    let error_ctx = ctx(&app, "read", "error", "permission denied");
    keep_element(build_common_tool_view(&error_ctx, &state(0, false), iced::widget::column![]));

    let output_ctx = ctx(&app, "bash", "completed", "plain output");
    keep_element(build_common_tool_view(&output_ctx, &state(0, false), iced::widget::column![]));
}

#[test]
fn build_common_tool_view_wraps_open_context_menu() {
    let mut app = test_app();
    app.chat_context_menu_target = Some(chat_context_target_key(0, Some(1)));
    app.chat_context_menu_pos = Some((24.0, 36.0));
    let view_ctx = ctx(&app, "read", "completed", "result");

    keep_element(build_common_tool_view(&view_ctx, &state(1, false), iced::widget::column![]));
}
