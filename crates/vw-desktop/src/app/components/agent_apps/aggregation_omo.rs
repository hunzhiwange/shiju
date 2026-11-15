use crate::app::Message;
use crate::app::state::aggregation_omo::Form as Form;
use iced::Element;
pub(crate) fn overlay<'a>(form: &'a Form, base: Element<'a, Message>) -> Element<'a, Message> {
    iced::widget::stack![base, desktop_apps_ui::aggregation::omo_view::overlay(form, iced::widget::Space::new().into())
        .map(crate::app::message::settings::aggregation_omo::message)].into()
}
