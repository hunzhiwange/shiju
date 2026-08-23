//! 解析旧版任务存储路径，并为原生端迁移后的禁用路径返回明确错误。

use std::io;
#[cfg(any(target_arch = "wasm32", test))]
use std::path::PathBuf;

#[cfg(any(target_arch = "wasm32", test))]
/// 返回旧版项目内任务目录。
pub(super) fn get_task_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join(vw_config_types::paths::HOME_CONFIG_DIR_NAME).join("tasks")
}

#[cfg(any(target_arch = "wasm32", test))]
/// 返回指定旧版任务的 JSON 文件路径。
pub(super) fn get_task_file_path(project_path: &str, task_id: &str) -> PathBuf {
    get_task_dir(project_path).join(format!("{task_id}.json"))
}

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn get_legacy_index_file_path(project_path: &str) -> PathBuf {
    get_task_dir(project_path).join("_index.json")
}

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn get_task_log_dir(project_path: &str) -> PathBuf {
    get_task_dir(project_path).join("logs")
}

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn ensure_task_dir(project_path: &str) -> io::Result<()> {
    std::fs::create_dir_all(get_task_dir(project_path))
}

#[cfg(not(target_arch = "wasm32"))]
/// 说明原生桌面任务持久化已迁移到全局代理调度器。
pub(super) fn native_cutover_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "native desktop task persistence moved to the global Agent Scheduler",
    )
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod paths_tests;
