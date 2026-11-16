// 验证面板弹出层的互斥规则、切换及前置加载。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;

fn test_app() -> App {
    App::new().0
}

fn set_all_popovers(app: &mut App, visible: bool) {
    app.popover_visibility.model = visible;
    app.popover_visibility.mode = visible;
    app.popover_visibility.send_mode = visible;
    app.popover_visibility.file = visible;
    app.popover_visibility.acp = visible;
    app.popover_visibility.reasoning_effort = visible;
    app.popover_visibility.usage = visible;
    app.popover_visibility.session_tool_selector = visible;
    app.popover_visibility.session_actions = visible;
    app.popover_visibility.executor = visible;
}

fn visibility(app: &App) -> [bool; 10] {
    [
        app.popover_visibility.model,
        app.popover_visibility.mode,
        app.popover_visibility.send_mode,
        app.popover_visibility.file,
        app.popover_visibility.acp,
        app.popover_visibility.reasoning_effort,
        app.popover_visibility.usage,
        app.popover_visibility.session_tool_selector,
        app.popover_visibility.session_actions,
        app.popover_visibility.executor,
    ]
}

#[test]
fn unrelated_view_message_leaves_popover_state_unchanged() {
    let mut app = test_app();
    set_all_popovers(&mut app, true);

    let _ = update(&mut app, ViewMessage::OpenKnowledge);

    assert_eq!(visibility(&app), [true; 10]);
}
