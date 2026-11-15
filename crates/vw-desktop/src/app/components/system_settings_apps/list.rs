#[path = "domains.rs"] mod domains;
pub(crate) fn view(state: &crate::app::state::AppsCatalogState, i18n: crate::app::i18n::I18n) -> iced::Element<'_, crate::app::Message> {
    let details = state.rows.iter().find(|row| Some(row.entry.id) == state.expanded).map(|row| super::details::view(state, row));
    desktop_settings_apps::system_settings_apps::list::view(&super::view_state(state), |title, description| crate::app::components::system_settings_common::settings_group_heading_with_help(i18n, None, i18n.__(title), i18n.__(description)), super::local::available(), domains::view(state), crate::app::components::app_routing::lifecycle::view(state), details, super::domain_event)
}
