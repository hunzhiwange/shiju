use super::help::view_overlays;
use crate::app::{App, Message};
use iced::{Element, widget::text};
fn keep_element(element: Element<'_, Message>) { std::hint::black_box(element); }
#[test]
fn view_overlays_builds_hidden_and_visible_help_states() {
    let mut app = App::new().0;

    keep_element(view_overlays(&app, text("dialog without help").into()));

    app.skills_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog behind help").into()));
}
