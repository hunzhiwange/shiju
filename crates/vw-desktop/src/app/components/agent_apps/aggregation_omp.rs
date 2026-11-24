use crate::app::Message;
use crate::app::message::settings::aggregation_omp::Form as Form;
use iced::Element;
pub(crate) fn overlay<'a>(i18n: desktop_i18n::I18n, form: &'a Form, base: Element<'a, Message>) -> Element<'a, Message> {
    iced::widget::stack![base, desktop_apps_ui::aggregation::omp_view::overlay(i18n, form, iced::widget::Space::new().into())
        .map(crate::app::message::settings::aggregation_omp::message)].into()
}
