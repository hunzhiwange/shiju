use super::super::types::ChangeFile;
use super::list_ui::{
    build_file_list_column, edit_change_counts, edit_file_change_block,
    edit_file_change_button_style, edit_file_expand_key, edit_file_name_link, edit_result_label,
    edit_verb_text, external_diff_button, fallback_edit_diff_payload, file_name_tooltip,
    file_row_label, tool_file_hover_key, tool_file_left, view_changes_pill,
    view_changes_secondary_text, view_changes_toggle_button,
};
use super::{FileListState, FilesViewContext};
use crate::app::components::chat_panel::tools::file_link::file_name_link;
use crate::app::{App, Message};
use std::collections::HashMap;

fn test_app() -> App {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/vibe-window".to_string());
    app
}

fn state(items: Vec<(String, String)>, is_search: bool) -> FileListState {
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
        is_search,
    }
}

fn ctx<'a>(app: &'a App, tool_name: &str, input: &str, output: &str) -> FilesViewContext<'a> {
    FilesViewContext {
        app,
        msg_idx: 1,
        tool_idx: 2,
        visible: format!("tool {tool_name}\n{{}}"),
        tool_name: tool_name.to_string(),
        error_text: None,
        input: input.to_string(),
        output: output.to_string(),
        verb: "写入",
        is_error: false,
        is_running: false,
        is_edit_like: matches!(tool_name, "write" | "file_write" | "file_edit"),
        read_range: None,
    }
}

fn keep_column(column: iced::widget::Column<'_, Message>) {
    std::hint::black_box(column);
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn column_child_count(column: iced::widget::Column<'_, Message>) -> usize {
    let element: iced::Element<'_, Message> = column.into();
    element.as_widget().children().len()
}

#[test]
fn tool_file_hover_key_includes_indices_and_path() {
    assert_eq!(tool_file_hover_key(1, 2, "/tmp/a.rs"), "1:2:/tmp/a.rs");
}

#[test]
fn file_row_label_uses_file_name_for_edit_tools() {
    let app = test_app();
    let view_ctx = ctx(&app, "write", r#"{"path":"src/main.rs"}"#, "");

    assert_eq!(
        file_row_label(&app, &view_ctx, "src/main.rs", "/tmp/vibe-window/src/main.rs"),
        "main.rs"
    );
}

#[test]
fn file_row_label_uses_file_name_for_non_edit_tools() {
    let app = test_app();
    let mut view_ctx = ctx(&app, "read", r#"{"path":"src/main.rs"}"#, "");
    view_ctx.is_edit_like = false;

    assert_eq!(
        file_row_label(&app, &view_ctx, "src/main.rs", "/tmp/vibe-window/src/main.rs"),
        "main.rs"
    );
    assert_eq!(file_row_label(&app, &view_ctx, "README", "/tmp/vibe-window/README"), "README");
}

#[test]
fn file_row_label_falls_back_when_file_name_is_unavailable() {
    let mut app = test_app();
    app.project_path = None;
    let view_ctx = ctx(&app, "write", r#"{"path":"src/main.rs"}"#, "");

    assert_eq!(
        file_row_label(&app, &view_ctx, "display.rs", "/tmp/vibe-window/src/main.rs"),
        "display.rs"
    );

    let mut read_ctx = ctx(&app, "read", "{}", "");
    read_ctx.is_edit_like = false;
    assert_eq!(file_row_label(&app, &read_ctx, "/", "/"), "/");
}

#[test]
fn fallback_edit_diff_payload_requires_edit_tool_matching_input_and_preview() {
    let app = test_app();
    let view_ctx = ctx(&app, "write", r#"{"path":"src/main.rs","content":"fn main() {}"}"#, "");

    let payload =
        fallback_edit_diff_payload(&app, &view_ctx, "src/main.rs", "/tmp/vibe-window/src/main.rs")
            .expect("write preview payload");

    assert_eq!(payload.0, "src/main.rs  写入内容");
    assert_eq!(payload.1, "src/main.rs");
    assert!(payload.2.contains("fn main"));
    assert!(
        fallback_edit_diff_payload(&app, &view_ctx, "src/lib.rs", "/tmp/vibe-window/src/lib.rs")
            .is_none()
    );
}

#[test]
fn fallback_edit_diff_payload_rejects_non_edit_mismatch_and_blank_preview() {
    let app = test_app();
    let mut non_edit_ctx =
        ctx(&app, "read", r#"{"path":"src/main.rs","content":"fn main() {}"}"#, "");
    non_edit_ctx.is_edit_like = false;
    assert!(
        fallback_edit_diff_payload(
            &app,
            &non_edit_ctx,
            "src/main.rs",
            "/tmp/vibe-window/src/main.rs"
        )
        .is_none()
    );

    let mismatch_ctx = ctx(&app, "write", r#"{"path":"src/lib.rs","content":"fn main() {}"}"#, "");
    assert!(
        fallback_edit_diff_payload(
            &app,
            &mismatch_ctx,
            "src/main.rs",
            "/tmp/vibe-window/src/main.rs"
        )
        .is_none()
    );

    let blank_ctx =
        ctx(&app, "write", r#"{"path":"src/main.rs","content":"<omitted 42 chars>"}"#, "");
    assert!(
        fallback_edit_diff_payload(&app, &blank_ctx, "src/main.rs", "/tmp/vibe-window/src/main.rs")
            .is_none()
    );

    let mut no_project_app = test_app();
    no_project_app.project_path = None;
    let absolute_ctx = ctx(
        &no_project_app,
        "write",
        r#"{"path":"/tmp/vibe-window/src/main.rs","content":"x"}"#,
        "",
    );
    let payload = fallback_edit_diff_payload(
        &no_project_app,
        &absolute_ctx,
        "shown.rs",
        "/tmp/vibe-window/src/main.rs",
    )
    .expect("absolute write preview payload");

    assert_eq!(payload.0, "shown.rs  写入内容");
    assert_eq!(payload.1, "shown.rs");
    assert_eq!(payload.2, "x");
}

#[test]
fn fallback_edit_diff_payload_rejects_missing_input_path() {
    let app = test_app();
    let view_ctx = ctx(&app, "write", r#"{"content":"after"}"#, "");

    assert!(
        fallback_edit_diff_payload(&app, &view_ctx, "src/main.rs", "/tmp/vibe-window/src/main.rs")
            .is_none()
    );
}

#[test]
fn edit_result_label_covers_write_and_edit_statuses() {
    assert_eq!(edit_result_label("write", true, false), "写入中");
    assert_eq!(edit_result_label("file_write", false, true), "写入失败");
    assert_eq!(edit_result_label("write", false, false), "已写入");
    assert_eq!(edit_result_label("file_edit", true, false), "编辑中");
    assert_eq!(edit_result_label("file_edit", false, true), "编辑失败");
    assert_eq!(edit_result_label("file_edit", false, false), "已编辑");
}

#[test]
fn edit_file_expand_key_includes_indices_and_path() {
    assert_eq!(edit_file_expand_key(1, 2, "src/main.rs"), "1:2:src/main.rs");
}

#[test]
fn edit_row_helpers_build_hovered_and_idle_elements() {
    keep_element(file_name_tooltip("src/main.rs".to_string()));
    keep_element(external_diff_button(Message::None));
    keep_element(view_changes_toggle_button(false, Message::None));
    keep_element(view_changes_toggle_button(true, Message::None));
    keep_element(view_changes_pill());
    keep_element(edit_verb_text("读取"));
    keep_element(edit_change_counts(3, 1));
    keep_element(tool_file_left(
        "读取",
        file_name_link("src/main.rs", "/tmp/vibe-window/src/main.rs"),
        true,
        Some("L1-L3".to_string()),
    ));
    keep_element(tool_file_left(
        "运行",
        file_name_link("Cargo.toml", "/tmp/vibe-window/Cargo.toml"),
        false,
        None,
    ));
    keep_element(edit_file_name_link("/tmp/vibe-window/src/main.rs", "src/main.rs".to_string()));

    let app = test_app();
    keep_element(edit_file_change_block(
        &app,
        1,
        2,
        "/tmp/vibe-window/src/main.rs",
        "src/main.rs".to_string(),
        "已编辑",
        "src/main.rs".to_string(),
        Some((2, 1)),
        "src/main.rs  +2-1".to_string(),
        "before".to_string(),
        "after".to_string(),
    ));

    let mut expanded = test_app();
    expanded.chat_tool_file_expanded.insert(edit_file_expand_key(1, 2, "src/main.rs"));
    expanded.chat_tool_file_hovered =
        Some(tool_file_hover_key(1, 2, "/tmp/vibe-window/src/main.rs"));
    keep_element(edit_file_change_block(
        &expanded,
        1,
        2,
        "/tmp/vibe-window/src/main.rs",
        "src/main.rs".to_string(),
        "已编辑",
        "src/main.rs".to_string(),
        Some((2, 1)),
        "src/main.rs  +2-1".to_string(),
        "before".to_string(),
        "after".to_string(),
    ));

    let _ = edit_file_change_button_style(&iced::Theme::Dark, iced::widget::button::Status::Active);
    let _ =
        edit_file_change_button_style(&iced::Theme::Dark, iced::widget::button::Status::Hovered);
    let _ = view_changes_secondary_text(&iced::Theme::Dark);
    let _ = view_changes_secondary_text(&iced::Theme::Light);
}

#[test]
fn build_file_list_column_handles_empty_search_and_plain_empty_states() {
    let app = test_app();
    let view_ctx = ctx(&app, "grep", "{}", "");
    let mut search_state = state(Vec::new(), true);
    search_state.filter_query = "missing".to_string();
    search_state.is_empty_filtered = true;

    keep_column(build_file_list_column(&app, &search_state, &view_ctx, &HashMap::new()));

    let plain_state = state(Vec::new(), false);
    keep_column(build_file_list_column(&app, &plain_state, &view_ctx, &HashMap::new()));

    let mut search_empty = state(Vec::new(), true);
    search_empty.total_items = 0;
    keep_column(build_file_list_column(&app, &search_empty, &view_ctx, &HashMap::new()));
}

#[test]
fn build_file_list_column_reports_expected_child_counts_for_empty_states() {
    let app = test_app();
    let view_ctx = ctx(&app, "grep", "{}", "");
    let mut filtered = state(Vec::new(), true);
    filtered.filter_query = "missing".to_string();
    filtered.is_empty_filtered = true;
    let plain = state(Vec::new(), false);

    assert_eq!(
        column_child_count(build_file_list_column(&app, &filtered, &view_ctx, &HashMap::new())),
        1
    );
    assert_eq!(
        column_child_count(build_file_list_column(&app, &plain, &view_ctx, &HashMap::new())),
        1
    );
}

#[test]
fn build_file_list_column_handles_tail_and_middle_omissions() {
    let app = test_app();
    let view_ctx = ctx(&app, "read", r#"{"path":"src/main.rs"}"#, "");
    let mut render_state = state(
        vec![
            ("src/a.rs".to_string(), "/tmp/vibe-window/src/a.rs".to_string()),
            ("src/z.rs".to_string(), "/tmp/vibe-window/src/z.rs".to_string()),
        ],
        false,
    );
    render_state.truncated_middle = true;
    render_state.max_items = 2;
    render_state.tail_omitted = 3;

    keep_column(build_file_list_column(&app, &render_state, &view_ctx, &HashMap::new()));
    assert_eq!(
        column_child_count(build_file_list_column(&app, &render_state, &view_ctx, &HashMap::new())),
        4
    );
}

#[test]
fn build_file_list_column_handles_search_items_and_truncated_labels() {
    let app = test_app();
    let view_ctx = ctx(&app, "grep", "{}", "");
    let render_state = state(
        vec![(
            "src/".to_string() + &"very-long-name".repeat(10) + ".rs",
            "/tmp/vibe-window/src/long.rs".to_string(),
        )],
        true,
    );

    keep_column(build_file_list_column(&app, &render_state, &view_ctx, &HashMap::new()));
}

#[test]
fn build_file_list_column_expands_inline_diff_for_edit_rows() {
    let mut app = test_app();
    let abs = "/tmp/vibe-window/src/main.rs";
    app.chat_tool_file_expanded.insert(edit_file_expand_key(1, 2, "src/main.rs"));
    let view_ctx = ctx(&app, "file_edit", r#"{"path":"src/main.rs","new_string":"after"}"#, "");
    let render_state = state(vec![("src/main.rs".to_string(), abs.to_string())], false);
    let mut changes_by_path = HashMap::new();
    changes_by_path.insert(
        "src/main.rs".to_string(),
        ChangeFile {
            path: "src/main.rs".to_string(),
            additions: 2,
            deletions: 1,
            before: "before".to_string(),
            after: "after".to_string(),
        },
    );

    assert_eq!(
        column_child_count(build_file_list_column(
            &app,
            &render_state,
            &view_ctx,
            &changes_by_path
        )),
        1
    );
}

#[test]
fn build_file_list_column_uses_unhovered_change_payload_for_edit_rows() {
    let app = test_app();
    let abs = "/tmp/vibe-window/src/main.rs";
    let view_ctx = ctx(&app, "file_edit", r#"{"path":"src/main.rs","new_string":"after"}"#, "");
    let render_state = state(vec![("src/main.rs".to_string(), abs.to_string())], false);
    let mut changes_by_path = HashMap::new();
    changes_by_path.insert(
        "src/main.rs".to_string(),
        ChangeFile {
            path: "src/main.rs".to_string(),
            additions: 1,
            deletions: 0,
            before: "before".to_string(),
            after: "after".to_string(),
        },
    );

    assert_eq!(
        column_child_count(build_file_list_column(
            &app,
            &render_state,
            &view_ctx,
            &changes_by_path
        )),
        1
    );
}

#[test]
fn build_file_list_column_uses_fallback_diff_payload_for_edit_rows() {
    let app = test_app();
    let abs = "/tmp/vibe-window/src/main.rs";
    let view_ctx = ctx(&app, "write", r#"{"path":"src/main.rs","content":"after"}"#, "");
    let render_state = state(vec![("src/main.rs".to_string(), abs.to_string())], false);

    keep_column(build_file_list_column(&app, &render_state, &view_ctx, &HashMap::new()));
}

#[test]
fn build_file_list_column_uses_hovered_fallback_diff_payload_without_project_relative_path() {
    let mut app = test_app();
    app.project_path = None;
    let abs = "/tmp/vibe-window/src/main.rs";
    app.chat_tool_file_hovered = Some(tool_file_hover_key(1, 2, abs));
    let view_ctx =
        ctx(&app, "write", r#"{"path":"/tmp/vibe-window/src/main.rs","content":"after"}"#, "");
    let render_state = state(vec![("shown.rs".to_string(), abs.to_string())], false);

    assert_eq!(
        column_child_count(build_file_list_column(&app, &render_state, &view_ctx, &HashMap::new())),
        1
    );
}
