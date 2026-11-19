//! Versioned repository editing; no optimistic list updates or discovery orchestration.
use super::Input;
use vw_api_app_skills::repos_api::*;

pub use crate::repo_modal::{Event, State};

pub enum Effect {
    OpenSource(String),
    None,
    Error(String),
    Request {
        generation: u64,
        added: bool,
        directory: String,
        host: String,
        snapshot: Option<SkillReposResponse>,
        change: Option<(SkillRepoEntry, bool)>,
    },
}
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {
    let Some(context) = input.context.clone() else {
        return Effect::Error(("无法连接仓库服务").into());
    };

    if matches!(event, Event::Open) {
        if state.open {
            return Effect::None;
        }
        *state = State {
            open: true,
            generation: state.generation.wrapping_add(1),
            context: Some(context.clone()),
            project: input.project.clone(),
            directory: input.directory.clone(),
            host: input.host.clone(),
            ..Default::default()
        };
    } else if matches!(event, Event::Close) {
        // Keep writes visible until settled; closing a read invalidates its response.
        if !state.busy {
            state.open = false;
            state.generation = state.generation.wrapping_add(1);
        }
        return Effect::None;
    }
    if !state.open {
        return Effect::None;
    }
    if state.context.as_ref() != Some(&context)
        || state.project != input.project
        || state.directory != input.directory
        || state.host != input.host
    {
        state.open = false;
        state.generation = state.generation.wrapping_add(1);
        return Effect::None;
    }
    if let Event::Completed { generation, added, result } = event {
        if generation != state.generation || !state.busy {
            return Effect::None;
        }
        state.busy = false;
        match result {
            Ok(response) => {
                if Some(response.scope.project_id.as_ref()) != state.project.as_deref()
                    || response.scope.target_host_id != state.host
                {
                    state.error = Some("仓库响应目标不匹配，请刷新".into());
                    return Effect::None;
                }
                if state.snapshot.as_ref().is_some_and(|old| {
                    old.scope != response.scope || old.version > response.version
                }) {
                    state.error = Some("仓库响应已过期，请刷新后重试".into());
                    return Effect::None;
                }
                state.snapshot = Some(response);
                state.error = None;
                if added {
                    state.identity.clear();
                    state.branch.clear();
                    state.source_link.clear();
                    state.source_preview = None;
                }
            }
            Err(error) => {
                state.error = Some(error.clone());
                return Effect::Error(error);
            }
        }
        return Effect::None;
    }
    if state.busy {
        return Effect::None;
    }
    match event {
        Event::SourceLink(value) => {
            state.source_link = value;
            state.source_preview = None;
            state.error = None;
            return Effect::None;
        }
        Event::PreviewSource => {
            state.source_preview = None;
            let Some(directory) = state.directory.clone() else {
                state.error = Some("请先打开项目".into());
                return Effect::None;
            };
            match desktop_platform::skill_deeplink::parse(
                &state.source_link,
                directory,
                state.host.clone(),
            ) {
                Ok(request) => {
                    state.source_preview = Some(request);
                    state.error = None;
                }
                Err(error) => {
                    state.error = Some(error);
                }
            }
            return Effect::None;
        }
        Event::CancelSource => {
            state.source_preview = None;
            return Effect::None;
        }
        Event::Remove(index) => {
            state.remove_pending =
                state.snapshot.as_ref().and_then(|s| s.repos.get(index)).map(|_| index);
            return Effect::None;
        }
        Event::CancelRemove => {
            state.remove_pending = None;
            return Effect::None;
        }
        Event::OpenSource(index) => {
            return state
                .snapshot
                .as_ref()
                .and_then(|s| s.repos.get(index))
                .and_then(|repo| parse_identity(&format!("{}/{}", repo.owner, repo.name)).ok())
                .map_or(Effect::None, |(owner, name)| {
                    Effect::OpenSource(format!("https://github.com/{owner}/{name}"))
                });
        }
        Event::Identity(value) => {
            state.identity_error = None;
            state.identity = value;
            return Effect::None;
        }
        Event::Branch(value) => {
            state.branch_error = None;
            state.branch = value;
            return Effect::None;
        }
        _ => {}
    }
    let added = matches!(event, Event::Add | Event::ConfirmSource);
    let change = match event {
        Event::ConfirmSource => {
            let Some(request) = &state.source_preview else {
                return Effect::None;
            };
            if Some(&request.directory) != state.directory.as_ref() || request.host != state.host {
                state.source_preview = None;
                return Effect::None;
            }
            Some((request.repo.clone(), false))
        }
        Event::Add => {
            let (owner, name) = match parse_identity(&state.identity) {
                Ok(value) => value,
                Err(error) => {
                    state.identity_error = Some(error);
                    return Effect::None;
                }
            };
            let branch = state.branch.trim();
            if !branch.is_empty()
                && (branch.starts_with('-')
                    || branch.starts_with('/')
                    || branch.ends_with('/')
                    || branch.ends_with('.')
                    || branch.contains("..")
                    || branch.contains("@{")
                    || branch.split('/').any(|part| {
                        part.is_empty() || part.starts_with('.') || part.ends_with(".lock")
                    })
                    || branch
                        .chars()
                        .any(|c| c.is_control() || c.is_whitespace() || "~^:?*[\\".contains(c)))
            {
                state.branch_error = Some("请输入有效的 Git 分支名称".into());
                return Effect::None;
            }
            Some((
                SkillRepoEntry {
                    owner: owner.into(),
                    name: name.into(),
                    branch: state.branch.trim().to_owned(),
                    enabled: true,
                },
                false,
            ))
        }
        Event::Toggle(_) | Event::ConfirmRemove => {
            let index = match event {
                Event::Toggle(index) => index,
                _ => match state.remove_pending.take() {
                    Some(index) => index,
                    None => return Effect::None,
                },
            };
            let Some(mut repo) = state.snapshot.as_ref().and_then(|s| s.repos.get(index)).cloned()
            else {
                return Effect::None;
            };
            let remove = matches!(event, Event::ConfirmRemove);
            if !remove {
                repo.enabled = !repo.enabled;
            }
            Some((repo, remove))
        }
        Event::Open | Event::Refresh => None,
        _ => return Effect::None,
    };
    if change.is_some() && state.snapshot.is_none() {
        return Effect::None;
    }
    let Some(directory) = state.directory.clone() else {
        state.error = Some("请先打开项目".into());
        return Effect::None;
    };
    let host = state.host.clone();
    let snapshot = state.snapshot.clone();
    state.remove_pending = None;
    state.busy = true;
    state.error = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    Effect::Request { generation, added, directory, host, snapshot, change }
}

#[cfg(test)]
#[path = "repo_modal_tests.rs"]
mod tests;

pub fn sync_inventory<'a>(
    state: &'a State,
    context: &Option<desktop_provider_apps::AgentAppsContext>,
    project: Option<&str>,
    host: &str,
) -> Option<&'a SkillReposResponse> {
    (state.context.as_ref() == context.as_ref()
        && state.project.as_deref() == project
        && state.host == host)
        .then_some(state.snapshot.as_ref())
        .flatten()
}

// Accept only a repository root on GitHub; never forward arbitrary user URLs.
fn parse_identity(value: &str) -> Result<(String, String), String> {
    let value = value.trim();
    let path = if let Some(path) = value.strip_prefix("https://github.com/") {
        path.trim_end_matches('/')
    } else {
        value
    };
    let (owner, name) = path.split_once('/').ok_or("请输入 owner/name 或 GitHub 仓库 URL")?;
    let name = name.strip_suffix(".git").unwrap_or(name);
    if owner.is_empty()
        || name.is_empty()
        || name == "."
        || name == ".."
        || !owner.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err("仓库格式无效，仅支持 GitHub 仓库根地址".into());
    }
    Ok((owner.into(), name.into()))
}
