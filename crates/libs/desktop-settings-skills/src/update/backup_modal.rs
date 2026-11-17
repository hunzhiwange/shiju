//! Backup selection is retained after conflicts and isolated by execution context.
use desktop_provider_apps::AgentAppsContext;
use super::Input;
use vw_api_app_skills::{backups_api::*, bindings_api::SkillBindingTarget, repos_api::SkillReposQuery};
#[derive(Clone, Debug, Default)]
pub struct State {
    pub open: bool, pub busy: bool,
    pub backups: Vec<SkillBackup>, pub selected: Option<String>,
    pub available: Vec<SkillBindingTarget>, pub targets: Vec<SkillBindingTarget>,
    pub version: Option<u64>, pub error: Option<String>,
    generation: u64, catalog: u64, context: Option<AgentAppsContext>, scope: Option<SkillReposQuery>,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open, Close, Refresh, Select(String), Target(SkillBindingTarget, bool), Delete, Restore,
    Completed(u64, Result<Readback, String>),
}
#[derive(Clone, Debug)]
pub enum Readback {
    List(SkillBackupsResponse, u64, Vec<SkillBindingTarget>),
    Deleted(DeleteSkillBackupResponse), Restored(RestoreSkillBackupResponse),
}
pub enum Change { Read, Delete(String), Restore(String, RestoreSkillBackupRequest) }
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

pub enum Effect { Changed { restored: bool }, None, Error(String),  Request { generation: u64, scope: SkillReposQuery, change: Change } }
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {
    let s = state;
    if matches!(event, Event::Close) {
        if !s.busy { *s = State { generation: s.generation.wrapping_add(1), ..Default::default() }; }
        return Effect::None;
    }
    let Some(context) = input.context.clone() else { return Effect::Error(("无法连接备份服务").into()); };
    if matches!(event, Event::Open) {
        if s.open { return Effect::None; }
        let project = vw_api_transport::local_settings_project(input.project.as_deref()).to_owned();
        *s = State { open: true, context: Some(context.clone()), catalog: input.catalog,
            scope: Some(SkillReposQuery { project_id: project.into(), target_host_id: input.host.clone() }),
            generation: s.generation.wrapping_add(1), ..Default::default() };
    }
    if !s.open { return Effect::None; }
    if s.context.as_ref() != Some(&context) || s.catalog != input.catalog || s.scope.as_ref().is_none_or(|scope|
        scope.project_id.as_ref() != vw_api_transport::local_settings_project(input.project.as_deref()) || scope.target_host_id != input.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Effect::None;
    }
    if let Event::Completed(generation, result) = event {
        if generation != s.generation || !s.busy { return Effect::None; }
        s.busy = false;
        let restored = matches!(&result, Ok(Readback::Restored(_)));
        let changed = matches!(&result, Ok(Readback::Deleted(_) | Readback::Restored(_)));
        if let Err(error) = result.and_then(|value| s.accept(value)) {
            s.error = Some(error.clone()); return Effect::Error(error);
        }
        if changed {
            return Effect::Changed { restored };
        }
        return Effect::None;
    }
    if s.busy { return Effect::None; }
    if let Event::Select(id) = event {
        if s.backups.iter().any(|item| item.backup_id == id) { s.selected = Some(id); }
        return Effect::None;
    }
    if let Event::Target(target, enabled) = event {
        if s.available.contains(&target) {
            s.targets.retain(|item| item != &target);
            if enabled { s.targets.push(target); }
        }
        return Effect::None;
    }
    let scope = s.scope.clone().expect("open scope");
    let change = match event {
        Event::Open | Event::Refresh => Change::Read,
        Event::Delete | Event::Restore => {
            let Some(id) = s.selected.clone().filter(|id| s.backups.iter().any(|item| &item.backup_id == id)) else { return Effect::None; };
            if matches!(event, Event::Delete) { Change::Delete(id) } else {
                let Some(version) = s.version else { return Effect::None; };
                Change::Restore(id, RestoreSkillBackupRequest { scope: scope.clone(), expected_version: version, apps: s.targets.clone() })
            }
        }
        _ => return Effect::None,
    };
    s.busy = true;
    s.error = None;
    s.generation = s.generation.wrapping_add(1);
    let generation = s.generation;
    Effect::Request { generation, scope, change }
}

#[cfg(test)]
#[path = "backup_modal_tests.rs"]
mod tests;
