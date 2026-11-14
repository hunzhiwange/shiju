//! Scoped update results; refresh errors never replace a completed operation.
use crate::app::{App, Message, message::SettingsMessage, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::vw_api_types::agent_apps_skills::{inventory_api::*, repos_api::SkillReposQuery, updates_api::*, uninstall_api::*};

#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub inventory: Option<SkillInventoryResponse>,
    pub updates: Option<CheckSkillUpdatesResponse>,
    pub outcome: Option<String>,
    pub backup: Option<UninstallSkillResponse>,
    pub error: Option<String>,
    generation: u64,
    context: Option<AgentAppsContext>,
    scope: Option<SkillReposQuery>,
    catalog: u64,
}
#[derive(Debug, Clone)]
pub(crate) enum Operation { Update(String), Uninstall(String) }
#[derive(Debug, Clone)]
pub(crate) enum Outcome { Updated(SkillUpdateTask), Uninstalled(String, UninstallSkillResponse) }
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open, Close, Refresh, Run(Operation),
    Finished(u64, Result<Outcome, String>),
    Loaded(u64, Result<SkillInventoryResponse, String>, Result<CheckSkillUpdatesResponse, String>),
}
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::SkillsUpdates(event)) }
impl State {
    fn loaded(&mut self, inventory: Result<SkillInventoryResponse, String>, updates: Result<CheckSkillUpdatesResponse, String>) {
        self.error = None;
        match inventory {
            Ok(data) if Some(&data.scope) == self.scope.as_ref() => self.inventory = Some(data),
            Ok(_) => self.error = Some("库存响应目标不匹配".into()),
            Err(e) => self.error = Some(format!("库存刷新失败：{e}")),
        }
        match updates {
            Ok(data) if Some(&data.scope) == self.scope.as_ref()
                && self.inventory.as_ref().is_some_and(|i| i.version == data.version) => self.updates = Some(data),
            Ok(_) => self.error = Some("更新响应目标或库存版本不匹配，请刷新".into()),
            Err(e) => self.error = Some(format!("更新检查失败：{e}")),
        }
    }
    fn finished(&mut self, result: Result<Outcome, String>) {
        self.outcome = Some(match result {
            Ok(Outcome::Updated(task)) if Some(&task.scope) == self.scope.as_ref() =>
                format!("更新任务：{:?}；需核对：{}；内容哈希：{}", task.phase, task.requires_reconciliation, task.content_hash.as_deref().unwrap_or("未提供")),
            Ok(Outcome::Uninstalled(id, response)) if Some(&response.scope) == self.scope.as_ref() => {
                if let Some(cache) = &mut self.updates { cache.updates.retain(|u| u.directory != id); }
                let result = format!("已卸载；清理未完成：{}", response.cleanup_incomplete);
                self.backup = Some(response);
                result
            }
            Ok(_) => "操作响应目标不匹配，请核对服务端状态".into(),
            Err(e) => format!("操作请求失败：{e}；正在重新查询实际状态"),
        });
        // Any mutation, including an ambiguous transport failure, invalidates revision-bound writes.
        self.inventory = None;
        self.updates = None;
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let s = &mut app.skills_settings.updates;
    if matches!(event, Event::Close) {
        if !s.busy { *s = State { generation: s.generation.wrapping_add(1), ..Default::default() }; }
        return Task::none();
    }
    if matches!(event, Event::Open) && !s.open {
        let Some(project) = app.project_id.clone() else { return app.show_error_toast("请先打开已登记的项目"); };
        *s = State { open: true, context: Some(context.clone()), catalog: app.apps_catalog.generation,
            scope: Some(SkillReposQuery { project_id: project.into(), target_host_id: app.apps_catalog.host.clone() }),
            generation: s.generation.wrapping_add(1), ..Default::default() };
    }
    if !s.open { return Task::none(); }
    if s.context.as_ref() != Some(&context) || s.catalog != app.apps_catalog.generation
        || s.scope.as_ref().is_none_or(|q| Some(q.project_id.as_ref()) != app.project_id.as_deref() || q.target_host_id != app.apps_catalog.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Task::none();
    }
    match event {
        Event::Loaded(g, inventory, updates) if g == s.generation && s.busy => {
            s.busy = false; s.loaded(inventory, updates);
            return match s.error.clone() { Some(e) => app.show_error_toast(e), None => Task::none() };
        }
        Event::Finished(g, result) if g == s.generation && s.busy => {
            s.busy = false; s.finished(result);
            return Task::batch(vec![update(app, Event::Refresh), super::refresh_task(app.project_path.clone())]);
        }
        Event::Loaded(..) | Event::Finished(..) => return Task::none(),
        _ if s.busy => return Task::none(),
        _ => {}
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        return app.show_error_toast("请先在应用设置中连接并授权目标主机");
    }
    let scope = s.scope.clone().expect("open scope");
    let operation = if let Event::Run(operation) = event {
        let Some(inventory) = &s.inventory else { return Task::none(); };
        let id = match &operation { Operation::Update(id) | Operation::Uninstall(id) => id };
        if !inventory.skills.iter().any(|skill| &skill.id == id) { return Task::none(); }
        let hash = match &operation {
            Operation::Update(id) => {
                let Some(cache) = &s.updates else { return Task::none(); };
                let Some(info) = cache.updates.iter().find(|u| &u.directory == id) else { return Task::none(); };
                let Some(hash) = info.current_hash.clone() else { return Task::none(); };
                Some(hash)
            }
            Operation::Uninstall(_) => None,
        };
        Some((operation, inventory.version, hash))
    } else { None };
    s.busy = true; s.error = None; s.generation = s.generation.wrapping_add(1);
    let g = s.generation;
    let token = app.apps_catalog.token.clone();
    if let Some(operation) = operation {
        s.backup = None; s.outcome = None;
        return Task::perform(crate::app::session_gateway::gateway_skills_mutation(scope, operation, token), move |r| message(Event::Finished(g, r)));
    }
    s.inventory = None; s.updates = None;
    Task::perform(async move {
        let inventory = client.agent_apps_skills_inventory(&scope, &token).await.map_err(|e| e.to_string());
        let updates = client.agent_apps_skills_check_updates(&CheckSkillUpdatesRequest { scope }, &token).await.map_err(|e| e.to_string());
        (inventory, updates)
    }, move |(i, u)| message(Event::Loaded(g, i, u)))
}

#[cfg(test)]
#[path = "updates_view_tests.rs"]
mod tests;
