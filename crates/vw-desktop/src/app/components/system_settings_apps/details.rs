use super::catalog;
use crate::app::{Message, state::AppsCatalogState};
use desktop_ui_controls::system_settings_common::settings_section_content;
use iced::{Element, widget::column};

pub(crate) fn view<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a AppsCatalogState,
    app: &'a catalog::CatalogRow,
) -> Element<'a, Message> {
    let mut project_profiles = column![];
    if let Some(scope) = vw_api_project_profiles::ProjectProfileScope::for_app(app.entry.id) {
        project_profiles = project_profiles.push(settings_section_content(
            crate::app::components::project_profile_selection::view(
                i18n,
                &state.project_profiles,
                scope,
            ),
        ));
    }

    desktop_settings_apps::system_settings_apps::details::view(
        i18n,
        &super::view_state(state),
        app,
        project_profiles.into(),
        super::uninstall::MANUAL_GUIDANCE,
        super::domain_event,
    )
}
