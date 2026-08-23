//! 验证上方覆盖层的位置计算、边界修正与组件构建。
//! 视口边缘和空间不足场景用于确认覆盖内容不会被放到可见区域之外。

use super::{AboveOverlay, PointAboveOverlay};
use iced::widget::text;
use iced::{Element, Point, Theme};

#[derive(Debug, Clone, PartialEq)]
enum TestMessage {
    Close,
    ReplaceClose,
}

fn label(value: &'static str) -> Element<'static, TestMessage> {
    text(value).into()
}

#[test]
fn above_overlay_new_sets_hidden_defaults() {
    let overlay = AboveOverlay::new(label("content"), label("overlay"));

    assert!(!overlay.show);
    assert_eq!(overlay.gap, 0.0);
    assert!(overlay.snap_within_viewport);
    assert!(overlay.on_close.is_none());
    assert!(!overlay.block_background_events);
}

#[test]
fn above_overlay_builder_updates_all_runtime_flags() {
    let overlay = AboveOverlay::new(label("content"), label("overlay"))
        .show(true)
        .gap(7.5)
        .snap_within_viewport(false)
        .block_background_events(true)
        .on_close(TestMessage::Close);

    assert!(overlay.show);
    assert_eq!(overlay.gap, 7.5);
    assert!(!overlay.snap_within_viewport);
    assert!(overlay.block_background_events);
    assert_eq!(overlay.on_close, Some(TestMessage::Close));
}

#[test]
fn above_overlay_repeated_builders_keep_latest_values() {
    let overlay = AboveOverlay::new(label("content"), label("overlay"))
        .show(true)
        .show(false)
        .gap(7.5)
        .gap(-2.5)
        .snap_within_viewport(false)
        .snap_within_viewport(true)
        .on_close(TestMessage::Close)
        .on_close(TestMessage::ReplaceClose);

    assert!(!overlay.show);
    assert_eq!(overlay.gap, -2.5);
    assert!(overlay.snap_within_viewport);
    assert_eq!(overlay.on_close, Some(TestMessage::ReplaceClose));
}

#[test]
fn point_above_overlay_new_sets_hidden_defaults() {
    let overlay = PointAboveOverlay::new(label("content"), label("overlay"));

    assert!(!overlay.show);
    assert_eq!(overlay.anchor, Point::ORIGIN);
    assert_eq!(overlay.gap, 0.0);
    assert!(overlay.snap_within_viewport);
    assert!(overlay.on_close.is_none());
}

#[test]
fn point_above_overlay_builder_updates_anchor_and_flags() {
    let overlay = PointAboveOverlay::new(label("content"), label("overlay"))
        .show(true)
        .anchor(Point::new(12.0, 18.0))
        .gap(4.0)
        .snap_within_viewport(false)
        .on_close(TestMessage::Close);

    assert!(overlay.show);
    assert_eq!(overlay.anchor, Point::new(12.0, 18.0));
    assert_eq!(overlay.gap, 4.0);
    assert!(!overlay.snap_within_viewport);
    assert_eq!(overlay.on_close, Some(TestMessage::Close));
}

#[test]
fn point_above_overlay_preserves_negative_anchor_and_gap() {
    let overlay = PointAboveOverlay::new(label("content"), label("overlay"))
        .anchor(Point::new(-12.0, -18.0))
        .gap(-4.0);

    assert_eq!(overlay.anchor, Point::new(-12.0, -18.0));
    assert_eq!(overlay.gap, -4.0);
}

#[test]
fn above_overlays_convert_to_elements() {
    let _: Element<'static, TestMessage, Theme> =
        AboveOverlay::new(label("content"), label("overlay")).into();
    let _: Element<'static, TestMessage, Theme> =
        PointAboveOverlay::new(label("content"), label("overlay")).into();
}
