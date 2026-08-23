//! 验证颜色选择器对指针边界、拖动生命周期和数值映射的处理。
//! 按下必须发生在边界内才启动拖动，释放时发布当前值并复位内部状态。

use std::cell::RefCell;
use std::rc::Rc;

use iced::mouse;
use iced::widget::canvas::{Event, Program};
use iced::{Point, Rectangle};

use super::{
    AlphaPicker, AlphaState, HuePicker, HueState, SaturationValuePicker, SaturationValueState,
    cursor_position_inclusive,
};
use crate::app::Message;
use crate::app::components::color_picker::Hsv;

// 非零原点可同时发现坐标未平移和边界宽高映射错误。
fn bounds() -> Rectangle {
    Rectangle::new(Point::new(10.0, 20.0), iced::Size::new(200.0, 100.0))
}

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "expected {expected}, got {actual}");
}

#[test]
fn cursor_position_inclusive_accepts_edges_and_rejects_unavailable_or_outside_cursor() {
    assert_eq!(
        cursor_position_inclusive(mouse::Cursor::Available(Point::new(10.0, 20.0)), bounds()),
        Some(Point::ORIGIN)
    );
    assert_eq!(
        cursor_position_inclusive(mouse::Cursor::Available(Point::new(210.0, 120.0)), bounds()),
        Some(Point::new(200.0, 100.0))
    );
    assert_eq!(cursor_position_inclusive(mouse::Cursor::Unavailable, bounds()), None);
    assert_eq!(
        cursor_position_inclusive(mouse::Cursor::Available(Point::new(9.9, 20.0)), bounds()),
        None
    );
    assert_eq!(
        cursor_position_inclusive(mouse::Cursor::Available(Point::new(210.1, 120.0)), bounds()),
        None
    );
}

#[test]
fn saturation_value_picker_publishes_press_drag_and_release_values() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let picker = SaturationValuePicker {
        hsv: Hsv { h: 120.0, s: 0.25, v: 0.75 },
        on_change: Box::new({
            let observed = observed.clone();
            move |hsv| {
                observed.borrow_mut().push(hsv);
                Message::None
            }
        }),
    };
    let mut state = SaturationValueState::default();

    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(60.0, 45.0)),
            )
            .is_some()
    );
    assert!(state.is_dragging);

    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(210.0, 120.0) }),
                bounds(),
                mouse::Cursor::Available(Point::new(210.0, 120.0)),
            )
            .is_some()
    );
    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(210.0, 120.0)),
            )
            .is_some()
    );
    assert!(!state.is_dragging);

    let observed = observed.borrow();
    assert_eq!(observed.len(), 3);
    assert_close(observed[0].h, 120.0);
    assert_close(observed[0].s, 0.25);
    assert_close(observed[0].v, 0.75);
    assert_close(observed[1].s, 1.0);
    assert_close(observed[1].v, 0.0);
    assert_close(observed[2].s, 0.25);
    assert_close(observed[2].v, 0.75);
}

#[test]
fn saturation_value_picker_ignores_events_without_cursor_inside_bounds() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let picker = SaturationValuePicker {
        hsv: Hsv { h: 10.0, s: 0.2, v: 0.3 },
        on_change: Box::new({
            let observed = observed.clone();
            move |hsv| {
                observed.borrow_mut().push(hsv);
                Message::None
            }
        }),
    };
    let mut state = SaturationValueState::default();

    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Unavailable,
            )
            .is_none()
    );
    assert!(
        picker
            .update(
                &mut state,
                &Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter),
                    modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter),
                    physical_key: iced::keyboard::key::Physical::Unidentified(
                        iced::keyboard::key::NativeCode::Unidentified,
                    ),
                    location: iced::keyboard::Location::Standard,
                    modifiers: iced::keyboard::Modifiers::default(),
                    text: None,
                    repeat: false,
                }),
                bounds(),
                mouse::Cursor::Available(Point::new(20.0, 30.0)),
            )
            .is_none()
    );
    assert!(observed.borrow().is_empty());
    assert!(!state.is_dragging);
}

#[test]
fn hue_picker_only_drags_after_left_press_and_maps_full_width_to_360_degrees() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let picker = HuePicker {
        hsv: Hsv { h: 45.0, s: 0.5, v: 0.8 },
        on_change: Box::new({
            let observed = observed.clone();
            move |hsv| {
                observed.borrow_mut().push(hsv);
                Message::None
            }
        }),
    };
    let mut state = HueState::default();

    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(110.0, 40.0) }),
                bounds(),
                mouse::Cursor::Available(Point::new(110.0, 40.0)),
            )
            .is_none()
    );
    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
                bounds(),
                mouse::Cursor::Available(Point::new(110.0, 40.0)),
            )
            .is_none()
    );
    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(210.0, 40.0)),
            )
            .is_some()
    );
    assert_close(observed.borrow()[0].h, 360.0);
    assert_close(observed.borrow()[0].s, 0.5);
    assert_close(observed.borrow()[0].v, 0.8);
}

#[test]
fn hue_picker_publishes_drag_updates_and_current_hue_on_release() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let picker = HuePicker {
        hsv: Hsv { h: 45.0, s: 0.5, v: 0.8 },
        on_change: Box::new({
            let observed = observed.clone();
            move |hsv| {
                observed.borrow_mut().push(hsv);
                Message::None
            }
        }),
    };
    let mut state = HueState::default();

    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(10.0, 40.0)),
            )
            .is_some()
    );
    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(110.0, 40.0) }),
                bounds(),
                mouse::Cursor::Available(Point::new(110.0, 40.0)),
            )
            .is_some()
    );
    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(110.0, 40.0)),
            )
            .is_some()
    );

    let observed = observed.borrow();
    assert_eq!(observed.len(), 3);
    assert_close(observed[0].h, 0.0);
    assert_close(observed[1].h, 180.0);
    assert_close(observed[2].h, 45.0);
    assert!(!state.is_dragging);
}

#[test]
fn alpha_picker_rejects_outside_events_and_restores_current_alpha_on_release() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let picker = AlphaPicker {
        rgb: iced::Color::from_rgb(0.2, 0.4, 0.6),
        alpha: 0.35,
        on_change: Box::new({
            let observed = observed.clone();
            move |alpha| {
                observed.borrow_mut().push(alpha);
                Message::None
            }
        }),
    };
    let mut state = AlphaState::default();

    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(220.0, 40.0)),
            )
            .is_none()
    );
    assert!(!state.is_dragging);
    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(110.0, 40.0)),
            )
            .is_none()
    );

    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(110.0, 40.0)),
            )
            .is_some()
    );
    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(110.0, 40.0)),
            )
            .is_some()
    );

    let observed = observed.borrow();
    assert_eq!(observed.len(), 2);
    assert_close(observed[0], 0.5);
    assert_close(observed[1], 0.35);
}

#[test]
fn alpha_picker_publishes_drag_updates_only_while_dragging() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let picker = AlphaPicker {
        rgb: iced::Color::from_rgb(0.2, 0.4, 0.6),
        alpha: 0.25,
        on_change: Box::new({
            let observed = observed.clone();
            move |alpha| {
                observed.borrow_mut().push(alpha);
                Message::None
            }
        }),
    };
    let mut state = AlphaState::default();

    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(60.0, 40.0) }),
                bounds(),
                mouse::Cursor::Available(Point::new(60.0, 40.0)),
            )
            .is_none()
    );
    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds(),
                mouse::Cursor::Available(Point::new(10.0, 40.0)),
            )
            .is_some()
    );
    assert!(
        picker
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(210.0, 40.0) }),
                bounds(),
                mouse::Cursor::Available(Point::new(210.0, 40.0)),
            )
            .is_some()
    );

    let observed = observed.borrow();
    assert_eq!(observed.len(), 2);
    assert_close(observed[0], 0.0);
    assert_close(observed[1], 1.0);
}
