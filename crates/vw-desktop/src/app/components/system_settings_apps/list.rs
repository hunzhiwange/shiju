pub(crate) fn view(
    state: &crate::app::state::AppsCatalogState,
    i18n: crate::app::i18n::I18n,
) -> iced::Element<'_, crate::app::Message> {
    let projection = super::view_state(state);
    let details = desktop_settings_apps::system_settings_apps::list::selected_app(&projection)
        .map(|row| super::details::view(state, row));
    desktop_settings_apps::system_settings_apps::list::view_with_updates(
        &projection,
        Some(&state.updates),
        |title, description| {
            crate::app::components::system_settings_common::settings_group_heading_with_help(
                i18n, None, i18n.__(title), i18n.__(description),
            )
        },
        super::local::available(),
        iced::widget::Space::new().into(),
        crate::app::components::app_routing::lifecycle::view(state),
        details,
        super::domain_event,
    )
}
