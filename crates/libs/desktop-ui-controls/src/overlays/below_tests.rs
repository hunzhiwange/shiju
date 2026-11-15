//! below_tests.rs 测试模块。
//!
//! 这些测试固定相邻解析器、视图辅助函数或状态计算的行为，防止后续 UI 重排时破坏边界契约。

use super::{
    BelowOverlay, available_space_above, available_space_below, compute_overlay_position,
    layout_with_backdrop, should_place_overlay_above,
};
use iced::advanced::layout;
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{Layout, Shell, clipboard, mouse, renderer};
use iced::theme::Palette;
use iced::widget::{Space, text};
use iced::{Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

#[derive(Debug, Clone, PartialEq)]
enum TestMessage {
    Close,
    Alternate,
}

/// Create label element with () renderer for testing.
fn label_el(value: &'static str) -> Element<'static, TestMessage, Theme, ()> {
    text(value).into()
}

/// Create label element with default renderer (for From impl tests).
fn label_default(value: &'static str) -> Element<'static, TestMessage> {
    text(value).into()
}

fn fixed_el(width: f32, height: f32) -> Element<'static, TestMessage, Theme, ()> {
    Space::new().width(width).height(height).into()
}

fn node_contains_bounds(node: &layout::Node, expected: Rectangle) -> bool {
    node.bounds() == expected
        || node.children().iter().any(|child| node_contains_bounds(child, expected))
}

#[test]
fn below_overlay_new_sets_hidden_defaults() {
    let overlay = BelowOverlay::new(label_default("content"), label_default("overlay"));

    assert!(!overlay.show);
    assert_eq!(overlay.gap, 0.0);
    assert!(overlay.snap_within_viewport);
    assert!(overlay.on_close.is_none());
    assert!(overlay.capture_outside_click);
}

#[test]
fn below_overlay_builder_updates_all_flags() {
    let overlay = BelowOverlay::new(label_default("content"), label_default("overlay"))
        .show(true)
        .gap(9.0)
        .snap_within_viewport(false)
        .capture_outside_click(false)
        .on_close(TestMessage::Close);

    assert!(overlay.show);
    assert_eq!(overlay.gap, 9.0);
    assert!(!overlay.snap_within_viewport);
    assert_eq!(overlay.on_close, Some(TestMessage::Close));
    assert!(!overlay.capture_outside_click);
}

#[test]
fn compute_overlay_position_snaps_within_viewport() {
    let target_bounds = Rectangle { x: 280.0, y: 24.0, width: 32.0, height: 24.0 };
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));
    let overlay_size = Size::new(120.0, 160.0);

    let position =
        compute_overlay_position(target_bounds, viewport, overlay_size, 4.0, true, false);

    assert_eq!(position, Point::new(180.0, 40.0));
}

#[test]
fn below_overlay_can_shrink_to_stay_attached_below_target() {
    let target_bounds = Rectangle { x: 280.0, y: 24.0, width: 32.0, height: 24.0 };
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));

    assert!(!should_place_overlay_above(target_bounds, viewport, 160.0, 4.0));

    let shrunk_overlay = Size::new(120.0, available_space_below(target_bounds, viewport, 4.0));
    let position =
        compute_overlay_position(target_bounds, viewport, shrunk_overlay, 4.0, true, false);

    assert_eq!(position, Point::new(180.0, 52.0));
}

#[test]
fn should_place_overlay_above_when_below_space_is_worse() {
    let target_bounds = Rectangle { x: 40.0, y: 160.0, width: 60.0, height: 24.0 };
    let viewport = Rectangle::with_size(Size::new(300.0, 220.0));

    assert_eq!(available_space_above(target_bounds, viewport, 4.0), 156.0);
    assert_eq!(available_space_below(target_bounds, viewport, 4.0), 32.0);
    assert!(should_place_overlay_above(target_bounds, viewport, 80.0, 4.0));
}

#[test]
fn compute_overlay_position_can_skip_viewport_snap() {
    let target_bounds = Rectangle { x: 280.0, y: 190.0, width: 32.0, height: 24.0 };
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));

    let position = compute_overlay_position(
        target_bounds,
        viewport,
        Size::new(120.0, 60.0),
        4.0,
        false,
        false,
    );

    assert_eq!(position, Point::new(280.0, 218.0));
}

#[test]
fn layout_with_backdrop_expands_to_full_viewport() {
    let viewport = Rectangle { x: 10.0, y: 20.0, width: 320.0, height: 240.0 };
    let overlay_node = layout::Node::new(Size::new(100.0, 80.0)).move_to(Point::new(40.0, 56.0));

    let root = layout_with_backdrop(viewport, overlay_node);

    assert_eq!(root.bounds(), viewport);
    assert_eq!(root.children().len(), 1);
    assert_eq!(
        root.children()[0].bounds(),
        Rectangle { x: 40.0, y: 56.0, width: 100.0, height: 80.0 }
    );
}

// --- Helper function edge cases ---

#[test]
fn available_space_below_zero_gap() {
    let target = Rectangle { x: 0.0, y: 0.0, width: 50.0, height: 50.0 };
    let viewport = Rectangle::with_size(Size::new(200.0, 200.0));
    assert_eq!(available_space_below(target, viewport, 0.0), 150.0);
}

#[test]
fn available_space_below_negative_gap() {
    let target = Rectangle { x: 0.0, y: 0.0, width: 50.0, height: 50.0 };
    let viewport = Rectangle::with_size(Size::new(200.0, 200.0));
    assert_eq!(available_space_below(target, viewport, -10.0), 160.0);
}

#[test]
fn available_space_above_target_at_viewport_top() {
    let target = Rectangle { x: 0.0, y: 10.0, width: 50.0, height: 50.0 };
    let viewport = Rectangle::with_size(Size::new(200.0, 200.0));
    assert_eq!(available_space_above(target, viewport, 4.0), 6.0);
}

#[test]
fn available_space_above_target_touches_viewport_top() {
    let target = Rectangle { x: 0.0, y: 0.0, width: 50.0, height: 50.0 };
    let viewport = Rectangle::with_size(Size::new(200.0, 200.0));
    assert_eq!(available_space_above(target, viewport, 0.0), 0.0);
}

#[test]
fn should_place_overlay_above_false_when_below_suffices() {
    let target = Rectangle { x: 40.0, y: 40.0, width: 60.0, height: 24.0 };
    let viewport = Rectangle::with_size(Size::new(300.0, 220.0));
    assert!(!should_place_overlay_above(target, viewport, 30.0, 4.0));
}

#[test]
fn should_place_overlay_above_false_when_below_exactly_fits() {
    let target = Rectangle { x: 40.0, y: 160.0, width: 60.0, height: 24.0 };
    let viewport = Rectangle::with_size(Size::new(300.0, 220.0));
    assert!(!should_place_overlay_above(target, viewport, 32.0, 4.0));
}

#[test]
fn should_place_overlay_above_false_when_spaces_are_equal() {
    let target = Rectangle { x: 40.0, y: 88.0, width: 60.0, height: 24.0 };
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));

    assert_eq!(available_space_above(target, viewport, 4.0), 84.0);
    assert_eq!(available_space_below(target, viewport, 4.0), 84.0);
    assert!(!should_place_overlay_above(target, viewport, 120.0, 4.0));
}

#[test]
fn compute_overlay_position_place_above_with_snap() {
    let target_bounds = Rectangle { x: 50.0, y: 160.0, width: 60.0, height: 24.0 };
    let viewport = Rectangle::with_size(Size::new(200.0, 200.0));
    let pos =
        compute_overlay_position(target_bounds, viewport, Size::new(80.0, 40.0), 4.0, true, true);
    assert_eq!(pos.y, 116.0); // 160 - 40 - 4 = 116
    assert_eq!(pos.x, 50.0); // within [0, 120]
}

#[test]
fn compute_overlay_position_place_above_no_snap() {
    let target_bounds = Rectangle { x: 150.0, y: 160.0, width: 60.0, height: 24.0 };
    let viewport = Rectangle::with_size(Size::new(200.0, 200.0));
    let pos =
        compute_overlay_position(target_bounds, viewport, Size::new(80.0, 40.0), 4.0, false, true);
    assert_eq!(pos.y, 116.0); // 160 - 40 - 4 = 116
    assert_eq!(pos.x, 150.0); // no clamping
}

#[test]
fn compute_overlay_position_clamps_x_and_y_with_snap() {
    let target_bounds = Rectangle { x: -50.0, y: 190.0, width: 60.0, height: 24.0 };
    let viewport = Rectangle::with_size(Size::new(200.0, 200.0));
    let pos =
        compute_overlay_position(target_bounds, viewport, Size::new(120.0, 60.0), 4.0, true, false);
    // x clamped to [0, 80], y clamped to [0, 140]
    assert_eq!(pos.x, 0.0);
    assert_eq!(pos.y, 140.0);
}

#[test]
fn compute_overlay_position_uses_viewport_origin_for_oversized_overlay() {
    let target = Rectangle { x: 500.0, y: 500.0, width: 60.0, height: 24.0 };
    let viewport = Rectangle { x: 10.0, y: 20.0, width: 100.0, height: 80.0 };

    let position =
        compute_overlay_position(target, viewport, Size::new(140.0, 120.0), 4.0, true, false);

    assert_eq!(position, Point::new(10.0, 20.0));
}

#[test]
fn available_space_respects_nonzero_viewport_origin() {
    let target = Rectangle { x: 30.0, y: 50.0, width: 60.0, height: 20.0 };
    let viewport = Rectangle { x: 10.0, y: 20.0, width: 200.0, height: 160.0 };

    assert_eq!(available_space_above(target, viewport, 5.0), 25.0);
    assert_eq!(available_space_below(target, viewport, 5.0), 105.0);
}

#[test]
fn layout_with_backdrop_preserves_zero_sized_overlay() {
    let viewport = Rectangle { x: 0.0, y: 0.0, width: 300.0, height: 200.0 };
    let overlay_node = layout::Node::new(Size::ZERO).move_to(Point::new(20.0, 30.0));
    let root = layout_with_backdrop(viewport, overlay_node);

    assert_eq!(root.bounds(), viewport);
    assert_eq!(root.children().len(), 1);
    assert_eq!(
        root.children()[0].bounds(),
        Rectangle { x: 20.0, y: 30.0, width: 0.0, height: 0.0 }
    );
}

// --- Builder edge cases ---

#[test]
fn below_overlay_builder_negative_gap() {
    let overlay = BelowOverlay::new(label_default("content"), label_default("overlay")).gap(-5.0);
    assert_eq!(overlay.gap, -5.0);
}

#[test]
fn below_overlay_builder_on_close_twice() {
    let overlay = BelowOverlay::new(label_default("content"), label_default("overlay"))
        .on_close(TestMessage::Close)
        .on_close(TestMessage::Alternate);
    assert_eq!(overlay.on_close, Some(TestMessage::Alternate));
}

// --- From impl (Widget ? Element) ---

#[test]
fn below_overlay_converts_to_element_with_default_renderer() {
    let _: Element<'_, TestMessage, Theme> =
        BelowOverlay::new(label_default("content"), label_default("overlay")).into();
}

// --- Widget trait tests ---

#[test]
fn widget_children_returns_two_trees() {
    let overlay = BelowOverlay::new(label_default("content"), label_default("overlay"));
    let children = Widget::children(&overlay);
    assert_eq!(children.len(), 2);
}

#[test]
fn widget_overlay_none_when_hidden() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay")).show(false);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let result = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    );

    // text has no overlay, and show=false, so overall should be None
    assert!(result.is_none());
}

#[test]
fn widget_overlay_some_when_shown() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay")).show(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let result = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    );

    assert!(result.is_some());
}

#[test]
fn widget_overlay_preserves_content_overlay_when_outer_overlay_is_hidden() {
    let inner = BelowOverlay::new(label_el("inner content"), label_el("inner overlay")).show(true);
    let mut outer = BelowOverlay::new(inner, label_el("outer overlay"));
    let mut tree = Tree::new(&outer as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut outer,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );

    let overlay = Widget::overlay(
        &mut outer,
        &mut tree,
        Layout::new(&node),
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    );

    assert!(overlay.is_some());
}

// --- BelowOverlayElement (Overlay impl) tests ---

#[test]
fn overlay_element_layout_returns_non_empty_backdrop() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay")).show(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let mut oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let overlay_node = oe.as_overlay_mut().layout(&(), Size::new(300.0, 200.0));

    assert_eq!(overlay_node.bounds(), Rectangle { x: 0.0, y: 0.0, width: 300.0, height: 200.0 });
    assert_eq!(overlay_node.children().len(), 1);
}

#[test]
fn overlay_element_layout_with_snap_disabled_and_gap() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"))
        .show(true)
        .gap(8.0)
        .snap_within_viewport(false);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(100.0, 100.0)),
    );
    let layout = Layout::new(&node);

    let mut oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(100.0, 100.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let overlay_node = oe.as_overlay_mut().layout(&(), Size::new(100.0, 100.0));
    assert_eq!(overlay_node.bounds(), Rectangle { x: 0.0, y: 0.0, width: 100.0, height: 100.0 });
}

#[test]
fn overlay_element_layout_shrinks_below_without_covering_target() {
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));
    let mut overlay =
        BelowOverlay::new(fixed_el(32.0, 24.0), fixed_el(120.0, 160.0)).show(true).gap(4.0);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );
    let layout = Layout::new(&node);
    let mut overlay_element =
        Widget::overlay(&mut overlay, &mut tree, layout, &(), &viewport, Vector::new(280.0, 24.0))
            .expect("shown overlay should exist");

    let overlay_node = overlay_element.as_overlay_mut().layout(&(), viewport.size());

    assert!(node_contains_bounds(
        &overlay_node,
        Rectangle { x: 180.0, y: 52.0, width: 120.0, height: 148.0 }
    ));
}

#[test]
fn overlay_element_layout_places_and_shrinks_above_when_space_is_better() {
    let viewport = Rectangle::with_size(Size::new(300.0, 220.0));
    let mut overlay =
        BelowOverlay::new(fixed_el(60.0, 24.0), fixed_el(80.0, 200.0)).show(true).gap(4.0);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );
    let layout = Layout::new(&node);
    let mut overlay_element =
        Widget::overlay(&mut overlay, &mut tree, layout, &(), &viewport, Vector::new(40.0, 160.0))
            .expect("shown overlay should exist");

    let overlay_node = overlay_element.as_overlay_mut().layout(&(), viewport.size());

    assert!(node_contains_bounds(
        &overlay_node,
        Rectangle { x: 40.0, y: 0.0, width: 80.0, height: 156.0 }
    ));
}

#[test]
fn overlay_element_layout_without_snap_preserves_unclamped_position_and_size() {
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));
    let mut overlay = BelowOverlay::new(fixed_el(32.0, 24.0), fixed_el(120.0, 160.0))
        .show(true)
        .gap(4.0)
        .snap_within_viewport(false);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );
    let layout = Layout::new(&node);
    let mut overlay_element =
        Widget::overlay(&mut overlay, &mut tree, layout, &(), &viewport, Vector::new(280.0, 190.0))
            .expect("shown overlay should exist");

    let overlay_node = overlay_element.as_overlay_mut().layout(&(), viewport.size());

    assert!(node_contains_bounds(
        &overlay_node,
        Rectangle { x: 280.0, y: 26.0, width: 120.0, height: 160.0 }
    ));
}

#[test]
fn overlay_element_update_click_outside_with_on_close_publishes_message() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"))
        .show(true)
        .on_close(TestMessage::Close)
        .capture_outside_click(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let mut oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let overlay_node = oe.as_overlay_mut().layout(&(), Size::new(300.0, 200.0));
    let overlay_layout = Layout::new(&overlay_node);

    let mut messages: Vec<TestMessage> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut cb = clipboard::Null;

    oe.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        overlay_layout,
        mouse::Cursor::Available(Point::new(9999.0, 9999.0)),
        &(),
        &mut cb,
        &mut shell,
    );

    let captured = shell.is_event_captured();
    drop(shell);
    assert_eq!(messages, vec![TestMessage::Close]);
    assert!(captured);
}

#[test]
fn overlay_element_update_click_on_target_does_not_close_or_capture() {
    let mut overlay = BelowOverlay::new(fixed_el(60.0, 24.0), fixed_el(80.0, 40.0))
        .show(true)
        .on_close(TestMessage::Close)
        .capture_outside_click(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let mut oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let overlay_node = oe.as_overlay_mut().layout(&(), Size::new(300.0, 200.0));
    let overlay_layout = Layout::new(&overlay_node);

    let mut messages: Vec<TestMessage> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut cb = clipboard::Null;

    oe.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        overlay_layout,
        mouse::Cursor::Available(Point::new(10.0, 10.0)),
        &(),
        &mut cb,
        &mut shell,
    );

    let captured = shell.is_event_captured();
    drop(shell);
    assert!(messages.is_empty());
    assert!(!captured);
}

#[test]
fn overlay_element_update_cursor_unavailable_does_not_close() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"))
        .show(true)
        .on_close(TestMessage::Close);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let mut oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let overlay_node = oe.as_overlay_mut().layout(&(), Size::new(300.0, 200.0));
    let overlay_layout = Layout::new(&overlay_node);

    let mut messages: Vec<TestMessage> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut cb = clipboard::Null;

    oe.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        overlay_layout,
        mouse::Cursor::Unavailable,
        &(),
        &mut cb,
        &mut shell,
    );

    assert!(messages.is_empty());
}

#[test]
fn overlay_element_update_non_mouse_event_does_not_close() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"))
        .show(true)
        .on_close(TestMessage::Close);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let mut oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let overlay_node = oe.as_overlay_mut().layout(&(), Size::new(300.0, 200.0));
    let overlay_layout = Layout::new(&overlay_node);

    let mut messages: Vec<TestMessage> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut cb = clipboard::Null;

    oe.as_overlay_mut().update(
        &Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Escape),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        }),
        overlay_layout,
        mouse::Cursor::Available(Point::new(9999.0, 9999.0)),
        &(),
        &mut cb,
        &mut shell,
    );

    drop(shell);
    assert!(messages.is_empty());
}

#[test]
fn overlay_element_mouse_interaction_works() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay")).show(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let mut oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let overlay_node = oe.as_overlay_mut().layout(&(), Size::new(300.0, 200.0));
    let overlay_layout = Layout::new(&overlay_node);

    let interaction =
        oe.as_overlay().mouse_interaction(overlay_layout, mouse::Cursor::Unavailable, &());

    assert_eq!(interaction, mouse::Interaction::None);
}

#[test]
fn overlay_element_draw_works() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay")).show(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let mut oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let overlay_node = oe.as_overlay_mut().layout(&(), Size::new(300.0, 200.0));
    let overlay_layout = Layout::new(&overlay_node);

    let mut renderer = ();
    let theme = Theme::custom("test", Palette::DARK);
    let style = renderer::Style::default();

    oe.as_overlay().draw(&mut renderer, &theme, &style, overlay_layout, mouse::Cursor::Unavailable);

    // Draw should not panic
}

#[test]
fn overlay_element_update_on_close_without_capture_outside_click_does_not_capture() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"))
        .show(true)
        .on_close(TestMessage::Close)
        .capture_outside_click(false);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let mut oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let overlay_node = oe.as_overlay_mut().layout(&(), Size::new(300.0, 200.0));
    let overlay_layout = Layout::new(&overlay_node);

    let mut messages: Vec<TestMessage> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut cb = clipboard::Null;

    oe.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        overlay_layout,
        mouse::Cursor::Available(Point::new(9999.0, 9999.0)),
        &(),
        &mut cb,
        &mut shell,
    );

    let captured = shell.is_event_captured();
    drop(shell);
    assert_eq!(messages, vec![TestMessage::Close]);
    assert!(!captured);
}

#[test]
fn overlay_element_update_mouse_over_overlay_captures_even_when_outside_capture_is_disabled() {
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));
    let mut overlay = BelowOverlay::new(fixed_el(60.0, 24.0), fixed_el(80.0, 40.0))
        .show(true)
        .gap(4.0)
        .capture_outside_click(false);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );
    let layout = Layout::new(&node);
    let mut overlay_element =
        Widget::overlay(&mut overlay, &mut tree, layout, &(), &viewport, Vector::new(10.0, 20.0))
            .expect("shown overlay should exist");
    let overlay_node = overlay_element.as_overlay_mut().layout(&(), viewport.size());
    let overlay_layout = Layout::new(&overlay_node);

    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut clipboard = clipboard::Null;
    overlay_element.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(20.0, 50.0) }),
        overlay_layout,
        mouse::Cursor::Available(Point::new(20.0, 50.0)),
        &(),
        &mut clipboard,
        &mut shell,
    );

    assert!(shell.is_event_captured());
}

/// Verify that as_widget() cast works for Element usage
#[test]
fn below_overlay_as_element_widget() {
    let element: Element<'_, TestMessage, Theme> =
        BelowOverlay::new(label_default("content"), label_default("overlay")).into();
    let _ = element.as_widget();
}

// --- Widget trait delegation tests ---

#[test]
fn widget_size_delegates_to_content() {
    let overlay = BelowOverlay::new(label_el("content"), label_el("overlay"));
    assert_eq!(Widget::size(&overlay), Size::new(Length::Shrink, Length::Shrink));
}

#[test]
fn widget_size_hint_does_not_panic() {
    let overlay = BelowOverlay::new(label_el("content"), label_el("overlay"));
    let _hint = Widget::size_hint(&overlay);
}

#[test]
fn widget_diff_does_not_panic() {
    let overlay = BelowOverlay::new(label_el("content"), label_el("overlay"));
    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    Widget::diff(&overlay, &mut tree);
    assert_eq!(tree.children.len(), 2);
}

#[test]
fn widget_draw_renders_content_without_panic() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"));
    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let mut renderer = ();
    let theme = Theme::custom("test", Palette::DARK);
    let style = renderer::Style::default();

    Widget::draw(
        &overlay,
        &tree,
        &mut renderer,
        &theme,
        &style,
        layout,
        mouse::Cursor::Unavailable,
        &Rectangle::with_size(Size::new(300.0, 200.0)),
    );
}

#[test]
fn widget_update_captures_outside_mouse_events() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"))
        .show(true)
        .capture_outside_click(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let mut messages: Vec<TestMessage> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut cb = clipboard::Null;

    Widget::update(
        &mut overlay,
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        layout,
        mouse::Cursor::Available(Point::new(9999.0, 9999.0)),
        &(),
        &mut cb,
        &mut shell,
        &Rectangle::with_size(Size::new(300.0, 200.0)),
    );

    assert!(shell.is_event_captured());
}

#[test]
fn widget_update_does_not_capture_when_hidden() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"))
        .show(false)
        .capture_outside_click(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let mut messages: Vec<TestMessage> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut cb = clipboard::Null;

    Widget::update(
        &mut overlay,
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        layout,
        mouse::Cursor::Available(Point::new(9999.0, 9999.0)),
        &(),
        &mut cb,
        &mut shell,
        &Rectangle::with_size(Size::new(300.0, 200.0)),
    );

    assert!(!shell.is_event_captured());
}

#[test]
fn widget_update_does_not_capture_inside_target_or_keyboard_events() {
    let mut overlay = BelowOverlay::new(fixed_el(60.0, 24.0), label_el("overlay"))
        .show(true)
        .capture_outside_click(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );

    let mut messages: Vec<TestMessage> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut cb = clipboard::Null;
    Widget::update(
        &mut overlay,
        &mut tree,
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(10.0, 10.0) }),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(10.0, 10.0)),
        &(),
        &mut cb,
        &mut shell,
        &Rectangle::with_size(Size::new(300.0, 200.0)),
    );
    assert!(!shell.is_event_captured());

    Widget::update(
        &mut overlay,
        &mut tree,
        &Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
            modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
            physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Tab),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        }),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(9999.0, 9999.0)),
        &(),
        &mut cb,
        &mut shell,
        &Rectangle::with_size(Size::new(300.0, 200.0)),
    );
    assert!(!shell.is_event_captured());
}

#[test]
fn widget_mouse_interaction_delegates_to_content() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"));
    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let interaction = Widget::mouse_interaction(
        &overlay,
        &tree,
        layout,
        mouse::Cursor::Unavailable,
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        &(),
    );

    assert_eq!(interaction, mouse::Interaction::None);
}

// --- Overlay element edge case tests ---

#[test]
fn overlay_element_mouse_interaction_no_children_returns_none() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay")).show(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let empty_node = layout::Node::new(Size::new(300.0, 200.0));
    let empty_layout = Layout::new(&empty_node);

    let interaction =
        oe.as_overlay().mouse_interaction(empty_layout, mouse::Cursor::Unavailable, &());

    assert_eq!(interaction, mouse::Interaction::None);
}

#[test]
fn overlay_element_draw_no_children_returns_early() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay")).show(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let empty_node = layout::Node::new(Size::new(300.0, 200.0));
    let empty_layout = Layout::new(&empty_node);

    let mut renderer = ();
    let theme = Theme::custom("test", Palette::DARK);
    let style = renderer::Style::default();

    oe.as_overlay().draw(&mut renderer, &theme, &style, empty_layout, mouse::Cursor::Unavailable);
}

#[test]
fn overlay_element_update_no_children_returns_without_close_or_capture() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"))
        .show(true)
        .on_close(TestMessage::Close);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);
    let mut overlay_element = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("shown overlay should exist");
    let empty_node = layout::Node::new(Size::new(300.0, 200.0));
    let empty_layout = Layout::new(&empty_node);

    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut clipboard = clipboard::Null;
    overlay_element.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        empty_layout,
        mouse::Cursor::Available(Point::new(9999.0, 9999.0)),
        &(),
        &mut clipboard,
        &mut shell,
    );

    let captured = shell.is_event_captured();
    drop(shell);
    assert!(messages.is_empty());
    assert!(!captured);
}

#[test]
fn overlay_element_update_mouse_event_captures_outside_target_with_capture() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"))
        .show(true)
        .capture_outside_click(true);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let mut oe = oe;
    let overlay_node = oe.as_overlay_mut().layout(&(), Size::new(300.0, 200.0));
    let overlay_layout = Layout::new(&overlay_node);

    let mut messages: Vec<TestMessage> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut cb = clipboard::Null;

    oe.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(9999.0, 9999.0) }),
        overlay_layout,
        mouse::Cursor::Available(Point::new(9999.0, 9999.0)),
        &(),
        &mut cb,
        &mut shell,
    );

    assert!(shell.is_event_captured());
}

#[test]
fn overlay_element_update_mouse_event_does_not_capture_outside_target_without_capture() {
    let mut overlay = BelowOverlay::new(label_el("content"), label_el("overlay"))
        .show(true)
        .capture_outside_click(false);

    let mut tree = Tree::new(&overlay as &dyn Widget<TestMessage, Theme, ()>);
    let node = Widget::layout(
        &mut overlay,
        &mut tree,
        &(),
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );
    let layout = Layout::new(&node);

    let oe = Widget::overlay(
        &mut overlay,
        &mut tree,
        layout,
        &(),
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(0.0, 0.0),
    )
    .expect("overlay should be Some when show=true");

    let mut oe = oe;
    let overlay_node = oe.as_overlay_mut().layout(&(), Size::new(300.0, 200.0));
    let overlay_layout = Layout::new(&overlay_node);

    let mut messages: Vec<TestMessage> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut cb = clipboard::Null;

    oe.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(9999.0, 9999.0) }),
        overlay_layout,
        mouse::Cursor::Available(Point::new(9999.0, 9999.0)),
        &(),
        &mut cb,
        &mut shell,
    );

    assert!(!shell.is_event_captured());
}
