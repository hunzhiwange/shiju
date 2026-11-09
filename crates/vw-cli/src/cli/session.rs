//! CLI 工作区状态与项目信息。

use crate::app::agent::shell::git_std_command;
use std::fmt::Write;
use std::path::Path;

/// CLI 版本号，从 Cargo 构建元数据中自动提取
///
/// 该常量用于在项目信息显示中标识当前 VibeWindow 版本，
/// 通过 `env!("CARGO_PKG_VERSION")` 宏在编译时注入。
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 构建项目信息字符串，用于 CLI 显示
///
/// 该函数收集工作树的基本信息，包括路径、当前 Git 分支和 VibeWindow 版本，
/// 生成适合在 CLI 标题栏或状态行显示的字符串。
///
/// # 参数
///
/// * `worktree` - Git 工作树的根目录路径
///
/// # 返回值
///
/// 返回格式化的项目信息字符串，格式为：
/// - 有分支时：`{path}:{branch} • VibeWindow {version}`
/// - 无分支时：`{path} • VibeWindow {version}`
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
///
/// let info = build_project_info(Path::new("/projects/myapp"));
/// // 可能返回: "/projects/myapp:main • ShiJu 1.0.0"
/// ```
pub(crate) fn build_project_info(worktree: &Path) -> String {
    let base = worktree.display().to_string();
    let branch = current_branch(worktree);

    // 根据是否存在分支信息构建基础字符串
    let mut out = if let Some(branch) = branch { format!("{base}:{branch}") } else { base };

    // 追加 VibeWindow 版本标识
    // write! 宏返回 Result，但此处忽略错误（字符串写入不应失败）
    let _ = write!(out, " • ShiJu {CLI_VERSION}");

    out
}

/// CLI/TUI 消费的 Git 工作区状态三态。
///
/// - `ReadyClean`：Git 可用且当前没有变更
/// - `ReadyDirty`：Git 可用且存在变更文件
/// - `Unavailable`：当前目录不是 Git 仓库，或 Git 查询失败
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum GitWorkspaceStatus {
    ReadyClean,
    ReadyDirty(Vec<String>),
    #[default]
    Unavailable,
}

impl GitWorkspaceStatus {
    pub(crate) fn modified_files(&self) -> &[String] {
        match self {
            Self::ReadyDirty(files) => files.as_slice(),
            Self::ReadyClean | Self::Unavailable => &[],
        }
    }
}

/// 收集工作树中已修改的文件列表
///
/// 该函数查询指定 Git 工作树的状态，返回所有被修改的文件路径列表，
/// 并按字母顺序排序以保证输出的一致性。
///
/// # 参数
///
/// * `worktree` - Git 工作树的根目录路径
///
/// # 返回值
///
/// 返回已修改文件的相对路径列表（相对于工作树根目录），
/// 按字母顺序升序排列。如果没有修改或查询失败，返回空列表。
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
///
/// let modified = collect_modified_files(Path::new("/projects/myapp"));
/// // 可能返回: ["Cargo.toml", "src/main.rs", "src/lib.rs"]
/// ```
pub(crate) fn collect_modified_files(worktree: &Path) -> Vec<String> {
    collect_git_workspace_status(worktree).modified_files().to_vec()
}

/// 收集当前工作区的 Git 状态语义。
///
/// 该函数显式区分 clean / dirty / unavailable，避免上层继续把“空列表”同时解释为
/// “没有变更”和“Git 不可用”。
pub(crate) fn collect_git_workspace_status(worktree: &Path) -> GitWorkspaceStatus {
    let output = git_std_command()
        .args(["-c", "core.quotepath=false", "status", "--porcelain", "--untracked-files=all"])
        .current_dir(worktree)
        .output();

    let Ok(out) = output else {
        return GitWorkspaceStatus::Unavailable;
    };

    if !out.status.success() {
        return GitWorkspaceStatus::Unavailable;
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut files = Vec::new();
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let Some(path) = parse_git_status_porcelain_path(line) else {
            return GitWorkspaceStatus::Unavailable;
        };
        files.push(path);
    }

    files.sort();
    if files.is_empty() {
        GitWorkspaceStatus::ReadyClean
    } else {
        GitWorkspaceStatus::ReadyDirty(files)
    }
}

pub(super) fn parse_git_status_porcelain_path(line: &str) -> Option<String> {
    let path = line.get(3..)?.trim();
    if path.is_empty() {
        return None;
    }

    Some(path.split_once(" -> ").map(|(_, renamed_path)| renamed_path).unwrap_or(path).to_string())
}

/// 获取当前 Git 分支名称
///
/// 该函数通过执行 `git rev-parse --abbrev-ref HEAD` 命令
/// 查询指定工作树当前检出的分支名称。
///
/// # 参数
///
/// * `worktree` - Git 工作树的根目录路径
///
/// # 返回值
///
/// - 成功时返回 `Some(branch_name)`
/// - 以下情况返回 `None`：
///   - 命令执行失败（如不在 Git 仓库中）
///   - 命令返回非零退出码
///   - 输出为空（如 detached HEAD 状态且无分支名）
///   - 输出包含非 UTF-8 字符
///
/// # 实现细节
///
/// - 使用 `Stdio::piped()` 捕获标准输出
/// - 使用 `Stdio::null()` 丢弃标准错误（避免干扰）
/// - 对输出进行 UTF-8 验证和空白修剪
///
/// # 安全性
///
/// 该函数不执行任何危险操作，仅进行只读查询。
/// Git 命令失败时会优雅地返回 `None`，不会 panic。
fn current_branch(worktree: &Path) -> Option<String> {
    // 执行 git rev-parse 命令查询当前分支
    let out = git_std_command()
        .current_dir(worktree)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    // 检查命令是否成功执行
    if !out.status.success() {
        return None;
    }

    // 解码输出并修剪空白字符
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();

    // 如果结果为空（如 detached HEAD），返回 None
    if s.is_empty() { None } else { Some(s) }
}
