use std::collections::{HashMap, HashSet};

use session_info::Info;

fn preserve_published_sessions(existing: &[Info], incoming: &mut [Info]) {
    for pending in incoming.iter_mut().filter(|info| info.is_first_response_pending()) {
        if let Some(published) = existing.iter().find(|info| {
            info.id == pending.id
                && info.project_id == pending.project_id
                && info.source_slug() == pending.source_slug()
                && !info.is_first_response_pending()
        }) {
            *pending = published.clone();
        }
    }
}

pub fn merge_project_sessions(existing: Option<Vec<Info>>, mut incoming: Vec<Info>) -> Vec<Info> {
    let Some(existing) = existing else {
        return incoming;
    };

    preserve_published_sessions(&existing, &mut incoming);
    let mut seen: HashSet<String> = incoming.iter().map(|session| session.id.clone()).collect();
    let mut merged = incoming;
    for session in existing {
        if seen.insert(session.id.clone()) {
            merged.push(session);
        }
    }
    merged
}

#[cfg(test)]
#[path = "session_open_tests.rs"]
mod tests;

pub enum Navigation {
    Ignore,
    Cached { project_path: String, id: String, info: Info },
    Draft { project_path: String, id: String },
    Resolve { id: String, cached: Option<Info>, path: String },
}
/// Select a local destination without touching chat, render caches or request state.
pub fn navigation(
    path: String,
    id: String,
    current_project: Option<&String>,
    project_id: Option<&str>,
    active_session: Option<&str>,
    sessions: &[Info],
    project_sessions: &HashMap<String, Vec<Info>>,
    requesting_draft: Option<(&str, &str)>,
) -> Navigation {
    let id = id.trim().to_owned();
    if id.is_empty() {
        return Navigation::Ignore;
    }
    let mut copies = sessions
        .iter()
        .chain(project_sessions.values().flatten())
        .filter(|info| info.id == id);
    let cached = copies.next();
    if cached.is_some_and(|info| info.is_agent_tool() || info.is_plan_run()) {
        return Navigation::Ignore;
    }
    // A running local draft can also be listed by the gateway as pending.
    if let Some((project_path, directory)) = requesting_draft
        && directory == path
    {
        return if active_session == Some(id.as_str()) {
            Navigation::Ignore
        } else {
            Navigation::Draft { project_path: project_path.to_owned(), id }
        };
    }
    // The active list and the project list can finish loading in either order.
    let cached = match cached {
        Some(pending) if pending.is_first_response_pending() => copies
            .find(|info| {
                info.project_id == pending.project_id
                    && info.source_slug() == pending.source_slug()
                    && info.is_user_navigable()
            })
            .or(Some(pending)),
        _ => cached,
    };
    if let Some(info) = cached {
        if !info.is_user_navigable() { return Navigation::Ignore; }
        if !info.directory.trim().is_empty() && info.fork_execution_started.is_none() {
            let project_path = current_project.filter(|path| {
                *path == &info.directory || project_id == Some(info.project_id.as_str())
            }).or_else(|| {
                project_sessions.iter().find_map(|(path, sessions)| {
                    (path != &info.directory && sessions.iter().any(|s| s.id == id)).then_some(path)
                })
            });
            if let Some(project_path) = project_path.cloned() {
                return Navigation::Cached { project_path, id, info: info.clone() };
            }
        }
    }
    Navigation::Resolve { id, cached: cached.cloned(), path }
}

pub async fn resolve_navigation(client: &vw_gateway_client::GatewayClient, id: String,
    cached: Option<Info>, path: String) -> Result<(String, Box<Info>), String> {
    let directory = cached.as_ref().map(|info| info.directory.clone())
        .filter(|directory| !directory.trim().is_empty())
        .or_else(|| (!path.trim().is_empty()).then_some(path));
    let info = match cached.filter(|info| !info.directory.trim().is_empty() && info.fork_execution_started.is_none()) {
        Some(info) => info,
        None => gateway_session::GatewaySessionClient::new(client.transport().clone())
            .session_get::<Info>(&id, directory.as_deref()).await?,
    };
    if info.id != id || !info.is_user_navigable() || info.directory.trim().is_empty() {
        return Err("Session is not available to open".into());
    }
    let project_path = super::gateway::gateway_session_project_path(client, &info).await?;
    Ok((project_path, Box::new(info)))
}

pub fn navigation_is_current(generation: u64, current_generation: u64,
    from_project: &Option<String>, current_project: &Option<String>,
    from_session: &Option<String>, active_session: &Option<String>,
) -> bool {
    generation == current_generation
        && from_project == current_project && from_session == active_session
}

/// The session-list projection borrows the existing collections; no parallel cache is created.
pub struct SessionLists<'a> {
    pub sessions: &'a mut Vec<Info>,
    pub previews: &'a mut HashMap<String, String>,
    pub archived: &'a mut HashSet<String>,
    pub projects: &'a mut HashMap<String, Vec<Info>>,
    pub counts: &'a mut HashMap<String, usize>,
    pub loading: &'a mut HashSet<String>,
    pub scrollbars: &'a mut HashMap<String, bool>,
}
pub enum ListEvent {
    Loaded(Result<Vec<Info>, String>),
    Bootstrap { project_path: Option<String>, result: Result<Vec<Info>, String>,
        previews: HashMap<String, String>, archived: HashSet<String> },
    ProjectLoaded(String, Result<Vec<Info>, String>),
    Scrollbar(String, bool),
    More(String),
    Load(String),
    Hover(String),
}
pub enum ListEffect { Notification(String), Error(String), Load(String) }
pub fn update_lists(lists: &mut SessionLists<'_>, current_project: &Option<String>, event: ListEvent) -> Vec<ListEffect> {
    match event {
        ListEvent::Loaded(result) => match result {
            Ok(mut sessions) => {
                preserve_published_sessions(lists.sessions, &mut sessions);
                *lists.sessions = sessions;
            }
            Err(error) => return vec![ListEffect::Notification(format!("Failed to load sessions: {}", error))],
        },
        ListEvent::Bootstrap { project_path, result, previews, archived } => {
            if let Some(path) = project_path.as_ref() { lists.loading.remove(path); }
            if &project_path != current_project { return vec![]; }
            *lists.previews = previews;
            *lists.archived = archived;
            match result {
                Ok(mut sessions) => {
                    if let Some(path) = project_path {
                        let existing = lists.projects.remove(&path);
                        let loaded_count = sessions.len();
                        let merged = merge_project_sessions(existing, sessions);
                        // The active list uses only the entries returned by this load.
                        *lists.sessions = merged[..loaded_count].to_vec();
                        lists.projects.insert(path, merged);
                    } else {
                        preserve_published_sessions(lists.sessions, &mut sessions);
                        *lists.sessions = sessions;
                    }
                }
                Err(error) => return vec![ListEffect::Notification(format!("Failed to load sessions: {}", error))],
            }
        }
        ListEvent::ProjectLoaded(path, result) => {
            lists.loading.remove(&path);
            match result {
                Ok(sessions) => {
                    let existing = lists.projects.get(&path).cloned();
                    lists.projects.insert(path.clone(), merge_project_sessions(existing, sessions));
                    lists.counts.insert(path, 10);
                }
                Err(error) => return vec![ListEffect::Error(format!("Failed to load project sessions: {}", error))],
            }
        }
        ListEvent::Scrollbar(path, visible) => { lists.scrollbars.insert(path, visible); }
        ListEvent::More(path) => {
            let count = lists.counts.get(&path).copied().unwrap_or(10);
            lists.counts.insert(path, count + 5);
        }
        ListEvent::Load(path) => {
            if lists.loading.contains(&path) { return vec![]; }
            lists.loading.insert(path.clone());
            return vec![ListEffect::Load(path)];
        }
        ListEvent::Hover(path) => {
            if lists.projects.contains_key(&path) || lists.loading.contains(&path) { return vec![]; }
            lists.loading.insert(path.clone());
            return vec![ListEffect::Load(path)];
        }
    }
    vec![]
}

/// Update only project metadata; chat/task-board capability changes stay with their owners.
fn apply_project_metadata(recent: &mut [super::edit_meta::RecentProjectMeta], path: &str,
    info: &project_types::Info) -> bool {
    let icon = info.icon.as_ref().and_then(|x| x.override_icon.clone()).filter(|x| !x.trim().is_empty());
    let icon_color = info.icon.as_ref().and_then(|x| x.color.clone()).filter(|x| !x.trim().is_empty());
    let start_command = info.commands.as_ref().and_then(|x| x.start.clone()).filter(|x| !x.trim().is_empty());
    if let Some(meta) = recent.iter_mut().find(|m| m.path == path)
        && (icon.is_some() || icon_color.is_some() || start_command.is_some()) {
        if icon.is_some() { meta.icon = icon; }
        if icon_color.is_some() { meta.icon_color = icon_color; }
        if start_command.is_some() { meta.worktree_start_command = start_command; }
        return true;
    }
    false
}

pub enum Creation { Ignore, Draft, Picker }
pub fn creation(creating: bool, worktree_enabled: bool) -> Creation {
    if creating { Creation::Ignore } else if worktree_enabled { Creation::Picker } else { Creation::Draft }
}

pub struct ProjectCompletion {
    pub id: String,
    pub updated_at: Option<u64>,
    pub selected_branch: Option<String>,
    pub is_git_repo: bool,
    pub metadata_changed: bool,
}
pub fn complete_project(current_project: Option<&str>, loaded_path: &str,
    recent: &mut [super::edit_meta::RecentProjectMeta], info: project_types::Info,
    selected_branch: Option<String>) -> Option<ProjectCompletion> {
    if current_project != Some(loaded_path) { return None; }
    let metadata_changed = apply_project_metadata(recent, loaded_path, &info);
    Some(ProjectCompletion {
        updated_at: if info.time.updated > 0 { Some(info.time.updated) } else { None },
        is_git_repo: matches!(info.vcs, Some(project_types::Vcs::Git)),
        id: info.id, selected_branch, metadata_changed,
    })
}
