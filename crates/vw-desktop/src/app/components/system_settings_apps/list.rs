pub(crate) fn view(
    state: &crate::app::state::AppsCatalogState,
    i18n: crate::app::i18n::I18n,
) -> iced::Element<'_, crate::app::Message> {
    iced::widget::responsive(move |size| {
        let projection = super::view_state(state);
        let details = desktop_settings_apps::system_settings_apps::list::selected_app(&projection)
            .map(|row| super::details::view(i18n, state, row));
        let catalog = desktop_settings_apps::system_settings_apps::list::view_with_updates(i18n,
            &projection,
            Some(&state.updates),
            size.width < 580.0,
            |title, description| {
                crate::app::components::system_settings_common::settings_group_heading_with_help(
                    i18n,
                    None,
                    i18n.__(title),
                    i18n.__(description),
                )
            },
            super::local::available(),
            None,
            None,
            details,
            super::domain_event,
        );
        iced::widget::container(catalog)
            .center_x(iced::Length::Fill)
            .height(iced::Length::Fill)
            .padding([16, 12])
            .into()
    })
    .into()
}
