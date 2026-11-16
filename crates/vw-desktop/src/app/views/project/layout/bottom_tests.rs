//! 宿主保留：验证 terminal_panel 内容与领域底部布局的实际装配。
//! 尺寸边界与拖动命中测试迁至 desktop-workspace::desktop_split_19_tests。
#[test]
fn bottom_panel_builds_terminal_when_visible() {
    let mut app = crate::app::App::new().0;
    app.terminal.is_visible = true;
    app.terminal.height = 240.0;
    std::hint::black_box(super::bottom_panel(&app));
}
