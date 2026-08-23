// 验证终端拖拽开始事件对光标与高度快照的记录。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::apply_drag_started;
use crate::app::TerminalState;

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("event_tests"));
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn drag_started_records_cursor_and_current_height() {
    let mut terminal = TerminalState::default();
    terminal.height = 275.5;

    apply_drag_started(&mut terminal, 418.25);

    assert!(terminal.is_dragging);
    assert_eq!(terminal.drag_anchor_y, Some(418.25));
    assert_eq!(terminal.drag_start_height, 275.5);
}

#[test]
fn drag_started_replaces_stale_drag_snapshot_at_boundary_values() {
    let mut terminal = TerminalState::default();
    terminal.is_dragging = true;
    terminal.drag_anchor_y = Some(91.0);
    terminal.drag_start_height = 240.0;
    terminal.height = 0.0;

    apply_drag_started(&mut terminal, -12.5);

    assert!(terminal.is_dragging);
    assert_eq!(terminal.drag_anchor_y, Some(-12.5));
    assert_eq!(terminal.drag_start_height, 0.0);
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn drag_started_update_uses_app_cursor_and_terminal_height() {
    let (mut app, _task) = crate::app::App::new();
    app.cursor_position = iced::Point::new(12.0, 321.5);
    app.terminal.height = 288.0;

    let _task = super::update(&mut app, super::TerminalMessage::DragStarted);

    assert!(app.terminal.is_dragging);
    assert_eq!(app.terminal.drag_anchor_y, Some(321.5));
    assert_eq!(app.terminal.drag_start_height, 288.0);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn unrelated_event_message_preserves_drag_state() {
    let (mut app, _task) = crate::app::App::new();
    app.terminal.is_dragging = true;
    app.terminal.drag_anchor_y = Some(45.0);
    app.terminal.drag_start_height = 210.0;

    let _task = super::update(&mut app, super::TerminalMessage::Add);

    assert!(app.terminal.is_dragging);
    assert_eq!(app.terminal.drag_anchor_y, Some(45.0));
    assert_eq!(app.terminal.drag_start_height, 210.0);
}
