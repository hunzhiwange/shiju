//! Backup selection is retained after conflicts and isolated by execution context.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
pub(crate) use desktop_settings_skills::update::backup_modal::Event;
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::SkillsBackups(event)) }

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    use desktop_settings_skills::update::backup_modal::{Effect, Readback, update as reduce};
    let input = super::update_input(app);
    let event = match event {
        Event::Completed(generation, Ok(Readback::List(list, version, saved)))
            if app.apps_catalog.third_party_skills.scope().is_some_and(|scope| {
                input.context.as_ref() == Some(&scope.context) && input.host == scope.host
                    && input.project == scope.project && input.directory == scope.directory
            }) =>
        {
            let page = &app.apps_catalog.third_party_skills;
            let available = if page.bindings.snapshot.is_some() {
                page.available_targets()
            } else {
                saved.into_iter().filter(|target| input.targets.contains(target)).collect()
            };
            Event::Completed(generation, Ok(Readback::List(list, version, available)))
        }
        event => event,
    };
    match reduce(&mut app.skills_settings.backups, &input, event) {
        Effect::None => Task::none(),
        Effect::Error(error) => app.show_message_toast(
            crate::app::state::ToastKind::Error,
            desktop_i18n::settings_extensions::feedback_message(&(error)),
        ),
        Effect::Changed { restored } => {
            let mut tasks = vec![
                app.show_message_toast(
                    crate::app::state::ToastKind::Success,
                    desktop_i18n::settings_extensions::feedback_message(&("备份操作完成")),
                ),
                update(app, Event::Refresh),
            ];
            if restored {
                tasks.push(super::refresh_workspace(app));
                let refresh_inventory = desktop_settings_skills::update::install_view::invalidate_inventory(&mut app.skills_settings.install);
                if refresh_inventory { tasks.push(super::install_view::update(app, super::install_view::Event::Refresh)); }
                tasks.push(super::refresh_task(app.navigation_tabs.project_path.clone()));
            }
            Task::batch(tasks)
        }
        Effect::Request { generation, scope, change } => {
            let token = app.apps_catalog.token.clone();
            Task::perform(async move { desktop_session::session_gateway::gateway_skill_backups_request(crate::app::config::gateway_client_endpoint(), scope, token, change).await }, move |result| message(Event::Completed(generation, result)))
        }
    }
}

#[cfg(test)]
pub(crate) use desktop_settings_skills::update::backup_modal::Change;
#[cfg(test)]
pub(crate) use desktop_settings_skills::update::backup_modal::Readback;
