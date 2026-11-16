//! Real persisted project fixtures for native MCP acceptance.
use std::path::Path;
use agent_database::GlobalDatabase;
use agent_project::{Info, TimeInfo, instance, store::ProjectStore};
use vw_api_transport::ProjectScope;
pub fn register_project(
    database: &GlobalDatabase,
    project: &ProjectScope,
    workspace: &Path,
) -> anyhow::Result<instance::Context> {
    let directory = workspace.display().to_string();
    let info = Info {
        id: project.project_id().as_ref().to_owned(),
        worktree: directory.clone(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    };
    ProjectStore::new(database.clone())?.create(project.tenant_scope(), &info)?;
    Ok(instance::Context { directory: directory.clone(), worktree: directory, project: info })
}
