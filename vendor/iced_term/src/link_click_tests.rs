use super::*;
use alacritty_terminal::index::{Column, Line};
use iced_core::mouse::{Button::Left, Event::*};

#[test]
fn command_click_opens_at_release_and_dragging_cancels_the_open() {
    let term = Terminal::attach(1, crate::settings::Settings::default());
    let view = TerminalView { term: &term };
    let mut state = TerminalViewState::new(term.widget_id().clone());
    state.focus();
    state.keyboard_modifiers = Modifiers::COMMAND;
    let content = term.backend.renderable_content();
    let cell_width = f32::from(content.terminal_size.cell_width);
    let cell_height = f32::from(content.terminal_size.cell_height);
    let bounds =
        Rectangle::with_size(Size::new(30.0 * cell_width, 10.0 * cell_height));
    let press = Point::new(3.5 * cell_width, 2.5 * cell_height);
    let drag = Point::new(13.5 * cell_width, press.y);
    let mut messages = Vec::new();
    let mut shell = iced_core::Shell::new(&mut messages);
    let mut send = |event, position| {
        view.handle_mouse_event(
            &mut state,
            bounds,
            Some(position),
            &event,
            &mut shell,
        )
    };

    send(ButtonPressed(Left), press);
    let click = send(ButtonReleased(Left), press);
    assert!(matches!(
        click.as_slice(),
        [Command::ProcessLink(LinkAction::Open, point)]
            if *point == TerminalGridPoint::new(Line(2), Column(3))
    ));

    send(ButtonPressed(Left), press);
    send(CursorMoved { position: drag }, drag);
    send(CursorMoved { position: press }, press);
    assert!(send(ButtonReleased(Left), press).is_empty());
}
