//! Host-scoped global distribution; independent of ACP session selection.
use desktop_provider_apps::AgentAppsContext;
use super::Input;
use vw_api_app_skills::{bindings_api::*, inventory_api::*, repos_api::SkillReposQuery};

#[derive(Debug, Clone, Default)]
pub struct State {
    pub open: bool,
    pub busy: bool,
    pub snapshot: Option<SkillBindingSnapshot>,
    pub inventory: Option<SkillInventoryResponse>,
    pub targets: Vec<SkillBindingTarget>,
    pub selected: Option<String>,
    pub results: Vec<SkillBindingItem>,
    pub error: Option<String>,
    generation: u64,
    catalog: u64,
    context: Option<AgentAppsContext>,
    scope: Option<SkillReposQuery>,
}
#[derive(Debug, Clone)]
pub enum Event {
    Open, Close, Refresh, Select(String), Target(SkillBindingTarget, bool), Save,
    Bind(bool), Resync,
    Completed(u64, Result<Readback, String>),
}
#[derive(Debug, Clone)]
pub struct Readback {
    pub snapshot: Option<SkillBindingSnapshot>,
    pub inventory: Option<SkillInventoryResponse>,
    pub results: Vec<SkillBindingItem>,
    pub error: Option<String>,
    pub changed: bool,
}
pub enum Change { Save(SkillBindingPreferences), Bind(String, SkillBindingsRequest), Resync(SkillResyncRequest) }

/// Missing, duplicate and unexpected results can never count as complete success.
pub fn complete_success(results: &[SkillBindingItem], targets: &[SkillBindingTarget], id: &str, enabled: bool) -> bool {
    !targets.is_empty() && results.len() == targets.len() && targets.iter().all(|target| {
        results.iter().filter(|item| &item.target == target && item.directory == id && item.status ==
            if enabled { SkillBindingStatus::Enabled } else { SkillBindingStatus::Disabled }).count() == 1
    })
}

pub enum Effect { None, Error(String), Success(String),  Request { generation: u64, scope: SkillReposQuery, change: Option<Change> } }
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {
    let s = state;
    if matches!(event, Event::Close) {
        if !s.busy { *s = State { generation: s.generation.wrapping_add(1), ..Default::default() }; }
        return Effect::None;
    }
    let Some(context) = input.context.clone() else { return Effect::Error(input.connection_error.clone()); };
    if matches!(event, Event::Open) {
        if s.open { return Effect::None; }
        let Some(project) = input.project.clone() else { return Effect::Error(("请先打开已登记的项目").into()); };
        *s = State { open: true, context: Some(context.clone()), catalog: input.catalog,
            scope: Some(SkillReposQuery { project_id: project.into(), target_host_id: input.host.clone() }),
            generation: s.generation.wrapping_add(1), ..Default::default() };
    }
    if !s.open { return Effect::None; }
    if s.context.as_ref() != Some(&context) || s.catalog != input.catalog || s.scope.as_ref().is_none_or(|scope|
        Some(scope.project_id.as_ref()) != input.project.as_deref() || scope.target_host_id != input.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Effect::None;
    }
    if let Event::Completed(generation, result) = event {
        if generation != s.generation || !s.busy { return Effect::None; }
        s.busy = false;
        match result {
            Ok(data) if data.snapshot.as_ref().is_none_or(|v| Some(&v.scope) == s.scope.as_ref())
                && data.inventory.as_ref().is_none_or(|v| Some(&v.scope) == s.scope.as_ref()) => {
                if let Some(snapshot) = data.snapshot { s.targets = snapshot.targets.clone(); s.snapshot = Some(snapshot); }
                if let Some(inventory) = data.inventory { s.inventory = Some(inventory); }
                s.results = data.results;
                s.error = data.error;
                if let Some(error) = s.error.clone() { return Effect::Error(error); }
                if data.changed { return Effect::Success(("操作完成，已重读目标状态").into()); }
            }
            Err(error) => {
                s.error = Some(error.clone());
                return Effect::Error(error);
            }
            Ok(_) => { s.error = Some("重读失败，请刷新确认；已发生的写入不会自动回滚".into());
                return Effect::Error(("重读失败，请刷新确认目标状态").into()); }
        }
        return Effect::None;
    }
    if s.busy { return Effect::None; }
    match event {
        Event::Select(id) => { if s.inventory.as_ref().is_some_and(|data| data.skills.iter().any(|item| item.id == id && item.present)) { s.selected = Some(id); } return Effect::None; }
        Event::Target(target, enabled) => {
            let supported = s.snapshot.as_ref().is_some_and(|data| data.capabilities.iter().any(|c| c.app_id == target.app_id && c.supported));
            let exists = input.targets.contains(&target);
            if supported && exists { s.targets.retain(|t| t != &target); if enabled { s.targets.push(target); } }
            return Effect::None;
        }
        _ => {}
    }
    if !input.authorized {
        s.error = Some("请先在应用设置中连接并授权目标主机".into());
        return Effect::Error(("请先在应用设置中连接并授权目标主机").into());
    }
    let scope = s.scope.clone().expect("open scope");
    let change = match event {
        Event::Save => { let Some(snapshot) = &s.snapshot else { return Effect::None; };
            Some(Change::Save(SkillBindingPreferences { scope: scope.clone(), expected_version: snapshot.version, targets: s.targets.clone() })) }
        Event::Bind(enabled) => { let Some(id) = &s.selected else { return Effect::None; };
            if s.targets.is_empty() { return Effect::None; }
            Some(Change::Bind(id.clone(), SkillBindingsRequest { scope: scope.clone(), targets: s.targets.clone(), enabled, mode: SkillBindingMode::Auto })) }
        Event::Resync => { if s.targets.is_empty() { return Effect::None; }
            Some(Change::Resync(SkillResyncRequest { scope: scope.clone(), targets: s.targets.clone() })) }
        Event::Open | Event::Refresh => None,
        _ => return Effect::None,
    };
    s.busy = true;
    s.error = None;
    s.generation = s.generation.wrapping_add(1);
    let generation = s.generation;
    Effect::Request { generation, scope, change }
}

#[cfg(test)]
#[path = "bindings_view_tests.rs"]
mod tests;
