//! 验证写文件工具的摘要、标题状态、详情回退和折叠视图契约。
//! 空白文件名、根路径和空列表用于约束计数回退，不把不可展示名称当作有效摘要。

use iced::{Length, Point, Theme};

use super::super::ToolPermissionState;
use super::write_view::{
    build_write_tool_view, write_tool_context_menu_message, write_tool_detail_color,
    write_tool_detail_text, write_tool_summary, write_tool_title,
};
use super::{FileListState, FilesViewContext};
use crate::app::{App, Message, message};

fn file_list_state(items: Vec<(String, String)>) -> FileListState {
    FileListState {
        total_items: items.len(),
        display_count: items.len(),
        items_for_display: items,
        truncated_middle: false,
        middle_omitted: 0,
        tail_omitted: 0,
        filter_query: String::new(),
        is_empty_filtered: false,
        max_items: 100,
        is_search: false,
    }
}

fn view_context<'a>(
    app: &'a App,
    visible: &str,
    error_text: Option<&str>,
    is_error: bool,
    is_running: bool,
) -> FilesViewContext<'a> {
    FilesViewContext {
        app,
        msg_idx: 1,
        tool_idx: 2,
        visible: visible.to_string(),
        tool_name: "write".to_string(),
        error_text: error_text.map(str::to_string),
        input: "{}".to_string(),
        output: String::new(),
        verb: "写入",
        is_error,
        is_running,
        is_edit_like: true,
        read_range: None,
    }
}

fn assert_closed_view(element: iced::Element<'_, crate::app::Message>) {
    assert_eq!(element.as_widget().children().len(), 1);
    assert_eq!(element.as_widget().size().width, Length::Fill);
}

#[test]
fn write_tool_summary_uses_first_file_name_and_total_count() {
    let state = file_list_state(vec![
        ("src/main.rs".to_string(), "/tmp/src/main.rs".to_string()),
        ("src/lib.rs".to_string(), "/tmp/src/lib.rs".to_string()),
    ]);

    assert_eq!(write_tool_summary(&state), Some("main.rs 等 2 个文件".to_string()));
}

#[test]
fn write_tool_summary_uses_single_file_name() {
    let state = file_list_state(vec![("src/main.rs".to_string(), "/tmp/src/main.rs".to_string())]);

    assert_eq!(write_tool_summary(&state), Some("main.rs".to_string()));
}

#[test]
fn write_tool_summary_falls_back_to_file_count_for_blank_names() {
    let state = file_list_state(vec![("".to_string(), "/tmp/src/main.rs".to_string())]);

    assert_eq!(write_tool_summary(&state), Some("1 个文件".to_string()));
}

#[test]
fn write_tool_summary_returns_none_for_empty_state() {
    assert_eq!(write_tool_summary(&file_list_state(Vec::new())), None);
}

#[test]
fn write_tool_summary_trims_the_first_file_name() {
    let state =
        file_list_state(vec![("src/main.rs  ".to_string(), "/tmp/src/main.rs".to_string())]);

    assert_eq!(write_tool_summary(&state), Some("main.rs".to_string()));
}

#[test]
fn write_tool_summary_uses_minimum_count_for_whitespace_only_name() {
    let mut state = file_list_state(vec![("   ".to_string(), "/tmp/blank".to_string())]);
    state.total_items = 0;

    assert_eq!(write_tool_summary(&state), Some("1 个文件".to_string()));
}

#[test]
fn write_tool_summary_preserves_root_path_without_file_name() {
    let state = file_list_state(vec![("/".to_string(), "/".to_string())]);

    assert_eq!(write_tool_summary(&state), Some("/".to_string()));
}

#[test]
fn write_tool_title_applies_running_permission_error_and_success_precedence() {
    assert_eq!(write_tool_title("写入", true, Some(ToolPermissionState::Rejected), true), "写入中");
    assert_eq!(
        write_tool_title("写入", false, Some(ToolPermissionState::ApprovalRequired), true,),
        "写入待批准"
    );
    assert_eq!(
        write_tool_title("写入", false, Some(ToolPermissionState::Rejected), true),
        "写入已拒绝"
    );
    assert_eq!(write_tool_title("写入", false, None, true), "写入失败");
    assert_eq!(write_tool_title("写入", false, None, false), "写入");
}

#[test]
fn write_tool_detail_text_prefers_running_then_non_blank_error_then_fallback() {
    assert_eq!(write_tool_detail_text("写入", true, Some("disk full"), "summary"), "写入中…");
    assert_eq!(
        write_tool_detail_text("写入", false, Some("  disk full  "), "summary"),
        "disk full"
    );
    assert_eq!(write_tool_detail_text("写入", false, Some("   "), "summary"), "summary");
    assert_eq!(write_tool_detail_text("写入", false, None, "summary"), "summary");
}

#[test]
fn write_tool_detail_color_uses_danger_for_errors_and_muted_color_otherwise() {
    let theme = Theme::Dark;
    let error = write_tool_detail_color(&theme, true);
    let normal = write_tool_detail_color(&theme, false);

    assert_eq!(error, theme.extended_palette().danger.base.color.scale_alpha(0.9));
    assert_ne!(normal, error);
}

#[test]
fn write_tool_context_menu_message_preserves_target_point_and_text() {
    let result =
        write_tool_context_menu_message(42, Point::new(12.5, 26.25), "src/main.rs".to_string());

    match result {
        Message::Chat(message::ChatMessage::OpenMessageContextMenu { target, x, y, text }) => {
            assert_eq!(target, 42);
            assert_eq!(x, 12.5);
            assert_eq!(y, 26.25);
            assert_eq!(text, "src/main.rs");
        }
        _ => panic!("expected write context-menu message"),
    }
}

#[test]
fn build_write_tool_view_handles_non_empty_file_list() {
    let app = App::new().0;
    let view_ctx = view_context(&app, "tool write\n{}", None, false, false);
    let render_state =
        file_list_state(vec![("src/main.rs".to_string(), "/tmp/src/main.rs".to_string())]);

    assert_closed_view(build_write_tool_view(&view_ctx, &render_state, iced::widget::column![]));
}

#[test]
fn build_write_tool_view_handles_empty_running_error_permission_and_fallback_states() {
    let app = App::new().0;
    let empty_state = file_list_state(Vec::new());

    let running = view_context(
        &app,
        r#"tool write
{"status":"running"}"#,
        None,
        false,
        true,
    );
    assert_closed_view(build_write_tool_view(&running, &empty_state, iced::widget::column![]));

    let error = view_context(
        &app,
        r#"tool write
{"status":"error","error":"disk full"}"#,
        Some("  disk full  "),
        true,
        false,
    );
    assert_closed_view(build_write_tool_view(&error, &empty_state, iced::widget::column![]));

    let denied = view_context(
        &app,
        r#"tool write
{"status":"denied","error":"permission denied","permission_request":{"updated_input":{"path":"src/main.rs"}}}"#,
        Some("permission denied"),
        true,
        false,
    );
    assert_closed_view(build_write_tool_view(&denied, &empty_state, iced::widget::column![]));

    let fallback = view_context(&app, "tool write\nnot-json", Some("   "), false, false);
    assert_closed_view(build_write_tool_view(&fallback, &empty_state, iced::widget::column![]));
}

#[test]
fn build_write_tool_view_keeps_the_same_tree_when_context_menu_opens() {
    let mut app = App::new().0;
    let closed_child_count = {
        let closed_view_ctx = view_context(&app, "tool write\n{}", None, false, false);
        let closed_element = build_write_tool_view(
            &closed_view_ctx,
            &file_list_state(Vec::new()),
            iced::widget::column![],
        );
        closed_element.as_widget().children().len()
    };

    app.chat_context_menu_target =
        Some(crate::app::components::chat_panel::utils::chat_context_target_key(1, Some(2)));
    app.chat_context_menu_pos = Some((24.0, 32.0));
    let view_ctx = view_context(&app, "tool write\n{}", None, false, false);

    let element =
        build_write_tool_view(&view_ctx, &file_list_state(Vec::new()), iced::widget::column![]);

    assert_eq!(element.as_widget().children().len(), closed_child_count);
    assert_eq!(element.as_widget().size().width, Length::Fill);
}
