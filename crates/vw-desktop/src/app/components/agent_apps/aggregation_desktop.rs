use crate::app::{
    Message,
    components::app_providers::list::Help,
    message::{
        app_providers::desktop::{Event as LegacyEvent, message as legacy},
        settings::aggregation_desktop::message,
    },
    state::app_providers::desktop::Form,
};
use iced::Element;
pub(crate) fn overlay<'a>(
    form: &'a Form,
    help: Help<'_>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let fields = desktop_provider_apps::components::desktop::mapping_fields(form, help.provider_help())
        .into_iter().map(crate::app::components::app_providers::edit::map).collect();
    desktop_apps_ui::aggregation::desktop_view::overlay(
        form,
        fields,
        message,
        legacy(LegacyEvent::Cancel),
        base,
    )
}
