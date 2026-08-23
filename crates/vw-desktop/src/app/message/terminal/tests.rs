// 验证终端顶层消息向标签、重命名和事件模块路由。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{TerminalMessage, update};
use crate::app::{App, FocusArea};

fn test_app() -> App {
    App::new().0
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn select_routes_to_tabs_and_closes_context_menu_for_unknown_tab() {
    let mut app = test_app();
    app.focus_area = FocusArea::Preview;
    app.terminal.tab_context_menu_id = Some(7);
    app.terminal.tab_context_menu_pos = Some((12.0, 24.0));

    let _task = update(&mut app, TerminalMessage::Select(u64::MAX));

    assert_eq!(app.focus_area, FocusArea::Terminal);
    assert_eq!(app.terminal.tab_context_menu_id, None);
    assert_eq!(app.terminal.tab_context_menu_pos, None);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn rename_cancel_routes_to_rename_and_closes_context_menu_for_unknown_tab() {
    let mut app = test_app();
    app.terminal.tab_context_menu_id = Some(9);
    app.terminal.tab_context_menu_pos = Some((3.0, 6.0));

    let _task = update(&mut app, TerminalMessage::RenameCancel(u64::MAX));

    assert_eq!(app.terminal.tab_context_menu_id, None);
    assert_eq!(app.terminal.tab_context_menu_pos, None);
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn drag_started_routes_to_event_and_captures_drag_origin() {
    let mut app = test_app();
    app.cursor_position.y = 48.5;
    app.terminal.height = 320.0;
    app.terminal.is_dragging = false;
    app.terminal.drag_anchor_y = None;
    app.terminal.drag_start_height = 0.0;

    let _task = update(&mut app, TerminalMessage::DragStarted);

    assert!(app.terminal.is_dragging);
    assert_eq!(app.terminal.drag_anchor_y, Some(48.5));
    assert_eq!(app.terminal.drag_start_height, 320.0);
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn tab_context_close_routes_to_tabs_and_clears_menu_coordinates() {
    let mut app = test_app();
    app.terminal.tab_context_menu_id = Some(17);
    app.terminal.tab_context_menu_pos = Some((8.0, 16.0));

    let _task = update(&mut app, TerminalMessage::TabContextClose);

    assert_eq!(app.terminal.tab_context_menu_id, None);
    assert_eq!(app.terminal.tab_context_menu_pos, None);
}
