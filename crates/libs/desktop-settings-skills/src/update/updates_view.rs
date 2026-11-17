//! Scoped update results; refresh errors never replace a completed operation.
use super::Input;
use vw_api_app_skills::repos_api::SkillReposQuery;

pub use crate::updates_view::{State, Operation, Outcome, Event};


pub enum Effect { None, Error(String), Refresh, Request { g: u64, scope: SkillReposQuery, operation: Option<(Operation, u64, Option<String>)> } }
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {
    let Some(context) = input.context.clone() else { return Effect::None; };
    let s = state;
    if matches!(event, Event::Close) {
        if !s.busy { *s = State { generation: s.generation.wrapping_add(1), ..Default::default() }; }
        return Effect::None;
    }
    if matches!(event, Event::Open) && !s.open {
        let project = vw_api_transport::local_settings_project(input.project.as_deref()).to_owned();
        *s = State { open: true, context: Some(context.clone()), catalog: input.catalog,
            scope: Some(SkillReposQuery { project_id: project.into(), target_host_id: input.host.clone() }),
            generation: s.generation.wrapping_add(1), ..Default::default() };
    }
    if !s.open { return Effect::None; }
    if s.context.as_ref() != Some(&context) || s.catalog != input.catalog
        || s.scope.as_ref().is_none_or(|q| Some(q.project_id.as_ref()) != input.project.as_deref() || q.target_host_id != input.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Effect::None;
    }
    match event {
        Event::Loaded(g, inventory, updates) if g == s.generation && s.busy => {
            s.busy = false; s.loaded(inventory, updates);
            return match s.error.clone() { Some(e) => Effect::Error(e), None => Effect::None };
        }
        Event::Finished(g, result) if g == s.generation && s.busy => {
            s.busy = false; s.finished(result);
            return Effect::Refresh;
        }
        Event::Loaded(..) | Event::Finished(..) => return Effect::None,
        _ if s.busy => return Effect::None,
        _ => {}
    }
    let scope = s.scope.clone().expect("open scope");
    let operation = if let Event::Run(operation) = event {
        let Some(inventory) = &s.inventory else { return Effect::None; };
        let id = match &operation { Operation::Update(id) | Operation::Uninstall(id) => id };
        if !inventory.skills.iter().any(|skill| &skill.id == id) { return Effect::None; }
        let hash = match &operation {
            Operation::Update(id) => {
                let Some(cache) = &s.updates else { return Effect::None; };
                let Some(info) = cache.updates.iter().find(|u| &u.directory == id) else { return Effect::None; };
                let Some(hash) = info.current_hash.clone() else { return Effect::None; };
                Some(hash)
            }
            Operation::Uninstall(_) => None,
        };
        Some((operation, inventory.version, hash))
    } else { None };
    s.busy = true; s.error = None; s.generation = s.generation.wrapping_add(1);
    let g = s.generation;
    if operation.is_some() { s.backup = None; s.outcome = None; }
    else { s.inventory = None; s.updates = None; }
    Effect::Request { g, scope, operation }
}
