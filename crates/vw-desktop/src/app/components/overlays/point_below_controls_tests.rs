use super::*;
use iced::advanced::renderer::Headless;

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Input(String),
    Pick(u8),
    Close,
}

fn check(picker: bool) {
    let renderer = iced::futures::executor::block_on(iced::Renderer::new(
        iced::Font::DEFAULT,
        14.0.into(),
        Some("tiny-skia"),
    ))
    .expect("renderer");
    let control: Element<'_, Message> = if picker {
        iced::widget::pick_list([1_u8, 2], Some(1), Message::Pick).width(160).into()
    } else {
        iced::widget::text_input("width", "").on_input(Message::Input).width(160).into()
    };
    let mut element: Element<'_, Message> =
        PointBelowOverlay::new(iced::widget::Space::new().width(20).height(20), control)
            .show(true)
            .capture_keyboard()
            .on_close(Message::Close)
            .into();
    let mut tree = widget::Tree::new(&element);
    let bounds = Rectangle::with_size(Size::new(400.0, 300.0));
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, bounds.size()),
    );
    let mut popup = element
        .as_widget_mut()
        .overlay(&mut tree, Layout::new(&node), &renderer, &bounds, Vector::ZERO)
        .expect("popup");
    let layout = popup.as_overlay_mut().layout(&renderer, bounds.size());
    let cursor = mouse::Cursor::Available(layout.bounds().position() + Vector::new(10.0, 10.0));
    let mut messages = Vec::new();
    let mut clipboard = iced::advanced::clipboard::Null;
    for event in [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ] {
        popup.as_overlay_mut().update(
            &event,
            Layout::new(&layout),
            cursor,
            &renderer,
            &mut clipboard,
            &mut Shell::new(&mut messages),
        );
    }
    if picker {
        assert!(
            popup.as_overlay_mut().overlay(Layout::new(&layout), &renderer).is_some(),
            "pick list must expose its menu above the parent popup"
        );
    } else {
        let key = iced::keyboard::Key::Character("2".into());
        let event = Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Digit2),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::empty(),
            repeat: false,
            text: Some("2".into()),
        });
        popup.as_overlay_mut().update(
            &event,
            Layout::new(&layout),
            cursor,
            &renderer,
            &mut clipboard,
            &mut Shell::new(&mut messages),
        );
        assert!(
            messages.contains(&Message::Input("2".into())),
            "focused input must receive text before the parent captures it"
        );
    }
}
#[test]
fn editor_config_popup_exposes_nested_picker() {
    check(true);
}
#[test]
fn editor_config_popup_delivers_keyboard_to_focused_input() {
    check(false);
}
