//! Storage drafts never replace the acknowledged execution-host root.
use crate::app::{App, Message, message::SettingsMessage, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::vw_api_types::agent_apps_skills::{bindings_api::*, repos_api::SkillReposQuery, storage_api::*};

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub current: Option<SkillStorageResponse>,
    pub pending: Option<SkillStorageLocation>,
    pub results: Vec<String>,
    pub error: Option<String>,
    generation: u64,
    catalog: u64,
    context: Option<AgentAppsContext>,
    scope: Option<SkillReposQuery>,
}
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Open, Close, Refresh, Select(SkillStorageLocation), Mode(SkillBindingMode), Move, Resync, OpenDirectory,
    Completed(u64, Result<Readback, String>),
}
#[derive(Clone, Debug)]
pub(crate) enum Readback {
    Storage(SkillStorageResponse),
    Move(SkillStorageMoveResponse),
    Sync(SkillBindingsResponse),
    Opened,
}
pub(crate) enum Change { OpenDirectory(String), Read, Mode(SkillStoragePatch), Move(SkillStorageMoveRequest), Resync(SkillResyncRequest) }
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::SkillsStorage(event)) }
impl State {
    fn accept(&mut self, result: Readback) -> Result<(), String> {
        match result {
            Readback::Opened => {},
            Readback::Storage(value) if Some(&value.scope) == self.scope.as_ref() => self.current = Some(value),
            Readback::Move(value) if Some(&value.storage.scope) == self.scope.as_ref() => {
                self.results = value.migrated.iter().map(|id| format!("已迁移：{id}")).chain(
                    value.skipped.iter().map(|id| format!("已跳过：{id}"))).collect();
                self.results.extend(value.errors.iter().cloned());
                if !value.switched || !value.errors.is_empty() {
                    return Err("迁移未完成，保留原主副本位置；请刷新确认执行端状态".into());
                }
                self.current = Some(value.storage);
                self.pending = None;
            }
            Readback::Sync(value) if Some(&value.scope) == self.scope.as_ref() => {
                self.results = value.results.iter().map(|item| format!("{} / {}：{:?}", item.target.installation_id, item.directory, item.status)).collect();
                if value.results.iter().any(|item| item.status == SkillBindingStatus::Failed) {
                    return Err("部分同步失败，请检查逐项结果".into());
                }
            }
            _ => return Err("响应作用域不匹配，请刷新".into()),
        }
        Ok(())
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let s = &mut app.skills_settings.storage;
    if matches!(event, Event::Close) {
        if !s.busy { *s = State { generation: s.generation.wrapping_add(1), ..Default::default() }; }
        return Task::none();
    }
    let Ok(client) = crate::app::config::gateway_client() else { return app.show_error_toast("无法连接存储服务"); };
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
        if let Err(error) = result.and_then(|value| s.accept(value)) {
            s.error = Some(error.clone()); return app.show_error_toast(error);
        }
        return Task::none();
    }
    if s.busy { return Task::none(); }
    if let Event::Select(location) = event { s.pending = Some(location); return Task::none(); }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        s.error = Some("请先在应用设置中连接并授权目标主机".into()); return Task::none();
    }
    let scope = s.scope.clone().expect("open scope");
    let change = match event {
        Event::Open | Event::Refresh => Change::Read,
        Event::Mode(mode) => { let Some(current) = &s.current else { return Task::none(); };
            Change::Mode(SkillStoragePatch { scope: scope.clone(), expected_version: current.version, mode }) }
        Event::Move => { let (Some(current), Some(location)) = (&s.current, s.pending) else { return Task::none(); };
            Change::Move(SkillStorageMoveRequest { scope: scope.clone(), expected_version: current.version, location, targets: Vec::new() }) }
        Event::OpenDirectory => {
            if !client.endpoint().is_loopback() || scope.target_host_id != vw_shared::local_apps::HOST_ID {
                return app.show_error_toast("远程目标目录不可在本机打开");
            }
            let Some(current) = &s.current else { return Task::none(); };
            Change::OpenDirectory(current.root.clone())
        }
        Event::Resync => Change::Resync(SkillResyncRequest { scope: scope.clone(), targets: Vec::new() }),
        _ => return Task::none(),
    };
    s.busy = true;
    s.error = None;
    s.results.clear();
    s.generation = s.generation.wrapping_add(1);
    let generation = s.generation;
    let token = app.apps_catalog.token.clone();
    Task::perform(crate::app::session_gateway::gateway_skill_storage_request(scope, token, change), move |result| message(Event::Completed(generation, result)))
}

#[cfg(target_os = "macos")]
pub(crate) fn open_local(home: &str, path: &str) -> Result<(), ()> {
    use std::path::Path;
    let home = Path::new(home);
    let path = Path::new(path);
    let relative = path.strip_prefix(home).map_err(|_| ())?;
    if relative.as_os_str().is_empty()
        || !relative.components().all(|part| matches!(part, std::path::Component::Normal(_)))
    {
        return Err(());
    }
    let actual = path.canonicalize().map_err(|_| ())?;
    if actual != home.canonicalize().map_err(|_| ())?.join(relative) || !actual.is_dir() {
        return Err(());
    }
    let status = std::process::Command::new("/usr/bin/open")
        .args(["-a", "Finder"])
        .arg(actual)
        .status()
        .map_err(|_| ())?;
    status.success().then_some(()).ok_or(())
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn open_local(_: &str, _: &str) -> Result<(), ()> {
    Err(())
}

#[cfg(test)]
#[path = "storage_modal_tests.rs"]
mod tests;
