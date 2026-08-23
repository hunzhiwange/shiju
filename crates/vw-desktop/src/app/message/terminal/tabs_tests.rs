// 验证终端标签选择、关闭、回收与上下文菜单状态。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{TerminalMessage, update};
use crate::app::{App, FocusArea, TerminalState};

fn test_app_with_blank_terminal() -> App {
    let mut app = App::new().0;
    app.terminal = TerminalState::default();
    app
}

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("tabs_tests"));
}

#[test]
fn select_clears_context_menu_and_focuses_visible_terminal() {
    let mut app = test_app_with_blank_terminal();
    app.terminal.height = 120.0;
    app.terminal.tab_context_menu_id = Some(7);
    app.terminal.tab_context_menu_pos = Some((12.0, 24.0));

    let _ = update(&mut app, TerminalMessage::Select(42));

    assert_eq!(app.terminal.tab_context_menu_id, None);
    assert_eq!(app.terminal.tab_context_menu_pos, None);
    assert_eq!(app.focus_area, FocusArea::Terminal);
    assert_eq!(app.terminal.active_id, Some(42));
    assert!(app.terminal.is_visible);
    assert_eq!(app.terminal.height, 200.0);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn select_preserves_terminal_height_at_minimum_boundary() {
    let mut app = test_app_with_blank_terminal();
    app.terminal.height = 160.0;

    let _ = update(&mut app, TerminalMessage::Select(17));

    assert_eq!(app.terminal.active_id, Some(17));
    assert!(app.terminal.is_visible);
    assert_eq!(app.terminal.height, 160.0);
    assert_eq!(app.focus_area, FocusArea::Terminal);
}

#[test]
fn close_preserves_only_terminal_and_clears_context_menu() {
    let mut app = test_app_with_blank_terminal();
    app.terminal.active_id = Some(9);
    app.terminal.tab_context_menu_id = Some(9);
    app.terminal.tab_context_menu_pos = Some((3.0, 4.0));

    let _ = update(&mut app, TerminalMessage::Close(9));

    assert_eq!(app.terminal.active_id, Some(9));
    assert!(app.terminal.tabs.is_empty());
    assert_eq!(app.terminal.tab_context_menu_id, None);
    assert_eq!(app.terminal.tab_context_menu_pos, None);
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn close_with_multiple_tabs_schedules_reap_after_retiring() {
    let mut app = test_app_with_blank_terminal();
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(app.terminal.add_terminal(Some(tmp.path().to_path_buf())));
    assert!(app.terminal.add_terminal(Some(tmp.path().to_path_buf())));
    let closed_id = app.terminal.tabs[0].id;

    let _task = update(&mut app, TerminalMessage::Close(closed_id));

    assert!(app.terminal.tabs.iter().all(|tab| tab.id != closed_id));
    assert!(app.terminal.has_retired());

    let _ = update(&mut app, TerminalMessage::ReapRetired);
    assert!(!app.terminal.has_retired());
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn tab_context_open_selects_terminal_and_records_anchor() {
    let mut app = test_app_with_blank_terminal();

    let _ = update(&mut app, TerminalMessage::TabContextOpen(23, 18.5, 37.25));

    assert_eq!(app.focus_area, FocusArea::Terminal);
    assert_eq!(app.terminal.active_id, Some(23));
    assert!(app.terminal.is_visible);
    assert_eq!(app.terminal.tab_context_menu_id, Some(23));
    assert_eq!(app.terminal.tab_context_menu_pos, Some((18.5, 37.25)));
}

#[test]
fn tab_context_close_clears_id_and_anchor_without_changing_selection() {
    let mut app = test_app_with_blank_terminal();
    app.terminal.active_id = Some(31);
    app.terminal.tab_context_menu_id = Some(31);
    app.terminal.tab_context_menu_pos = Some((8.0, 16.0));

    let _ = update(&mut app, TerminalMessage::TabContextClose);

    assert_eq!(app.terminal.active_id, Some(31));
    assert_eq!(app.terminal.tab_context_menu_id, None);
    assert_eq!(app.terminal.tab_context_menu_pos, None);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn unrelated_message_is_ignored_without_mutating_terminal_state() {
    let mut app = test_app_with_blank_terminal();
    app.terminal.active_id = Some(5);
    app.terminal.tab_context_menu_id = Some(5);
    app.terminal.tab_context_menu_pos = Some((1.0, 2.0));

    let _ = update(&mut app, TerminalMessage::RenameCancel(5));

    assert_eq!(app.terminal.active_id, Some(5));
    assert_eq!(app.terminal.tab_context_menu_id, Some(5));
    assert_eq!(app.terminal.tab_context_menu_pos, Some((1.0, 2.0)));
}
