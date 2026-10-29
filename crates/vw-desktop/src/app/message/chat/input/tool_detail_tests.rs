//! 验证工具原始输出在成功、失败、拒绝和非结构化状态下的详情解析。
//!
//! 解析样例覆盖普通工具与 MCP 载荷，标题状态和正文回退必须彼此独立。

#![allow(unused_must_use)]
#[test]
fn tool_detail_tests_module_is_wired() {
    assert!(module_path!().ends_with("tool_detail_tests"));
}

#[test]
fn tool_detail_from_raw_uses_output_text_for_success_payload() {
    let raw = r#"tool shell
{"status":"success","output":"done\n"}"#;

    let (title, content) = super::tool_detail_from_raw(raw).expect("valid tool detail");

    assert!(!title.is_empty());
    assert_eq!(content, "done");
}

#[test]
fn tool_detail_from_raw_marks_error_title_and_uses_error_text() {
    let raw = r#"tool shell
{"status":"error","error":"permission denied"}"#;

    let (title, content) = super::tool_detail_from_raw(raw).expect("valid tool detail");

    assert!(title.ends_with("失败"));
    assert_eq!(content, "permission denied");
}

#[test]
fn tool_detail_from_raw_marks_denied_title_and_uses_output_text() {
    let raw = r#"tool shell
{"status":"denied","output":"blocked by policy"}"#;

    let (title, content) = super::tool_detail_from_raw(raw).expect("valid tool detail");

    assert!(title.ends_with("失败"));
    assert_eq!(content, "blocked by policy");
}

#[test]
fn tool_detail_from_raw_marks_running_title() {
    let raw = r#"tool shell
{"status":"running","output":""}"#;

    let (title, _) = super::tool_detail_from_raw(raw).expect("valid tool detail");

    assert!(title.ends_with("运行中"));
}

#[test]
fn tool_detail_from_raw_falls_back_to_trimmed_rest_for_non_json() {
    let raw = "tool unknown\n  plain output  ";

    let (title, content) = super::tool_detail_from_raw(raw).expect("valid tool detail");

    assert_eq!(title, "工具");
    assert_eq!(content, "plain output");
}

#[test]
fn tool_detail_from_raw_preserves_trimmed_json_without_output_or_error() {
    let raw = "  tool shell  \n  {\"status\":\"success\",\"metadata\":{\"attempt\":1}}  ";

    let (title, content) = super::tool_detail_from_raw(raw).expect("valid tool detail");

    assert!(!title.ends_with("失败"));
    assert!(!title.ends_with("运行中"));
    assert_eq!(content, r#"{"status":"success","metadata":{"attempt":1}}"#);
}

#[test]
fn tool_detail_from_raw_formats_mcp_json_and_uses_mcp_title() {
    let raw = r#"tool MCP
{"status":"success","output":"[{\"server\":{\"name\":\"rustcodegraph\"}}]"}"#;

    let (title, content) = super::tool_detail_from_raw(raw).expect("valid MCP detail");

    assert_eq!(title, "MCP");
    assert_eq!(
        content,
        "[\n  {\n    \"server\": {\n      \"name\": \"rustcodegraph\"\n    }\n  }\n]"
    );
}

#[test]
fn tool_detail_from_raw_formats_generic_mcp_payload() {
    let raw = r#"tool tools
{"status":"success","output":"[{\"server\":{\"name\":\"rustcodegraph\"},\"resources\":[]}]"}"#;

    let (title, content) = super::tool_detail_from_raw(raw).expect("valid MCP detail");

    assert_eq!(title, "MCP");
    assert!(content.contains("\"resources\": []"));
}

#[test]
fn tool_detail_from_raw_rejects_missing_tool_header_or_body() {
    assert!(super::tool_detail_from_raw("tool shell").is_none());
    assert!(super::tool_detail_from_raw("shell\n{}").is_none());
}

#[test]
fn tool_detail_from_raw_formats_workflow_node_title_and_pretty_body() {
    let raw = r#"tool workflow_node
{"status":"completed","output":"{\"answer\":\"long node body\",\"usage\":{\"total_tokens\":3}}","metadata":{"title":"实现入口","node_type":"llm"}}"#;

    let (title, content) = super::tool_detail_from_raw(raw).expect("workflow detail");

    assert_eq!(title, "实现入口");
    assert!(content.contains("long node body"));
    assert!(content.contains('\n'));
}

#[test]
fn tool_detail_from_raw_marks_workflow_node_running_and_failed_titles() {
    let running = r#"tool workflow_node
{"status":"running","output":"正在执行工作流节点","metadata":{"title":"规划任务"}}"#;
    let failed = r#"tool workflow_node
{"status":"error","error":"boom","metadata":{"title":"规划任务"}}"#;

    let (running_title, _) = super::tool_detail_from_raw(running).expect("running");
    let (failed_title, failed_content) = super::tool_detail_from_raw(failed).expect("failed");

    assert_eq!(running_title, "规划任务 · 运行中");
    assert_eq!(failed_title, "规划任务 · 失败");
    assert_eq!(failed_content, "boom");
}

#[test]
fn close_tool_detail_context_menu_clears_open_state_and_position() {
    let (mut app, _task) = crate::app::App::new();
    super::handle_open_tool_detail(&mut app, 2, 3, "tool shell\n{}".to_string());
    let dialog = app.tool_detail_dialog.as_mut().expect("dialog opens");
    dialog.context_menu_open = true;
    dialog.context_menu_pos = Some((12.0, 24.0));

    super::close_tool_detail_context_menu(dialog);

    assert!(!dialog.context_menu_open);
    assert_eq!(dialog.context_menu_pos, None);
}

#[test]
fn handle_open_tool_detail_sets_dialog_for_valid_raw_and_ignores_invalid_raw() {
    let (mut app, _task) = crate::app::App::new();

    super::handle_open_tool_detail(
        &mut app,
        4,
        5,
        "tool shell\n{\"status\":\"success\",\"output\":\"hello\"}".to_string(),
    );

    let dialog = app.tool_detail_dialog.as_ref().expect("dialog opens");
    assert_eq!(dialog.msg_idx, 4);
    assert_eq!(dialog.tool_idx, 5);
    assert_eq!(dialog.content, "hello");
    assert_eq!(dialog.editor.text(), "hello");

    super::handle_open_tool_detail(&mut app, 1, 1, "not a tool".to_string());

    assert_eq!(app.tool_detail_dialog.as_ref().expect("dialog remains").msg_idx, 4);
}

#[test]
fn tool_detail_scroll_helpers_clamp_to_available_lines() {
    let (mut app, _task) = crate::app::App::new();
    app.current_line_height = 10.0;
    super::handle_open_tool_detail(&mut app, 0, 0, "tool shell\nline1\nline2\nline3\nline4".into());
    app.tool_detail_dialog.as_mut().expect("dialog").viewport_height = 20.0;

    assert_eq!(super::tool_detail_max_scroll_top_line(&app), 2.0);

    super::apply_tool_detail_scroll_lines(&mut app, 10);
    assert_eq!(app.tool_detail_dialog.as_ref().expect("dialog").scroll_top_line, 2.0);

    super::apply_tool_detail_scroll_lines(&mut app, -10);
    assert_eq!(app.tool_detail_dialog.as_ref().expect("dialog").scroll_top_line, 0.0);
}

#[test]
fn tool_detail_handlers_are_safe_without_an_open_dialog() {
    let (mut app, _task) = crate::app::App::new();

    super::handle_tool_detail_editor_action(&mut app, iced::widget::text_editor::Action::SelectAll);
    super::handle_tool_detail_open_context_menu(&mut app, 1.0, 2.0);
    super::handle_tool_detail_close_context_menu(&mut app);
    super::handle_tool_detail_undo(&mut app);
    super::handle_tool_detail_redo(&mut app);
    super::handle_tool_detail_context_menu_copy(&mut app);
    super::handle_tool_detail_context_menu_cut(&mut app);
    super::handle_tool_detail_context_menu_paste(&mut app);
    super::handle_tool_detail_context_menu_delete(&mut app);
    super::handle_tool_detail_editor_wheel_scrolled(
        &mut app,
        iced::mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 },
        20.0,
    );
    super::handle_tool_detail_scrollbar_changed(&mut app, 2.0, 20.0);
    super::apply_tool_detail_scroll_lines(&mut app, 1);

    assert!(app.tool_detail_dialog.is_none());
    assert_eq!(super::tool_detail_max_scroll_top_line(&app), 0.0);
}

#[test]
fn context_menu_handlers_update_state_and_preserve_unselected_content() {
    let (mut app, _task) = crate::app::App::new();
    super::handle_open_tool_detail(&mut app, 0, 0, "tool shell\nhello".into());

    super::handle_tool_detail_open_context_menu(&mut app, 12.0, 24.0);
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert!(dialog.context_menu_open);
    assert_eq!(dialog.context_menu_pos, Some((12.0, 24.0)));

    super::handle_tool_detail_context_menu_copy(&mut app);
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert!(!dialog.context_menu_open);
    assert_eq!(dialog.context_menu_pos, None);
    assert_eq!(dialog.editor.text(), "hello");

    super::handle_tool_detail_open_context_menu(&mut app, 3.0, 4.0);
    super::handle_tool_detail_context_menu_paste(&mut app);
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert!(!dialog.context_menu_open);
    assert_eq!(dialog.context_menu_pos, None);
    assert_eq!(dialog.editor.text(), "hello");

    super::handle_tool_detail_open_context_menu(&mut app, 5.0, 6.0);
    super::handle_tool_detail_close_context_menu(&mut app);
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert!(!dialog.context_menu_open);
    assert_eq!(dialog.context_menu_pos, None);
}

#[test]
fn cut_and_delete_handlers_remove_selected_content_and_close_menu() {
    let (mut app, _task) = crate::app::App::new();
    super::handle_open_tool_detail(&mut app, 0, 0, "tool shell\ncut me".into());
    {
        let dialog = app.tool_detail_dialog.as_mut().expect("dialog");
        dialog.editor.perform(iced::widget::text_editor::Action::SelectAll);
        dialog.context_menu_open = true;
        dialog.context_menu_pos = Some((1.0, 2.0));
    }

    super::handle_tool_detail_context_menu_cut(&mut app);
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert_eq!(dialog.editor.text(), "");
    assert!(!dialog.context_menu_open);
    assert_eq!(dialog.context_menu_pos, None);

    super::handle_tool_detail_editor_action(
        &mut app,
        iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Paste(
            std::sync::Arc::new("delete me".to_string()),
        )),
    );
    {
        let dialog = app.tool_detail_dialog.as_mut().expect("dialog");
        dialog.editor.perform(iced::widget::text_editor::Action::SelectAll);
        dialog.context_menu_open = true;
        dialog.context_menu_pos = Some((3.0, 4.0));
    }

    super::handle_tool_detail_context_menu_delete(&mut app);
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert_eq!(dialog.editor.text(), "");
    assert!(!dialog.context_menu_open);
    assert_eq!(dialog.context_menu_pos, None);
}

#[test]
fn editor_scroll_action_updates_position_and_closes_context_menu() {
    let (mut app, _task) = crate::app::App::new();
    app.current_line_height = 10.0;
    super::handle_open_tool_detail(&mut app, 0, 0, "tool shell\none\ntwo\nthree\nfour".into());
    {
        let dialog = app.tool_detail_dialog.as_mut().expect("dialog");
        dialog.viewport_height = 20.0;
        dialog.context_menu_open = true;
        dialog.context_menu_pos = Some((8.0, 9.0));
    }

    super::handle_tool_detail_editor_action(
        &mut app,
        iced::widget::text_editor::Action::Scroll { lines: 2 },
    );

    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert_eq!(dialog.scroll_top_line, 2.0);
    assert!(!dialog.context_menu_open);
    assert_eq!(dialog.context_menu_pos, None);
}

#[test]
fn wheel_scroll_accumulates_fractional_lines_and_supports_pixels() {
    let (mut app, _task) = crate::app::App::new();
    app.current_line_height = 10.0;
    super::handle_open_tool_detail(&mut app, 0, 0, "tool shell\none\ntwo\nthree\nfour".into());
    super::handle_tool_detail_open_context_menu(&mut app, 1.0, 2.0);

    super::handle_tool_detail_editor_wheel_scrolled(
        &mut app,
        iced::mouse::ScrollDelta::Lines { x: 0.0, y: -0.5 },
        -5.0,
    );
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert_eq!(dialog.viewport_height, 0.0);
    assert_eq!(dialog.scroll_top_line, 0.0);
    assert_eq!(dialog.scroll_remainder, 0.625);
    assert!(!dialog.context_menu_open);

    super::handle_tool_detail_editor_wheel_scrolled(
        &mut app,
        iced::mouse::ScrollDelta::Lines { x: 0.0, y: -0.5 },
        20.0,
    );
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert_eq!(dialog.scroll_top_line, 1.0);
    assert_eq!(dialog.scroll_remainder, 0.25);

    super::handle_tool_detail_editor_wheel_scrolled(
        &mut app,
        iced::mouse::ScrollDelta::Pixels { x: 0.0, y: -15.0 },
        20.0,
    );
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert_eq!(dialog.scroll_top_line, 2.0);
    assert_eq!(dialog.scroll_remainder, 0.75);

    super::handle_tool_detail_editor_wheel_scrolled(
        &mut app,
        iced::mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
        20.0,
    );
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert_eq!(dialog.scroll_top_line, 2.0);
    assert_eq!(dialog.scroll_remainder, -0.5);

    super::handle_tool_detail_editor_wheel_scrolled(
        &mut app,
        iced::mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
        20.0,
    );
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert_eq!(dialog.scroll_top_line, 1.0);
    assert_eq!(dialog.scroll_remainder, -0.75);
}

#[test]
fn scrollbar_change_rounds_and_clamps_requested_top_line() {
    let (mut app, _task) = crate::app::App::new();
    app.current_line_height = 10.0;
    super::handle_open_tool_detail(&mut app, 0, 0, "tool shell\none\ntwo\nthree\nfour".into());
    {
        let dialog = app.tool_detail_dialog.as_mut().expect("dialog");
        dialog.viewport_height = 20.0;
        dialog.context_menu_open = true;
        dialog.context_menu_pos = Some((4.0, 8.0));
    }

    super::handle_tool_detail_scrollbar_changed(&mut app, 1.6, 20.0);
    let dialog = app.tool_detail_dialog.as_ref().expect("dialog");
    assert_eq!(dialog.scroll_top_line, 2.0);
    assert!(!dialog.context_menu_open);
    assert_eq!(dialog.context_menu_pos, None);

    super::handle_tool_detail_scrollbar_changed(&mut app, -100.0, 20.0);
    assert_eq!(app.tool_detail_dialog.as_ref().expect("dialog").scroll_top_line, 0.0);

    super::handle_tool_detail_scrollbar_changed(&mut app, 100.0, 20.0);
    assert_eq!(app.tool_detail_dialog.as_ref().expect("dialog").scroll_top_line, 2.0);
}

#[test]
fn close_tool_detail_removes_dialog() {
    let (mut app, _task) = crate::app::App::new();
    super::handle_open_tool_detail(&mut app, 7, 8, "tool shell\ncontent".into());
    assert!(app.tool_detail_dialog.is_some());

    super::handle_close_tool_detail(&mut app);

    assert!(app.tool_detail_dialog.is_none());
}

#[test]
fn tool_detail_history_restores_paste_cut_and_delete_and_clears_redo_on_edit() {
    use iced::widget::text_editor::{Action, Edit};

    let (mut app, _task) = crate::app::App::new();
    super::handle_open_tool_detail(&mut app, 0, 0, "tool shell\n原始输出".into());
    super::handle_tool_detail_editor_action(&mut app, Action::SelectAll);
    super::handle_tool_detail_editor_action(&mut app, super::paste_action("修改内容".into()));
    super::handle_tool_detail_editor_action(&mut app, Action::SelectAll);
    super::handle_tool_detail_context_menu_cut(&mut app);
    assert_eq!(app.tool_detail_dialog.as_ref().unwrap().editor.text(), "");

    super::handle_tool_detail_undo(&mut app);
    assert_eq!(app.tool_detail_dialog.as_ref().unwrap().editor.text(), "修改内容");
    super::handle_tool_detail_undo(&mut app);
    assert_eq!(app.tool_detail_dialog.as_ref().unwrap().editor.text(), "原始输出");
    super::handle_tool_detail_redo(&mut app);
    assert_eq!(app.tool_detail_dialog.as_ref().unwrap().editor.text(), "修改内容");
    super::handle_tool_detail_redo(&mut app);
    assert_eq!(app.tool_detail_dialog.as_ref().unwrap().editor.text(), "");
    super::handle_tool_detail_undo(&mut app);

    // 选择、复制和没有选区的删除均不应清空可重做的历史。
    super::handle_tool_detail_editor_action(
        &mut app,
        Action::Move(iced::widget::text_editor::Motion::End),
    );
    super::handle_tool_detail_context_menu_copy(&mut app);
    super::handle_tool_detail_context_menu_delete(&mut app);
    assert!(!app.tool_detail_dialog.as_ref().unwrap().redo_stack.is_empty());

    super::handle_tool_detail_editor_action(&mut app, Action::SelectAll);
    super::handle_tool_detail_context_menu_delete(&mut app);
    assert!(app.tool_detail_dialog.as_ref().unwrap().redo_stack.is_empty());
    super::handle_tool_detail_undo(&mut app);
    assert_eq!(app.tool_detail_dialog.as_ref().unwrap().editor.text(), "修改内容");
    super::handle_tool_detail_editor_action(&mut app, Action::Edit(Edit::Insert('新')));
    super::handle_tool_detail_redo(&mut app);
    assert!(app.tool_detail_dialog.as_ref().unwrap().editor.text().contains('新'));
    assert_eq!(app.tool_detail_dialog.as_ref().unwrap().content, "原始输出");
}

#[test]
fn tool_detail_history_is_bounded_and_resets_when_another_detail_opens() {
    use iced::widget::text_editor::{Action, Edit};

    let (mut app, _task) = crate::app::App::new();
    super::handle_open_tool_detail(&mut app, 0, 0, "tool shell\ninitial".into());
    for _ in 0..=super::TOOL_DETAIL_HISTORY_LIMIT {
        super::handle_tool_detail_editor_action(&mut app, Action::Edit(Edit::Insert('a')));
    }
    assert_eq!(
        app.tool_detail_dialog.as_ref().unwrap().undo_stack.len(),
        super::TOOL_DETAIL_HISTORY_LIMIT
    );
    super::handle_tool_detail_open_context_menu(&mut app, 1.0, 2.0);
    {
        let dialog = app.tool_detail_dialog.as_mut().unwrap();
        dialog.scroll_top_line = 5.0;
        dialog.scroll_remainder = 0.5;
    }
    super::handle_tool_detail_undo(&mut app);
    let dialog = app.tool_detail_dialog.as_ref().unwrap();
    assert!(!dialog.context_menu_open);
    assert_eq!(dialog.context_menu_pos, None);
    assert_eq!(dialog.scroll_top_line, 0.0);
    assert_eq!(dialog.scroll_remainder, 0.0);

    super::handle_open_tool_detail(&mut app, 1, 0, "tool shell\nother".into());
    super::handle_tool_detail_undo(&mut app);
    super::handle_tool_detail_redo(&mut app);
    let dialog = app.tool_detail_dialog.as_ref().unwrap();
    assert_eq!(dialog.editor.text(), "other");
    assert!(dialog.undo_stack.is_empty());
    assert!(dialog.redo_stack.is_empty());
}
