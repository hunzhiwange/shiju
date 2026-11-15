#![allow(unused_must_use)]
fn test_state() -> super::ToolDetailState {
 super::ToolDetailState { tool_detail_dialog: None, current_line_height: 20.0 }
}
fn open_tool_detail(state: &mut super::ToolDetailState, msg_idx: usize, tool_idx: usize, raw: String) {
 let detail = super::tool_detail_from_raw(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese), &raw, false, |_, _| unreachable!("shell fixture has no workflow body"));
 let _ = super::handle_open_tool_detail(state, msg_idx, tool_idx, detail);
}
#[test]
fn close_tool_detail_context_menu_clears_open_state_and_position() {
    let (mut app, _task) = (test_state(), ());
    open_tool_detail(&mut app, 2, 3, "tool shell\n{}".to_string());
    let dialog = app.tool_detail_dialog.as_mut().expect("dialog opens");
    dialog.context_menu_open = true;
    dialog.context_menu_pos = Some((12.0, 24.0));

    super::close_tool_detail_context_menu(dialog);

    assert!(!dialog.context_menu_open);
    assert_eq!(dialog.context_menu_pos, None);
}

#[test]
fn handle_open_tool_detail_sets_dialog_for_valid_raw_and_ignores_invalid_raw() {
    let (mut app, _task) = (test_state(), ());

    open_tool_detail(
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

    open_tool_detail(&mut app, 1, 1, "not a tool".to_string());

    assert_eq!(app.tool_detail_dialog.as_ref().expect("dialog remains").msg_idx, 4);
}

#[test]
fn tool_detail_scroll_helpers_clamp_to_available_lines() {
    let (mut app, _task) = (test_state(), ());
    app.current_line_height = 10.0;
    open_tool_detail(&mut app, 0, 0, "tool shell\nline1\nline2\nline3\nline4".into());
    app.tool_detail_dialog.as_mut().expect("dialog").viewport_height = 20.0;

    assert_eq!(super::tool_detail_max_scroll_top_line(&app), 2.0);

    super::apply_tool_detail_scroll_lines(&mut app, 10);
    assert_eq!(app.tool_detail_dialog.as_ref().expect("dialog").scroll_top_line, 2.0);

    super::apply_tool_detail_scroll_lines(&mut app, -10);
    assert_eq!(app.tool_detail_dialog.as_ref().expect("dialog").scroll_top_line, 0.0);
}

#[test]
fn tool_detail_handlers_are_safe_without_an_open_dialog() {
    let (mut app, _task) = (test_state(), ());

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
    let (mut app, _task) = (test_state(), ());
    open_tool_detail(&mut app, 0, 0, "tool shell\nhello".into());

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
    let (mut app, _task) = (test_state(), ());
    open_tool_detail(&mut app, 0, 0, "tool shell\ncut me".into());
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
    let (mut app, _task) = (test_state(), ());
    app.current_line_height = 10.0;
    open_tool_detail(&mut app, 0, 0, "tool shell\none\ntwo\nthree\nfour".into());
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
    let (mut app, _task) = (test_state(), ());
    app.current_line_height = 10.0;
    open_tool_detail(&mut app, 0, 0, "tool shell\none\ntwo\nthree\nfour".into());
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
    let (mut app, _task) = (test_state(), ());
    app.current_line_height = 10.0;
    open_tool_detail(&mut app, 0, 0, "tool shell\none\ntwo\nthree\nfour".into());
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
    let (mut app, _task) = (test_state(), ());
    open_tool_detail(&mut app, 7, 8, "tool shell\ncontent".into());
    assert!(app.tool_detail_dialog.is_some());

    super::handle_close_tool_detail(&mut app);

    assert!(app.tool_detail_dialog.is_none());
}

#[test]
fn tool_detail_history_restores_paste_cut_and_delete_and_clears_redo_on_edit() {
    use iced::widget::text_editor::{Action, Edit};

    let (mut app, _task) = (test_state(), ());
    open_tool_detail(&mut app, 0, 0, "tool shell\n原始输出".into());
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

    let (mut app, _task) = (test_state(), ());
    open_tool_detail(&mut app, 0, 0, "tool shell\ninitial".into());
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

    open_tool_detail(&mut app, 1, 0, "tool shell\nother".into());
    super::handle_tool_detail_undo(&mut app);
    super::handle_tool_detail_redo(&mut app);
    let dialog = app.tool_detail_dialog.as_ref().unwrap();
    assert_eq!(dialog.editor.text(), "other");
    assert!(dialog.undo_stack.is_empty());
    assert!(dialog.redo_stack.is_empty());
}
