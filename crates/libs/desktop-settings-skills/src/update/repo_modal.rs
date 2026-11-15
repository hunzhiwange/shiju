//! Versioned repository editing; no optimistic list updates or discovery orchestration.
use super::Input;
use vw_api_app_skills::repos_api::*;

pub use crate::repo_modal::{State, Event};


pub enum Effect { None, Error(String),  Request { generation: u64, added: bool, directory: String, host: String, snapshot: Option<SkillReposResponse>, change: Option<(SkillRepoEntry, bool)> } }
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {
    let Some(context) = input.context.clone() else { return Effect::Error(("无法连接仓库服务").into()); };

    if matches!(event, Event::Open) {
        if state.open { return Effect::None; }
        *state = State { open: true, generation: state.generation.wrapping_add(1),
            context: Some(context.clone()), project: input.project.clone(),
            directory: input.directory.clone(), host: input.host.clone(),
            ..Default::default() };
    } else if matches!(event, Event::Close) {
        // Keep writes visible until settled; closing a read invalidates its response.
        if !state.busy {
            state.open = false;
            state.generation = state.generation.wrapping_add(1);
        }
        return Effect::None;
    }
    if !state.open { return Effect::None; }
    if state.context.as_ref() != Some(&context) || state.project != input.project
        || state.directory != input.directory || state.host != input.host {
        state.open = false;
        state.generation = state.generation.wrapping_add(1);
        return Effect::None;
    }
    if let Event::Completed { generation, added, result } = event {
        if generation != state.generation || !state.busy { return Effect::None; }
        state.busy = false;
        match result {
            Ok(response) => {
                if Some(response.scope.project_id.as_ref()) != state.project.as_deref()
                    || response.scope.target_host_id != state.host {
                    state.error = Some("仓库响应目标不匹配，请刷新".into());
                    return Effect::None;
                }
                if state.snapshot.as_ref().is_some_and(|old|
                    old.scope != response.scope || old.version > response.version) {
                    state.error = Some("仓库响应已过期，请刷新后重试".into());
                    return Effect::None;
                }
                state.snapshot = Some(response);
                state.error = None;
                if added { state.identity.clear(); state.branch.clear();
                    state.source_link.clear(); state.source_preview = None; }
            }
            Err(error) => {
                state.error = Some(error.clone());
                return Effect::Error(error);
            }
        }
        return Effect::None;
    }
    if state.busy { return Effect::None; }
    match event {
        Event::SourceLink(value) => {
            state.source_link = value; state.source_preview = None; state.error = None;
            return Effect::None;
        }
        Event::PreviewSource => {
            state.source_preview = None;
            let Some(directory) = state.directory.clone() else {
                state.error = Some("请先打开项目".into()); return Effect::None;
            };
            match desktop_platform::skill_deeplink::parse(&state.source_link, directory, state.host.clone()) {
                Ok(request) => { state.source_preview = Some(request); state.error = None; }
                Err(error) => { state.error = Some(error); }
            }
            return Effect::None;
        }
        Event::CancelSource => { state.source_preview = None; return Effect::None; }
        Event::Identity(value) => { state.identity = value; return Effect::None; }
        Event::Branch(value) => { state.branch = value; return Effect::None; }
        _ => {}
    }
    let added = matches!(event, Event::Add | Event::ConfirmSource);
    let change = match event {
        Event::ConfirmSource => {
            let Some(request) = &state.source_preview else { return Effect::None; };
            if Some(&request.directory) != state.directory.as_ref() || request.host != state.host {
                state.source_preview = None; return Effect::None;
            }
            Some((request.repo.clone(), false))
        }
        Event::Add => {
            let Some((owner, name)) = state.identity.trim().split_once('/') else {
                state.error = Some("请输入 owner/name 格式的仓库".into());
                return Effect::None;
            };
            if owner.is_empty() || name.is_empty() || !owner.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)) {
                state.error = Some("仓库名称无效".into()); return Effect::None;
            }
            Some((SkillRepoEntry { owner: owner.into(), name: name.into(),
                branch: state.branch.trim().to_owned(), enabled: true }, false))
        }
        Event::Toggle(index) | Event::Remove(index) => {
            let Some(mut repo) = state.snapshot.as_ref().and_then(|s| s.repos.get(index)).cloned() else {
                return Effect::None;
            };
            let remove = matches!(event, Event::Remove(_));
            if !remove { repo.enabled = !repo.enabled; }
            Some((repo, remove))
        }
        Event::Open | Event::Refresh => None,
        _ => return Effect::None,
    };
    if change.is_some() && state.snapshot.is_none() { return Effect::None; }
    let Some(directory) = state.directory.clone() else {
        state.error = Some("请先打开项目".into()); return Effect::None;
    };
    let host = state.host.clone();
    let snapshot = state.snapshot.clone();
    state.busy = true;
    state.error = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    Effect::Request { generation, added, directory, host, snapshot, change }
}

#[cfg(test)]
#[path = "repo_modal_tests.rs"]
mod tests;

pub fn sync_inventory<'a>(state: &'a State, context: &Option<desktop_provider_apps::AgentAppsContext>, project: Option<&str>, host: &str) -> Option<&'a SkillReposResponse> {
    (state.context.as_ref() == context.as_ref() && state.project.as_deref() == project && state.host == host).then_some(state.snapshot.as_ref()).flatten()
}
