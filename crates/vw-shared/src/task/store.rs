//! 任务池兼容存储层。
//!
//! 原生调用方已迁移到全局 Agent Scheduler 存储；这里只保留 WASM JSON 兼容路径，
//! 原生变更入口默认拒绝，避免继续写入旧存储。

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

/// 从 WASM 任务 JSON 文件重建索引；原生调用方必须改用 Agent Scheduler。
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

/// 加载 WASM JSON 索引；原生调用方仅获得空的兼容视图。
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

/// 保存 WASM JSON 索引；原生调用会以“不支持”错误拒绝。
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

/// 加载单个 WASM JSON 任务；原生调用方仅获得不存在结果。
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

/// 保存单个 WASM JSON 任务；原生调用会以“不支持”错误拒绝。
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

/// 删除单个 WASM JSON 任务；目标不存在时视为成功，原生调用则默认拒绝。
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

/// 加载所有未删除的 WASM JSON 任务；原生调用方获得空集合。
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

/// 按状态分组并排序 WASM JSON 任务；原生调用方获得各状态均为空的视图。
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

/// 创建 WASM JSON 任务并分配当日递增编号；原生调用默认拒绝。
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

/// 更新 WASM JSON 任务状态及索引顺序；任务不存在时返回 `Ok(None)`，原生调用默认拒绝。
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

/// 保存任务看板展示设置；这些设置不属于调度器事实数据。
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

/// 加载任务看板展示设置；文件缺失或损坏时返回默认值。
pub fn load_task_board_settings(project_path: &str) -> super::models::TaskBoardSettings {
    std::fs::read_to_string(get_task_dir(project_path).join("board_settings.json"))
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// 原生任务执行结果已迁移到 Agent Scheduler，本兼容入口始终拒绝写入。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_task_execution_result(
    project_path: &str,
    task_id: &str,
    payload: &serde_json::Value,
) -> io::Result<()> {
    let _ = (project_path, task_id, payload);
    Err(native_cutover_error())
}

/// 原生任务执行结果已迁移到 Agent Scheduler，本兼容入口始终拒绝读取。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_task_execution_result(
    project_path: &str,
    task_id: &str,
) -> io::Result<Option<serde_json::Value>> {
    let _ = (project_path, task_id);
    Err(native_cutover_error())
}

/// 原生代码审查结果已迁移到 Agent Scheduler，本兼容入口始终拒绝写入。
#[cfg(not(target_arch = "wasm32"))]
pub fn save_task_code_review_result(
    project_path: &str,
    task_id: &str,
    payload: &serde_json::Value,
) -> io::Result<()> {
    let _ = (project_path, task_id, payload);
    Err(native_cutover_error())
}

/// 原生代码审查结果已迁移到 Agent Scheduler，本兼容入口始终拒绝读取。
#[cfg(not(target_arch = "wasm32"))]
pub fn load_task_code_review_result(
    project_path: &str,
    task_id: &str,
) -> io::Result<Option<serde_json::Value>> {
    let _ = (project_path, task_id);
    Err(native_cutover_error())
}

/// 返回旧任务池的日志目录，仅用于兼容路径定位。
pub fn get_task_logs_dir(project_path: &str) -> PathBuf {
    get_task_log_dir(project_path)
}

/// 返回旧任务池的根目录，仅用于兼容路径定位。
pub fn get_task_root_dir(project_path: &str) -> PathBuf {
    get_task_dir(project_path)
}

/// 检查 WASM JSON 任务文件是否存在；原生平台固定返回 `false`。
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

/// 将项目路径借用为 [`Path`]，不执行清理、规范化或权限放宽。
pub fn sanitize_project_path(project_path: &str) -> &Path {
    Path::new(project_path)
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
