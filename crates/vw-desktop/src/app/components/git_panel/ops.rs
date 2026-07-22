//! Git 操作模块
//!
//! 该模块提供 Git 仓库操作的封装函数，包括：
//! - 获取当前分支名称
//! - 列出所有本地分支
//! - 切换分支
//! - 在终端中打开仓库目录
//! - 获取工作区与 HEAD 之间的差异文件元数据
//! - 加载差异文件的新旧内容
//! - 获取已更改文件的路径列表
//!
//! # 平台支持
//!
//! 大多数功能仅在非 WebAssembly 目标平台上可用（`not(target_arch = "wasm32")`）。
//! 在 WebAssembly 平台上，相关函数会返回空值或错误。

use super::utils::FileStatus;
use crate::app::App;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;

/// 根据应用状态确定 Git 仓库路径
///
/// 优先使用活动会话的目录作为仓库路径，如果该目录不存在或不是有效的 Git 仓库，
/// 则回退到项目路径。
///
/// # 参数
///
/// - `app`: 应用状态引用，包含会话信息和项目路径
///
/// # 返回值
///
/// 返回有效的 Git 仓库路径字符串，如果没有可用的仓库则返回 `None`
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn git_repo_path_for_app(app: &App) -> Option<String> {
    if let Some(dir) = app.selected_git_worktree_directory.as_ref() {
        let dir = dir.trim();
        if !dir.is_empty() {
            let dir_path = std::path::Path::new(dir);
            if dir_path.exists() && git2::Repository::open(dir).is_ok() {
                return Some(dir.to_string());
            }
        }
        return None;
    }

    if let Some(active_id) = app.active_session_id.as_ref()
        && let Some(info) = app.sessions.iter().find(|s| &s.id == active_id)
    {
        let dir = info.directory.trim();
        if !dir.is_empty() {
            let dir_path = std::path::Path::new(dir);
            // 确保目录存在且是有效的 Git 仓库
            if dir_path.exists() && git2::Repository::open(dir).is_ok() {
                return Some(dir.to_string());
            }
        }
    }
    // 回退到项目路径
    app.project_path.clone()
}

/// 差异文件的元数据（非 WebAssembly 平台）
///
/// 包含文件的变更状态、大小、行数统计等信息。
/// 此版本包含 Git 对象 ID（`old_oid`），用于从仓库中检索旧文件内容。
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct DiffFileMeta {
    /// 文件路径（相对于仓库根目录）
    pub path: String,
    /// 文件变更状态（新增、修改、删除等）
    pub status: FileStatus,
    /// 旧文件的 Git 对象 ID（如果存在）
    pub old_oid: Option<git2::Oid>,
    /// 旧文件大小（字节）
    pub old_size: u64,
    /// 新文件大小（字节）
    pub new_size: u64,
    /// 新文件是否存在于工作区
    pub new_exists: bool,
    /// 新增行数
    pub insertions: usize,
    /// 删除行数
    pub deletions: usize,
}

/// 差异文件的元数据（WebAssembly 平台）
///
/// WebAssembly 版本的简化元数据结构，不包含 Git 对象 ID。
/// 在 Web 平台上，部分 Git 功能不可用。
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone)]
pub struct DiffFileMeta {
    /// 文件路径（相对于仓库根目录）
    pub path: String,
    /// 文件变更状态（新增、修改、删除等）
    pub status: FileStatus,
    /// 旧文件大小（字节）
    pub old_size: u64,
    /// 新文件大小（字节）
    pub new_size: u64,
    /// 新文件是否存在于工作区
    pub new_exists: bool,
    /// 新增行数
    pub insertions: usize,
    /// 删除行数
    pub deletions: usize,
}

/// 获取当前 Git 分支名称（非 WebAssembly 平台）
///
/// 打开指定路径的 Git 仓库，读取 HEAD 引用以确定当前分支。
///
/// # 参数
///
/// - `path`: Git 仓库的文件系统路径
///
/// # 返回值
///
/// - `Some(String)`: 当前分支名称（不包含 `refs/heads/` 前缀）
/// - `None`: 如果处于分离 HEAD 状态或发生错误
///
/// # 示例
///
/// ```ignore
/// let branch = current_branch("/path/to/repo");
/// println!("当前分支: {:?}", branch);
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn current_branch(path: &str) -> Option<String> {
    let repo = git2::Repository::open(path).ok()?;
    let head = repo.head().ok()?;
    let name = head.name().ok()?;
    // 去除 "refs/heads/" 前缀以获取简短分支名
    let short = name.strip_prefix("refs/heads/").unwrap_or(name);
    // 如果是 "HEAD" 则表示分离 HEAD 状态
    if short == "HEAD" { None } else { Some(short.to_string()) }
}

/// 获取当前 Git 分支名称（WebAssembly 平台）
///
/// 在 Web 平台上不支持此功能，始终返回 `None`。
#[cfg(target_arch = "wasm32")]
pub fn current_branch(_path: &str) -> Option<String> {
    None
}

/// 列出所有本地分支名称（非 WebAssembly 平台）
///
/// 打开指定路径的 Git 仓库，枚举所有本地分支。
///
/// # 参数
///
/// - `path`: Git 仓库的文件系统路径
///
/// # 返回值
///
/// - `Ok(Vec<String>)`: 本地分支名称列表
/// - `Err(git2::Error)`: 如果无法打开仓库或读取分支
///
/// # 示例
///
/// ```ignore
/// match list_branches("/path/to/repo") {
///     Ok(branches) => println!("分支列表: {:?}", branches),
///     Err(e) => eprintln!("错误: {}", e),
/// }
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn list_branches(path: &str) -> Result<Vec<String>, git2::Error> {
    let repo = git2::Repository::open(path)?;
    let mut names = Vec::new();
    // 只枚举本地分支
    let branches = repo.branches(Some(git2::BranchType::Local))?;
    for (branch, _) in branches.flatten() {
        if let Some(name) = branch.name()? {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

/// 列出所有本地分支名称（WebAssembly 平台）
///
/// 在 Web 平台上不支持此功能，始终返回空列表。
#[cfg(target_arch = "wasm32")]
pub fn list_branches(_path: &str) -> Result<Vec<String>, String> {
    Ok(vec![])
}

/// 切换到指定分支（非 WebAssembly 平台）
///
/// 执行 Git checkout 操作，将工作区切换到指定的本地分支。
///
/// # 参数
///
/// - `path`: Git 仓库的文件系统路径
/// - `branch`: 目标分支名称（不含 `refs/heads/` 前缀）
///
/// # 返回值
///
/// - `Ok(())`: 切换成功
/// - `Err(git2::Error)`: 如果分支不存在或切换失败
///
/// # 注意
///
/// 此函数不会处理未提交的更改，如有未提交更改可能导致切换失败。
#[cfg(not(target_arch = "wasm32"))]
pub fn checkout_branch(path: &str, branch: &str) -> Result<(), git2::Error> {
    let repo = git2::Repository::open(path)?;
    let refname = format!("refs/heads/{}", branch);
    // 解析分支引用为 Git 对象
    let obj = repo.revparse_single(&refname)?;
    // 将工作区切换到该对象指向的树
    repo.checkout_tree(&obj, None)?;
    // 更新 HEAD 指向新分支
    repo.set_head(&refname)?;
    Ok(())
}

/// 切换到指定分支（WebAssembly 平台）
///
/// 在 Web 平台上不支持此功能，始终返回错误。
#[cfg(target_arch = "wasm32")]
pub fn checkout_branch(_path: &str, _branch: &str) -> Result<(), String> {
    Err("Not supported on Web".to_string())
}

/// 在终端中打开指定目录（非 WebAssembly 平台）
///
/// 仅支持 macOS，使用 `open -a Terminal` 命令在 Terminal.app 中打开目录。
///
/// # 参数
///
/// - `path`: 要在终端中打开的目录路径
///
/// # 返回值
///
/// - `Ok(())`: 打开成功
/// - `Err(std::io::Error)`: 如果命令执行失败
#[cfg(not(target_arch = "wasm32"))]
pub fn open_terminal(path: &str) -> Result<(), std::io::Error> {
    Command::new("open").args(["-a", "Terminal", path]).status()?;
    Ok(())
}

/// 在终端中打开指定目录（WebAssembly 平台）
///
/// 在 Web 平台上不支持此功能，始终返回错误。
#[cfg(target_arch = "wasm32")]
pub fn open_terminal(_path: &str) -> Result<(), String> {
    Err("Not supported on Web".to_string())
}

/// 获取工作区与 HEAD 之间的差异文件元数据（非 WebAssembly 平台）
///
/// 计算当前工作区与 HEAD 提交之间的差异，返回所有已更改文件的元数据。
/// 包括新增、修改、删除、重命名和未跟踪的文件。
///
/// # 参数
///
/// - `app`: 应用状态引用，用于确定仓库路径
///
/// # 返回值
///
/// 返回差异文件元数据向量，如果没有可用的仓库或发生错误则返回空向量。
///
/// # 实现细节
///
/// 1. 确定仓库路径（优先使用活动会话目录）
/// 2. 打开仓库并获取 HEAD 提交的树
/// 3. 计算树到工作区（含索引）的差异
/// 4. 遍历差异统计每个文件的新增/删除行数
/// 5. 为每个变更的文件构建完整的元数据
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn get_diff_file_metas(app: &App) -> Vec<DiffFileMeta> {
    // 确定仓库路径
    let Some(path) = git_repo_path_for_app(app) else {
        return vec![];
    };
    get_diff_file_metas_for_repo_path(&path)
}

/// 获取指定 Git 仓库路径的差异文件元数据（非 WebAssembly 平台）
///
/// 直接基于仓库根目录路径计算工作区与 HEAD 之间的差异文件元数据，
/// 适合在后台任务中执行，避免把整个应用状态传入阻塞线程。
#[cfg(not(target_arch = "wasm32"))]
pub fn get_diff_file_metas_for_repo_path(path: &str) -> Vec<DiffFileMeta> {
    let Ok(repo) = git2::Repository::open(path) else {
        return vec![];
    };
    // 获取 HEAD 引用
    let Ok(head) = repo.head() else {
        return vec![];
    };
    // 获取 HEAD 指向的树对象
    let Ok(tree) = head.peel_to_tree() else {
        return vec![];
    };

    // 配置差异选项：包含未跟踪的文件
    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true);
    // 合并 index 信息，面板才能区分已暂存新增和未跟踪文件。
    let Ok(diff) = repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts)) else {
        return vec![];
    };

    // 收集每个文件的行级统计（新增和删除行数）
    let mut line_stats: std::collections::HashMap<String, (usize, usize)> =
        std::collections::HashMap::new();
    let _ = diff.foreach(
        &mut |_, _| true, // 文件回调（跳过）
        None,             // 二进制回调
        None,             // hunk 回调
        Some(&mut |delta, _hunk, line| {
            // 确定文件路径（优先使用新文件路径，回退到旧文件路径）
            let old_file = delta.old_file();
            let new_file = delta.new_file();
            let path_str: String = new_file
                .path()
                .or_else(|| old_file.path())
                .and_then(|p| p.to_str())
                .unwrap_or("unknown")
                .to_string();
            // 更新行统计
            let entry = line_stats.entry(path_str).or_insert((0, 0));
            match line.origin() {
                '+' => entry.0 += 1, // 新增行
                '-' => entry.1 += 1, // 删除行
                _ => {}
            }
            true
        }),
    );

    let mut results: Vec<DiffFileMeta> = Vec::new();

    // 遍历所有差异条目，构建元数据
    for delta in diff.deltas() {
        let old_file = delta.old_file();
        let new_file = delta.new_file();

        // 将 Git 变更状态映射到应用内部的文件状态枚举
        let status = match delta.status() {
            git2::Delta::Modified => FileStatus::Modified,
            git2::Delta::Added => FileStatus::Added,
            git2::Delta::Deleted => FileStatus::Deleted,
            git2::Delta::Renamed => FileStatus::Renamed,
            git2::Delta::Untracked => FileStatus::Untracked,
            _ => FileStatus::Unknown,
        };

        // 确定文件路径
        let path_str: String = new_file
            .path()
            .or_else(|| old_file.path())
            .and_then(|p| p.to_str())
            .unwrap_or("unknown")
            .to_string();

        // 获取旧文件的信息（如果存在）
        let (old_oid, old_size) = if old_file.exists() {
            let oid = old_file.id();
            let size = repo.find_blob(oid).map(|b| b.size() as u64).unwrap_or(0);
            (Some(oid), size)
        } else {
            (None, 0)
        };

        // 获取新文件的信息（如果存在）
        let (new_exists, new_size) =
            if new_file.exists() { (true, new_file.size()) } else { (false, 0) };

        // 获取行级统计（新增/删除行数）
        let (mut insertions, mut deletions) =
            line_stats.get(&path_str).copied().unwrap_or((0usize, 0usize));

        // 对于新增或未跟踪的文件，如果行统计为 0，则直接计算总行数
        if matches!(status, FileStatus::Added | FileStatus::Untracked) && insertions == 0 {
            let full_path = std::path::Path::new(path).join(&path_str);
            let content = std::fs::read_to_string(full_path).unwrap_or_default();
            insertions = content.lines().count();
            deletions = 0;
        }

        // 对于删除的文件，如果行统计为 0，则从仓库中读取旧内容计算行数
        if matches!(status, FileStatus::Deleted)
            && deletions == 0
            && let Some(oid) = old_oid
        {
            let content = repo
                .find_blob(oid)
                .map(|b| String::from_utf8_lossy(b.content()).to_string())
                .unwrap_or_default();
            deletions = content.lines().count();
            insertions = 0;
        }

        results.push(DiffFileMeta {
            path: path_str,
            status,
            old_oid,
            old_size,
            new_size,
            new_exists,
            insertions,
            deletions,
        });
    }

    results
}

/// 加载指定差异文件的新旧内容（非 WebAssembly 平台）
#[cfg(not(target_arch = "wasm32"))]
pub fn load_diff_content_for_repo_path(repo_path: &str, meta: &DiffFileMeta) -> (String, String) {
    let Ok(repo) = git2::Repository::open(repo_path) else {
        return (String::new(), String::new());
    };

    let old_content = if let Some(oid) = meta.old_oid {
        repo.find_blob(oid)
            .map(|b| String::from_utf8_lossy(b.content()).to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let new_content = if meta.new_exists && !matches!(meta.status, FileStatus::Deleted) {
        let full_path = std::path::Path::new(repo_path).join(&meta.path);
        std::fs::read_to_string(full_path).unwrap_or_default()
    } else {
        String::new()
    };

    (old_content, new_content)
}

/// 获取仓库中已更改文件的路径列表（非 WebAssembly 平台）
///
/// 扫描仓库状态，返回所有已更改（包括索引和工作区变更）以及未跟踪的文件路径。
///
/// # 参数
///
/// - `repo_path`: Git 仓库的文件系统路径
///
/// # 返回值
///
/// 返回已更改文件的相对路径列表（已排序），如果没有可用的仓库则返回空列表。
///
/// # 过滤规则
///
/// 排除以下文件和目录：
/// - `.git/` 目录下的文件
/// - 路径中包含 `.git`、`node_modules` 或 `target` 的文件
/// - `.DS_Store` 文件
#[cfg(not(target_arch = "wasm32"))]
pub fn get_changed_file_paths(repo_path: &str) -> Vec<String> {
    let Ok(repo) = git2::Repository::open(repo_path) else {
        return vec![];
    };
    let mut out: Vec<String> = Vec::new();

    // 配置状态选项：包含未跟踪文件和目录
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_unmodified(false)
        .exclude_submodules(true);

    let Ok(statuses) = repo.statuses(Some(&mut opts)) else {
        return vec![];
    };

    for entry in statuses.iter() {
        let status = entry.status();

        // 检查是否有我们关心的变更类型
        let is_changed = status.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE
                | git2::Status::WT_NEW
                | git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_RENAMED
                | git2::Status::WT_TYPECHANGE,
        );
        if !is_changed {
            continue;
        }

        let Ok(path) = entry.path() else {
            continue;
        };

        // 排除 .git 目录下的文件
        if path.starts_with(".git/") {
            continue;
        }

        // 排除特定目录和文件
        if path.split('/').any(|p| p == ".git" || p == "node_modules" || p == "target") {
            continue;
        }
        if path.ends_with("/.DS_Store") || path == ".DS_Store" {
            continue;
        }

        // 避免重复添加
        if !out.iter().any(|x| x == path) {
            out.push(path.to_string());
        }
    }

    out.sort();
    out
}

/// 获取差异文件元数据（WebAssembly 平台）
///
/// 在 Web 平台上不支持此功能，始终返回空列表。
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub fn get_diff_file_metas(_app: &App) -> Vec<DiffFileMeta> {
    vec![]
}

/// 加载差异文件内容（WebAssembly 平台）
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub fn load_diff_content_for_repo_path(_repo_path: &str, _meta: &DiffFileMeta) -> (String, String) {
    (String::new(), String::new())
}

/// 获取已更改文件路径列表（WebAssembly 平台）
///
/// 在 Web 平台上不支持此功能，始终返回空列表。
#[cfg(target_arch = "wasm32")]
pub fn get_changed_file_paths(_repo_path: &str) -> Vec<String> {
    vec![]
}
