use crate::help::view_overlays;
use iced::{Element, widget::text};
#[derive(Clone)]
enum Message {
    None,
}
fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}
#[test]
fn view_overlays_builds_hidden_and_visible_help_states() {
    keep_element(view_overlays(
        false,
        Default::default(),
        None,
        text("dialog without help").into(),
        |_| Message::None,
        Message::None,
    ));

    keep_element(view_overlays(
        true,
        Default::default(),
        None,
        text("dialog behind help").into(),
        |_| Message::None,
        Message::None,
    ));
}
