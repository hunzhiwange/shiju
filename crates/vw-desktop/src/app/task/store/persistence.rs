//! 负责旧版任务索引与任务文件的序列化、加载和原子写入。

use std::io;

use crate::app::task::models::{Task, TaskIndex};

#[cfg(not(target_arch = "wasm32"))]
use super::paths::native_cutover_error;
#[cfg(target_arch = "wasm32")]
use super::paths::{ensure_task_dir, get_legacy_index_file_path, get_task_dir, get_task_file_path};

#[cfg(target_arch = "wasm32")]
pub(super) fn max_sequence_for_date(index: &TaskIndex, date: &str) -> u32 {
    let prefix = format!("T{date}.");
    index
        .tasks
        .keys()
        .filter_map(|task_id| task_id.strip_prefix(&prefix))
        .filter(|sequence| sequence.len() == 4)
        .filter_map(|sequence| sequence.parse::<u32>().ok())
        .max()
        .unwrap_or_default()
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
pub(super) fn load_index_unlocked(project_path: &str) -> TaskIndex {
    std::fs::read_to_string(get_legacy_index_file_path(project_path))
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn save_index_unlocked(project_path: &str, index: &TaskIndex) -> io::Result<()> {
    ensure_task_dir(project_path)?;
    let path = get_legacy_index_file_path(project_path);
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(index)?)?;
    std::fs::rename(temporary, path)
}

/// 扫描有效任务文件重建索引，忽略无法解析的非任务文件。
pub fn rebuild_index_from_task_files(project_path: &str) -> io::Result<TaskIndex> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = project_path;
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let directory = get_task_dir(project_path);
        let mut tasks = Vec::new();
        if directory.exists() {
            for entry in std::fs::read_dir(directory)? {
                let path = entry?.path();
                if path.extension().and_then(|extension| extension.to_str()) != Some("json")
                    || path.file_name().and_then(|name| name.to_str()) == Some("_index.json")
                {
                    continue;
                }
                tasks.push(serde_json::from_str::<Task>(&std::fs::read_to_string(path)?)?);
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
        save_index_unlocked(project_path, &index)?;
        Ok(index)
    }
}

/// 加载旧版索引；缺失或损坏时按平台策略回退为空索引。
pub fn load_index(project_path: &str) -> TaskIndex {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = project_path;
        TaskIndex::new()
    }
    #[cfg(target_arch = "wasm32")]
    {
        load_index_unlocked(project_path)
    }
}

/// 保存旧版任务索引；原生端返回迁移后的不支持错误。
pub fn save_index(project_path: &str, index: &TaskIndex) -> io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, index);
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        save_index_unlocked(project_path, index)
    }
}

/// 加载指定任务，文件不存在或内容无效时返回 `None`。
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

/// 保存单个旧版任务；原生端拒绝写入项目内存储。
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

/// 加载索引引用的全部有效任务，并跳过已丢失或损坏的记录。
pub fn load_all_tasks(project_path: &str) -> Vec<Task> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = project_path;
        Vec::new()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut tasks = load_index_unlocked(project_path)
            .tasks
            .keys()
            .filter_map(|task_id| load_task(project_path, task_id))
            .filter(|task| !task.deleted)
            .collect::<Vec<_>>();
        tasks.sort_by(|left, right| {
            right
                .created_at_ms
                .cmp(&left.created_at_ms)
                .then_with(|| right.priority.cmp(&left.priority))
                .then_with(|| left.order.cmp(&right.order))
                .then_with(|| left.id.cmp(&right.id))
        });
        tasks
    }
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod persistence_tests;
