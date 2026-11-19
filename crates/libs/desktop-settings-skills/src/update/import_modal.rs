//! Unmanaged imports use server selections and explicit application choices.
use super::Input;

pub use crate::import_modal::{APPS, Event, State, eligible};

pub enum Effect {
    None,
    Error(String),
    Refresh,
    Request {
        generation: u64,
        project: String,
        host: String,
        apps: Vec<vw_api_app_identity::AppId>,
        targets: Vec<vw_api_app_skills::bindings_api::SkillBindingTarget>,
        selected: Vec<vw_api_app_skills::import_api::UnmanagedSkillItem>,
    },
}
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {
    if matches!(event, Event::Close) {
        if !state.busy {
            *state = State { generation: state.generation.wrapping_add(1), ..Default::default() };
        }
        return Effect::None;
    }
    let Some(context) = input.context.clone() else {
        return Effect::None;
    };
    if matches!(event, Event::Open) {
        if state.open {
            return Effect::None;
        }
        *state = State {
            open: true,
            generation: state.generation.wrapping_add(1),
            context: Some(context.clone()),
            catalog: input.catalog,
            available_targets: input.targets.clone(),
            project: input.project.clone(),
            host: input.host.clone(),
            ..Default::default()
        };
        return Effect::None;
    }
    if !state.open {
        return Effect::None;
    }
    if state.context.as_ref() != Some(&context)
        || state.catalog != input.catalog
        || state.project != input.project
        || state.host != input.host
    {
        *state = State { generation: state.generation.wrapping_add(1), ..Default::default() };
        return Effect::None;
    }
    if let Event::Completed(generation, outcomes, result) = event {
        if generation != state.generation || !state.busy {
            return Effect::None;
        }
        state.busy = false;
        if result.as_ref().is_ok_and(|report| {
            report.scope.project_id.as_ref()
                != vw_api_transport::local_settings_project(state.project.as_deref())
                || report.scope.target_host_id != state.host
        }) {
            state.report = None;
            state.error = Some("扫描响应作用域不匹配，请重新扫描".into());
            return Effect::None;
        }
        for (id, result) in outcomes {
            if result.is_ok() {
                state.selected.retain(|selected| selected != &id);
            }
            state.outcomes.retain(|(old, _)| old != &id);
            state.outcomes.push((id, result));
        }
        match result {
            Ok(report)
                if report.scope.project_id.as_ref()
                    == vw_api_transport::local_settings_project(state.project.as_deref())
                    && report.scope.target_host_id == state.host =>
            {
                state.report = Some(report);
                state.error = None;
            }
            Ok(_) => state.error = Some("导入响应作用域不匹配，请重新扫描".into()),
            Err(error) => {
                state.report = None;
                state.error = Some(error.clone());
                return if state.outcomes.iter().any(|(_, result)| result.is_ok()) {
                    Effect::Refresh
                } else {
                    Effect::Error(error)
                };
            }
        }
        return Effect::Refresh;
    }
    if state.busy {
        return Effect::None;
    }
    match event {
        Event::Query(query) => {
            state.query = query;
            return Effect::None;
        }
        Event::Target(target, enabled) => {
            if !input.targets.contains(&target) || !state.available_targets.contains(&target) {
                return Effect::None;
            }
            state.targets.retain(|old| old != &target);
            if enabled {
                state.targets.push(target);
            }
            return Effect::None;
        }
        Event::SelectVisible(enabled) => {
            if let Some(report) = &state.report {
                for row in report.skills.iter().filter(|row| {
                    eligible(row) && crate::import_modal::matches_query(row, &state.query)
                }) {
                    state.selected.retain(|id| id != &row.directory);
                    if enabled
                        && !state
                            .outcomes
                            .iter()
                            .any(|(id, result)| id == &row.directory && result.is_ok())
                    {
                        state.selected.push(row.directory.clone());
                    }
                }
            }
            return Effect::None;
        }
        Event::App(id, enabled) => {
            if !APPS.contains(&id) {
                return Effect::None;
            }
            state.apps.retain(|app| *app != id);
            if enabled {
                state.apps.push(id);
            }
            state.report = None;
            state.selected.clear();
            state.error = None;
            return Effect::None;
        }
        Event::Select(id, enabled) => {
            if !state
                .report
                .as_ref()
                .is_some_and(|r| r.skills.iter().any(|s| s.directory == id && eligible(s)))
            {
                return Effect::None;
            }
            state.selected.retain(|old| old != &id);
            if enabled && !state.outcomes.iter().any(|(old, result)| old == &id && result.is_ok()) {
                state.selected.push(id);
            }
            return Effect::None;
        }
        Event::Scan | Event::Import => {}
        _ => return Effect::None,
    }
    let project = vw_api_transport::local_settings_project(state.project.as_deref()).to_owned();
    if state.apps.is_empty() {
        state.error = Some("请选择应用".into());
        return Effect::None;
    }
    let selected: Vec<_> = if matches!(event, Event::Import) {
        state
            .report
            .as_ref()
            .map(|r| {
                r.skills
                    .iter()
                    .filter(|s| eligible(s) && state.selected.contains(&s.directory))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if matches!(event, Event::Import) && selected.is_empty() {
        return Effect::None;
    }
    state.busy = true;
    state.error = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let targets = state.targets.iter().filter(|t| input.targets.contains(t)).cloned().collect();
    Effect::Request {
        generation,
        project,
        targets,
        host: state.host.clone(),
        apps: state.apps.clone(),
        selected,
    }
}
