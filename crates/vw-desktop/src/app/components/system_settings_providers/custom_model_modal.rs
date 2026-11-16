use crate::app::{App, Message};
use iced::Element;
use crate::app::components::system_settings_models::{map_view_message, view_style};
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    desktop_settings_providers::custom_model_modal::view_overlays(
        app.provider_settings.custom_model_modal.as_ref(),
        app.provider_settings.save_error.as_ref(),
        view_style(app),
        dialog,
        map_view_message,
    )
}
#[cfg(test)]
#[path = "custom_model_modal_ui_refresh_09_tests.rs"]
mod ui_refresh_09_tests;

pub(super) use desktop_settings_providers::custom_model_modal::{custom_modal_card_style, custom_segment_style, stacked_field_row};
pub(super) fn responsive_field_with_help<'a, Control>(
    app: &'a App, label: &'a str, description: impl Into<std::borrow::Cow<'a,str>>,
    control: impl Fn() -> Control + 'a,
) -> Element<'a,Message>
where Control: Into<Element<'a,Message>> {
    use crate::app::components::system_settings_common::{settings_hover_help_label, settings_field_label};
    desktop_settings_providers::custom_model_modal::responsive_field(description, control, move |description, wide| {
        let label = settings_hover_help_label(crate::app::i18n::I18n::new(app.general_settings.desktop_locale), &app.appearance.app_theme, label, settings_field_label(label), description);
        if wide { label.width(iced::Length::Fill).wrap().into() } else { label.wrap().into() }
    })
}
