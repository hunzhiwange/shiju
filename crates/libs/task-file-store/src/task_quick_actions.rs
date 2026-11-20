//! 项目任务快捷操作目录，独立于任务调度存储。

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskQuickAction {
    pub id: u64,
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    archived: bool,
}

pub fn list(project: &Path) -> io::Result<Vec<TaskQuickAction>> {
    let Some(directory) = directory(project, false)? else {
        return Ok(Vec::new());
    };
    Ok(read(&directory.join("catalog.json"))?
        .into_iter()
        .filter(|action| !action.archived)
        .collect())
}

pub fn save(
    project: &Path,
    previous: Option<&TaskQuickAction>,
    name: &str,
    prompt: &str,
) -> io::Result<Vec<TaskQuickAction>> {
    let (name, prompt) = (name.trim(), prompt.trim());
    if name.is_empty() || prompt.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty name or prompt"));
    }
    update(project, |actions| {
        if let Some(previous) = previous {
            let action = current_action(actions, previous)?;
            action.name = name.to_owned();
            action.prompt = prompt.to_owned();
        } else {
            let id = actions.iter().map(|action| action.id).max().unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| io::Error::other("quick action ID exhausted"))?;
            actions.push(TaskQuickAction {
                id,
                name: name.to_owned(),
                prompt: prompt.to_owned(),
                archived: false,
            });
        }
        Ok(())
    })
}

pub fn archive(project: &Path, previous: &TaskQuickAction) -> io::Result<Vec<TaskQuickAction>> {
    update(project, |actions| {
        current_action(actions, previous)?.archived = true;
        Ok(())
    })
}

fn current_action<'a>(
    actions: &'a mut [TaskQuickAction],
    previous: &TaskQuickAction,
) -> io::Result<&'a mut TaskQuickAction> {
    actions.iter_mut().find(|action| !action.archived && &**action == previous)
        .ok_or_else(|| io::Error::other("quick action changed or was archived"))
}

fn directory(project: &Path, create: bool) -> io::Result<Option<PathBuf>> {
    if !project.is_absolute() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected project root"));
    }
    let mut path = project.canonicalize()?;
    if !path.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotADirectory, "expected project root"));
    }
    for name in [config_runtime_paths::HOME_CONFIG_DIR_NAME, "task-quick-actions"] {
        path.push(name);
        if create
            && let Err(error) = fs::create_dir(&path)
            && error.kind() != io::ErrorKind::AlreadyExists
        {
            return Err(error);
        }
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected local directory")),
            Err(error) if !create && error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        }
    }
    Ok(Some(path))
}

fn read(path: &Path) -> io::Result<Vec<TaskQuickAction>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected local catalog")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    }
    Ok(serde_json::from_reader(fs::File::open(path)?)?)
}

fn update(
    project: &Path,
    change: impl FnOnce(&mut Vec<TaskQuickAction>) -> io::Result<()>,
) -> io::Result<Vec<TaskQuickAction>> {
    let directory = directory(project, true)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "quick action directory missing"))?;
    let path = directory.join("catalog.json");
    let lock = OpenOptions::new().create(true).truncate(false).write(true)
        .open(directory.join("catalog.lock"))?;
    lock.lock()?;
    let mut actions = read(&path)?;
    change(&mut actions)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer_pretty(temporary.as_file_mut(), &actions)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    actions.retain(|action| !action.archived);
    Ok(actions)
}
