//! Unmanaged imports use server selections and explicit application choices.
use super::Input;

pub use crate::import_modal::{State, Event, APPS, eligible};


pub enum Effect { None, Error(String), Refresh, Request { generation: u64, project: String, host: String, apps: Vec<vw_api_app_identity::AppId>, selected: Vec<vw_api_app_skills::import_api::UnmanagedSkillItem> } }
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {

    if matches!(event, Event::Close) {
        if !state.busy { *state = State { generation: state.generation.wrapping_add(1), ..Default::default() }; }
        return Effect::None;
    }
    let Some(context) = input.context.clone() else { return Effect::None; };
    if matches!(event, Event::Open) {
        if state.open { return Effect::None; }
        *state = State { open: true, generation: state.generation.wrapping_add(1),
            context: Some(context.clone()), catalog: input.catalog,
            project: input.project.clone(), host: input.host.clone(), ..Default::default() };
        return Effect::None;
    }
    if !state.open { return Effect::None; }
    if state.context.as_ref() != Some(&context) || state.catalog != input.catalog
        || state.project != input.project || state.host != input.host {
        *state = State { generation: state.generation.wrapping_add(1), ..Default::default() };
        return Effect::None;
    }
    if let Event::Completed(generation, outcomes, result) = event {
        if generation != state.generation || !state.busy { return Effect::None; }
        state.busy = false;
        for (id, result) in outcomes {
            if result.is_ok() { state.selected.retain(|selected| selected != &id); }
            state.outcomes.retain(|(old, _)| old != &id);
            state.outcomes.push((id, result));
        }
        match result {
            Ok(report) if Some(report.scope.project_id.as_ref()) == state.project.as_deref()
                && report.scope.target_host_id == state.host => {
                state.report = Some(report); state.error = None;
            }
            Ok(_) => state.error = Some("导入响应作用域不匹配，请重新扫描".into()),
            Err(error) => { state.error = Some(error.clone()); return Effect::Error(error); }
        }
        return Effect::Refresh;
    }
    if state.busy { return Effect::None; }
    match event {
        Event::App(id, enabled) => {
            if !APPS.contains(&id) { return Effect::None; }
            state.apps.retain(|app| *app != id);
            if enabled { state.apps.push(id); }
            state.report = None; state.selected.clear(); state.error = None;
            return Effect::None;
        }
        Event::Select(id, enabled) => {
            if !state.report.as_ref().is_some_and(|r| r.skills.iter()
                .any(|s| s.directory == id && eligible(s))) { return Effect::None; }
            state.selected.retain(|old| old != &id);
            if enabled { state.selected.push(id); }
            return Effect::None;
        }
        Event::Scan | Event::Import => {}
        _ => return Effect::None,
    }
    let Some(project) = state.project.clone() else {
        state.error = Some("请先打开已登记的项目".into()); return Effect::None;
    };
    if state.apps.is_empty() || !input.authorized {
        state.error = Some("请选择应用并在应用设置中授权目标主机".into()); return Effect::None;
    }
    let selected: Vec<_> = if matches!(event, Event::Import) {
        state.report.as_ref().map(|r| r.skills.iter().filter(|s| eligible(s)
            && state.selected.contains(&s.directory)).cloned().collect()).unwrap_or_default()
    } else { Vec::new() };
    if matches!(event, Event::Import) && selected.is_empty() { return Effect::None; }
    state.busy = true; state.error = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    Effect::Request { generation, project, host: state.host.clone(), apps: state.apps.clone(), selected }
}
