//! Task-pool compatibility storage.
//!
//! Native callers use the global Agent Scheduler store. Only the WASM JSON
//! compatibility path remains here; native mutation entry points fail closed.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(target_arch = "wasm32")]
use time::OffsetDateTime;

use super::models::{Task, TaskIndex, TaskStatus};

fn get_task_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join(vw_config_types::paths::HOME_CONFIG_DIR_NAME).join("tasks")
}

fn get_task_log_dir(project_path: &str) -> PathBuf {
    get_task_dir(project_path).join("logs")
}

fn ensure_task_dir(project_path: &str) -> io::Result<()> {
    std::fs::create_dir_all(get_task_dir(project_path))
}

#[cfg(target_arch = "wasm32")]
fn get_task_file_path(project_path: &str, task_id: &str) -> PathBuf {
    get_task_dir(project_path).join(format!("{task_id}.json"))
}

#[cfg(target_arch = "wasm32")]
fn get_legacy_index_file_path(project_path: &str) -> PathBuf {
    get_task_dir(project_path).join("_index.json")
}

#[cfg(not(target_arch = "wasm32"))]
fn native_cutover_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "native task-pool persistence moved to the global Agent Scheduler store",
    )
}

#[cfg(target_arch = "wasm32")]
fn load_index_json(project_path: &str) -> TaskIndex {
    let path = get_legacy_index_file_path(project_path);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn save_index_json(project_path: &str, index: &TaskIndex) -> io::Result<()> {
    ensure_task_dir(project_path)?;
    let path = get_legacy_index_file_path(project_path);
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(index)?)?;
    std::fs::rename(temporary, path)
}

#[cfg(target_arch = "wasm32")]
fn parse_task_id_sequence(task_id: &str) -> Option<(&str, u32)> {
    let rest = task_id.strip_prefix('T')?;
    let (date, sequence) = rest.split_once('.')?;
    if date.len() != 8 || sequence.len() != 4 {
        return None;
    }
    Some((date, sequence.parse().ok()?))
}

#[cfg(target_arch = "wasm32")]
fn max_sequence_for_date(index: &TaskIndex, date: &str) -> u32 {
    let prefix = format!("T{date}.");
    index
        .tasks
        .keys()
        .filter_map(|task_id| task_id.strip_prefix(&prefix))
        .filter(|sequence| sequence.len() == 4)
        .filter_map(|sequence| sequence.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
}

/// Rebuilds the WASM JSON index. Native callers must use Agent Scheduler.
pub fn rebuild_index_from_task_files(project_path: &str) -> io::Result<TaskIndex> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = project_path;
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut tasks = Vec::new();
        let directory = get_task_dir(project_path);
        if directory.exists() {
            for entry in std::fs::read_dir(directory)? {
                let path = entry?.path();
                if path.extension().and_then(|extension| extension.to_str()) != Some("json")
                    || path.file_name().and_then(|name| name.to_str()) == Some("_index.json")
                {
                    continue;
                }
                let content = std::fs::read_to_string(path)?;
                tasks.push(serde_json::from_str::<Task>(&content)?);
            }
        }
        tasks.sort_by(|left, right| {
            left.status
                .to_string_key()
                .cmp(right.status.to_string_key())
                .then_with(|| left.order.cmp(&right.order))
                .then_with(|| left.created_at_ms.cmp(&right.created_at_ms))
                .then_with(|| left.id.cmp(&right.id))
        });
        let mut index = TaskIndex::new();
        for task in tasks {
            let status_key = task.status.to_string_key().to_string();
            index.tasks.insert(task.id.clone(), status_key.clone());
            index.order_by_status.entry(status_key).or_default().push(task.id.clone());
            if let Some((date, sequence)) = parse_task_id_sequence(&task.id) {
                let replace = index.last_task_date.as_deref().is_none_or(|current| {
                    current < date || (current == date && index.last_task_seq < sequence)
                });
                if replace {
                    index.last_task_date = Some(date.to_string());
                    index.last_task_seq = sequence;
                }
            }
        }
        save_index_json(project_path, &index)?;
        Ok(index)
    }
}

/// Loads the WASM JSON index. Native callers receive an empty compatibility view.
pub fn load_index(project_path: &str) -> TaskIndex {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = project_path;
        TaskIndex::new()
    }
    #[cfg(target_arch = "wasm32")]
    {
        load_index_json(project_path)
    }
}

/// Saves the WASM JSON index. Native callers fail closed.
pub fn save_index(project_path: &str, index: &TaskIndex) -> io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, index);
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        save_index_json(project_path, index)
    }
}

/// Loads a WASM JSON task. Native callers must use Agent Scheduler.
pub fn load_task(project_path: &str, task_id: &str) -> Option<Task> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, task_id);
        None
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::fs::read_to_string(get_task_file_path(project_path, task_id))
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
    }
}

/// Saves a WASM JSON task. Native callers fail closed.
pub fn save_task(project_path: &str, task: &Task) -> io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, task);
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        ensure_task_dir(project_path)?;
        std::fs::write(get_task_file_path(project_path, &task.id), serde_json::to_vec_pretty(task)?)
    }
}

/// Deletes a WASM JSON task. Native callers fail closed.
pub fn delete_task_file(project_path: &str, task_id: &str) -> io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, task_id);
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        match std::fs::remove_file(get_task_file_path(project_path, task_id)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

/// Loads all WASM JSON tasks. Native callers must use Agent Scheduler.
pub fn load_all_tasks(project_path: &str) -> Vec<Task> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = project_path;
        Vec::new()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let index = load_index_json(project_path);
        index
            .tasks
            .keys()
            .filter_map(|task_id| load_task(project_path, task_id))
            .filter(|task| !task.deleted)
            .collect()
    }
}

/// Groups WASM JSON tasks by status. Native callers receive an empty view.
pub fn load_tasks_by_status(project_path: &str) -> HashMap<TaskStatus, Vec<Task>> {
    let mut grouped =
        TaskStatus::all().into_iter().map(|status| (status, Vec::new())).collect::<HashMap<_, _>>();
    for task in load_all_tasks(project_path) {
        grouped.entry(task.status).or_default().push(task);
    }
    for tasks in grouped.values_mut() {
        tasks.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.order.cmp(&right.order))
                .then_with(|| left.created_at_ms.cmp(&right.created_at_ms))
        });
    }
    grouped
}

/// Creates a WASM JSON task. Native callers fail closed.
pub fn create_task(project_path: &str, task: Task) -> io::Result<Task> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, task);
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut task = task;
        let mut index = load_index_json(project_path);
        let now_ms = crate::time::now_ms();
        let seconds = i64::try_from(now_ms / 1_000).unwrap_or_default();
        let date_time =
            OffsetDateTime::from_unix_timestamp(seconds).unwrap_or(OffsetDateTime::UNIX_EPOCH);
        let month: u8 = date_time.month().into();
        let date = format!("{:04}{:02}{:02}", date_time.year(), month, date_time.day());
        let last_sequence = if index.last_task_date.as_deref() == Some(date.as_str()) {
            index.last_task_seq
        } else {
            0
        }
        .max(max_sequence_for_date(&index, &date));
        let sequence = last_sequence.saturating_add(1);
        task.id = format!("T{date}.{sequence:04}");
        task.order = index
            .order_by_status
            .get(task.status.to_string_key())
            .map_or(0, |tasks| tasks.len() as u32);
        index.last_task_date = Some(date);
        index.last_task_seq = sequence;
        let status_key = task.status.to_string_key().to_string();
        index.tasks.insert(task.id.clone(), status_key.clone());
        index.order_by_status.entry(status_key).or_default().push(task.id.clone());
        save_task(project_path, &task)?;
        save_index_json(project_path, &index)?;
        Ok(task)
    }
}

/// Updates a WASM JSON task status. Native callers fail closed.
pub fn update_task_status(
    project_path: &str,
    task_id: &str,
    new_status: TaskStatus,
) -> io::Result<Option<Task>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, task_id, new_status);
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let Some(mut task) = load_task(project_path, task_id) else {
            return Ok(None);
        };
        if task.status == new_status {
            return Ok(Some(task));
        }
        let mut index = load_index_json(project_path);
        let old_key = task.status.to_string_key().to_string();
        let new_key = new_status.to_string_key().to_string();
        if let Some(tasks) = index.order_by_status.get_mut(&old_key) {
            tasks.retain(|id| id != task_id);
        }
        task.order = index.order_by_status.get(&new_key).map_or(0, |tasks| tasks.len() as u32);
        task.set_status(new_status);
        index.tasks.insert(task_id.to_string(), new_key.clone());
        index.order_by_status.entry(new_key).or_default().push(task_id.to_string());
        save_task(project_path, &task)?;
        save_index_json(project_path, &index)?;
        Ok(Some(task))
    }
}

/// Saves task-board presentation settings, which are not scheduler facts.
pub fn save_task_board_settings(
    project_path: &str,
    settings: &super::models::TaskBoardSettings,
) -> io::Result<()> {
    ensure_task_dir(project_path)?;
    std::fs::write(
        get_task_dir(project_path).join("board_settings.json"),
        serde_json::to_vec_pretty(settings)?,
    )
}

/// Loads task-board presentation settings.
pub fn load_task_board_settings(project_path: &str) -> super::models::TaskBoardSettings {
    std::fs::read_to_string(get_task_dir(project_path).join("board_settings.json"))
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// Native raw task artifacts moved to Agent Scheduler.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_task_execution_result(
    project_path: &str,
    task_id: &str,
    payload: &serde_json::Value,
) -> io::Result<()> {
    let _ = (project_path, task_id, payload);
    Err(native_cutover_error())
}

/// Native raw task artifacts moved to Agent Scheduler.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_task_execution_result(
    project_path: &str,
    task_id: &str,
) -> io::Result<Option<serde_json::Value>> {
    let _ = (project_path, task_id);
    Err(native_cutover_error())
}

/// Native raw task artifacts moved to Agent Scheduler.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_task_code_review_result(
    project_path: &str,
    task_id: &str,
    payload: &serde_json::Value,
) -> io::Result<()> {
    let _ = (project_path, task_id, payload);
    Err(native_cutover_error())
}

/// Native raw task artifacts moved to Agent Scheduler.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_task_code_review_result(
    project_path: &str,
    task_id: &str,
) -> io::Result<Option<serde_json::Value>> {
    let _ = (project_path, task_id);
    Err(native_cutover_error())
}

pub fn get_task_logs_dir(project_path: &str) -> PathBuf {
    get_task_log_dir(project_path)
}

pub fn get_task_root_dir(project_path: &str) -> PathBuf {
    get_task_dir(project_path)
}

pub fn task_file_exists(project_path: &str, task_id: &str) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, task_id);
        false
    }
    #[cfg(target_arch = "wasm32")]
    {
        get_task_file_path(project_path, task_id).exists()
    }
}

pub fn sanitize_project_path(project_path: &str) -> &Path {
    Path::new(project_path)
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
