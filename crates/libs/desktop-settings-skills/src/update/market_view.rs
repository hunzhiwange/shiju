//! Read-only discovery and market search share one request generation.
use super::Input;

pub use crate::market_view::{Results, State, Event};


pub enum Effect { None, Error(String),  Request { generation: u64, project: String, host: String, query: String } }
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {

    if matches!(event, Event::Close) {
        *state = State { generation: state.generation.wrapping_add(1), ..Default::default() };
        return Effect::None;
    }
    let Some(context) = input.context.clone() else { return Effect::None; };
    if matches!(event, Event::Open) {
        if state.open { return Effect::None; }
        *state = State { open: true, generation: state.generation.wrapping_add(1),
            context: Some(context.clone()), catalog: input.catalog,
            project: input.project.clone(), host: input.host.clone(), ..Default::default() };
    }
    if !state.open { return Effect::None; }
    if state.context.as_ref() != Some(&context) || state.catalog != input.catalog
        || state.project != input.project || state.host != input.host {
        *state = State { generation: state.generation.wrapping_add(1), ..Default::default() };
        return Effect::None;
    }
    if let Event::Query(query) = event {
        state.query = query;
        state.generation = state.generation.wrapping_add(1);
        state.busy = false;
        state.results = None;
        state.error = None;
        return Effect::None;
    }
    if let Event::Completed(generation, result) = event {
        if generation != state.generation || !state.busy { return Effect::None; }
        state.busy = false;
        let configured_version = input.repository_version;
        match result {
            Ok(Results::Discovery(report)) if report.version < state.version.max(configured_version) => {
                state.error = Some("仓库配置已更新，请重新发现".into());
            }
            Ok(Results::Search(report)) if report.query != state.query.trim() => {
                state.error = Some("搜索响应与当前查询不匹配，请重试".into());
            }
            Ok(results) => {
                if let Results::Discovery(report) = &results { state.version = report.version; }
                state.results = Some(results);
                state.error = None;
            }
            Err(error) => {
                state.error = Some(error.clone());
                return Effect::Error(error);
            }
        }
        return Effect::None;
    }
    if state.busy { return Effect::None; }
    let project = vw_api_transport::local_settings_project(state.project.as_deref()).to_owned();
    state.busy = true;
    state.results = None;
    state.error = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    Effect::Request { generation, project, host: state.host.clone(), query: state.query.trim().to_owned() }
}

#[cfg(test)]
#[path = "market_view_tests.rs"]
mod tests;
