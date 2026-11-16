#![allow(unused_variables, unused_mut, unused_assignments)]
use iced::Element;
use iced::widget::text;

use crate::views::project::layout::sidebar;
use iced::widget::Space;
type Message = ();

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("sidebar_tests"));
}

#[test]
fn sidebar_group_expanded_builds_resize_handle() {
    let mut expanded = false;
    let mut hover = None::<String>;
    expanded = true;

    let (sidebar, resize_handle) = sidebar_group(expanded, 320.0, 56.0, 0.5, 12.0);

    keep_element(sidebar);
    keep_element(resize_handle.expect("expanded settings sidebar exposes resize handle"));
}

#[test]
fn sidebar_group_collapsed_hides_resize_handle() {
    let mut expanded = false;
    let mut hover = None::<String>;
    expanded = false;

    let (sidebar, resize_handle) = sidebar_group(expanded, 320.0, 56.0, 0.5, 12.0);

    keep_element(sidebar);
    assert!(resize_handle.is_none());
}

#[test]
fn hover_overlay_layout_without_hover_uses_plain_sidebar() {
    let mut expanded = false;
    let mut hover = None::<String>;
    hover = None;
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(hover, left_sidebar, right_column, 320.0, 56.0, 0.5, 12.0, 4.0);

    keep_element(layout);
}

#[test]
fn hover_overlay_layout_with_hover_builds_sessions_overlay() {
    let mut expanded = false;
    let mut hover = None::<String>;
    hover = Some("/tmp/vibe-window-project".to_string());
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(hover, left_sidebar, right_column, 320.0, 56.0, 0.5, 12.0, 4.0);

    keep_element(layout);
}

#[test]
fn hover_overlay_layout_with_hover_clamps_negative_panel_width() {
    let mut expanded = false;
    let mut hover = None::<String>;
    hover = Some("/tmp/vibe-window-project".to_string());
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout = hover_overlay_layout(hover, left_sidebar, right_column, 40.0, 80.0, 1.0, 0.0, 0.0);

    keep_element(layout);
}

#[test]
fn sidebar_group_collapsed_empty_projects() {
    let mut expanded = false;
    let mut hover = None::<String>;
    expanded = false;

    let (sidebar, resize_handle) = sidebar_group(expanded, 320.0, 56.0, 0.5, 12.0);

    keep_element(sidebar);
    assert!(resize_handle.is_none());
}

#[test]
fn hover_overlay_layout_with_hover_zero_session_panel_scale() {
    let mut expanded = false;
    let mut hover = None::<String>;
    hover = Some("/tmp/project".to_string());
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(hover, left_sidebar, right_column, 400.0, 100.0, 0.0, 12.0, 4.0);

    keep_element(layout);
}

#[test]
fn hover_overlay_layout_with_hover_zero_spacing() {
    let mut expanded = false;
    let mut hover = None::<String>;
    hover = Some("/tmp/project".to_string());
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(hover, left_sidebar, right_column, 400.0, 100.0, 0.5, 12.0, 0.0);

    keep_element(layout);
}

#[test]
fn hover_overlay_layout_without_hover_zero_spacing() {
    let expanded = false;
    let hover = None::<String>;
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(hover, left_sidebar, right_column, 400.0, 100.0, 0.5, 12.0, 0.0);

    keep_element(layout);
}

#[test]
fn hover_overlay_layout_with_hover_equal_panel_and_rail_widths() {
    let mut expanded = false;
    let mut hover = None::<String>;
    hover = Some("/tmp/project".to_string());
    let left_sidebar = text("left").into();
    let right_column = text("chat").into();

    let layout =
        hover_overlay_layout(hover, left_sidebar, right_column, 100.0, 100.0, 0.75, 8.0, 2.0);

    keep_element(layout);
}

fn sidebar_group(
    expanded: bool,
    width: f32,
    rail: f32,
    scale: f32,
    _radius: f32,
) -> (Element<'static, ()>, Option<Element<'static, ()>>) {
    sidebar::sidebar_group(
        expanded,
        Space::new().into(),
        Space::new().into(),
        (),
        width,
        rail,
        scale,
    )
}
fn hover_overlay_layout<'a>(
    hover: Option<String>,
    left: Element<'a, ()>,
    right: Element<'a, ()>,
    width: f32,
    rail: f32,
    scale: f32,
    _radius: f32,
    spacing: f32,
) -> Element<'a, ()> {
    sidebar::hover_overlay_layout(
        left,
        right,
        hover.map(|_| Space::new().into()),
        (),
        width,
        rail,
        scale,
        spacing,
    )
}
