//! Backup selection is retained after conflicts and isolated by execution context.
use crate::app::{App, Message, message::SettingsMessage, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::vw_api_types::agent_apps_skills::{backups_api::*, bindings_api::SkillBindingTarget, repos_api::SkillReposQuery};
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub open: bool, pub busy: bool,
    pub backups: Vec<SkillBackup>, pub selected: Option<String>,
    pub available: Vec<SkillBindingTarget>, pub targets: Vec<SkillBindingTarget>,
    pub version: Option<u64>, pub error: Option<String>,
    generation: u64, catalog: u64, context: Option<AgentAppsContext>, scope: Option<SkillReposQuery>,
}
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Open, Close, Refresh, Select(String), Target(SkillBindingTarget, bool), Delete, Restore,
    Completed(u64, Result<Readback, String>),
}
#[derive(Clone, Debug)]
pub(crate) enum Readback {
    List(SkillBackupsResponse, u64, Vec<SkillBindingTarget>),
    Deleted(DeleteSkillBackupResponse), Restored(RestoreSkillBackupResponse),
}
pub(crate) enum Change { Read, Delete(String), Restore(String, RestoreSkillBackupRequest) }
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::SkillsBackups(event)) }
impl State {
    fn accept(&mut self, result: Readback) -> Result<(), String> {
        match result {
            Readback::List(value, version, available) if Some(&value.scope) == self.scope.as_ref() => {
                self.backups = value.backups;
                self.version = Some(version);
                self.targets.retain(|target| available.contains(target));
                self.available = available;
                if self.selected.as_ref().is_some_and(|id| !self.backups.iter().any(|item| &item.backup_id == id)) { self.selected = None; }
            }
            Readback::Deleted(value) if Some(&value.scope) == self.scope.as_ref() && self.selected.as_ref() == Some(&value.backup_id) => {
                self.backups.retain(|item| item.backup_id != value.backup_id);
                self.selected = None;
            }
            Readback::Restored(value) if Some(&value.scope) == self.scope.as_ref() && self.selected.as_ref() == Some(&value.backup_id) => {
                self.version = Some(value.version);
                // Restore preserves the backup on the execution host.
            }
            _ => return Err("备份响应作用域或身份不匹配，请刷新".into()),
        }
        Ok(())
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let s = &mut app.skills_settings.backups;
    if matches!(event, Event::Close) {
        if !s.busy { *s = State { generation: s.generation.wrapping_add(1), ..Default::default() }; }
        return Task::none();
    }
    let Ok(client) = crate::app::config::gateway_client() else { return app.show_error_toast("无法连接备份服务"); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    if matches!(event, Event::Open) {
        if s.open { return Task::none(); }
        let Some(project) = app.project_id.clone() else { return app.show_error_toast("请先打开已登记的项目"); };
        *s = State { open: true, context: Some(context.clone()), catalog: app.apps_catalog.generation,
            scope: Some(SkillReposQuery { project_id: project.into(), target_host_id: app.apps_catalog.host.clone() }),
            generation: s.generation.wrapping_add(1), ..Default::default() };
    }
    if !s.open { return Task::none(); }
    if s.context.as_ref() != Some(&context) || s.catalog != app.apps_catalog.generation || s.scope.as_ref().is_none_or(|scope|
        Some(scope.project_id.as_ref()) != app.project_id.as_deref() || scope.target_host_id != app.apps_catalog.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Task::none();
    }
    if let Event::Completed(generation, result) = event {
        if generation != s.generation || !s.busy { return Task::none(); }
        s.busy = false;
        let restored = matches!(&result, Ok(Readback::Restored(_)));
        let changed = matches!(&result, Ok(Readback::Deleted(_) | Readback::Restored(_)));
        if let Err(error) = result.and_then(|value| s.accept(value)) {
            s.error = Some(error.clone()); return app.show_error_toast(error);
        }
        if changed {
            let mut tasks = vec![app.show_success_toast("备份操作完成"), update(app, Event::Refresh)];
            if restored {
                app.skills_settings.install.inventory = None;
                if app.skills_settings.install.open {
                    tasks.push(super::install_view::update(app, super::install_view::Event::Refresh));
                }
                tasks.push(super::refresh_task(app.project_path.clone()));
            }
            return Task::batch(tasks);
        }
        return Task::none();
    }
    if s.busy { return Task::none(); }
    if let Event::Select(id) = event {
        if s.backups.iter().any(|item| item.backup_id == id) { s.selected = Some(id); }
        return Task::none();
    }
    if let Event::Target(target, enabled) = event {
        if s.available.contains(&target) {
            s.targets.retain(|item| item != &target);
            if enabled { s.targets.push(target); }
        }
        return Task::none();
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        s.error = Some("请先在应用设置中连接并授权目标主机".into()); return Task::none();
    }
    let scope = s.scope.clone().expect("open scope");
    let change = match event {
        Event::Open | Event::Refresh => Change::Read,
        Event::Delete | Event::Restore => {
            let Some(id) = s.selected.clone().filter(|id| s.backups.iter().any(|item| &item.backup_id == id)) else { return Task::none(); };
            if matches!(event, Event::Delete) { Change::Delete(id) } else {
                let Some(version) = s.version else { return Task::none(); };
                Change::Restore(id, RestoreSkillBackupRequest { scope: scope.clone(), expected_version: version, apps: s.targets.clone() })
            }
        }
        _ => return Task::none(),
    };
    s.busy = true;
    s.error = None;
    s.generation = s.generation.wrapping_add(1);
    let generation = s.generation;
    let token = app.apps_catalog.token.clone();
    Task::perform(crate::app::session_gateway::gateway_skill_backups_request(scope, token, change), move |result| message(Event::Completed(generation, result)))
}

#[cfg(test)]
#[path = "backup_modal_tests.rs"]
mod tests;
