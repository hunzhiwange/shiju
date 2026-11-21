//! Unmanaged imports use server selections and explicit application choices.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;

pub(crate) use desktop_settings_skills::import_modal::{Event};
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::SkillsImport(event))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    use desktop_settings_skills::update::import_modal::{Effect, update as reduce};
    let mut input = super::update_input(app);
    if crate::app::components::third_party_settings::is_visible(app)
        && app.apps_catalog.third_party_page == desktop_ui_controls::settings_navigation::ThirdPartyPage::Skills
    {
        input.targets = app.apps_catalog.third_party_skills.available_targets();
    }
    let effect = reduce(&mut app.skills_settings.import, &input, event);
    if matches!(effect, Effect::Refresh)
        && app.apps_catalog.third_party_skills.scope().is_some_and(|scope| {
            input.context.as_ref() == Some(&scope.context) && input.host == scope.host
                && input.project == scope.project && input.directory == scope.directory
        })
    {
        app.apps_catalog.third_party_skills.workspace.importable =
            desktop_settings_skills::third_party_add::import_count(&app.skills_settings.import, &input);
    }
    match effect {
        Effect::None => Task::none(),
        Effect::Error(error) => app.show_error_toast(error),
        Effect::Refresh => Task::batch([super::refresh_workspace(app), super::refresh_task(app.navigation_tabs.project_path.clone())]),
        Effect::Request { generation, project, host, apps, targets, selected } => {
            let token = app.apps_catalog.token.clone();
            Task::perform(async move { desktop_session::session_gateway::gateway_skills_import_request(crate::app::config::gateway_client_endpoint(),
                project, host, apps, selected, targets, token).await },
                move |(outcomes, result)| message(Event::Completed(generation, outcomes, result)))
        }
    }
}
