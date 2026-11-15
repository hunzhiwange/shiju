use crate::app::{Message, components::app_providers::{desktop::mapping_fields, list::Help},
    message::{app_providers::desktop::{Event as LegacyEvent, message as legacy}, settings::aggregation_desktop::message},
    state::app_providers::desktop::Form};
use iced::Element;
pub(crate) fn overlay<'a>(form: &'a Form, help: Help<'_>, base: Element<'a, Message>) -> Element<'a, Message> {
    let fields = if form.mapping_editable { mapping_fields(form, help) } else { vec![] };
    desktop_apps_ui::aggregation::desktop_view::overlay(form, fields, message, legacy(LegacyEvent::Cancel), base)
}
