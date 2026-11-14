//! Observe durable jobs independently of the lifetime of their dialog.
use crate::app::{App, Message, message::SettingsMessage, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::vw_api_types::{agent_apps_skills::{jobs_api::*, repos_api::SkillReposQuery}, id::TaskId};

#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub snapshot: Option<SkillJobResponse>,
    pub error: Option<String>,
    pub task_id: Option<TaskId>,
    context: Option<AgentAppsContext>,
    scope: Option<SkillReposQuery>,
    generation: u64,
    notified: bool,
}
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open, Close, Refresh, Cancel,
    Loaded(u64, Result<SkillJobResponse, String>),
    Poll(u64),
}
pub(crate) fn message(e: Event) -> Message { Message::Settings(SettingsMessage::SkillsJobs(e)) }
pub(crate) fn terminal(phase: SkillJobPhase) -> bool {
    matches!(phase, SkillJobPhase::Completed | SkillJobPhase::Failed | SkillJobPhase::Interrupted)
}
pub(crate) fn submitted(app: &mut App, scope: SkillReposQuery, id: TaskId) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let generation = app.skills_settings.jobs.generation.wrapping_add(1);
    app.skills_settings.jobs = State { open: true, task_id: Some(id), scope: Some(scope),
        context: Some(AgentAppsContext { gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() }),
        generation, ..Default::default() };
    update(app, Event::Refresh)
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let s = &mut app.skills_settings.jobs;
    if matches!(event, Event::Close) { s.open = false; return Task::none(); }
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    if s.context.as_ref() != Some(&context) || s.scope.as_ref().is_none_or(|scope|
        Some(scope.project_id.as_ref()) != app.project_id.as_deref() || scope.target_host_id != app.apps_catalog.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Task::none();
    }
    match event {
        Event::Loaded(g, result) if g == s.generation && s.busy => {
            s.busy = false;
            match result {
                Ok(job) if Some(&job.scope) == s.scope.as_ref() && Some(&job.task_id) == s.task_id.as_ref() => {
                    let done = terminal(job.phase);
                    let success = job.phase == SkillJobPhase::Completed && !job.requires_reconciliation;
                    s.snapshot = Some(job); s.error = None;
                    if done && !s.notified {
                        s.notified = true;
                        let notice = if success { app.show_toast(crate::app::state::ToastKind::Success, "技能任务已完成") }
                            else { app.show_error_toast("技能任务已结束，请核对最终状态及目标目录") };
                        // Failed or interrupted tasks may also have published partial results.
                        app.skills_settings.install.inventory = None;
                        let inventory = if app.skills_settings.install.open {
                            super::install_view::update(app, super::install_view::Event::Refresh)
                        } else { Task::none() };
                        return Task::batch([notice, inventory, super::refresh_task(app.project_path.clone())]);
                    }
                    if !done {
                        return Task::perform(async move { tokio::time::sleep(std::time::Duration::from_secs(1)).await; g },
                            |g| message(Event::Poll(g)));
                    }
                }
                Ok(_) => s.error = Some("任务响应目标不匹配".into()),
                Err(e) => { s.error = Some(e.clone()); return app.show_error_toast(e); }
            }
            return Task::none();
        }
        Event::Loaded(..) => return Task::none(),
        Event::Poll(g) if g != s.generation => return Task::none(),
        Event::Open => s.open = true,
        _ => {}
    }
    if s.busy { return Task::none(); }
    let cancel = matches!(event, Event::Cancel);
    if cancel && s.snapshot.as_ref().is_none_or(|job| terminal(job.phase) || job.phase == SkillJobPhase::Committing) {
        return Task::none();
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        s.error = Some("请重新连接并授权目标主机后刷新".into()); return Task::none();
    }
    let (Some(scope), Some(id)) = (s.scope.clone(), s.task_id.clone()) else { return Task::none(); };
    s.busy = true; s.generation = s.generation.wrapping_add(1);
    let g = s.generation;
    let token = app.apps_catalog.token.clone();
    Task::perform(crate::app::session_gateway::gateway_skills_job(scope, id, token, cancel),
        move |r| message(Event::Loaded(g, r)))
}
