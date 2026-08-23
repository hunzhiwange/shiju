// 验证终端标签重命名草稿、缺失标签与菜单清理行为。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::terminal::TerminalState;

fn terminal_with_context_menu() -> TerminalState {
    let mut terminal = TerminalState::default();
    terminal.tab_context_menu_id = Some(9);
    terminal.tab_context_menu_pos = Some((12.0, 24.0));
    terminal
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn rename_start_closes_context_menu_and_selects_missing_tab() {
    let mut terminal = terminal_with_context_menu();
    terminal.height = 120.0;

    let _ = super::update_terminal(&mut terminal, super::TerminalMessage::RenameStart(42));

    assert_eq!(terminal.tab_context_menu_id, None);
    assert_eq!(terminal.tab_context_menu_pos, None);
    assert_eq!(terminal.active_id, Some(42));
    assert!(terminal.is_visible);
    assert_eq!(terminal.height, 200.0);
    assert!(terminal.tabs.is_empty());
}

#[test]
fn rename_changed_for_missing_tab_leaves_terminal_state_unchanged() {
    let mut terminal = terminal_with_context_menu();
    terminal.active_id = Some(7);

    let _ = super::update_terminal(
        &mut terminal,
        super::TerminalMessage::RenameChanged(42, "new title".to_string()),
    );

    assert_eq!(terminal.tab_context_menu_id, Some(9));
    assert_eq!(terminal.tab_context_menu_pos, Some((12.0, 24.0)));
    assert_eq!(terminal.active_id, Some(7));
    assert!(terminal.tabs.is_empty());
}

#[test]
fn rename_save_for_missing_tab_still_closes_context_menu() {
    let mut terminal = terminal_with_context_menu();

    let _ = super::update_terminal(&mut terminal, super::TerminalMessage::RenameSave(42));

    assert_eq!(terminal.tab_context_menu_id, None);
    assert_eq!(terminal.tab_context_menu_pos, None);
    assert!(terminal.tabs.is_empty());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn rename_cancel_for_missing_tab_still_closes_context_menu() {
    let mut terminal = terminal_with_context_menu();

    let _ = super::update_terminal(&mut terminal, super::TerminalMessage::RenameCancel(42));

    assert_eq!(terminal.tab_context_menu_id, None);
    assert_eq!(terminal.tab_context_menu_pos, None);
    assert!(terminal.tabs.is_empty());
}

#[test]
fn unrelated_message_is_ignored() {
    let mut terminal = terminal_with_context_menu();

    let _ = super::update_terminal(&mut terminal, super::TerminalMessage::DragStarted);

    assert_eq!(terminal.tab_context_menu_id, Some(9));
    assert_eq!(terminal.tab_context_menu_pos, Some((12.0, 24.0)));
    assert_eq!(terminal.active_id, None);
}

#[test]
fn start_and_change_edit_preserve_exact_text() {
    let mut edit_title = None;

    super::start_edit("Original", &mut edit_title);
    assert_eq!(edit_title.as_deref(), Some("Original"));

    super::change_edit(&mut edit_title, "  新标题  ".to_string());
    assert_eq!(edit_title.as_deref(), Some("  新标题  "));
}

#[test]
fn save_edit_trims_nonempty_title_and_consumes_draft() {
    let mut title = "Original".to_string();
    let mut edit_title = Some("  Renamed  ".to_string());

    super::save_edit(&mut title, &mut edit_title);

    assert_eq!(title, "Renamed");
    assert_eq!(edit_title, None);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn save_edit_rejects_blank_title_and_consumes_draft() {
    let mut title = "Original".to_string();
    let mut edit_title = Some(" \t\n ".to_string());

    super::save_edit(&mut title, &mut edit_title);

    assert_eq!(title, "Original");
    assert_eq!(edit_title, None);
}

#[test]
fn save_edit_without_draft_preserves_title() {
    let mut title = "Original".to_string();
    let mut edit_title = None;

    super::save_edit(&mut title, &mut edit_title);

    assert_eq!(title, "Original");
    assert_eq!(edit_title, None);
}

#[test]
fn cancel_edit_discards_draft() {
    let mut edit_title = Some("Changed".to_string());

    super::cancel_edit(&mut edit_title);

    assert_eq!(edit_title, None);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn public_update_for_missing_tab_still_applies_cancel_cleanup() {
    let (mut app, _task) = crate::app::App::new();
    app.terminal.tab_context_menu_id = Some(31);
    app.terminal.tab_context_menu_pos = Some((4.0, 5.0));

    let _task = super::update(&mut app, super::TerminalMessage::RenameCancel(u64::MAX));

    assert_eq!(app.terminal.tab_context_menu_id, None);
    assert_eq!(app.terminal.tab_context_menu_pos, None);
}
