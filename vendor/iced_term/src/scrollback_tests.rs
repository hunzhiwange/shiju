use super::*;
use crate::terminal::Command::ProxyToBackend;

#[test]
fn wheel_accumulates_fractional_rows_in_both_directions() {
    let mut state = TerminalViewState::new(iced::widget::Id::unique());
    let pixels = |y| ScrollDelta::Pixels { x: 0.0, y };
    let rows = |y| ScrollDelta::Lines { x: 0.0, y };
    assert_eq!(state.wheel_lines(pixels(5.0), 20.0), Some(0));
    assert_eq!(state.wheel_lines(pixels(15.0), 20.0), Some(1));
    assert_eq!(state.wheel_lines(rows(-0.5), 20.0), Some(0));
    assert_eq!(state.wheel_lines(pixels(-10.0), 20.0), Some(-1));
    assert_eq!(state.wheel_lines(rows(3.0), 20.0), Some(3));
    assert_eq!(state.wheel_lines(rows(0.0), 20.0), None);
}

#[test]
fn fullscreen_wheel_reports_scroll_events_to_the_application() {
    let mut term = Terminal::attach(1, crate::settings::Settings::default());
    term.feed_attached_output(
        b"\x1b[?1049h\x1b[?1002h\x1b[?1006h",
        false,
        false,
    );
    let size = term.backend.renderable_content().terminal_size;
    let position = (
        2.5 * f32::from(size.cell_width),
        1.5 * f32::from(size.cell_height),
    );

    for (lines, report) in
        [(2, "\x1b[<64;3;2M\x1b[<64;3;2M"), (-1, "\x1b[<65;3;2M")]
    {
        term.handle(ProxyToBackend(Command::WheelScroll {
            lines,
            modifiers: Modifiers::empty(),
            position,
        }));
        assert_eq!(term.take_attached_input().0, report.as_bytes());
    }

    term.feed_attached_output(b"\x1b[?1002l\x1b[?1l", false, false);
    term.handle(ProxyToBackend(Command::WheelScroll {
        lines: 1,
        modifiers: Modifiers::empty(),
        position,
    }));
    assert_eq!(term.take_attached_input().0, b"\x1b[A");

    term.feed_attached_output(b"\x1b[?1h", false, false);
    term.handle(ProxyToBackend(Command::WheelScroll {
        lines: -1,
        modifiers: Modifiers::empty(),
        position,
    }));
    assert_eq!(term.take_attached_input().0, b"\x1bOB");
}

#[test]
fn scrollbar_tracks_local_history_and_drags_from_bottom_to_top() {
    let mut term = Terminal::attach(1, crate::settings::Settings::default());
    let bounds =
        Rectangle::new(Point::new(10.0, 20.0), Size::new(500.0, 300.0));
    let mut state = TerminalViewState::new(term.widget_id().clone());
    term.handle(ProxyToBackend(Command::Resize(
        Some(text_bounds(bounds).size()),
        None,
    )));
    assert!(Scrollbar::new(bounds, term.backend.renderable_content()).is_none());
    term.feed_attached_output(&b"history\r\n".repeat(100), false, false);

    let scroll = Command::WheelScroll {
        lines: 3,
        modifiers: Modifiers::empty(),
        position: (20.0, 20.0),
    };
    term.handle(ProxyToBackend(scroll));
    assert_eq!(term.backend.renderable_content().grid.display_offset(), 3);
    assert!(term.take_attached_input().0.is_empty());
    term.handle(ProxyToBackend(Command::ScrollTo(0)));

    let bar =
        Scrollbar::new(bounds, term.backend.renderable_content()).unwrap();
    let thumb_bottom = bar.thumb.y + bar.thumb.height;
    assert!((thumb_bottom - bar.track.y - bar.track.height).abs() < 0.01);
    let bottom = bar.thumb.center();
    let top = Point::new(bottom.x, bar.track.y);
    for (event, cursor) in [
        (mouse::Event::ButtonPressed(mouse::Button::Left), bottom),
        (mouse::Event::CursorMoved { position: top }, top),
        (mouse::Event::ButtonReleased(mouse::Button::Left), top),
    ] {
        let mut messages = Vec::new();
        let mut shell = iced_core::Shell::new(&mut messages);
        assert!(TerminalView { term: &term }.handle_scrollbar(
            &mut state,
            bounds,
            Some(cursor),
            &event,
            &mut shell,
        ));
        assert!(shell.is_event_captured());
        for Event::BackendCall(_, cmd) in messages {
            term.handle(ProxyToBackend(cmd));
        }
    }
    let content = term.backend.renderable_content();
    assert_eq!(content.grid.display_offset(), content.grid.history_size());
    assert!(state.scrollbar_grab.is_none());
    assert!(!term.has_selection());
}
