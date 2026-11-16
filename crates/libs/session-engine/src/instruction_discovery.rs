//! Host configuration adapters for instruction discovery.
use runtime_flags as flag;
use agent_paths as global;
use agent_project::instance;
use agent_util::filesystem;
use std::path::{Path, PathBuf};
pub use session_instructions::instruction_discovery::*;
/// 使用当前会话配置发现系统级 AGENTS 来源。
pub fn system_sources() -> Vec<Source> {
    let managed_config = flag::vibewindow_config_dir().map(PathBuf::from);
    let current_directory = (!flag::vibewindow_disable_project_config())
        .then(|| filesystem::normalize_path(instance::directory()));
    let worktree = (!flag::vibewindow_disable_project_config()).then(|| {
        let worktree = instance::worktree();
        if worktree.is_empty() { None } else { Some(filesystem::normalize_path(worktree)) }
    });

    discover(&DiscoveryRoots {
        managed_config,
        user_config: Some(global::paths().config.clone()),
        current_directory,
        worktree: worktree.flatten(),
        project_config_enabled: !flag::vibewindow_disable_project_config(),
    })
}

/// 使用请求显式传入的工作区解析 AGENTS 来源。
///
/// 会话处理器运行在独立线程时，项目 task-local 可能已经离开原异步作用域；因此模型
/// 首步不能依赖 [`instance::directory`] 猜测当前项目。显式工作区同时作为当前目录和
/// 发现边界，确保启动请求稳定读取所选项目根目录的 AGENTS 文件族。
pub fn workspace_sources(workspace: &Path) -> Vec<Source> {
    let workspace = filesystem::normalize_path(workspace);
    discover(&DiscoveryRoots {
        managed_config: flag::vibewindow_config_dir().map(PathBuf::from),
        user_config: Some(global::paths().config.clone()),
        current_directory: Some(workspace.clone()),
        worktree: Some(workspace),
        project_config_enabled: !flag::vibewindow_disable_project_config(),
    })
}
