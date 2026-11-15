//! Observe durable jobs independently of the lifetime of their dialog.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
use {
    vw_api_app_skills::repos_api::SkillReposQuery,
    vw_api_transport::id::TaskId,
};

pub(crate) use desktop_settings_skills::jobs_view::{Event, terminal};
pub(crate) fn message(e: Event) -> Message { Message::Settings(SettingsMessage::SkillsJobs(e)) }
pub(crate) fn submitted(app: &mut App, scope: SkillReposQuery, id: TaskId) -> Task<Message> {
    let input = super::update_input(app);
    let effect = desktop_settings_skills::update::jobs_view::submitted(&mut app.skills_settings.jobs, &input, scope, id);
    execute(app, effect)
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    use desktop_settings_skills::update::jobs_view::{update as reduce};
    let input = super::update_input(app);
    let effect = reduce(&mut app.skills_settings.jobs, &input, event);
    execute(app, effect)
}
fn execute(app: &mut App, effect: desktop_settings_skills::update::jobs_view::Effect) -> Task<Message> {
    use desktop_settings_skills::update::jobs_view::Effect;
    match effect {
        Effect::None => Task::none(),
        Effect::Error(error) => app.show_error_toast(error),
        Effect::Poll(g) => Task::perform(async move { tokio::time::sleep(std::time::Duration::from_secs(1)).await; g }, |g| message(Event::Poll(g))),
        Effect::Finished(success) => {
            let notice = if success { app.show_success_toast("技能任务已完成") } else { app.show_error_toast("技能任务已结束，请核对最终状态及目标目录") };
            app.skills_settings.install.inventory = None;
            let inventory = if app.skills_settings.install.open { super::install_view::update(app, super::install_view::Event::Refresh) } else { Task::none() };
            Task::batch([notice, inventory, super::refresh_task(app.project_path.clone())])
        }
        Effect::Request { g, scope, id, cancel } => {
            let token = app.apps_catalog.token.clone();
            Task::perform(crate::app::session_gateway::gateway_skills_job(scope, id, token, cancel),
                move |r| message(Event::Loaded(g, r)))
        }
    }
}
