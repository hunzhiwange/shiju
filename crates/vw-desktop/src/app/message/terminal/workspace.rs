use crate::app::{App, Message, Screen, TerminalState, project_dirs};
use desktop_terminal::workspace::{read_json, workspace_has_tabs, write_json};
use iced::Task;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn storage_dir() -> Option<PathBuf> {
    project_dirs().map(|dirs| dirs.data_local_dir().join("terminal-workspaces"))
}

fn project_file(storage: &Path, project: &str) -> PathBuf {
    let key = hex::encode(Sha256::digest(project.as_bytes()));
    storage.join(key).with_extension("json")
}

pub(crate) fn save(project: &str, terminal: &mut TerminalState) {
    let Some(storage) = storage_dir() else { return };
    if let Err(error) = terminal.save_workspace(&project_file(&storage, project)) {
        tracing::warn!(kind = ?error.kind(), "failed to save local terminal workspace");
    }
}

pub(crate) fn restore(project: &str, terminal: &mut TerminalState) -> bool {
    let Some(storage) = storage_dir() else { return false };
    match terminal.restore_workspace(&project_file(&storage, project), Path::new(project)) {
        Ok(restored) => restored,
        Err(error) => {
            tracing::warn!(kind = ?error.kind(), "failed to restore local terminal workspace");
            false
        }
    }
}

pub(crate) fn remember_project(project: Option<&str>) {
    let Some(storage) = storage_dir() else { return };
    let path = storage.join("active-project.json");
    let result = match project {
        Some(project) => write_json(&path, &project),
        None => match std::fs::remove_file(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            result => result,
        },
    };
    if let Err(error) = result {
        tracing::warn!(kind = ?error.kind(), "failed to save active terminal project");
    }
}

pub(crate) fn checkpoint(app: &mut App) {
    if let Some(project) = app.terminal_project_override.as_deref().or(app.navigation_tabs.project_path.as_deref()) {
        save(project, &mut app.terminal);
    }
}

pub(crate) fn flush(app: &mut App) {
    checkpoint(app);
    for (project, terminal) in &mut app.terminals_by_project {
        save(project, terminal);
    }
    app.cli_workbench.checkpoint_layout();
    app.app_terminal.checkpoint_layout();
    app.terminal_ssh.checkpoint_layout();
}

pub(crate) fn reopen_project(app: &mut App) -> Task<Message> {
    if app.shutdown_requested
        || app.navigation_tabs.screen != Screen::Home
        || app.navigation_tabs.project_path.is_some()
        || app.session_state.active_session_id.is_some()
    {
        return Task::none();
    }
    let Some(storage) = storage_dir() else { return Task::none() };
    let project = match read_json::<String>(&storage.join("active-project.json")) {
        Ok(Some(project)) => project,
        Ok(None) => return Task::none(),
        Err(error) => {
            tracing::warn!(kind = ?error.kind(), "failed to read active terminal project");
            return Task::none();
        }
    };
    let directory = Path::new(&project);
    if !directory.is_absolute() || !directory.is_dir() {
        return Task::none();
    }
    match workspace_has_tabs(&project_file(&storage, &project)) {
        Ok(true) => app.open_project_and_index(project),
        Ok(false) => Task::none(),
        Err(error) => {
            tracing::warn!(kind = ?error.kind(), "failed to read local terminal workspace");
            Task::none()
        }
    }
}
