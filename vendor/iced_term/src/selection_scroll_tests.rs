use super::*;
use crate::terminal::Command::ProxyToBackend;
use iced::advanced::renderer::Headless;
use iced_core::clipboard;
use iced_core::layout::Node;
use iced_core::mouse::Button::Left;
use iced_core::window::RedrawRequest;

#[tokio::test]
async fn dragging_at_either_edge_scrolls_and_extends_the_selection_until_release(
) {
    let renderer = <iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .expect("tiny-skia renderer");

    for (initial_offset, direction, selected_row, mouse_tracking) in [
        (0, 1, "row-07", false),
        (3, -1, "row-10", false),
        (0, 1, "row-07", true),
    ]
    {
        let mut term =
            Terminal::attach(1, crate::settings::Settings::default());
        let cell_width = f32::from(term.font.measure.width as u16);
        let cell_height = f32::from(term.font.measure.height as u16);
        let size = Size::new(18.0 * cell_width, 4.0 * cell_height);
        let origin = Point::new(30.0, 40.0);
        let node = Node::new(size).move_to(origin);
        let layout = iced_core::Layout::new(&node);
        let mut tree =
            Tree::new(Element::from(TerminalView { term: &term }).as_widget());
        term.handle(ProxyToBackend(Command::Resize(Some(size), None)));
        let output = (0..12)
            .map(|row| format!("row-{row:02}"))
            .collect::<Vec<_>>()
            .join("\r\n");
        term.feed_attached_output(output.as_bytes(), false, false);
        term.handle(ProxyToBackend(Command::Scroll(initial_offset)));
        if mouse_tracking {
            term.feed_attached_output(b"\x1b[?1002h", false, false);
        }

        let mut dispatch = |term: &mut Terminal, event, cursor| {
            let mut messages = Vec::new();
            let mut shell = iced_core::Shell::new(&mut messages);
            TerminalView { term }.update(
                &mut tree,
                &event,
                layout,
                cursor,
                &renderer,
                &mut clipboard::Null,
                &mut shell,
                &layout.bounds(),
            );
            let redraw = shell.redraw_request();
            for Event::BackendCall(_, command) in messages {
                term.handle(ProxyToBackend(command));
            }
            redraw
        };

        let x = origin.x + 10.5 * cell_width;
        let start = Point::new(x, origin.y + 2.5 * cell_height);
        if mouse_tracking {
            dispatch(
                &mut term,
                iced::Event::Keyboard(KeyEvent::ModifiersChanged(Modifiers::SHIFT)),
                Cursor::Available(start),
            );
        }
        dispatch(
            &mut term,
            iced::Event::Mouse(mouse::Event::ButtonPressed(Left)),
            Cursor::Available(start),
        );
        if mouse_tracking {
            dispatch(
                &mut term,
                iced::Event::Keyboard(KeyEvent::ModifiersChanged(Modifiers::empty())),
                Cursor::Available(start),
            );
        }
        let edge_y = if direction > 0 {
            origin.y
        } else {
            origin.y + size.height
        };
        let edge = Point::new(x, edge_y);
        let RedrawRequest::At(first_tick) = dispatch(
            &mut term,
            iced::Event::Mouse(mouse::Event::CursorMoved { position: edge }),
            Cursor::Available(edge),
        ) else {
            panic!("dragging to the edge should schedule scrolling");
        };
        let outside = Point::new(x, edge_y - direction as f32 * cell_height);
        assert_eq!(
            dispatch(
                &mut term,
                iced::Event::Mouse(mouse::Event::CursorMoved {
                    position: outside
                }),
                Cursor::Unavailable,
            ),
            RedrawRequest::At(first_tick),
        );

        let tick = |now| {
            iced::Event::Window(iced::window::Event::RedrawRequested(now))
        };
        dispatch(&mut term, tick(first_tick), Cursor::Unavailable);
        dispatch(
            &mut term,
            tick(first_tick + SELECTION_SCROLL_INTERVAL),
            Cursor::Unavailable,
        );
        let offset = |term: &Terminal| {
            term.backend.renderable_content().grid.display_offset()
        };
        assert_eq!(offset(&term), (initial_offset + 2 * direction) as usize);
        let selected = term.selected_text();
        assert!(selected.contains(selected_row), "{selected:?}");

        dispatch(
            &mut term,
            iced::Event::Mouse(mouse::Event::ButtonReleased(Left)),
            Cursor::Unavailable,
        );
        assert_eq!(
            dispatch(
                &mut term,
                tick(first_tick + 2 * SELECTION_SCROLL_INTERVAL),
                Cursor::Unavailable
            ),
            RedrawRequest::Wait,
        );
        dispatch(
            &mut term,
            iced::Event::Mouse(mouse::Event::CursorMoved { position: start }),
            Cursor::Available(start),
        );
        assert_eq!(offset(&term), (initial_offset + 2 * direction) as usize);
        assert_eq!(term.selected_text(), selected);
        assert!(term.take_attached_input().0.is_empty());
    }
}
