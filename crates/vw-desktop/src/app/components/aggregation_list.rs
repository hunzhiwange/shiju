use crate::app::Message;
use crate::app::message::settings::aggregation_list::Form;
pub(crate) fn overlay<'a>(form: &'a Form, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    iced::widget::stack![base, desktop_apps_ui::aggregation::list_view::overlay(form, iced::widget::Space::new().into()).map(crate::app::message::settings::aggregation_list::message)].into()
}
