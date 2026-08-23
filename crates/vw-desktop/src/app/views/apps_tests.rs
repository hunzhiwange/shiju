//! 应用中心视图组合测试，验证基础滚动内容与书签弹窗覆盖层的层级。
//! 即使新增和编辑标志同时出现，也只能渲染一个确定优先级的弹窗栈。

use crate::app::App;

fn root_child_count(app: &App) -> usize {
    super::view(app).as_widget().children().len()
}

#[test]
fn view_without_active_modal_returns_scrollable_base_content() {
    let app = App::new().0;

    assert_eq!(root_child_count(&app), 1);
}

#[test]
fn view_with_add_bookmark_modal_stacks_overlay_above_base_content() {
    let mut app = App::new().0;
    app.show_web_links_menu = true;

    assert_eq!(root_child_count(&app), 2);
}

#[test]
fn view_with_edit_bookmark_modal_stacks_overlay_above_base_content() {
    let mut app = App::new().0;
    app.editing_web_bookmark = Some(0);

    assert_eq!(root_child_count(&app), 2);
}

#[test]
fn view_with_both_modal_flags_keeps_single_modal_stack() {
    let mut app = App::new().0;
    app.show_web_links_menu = true;
    app.editing_web_bookmark = Some(0);

    assert_eq!(root_child_count(&app), 2);
}
