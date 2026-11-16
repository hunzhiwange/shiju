//! 侧栏尺寸边界只依赖领域布局；实际项目轨道与悬浮会话内容仍由宿主覆盖。
use crate::views::project::layout::sidebar::sidebar_group;
use iced::{Element, widget::Space};
fn keep_element(element: Element<'_, ()>) {
    std::hint::black_box(element);
}
#[test]
fn sidebar_group_expanded_clamps_negative_panel_width() {
    let (sidebar, resize_handle) =
        sidebar_group(true, Space::new().into(), Space::new().into(), (), 40.0, 80.0, 1.0);

    keep_element(sidebar);
    keep_element(resize_handle.expect("resize handle is still available when panel width clamps"));
}

#[test]
fn sidebar_group_expanded_zero_scale_uses_left_rail_width_only() {
    let (sidebar, resize_handle) =
        sidebar_group(true, Space::new().into(), Space::new().into(), (), 400.0, 100.0, 0.0);

    keep_element(sidebar);
    keep_element(resize_handle.expect("resize handle present when expanded"));
}

#[test]
fn sidebar_group_expanded_full_scale_uses_max_width() {
    let (sidebar, resize_handle) =
        sidebar_group(true, Space::new().into(), Space::new().into(), (), 400.0, 100.0, 1.0);

    keep_element(sidebar);
    keep_element(resize_handle.expect("resize handle present when expanded"));
}

#[test]
fn sidebar_group_expanded_equal_panel_and_rail_widths() {
    let (sidebar, resize_handle) =
        sidebar_group(true, Space::new().into(), Space::new().into(), (), 100.0, 100.0, 0.75);

    keep_element(sidebar);
    keep_element(resize_handle.expect("resize handle present when expanded"));
}

#[test]
fn sidebar_group_expanded_large_left_rail() {
    let (sidebar, resize_handle) =
        sidebar_group(true, Space::new().into(), Space::new().into(), (), 120.0, 200.0, 0.5);

    keep_element(sidebar);
    keep_element(resize_handle.expect("resize handle present when expanded"));
}
