use crate::app::{App, Message};
use crate::app::message::app_providers::list::{self, ListMessage};
use iced::Task;
pub use desktop_settings_apps::aggregation::pi::*;
use vw_api_app_identity::AppId;
use vw_api_transport::id::ProjectId;
pub(crate) fn message(event: Event) -> Message { list::message(ListMessage::PiDirectory(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::None) { return Task::none(); }
    let Some(scope) = app.app_providers.scope.clone().filter(|s| s.app == AppId::Pi) else {
        return Task::none();
    };
    if matches!(event, Event::Close) {
        if app.app_providers.pi_directory.as_ref().is_some_and(|form| form.pending.is_none()) {
            app.app_providers.pi_directory = None;
        }
        return Task::none();
    }
    let Ok((client, actual)) = list::current(app, AppId::Pi) else {
        app.app_providers.pi_directory = None;
        return app.show_error_toast("无法连接网关");
    };
    if scope != actual {
        app.app_providers.pi_directory = None;
        return Task::none();
    }
    let Some(project) = app.workspace_state.project_id.as_deref().filter(|id| !id.is_empty()).map(ProjectId::from)
    else {
        app.app_providers.pi_directory = None;
        return app.show_error_toast("请先打开已注册的项目");
    };
    let access = app.apps_catalog.generation;
    let mut notice = None;
    let task = desktop_settings_apps::aggregation::pi::update(&mut app.app_providers.pi_directory, &mut app.app_providers.generation, app.app_providers.loading, app.app_providers.form.is_some(), app.app_providers.editor.is_some(), client, scope, project, access, &mut notice, event).map(message);
    let feedback = match notice {
        Some(desktop_settings_apps::aggregation::Notice::Error(s)) => app.show_error_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Info(s)) => app.show_info_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Success(s)) => app.show_success_toast(s),
        Some(desktop_settings_apps::aggregation::Notice::Warning(s)) => app.show_warning_toast(s),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}
