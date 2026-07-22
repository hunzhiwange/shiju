//! CLI 会话管理模块
//!
//! 本模块提供 CLI 环境下的会话创建、标题管理和项目信息收集功能。
//! 作为 VibeWindow 代理运行时的一部分，该模块负责处理命令行界面与
//! 会话系统的交互，包括会话初始化、标题生成以及工作树状态查询。
//!
//! # 主要功能
//!
//! - **会话创建**：在指定项目目录中创建新的 CLI 会话
//! - **标题管理**：基于用户输入生成和刷新会话标题
//! - **项目信息**：收集当前 Git 分支、版本信息等元数据
//! - **文件状态**：查询工作树中被修改的文件列表
//!
//! # 架构位置
//!
//! 该模块位于 `agent/loop_/cli/` 层级，作为主循环与 CLI 前端之间的桥梁，
//! 依赖于会话管理（`session`）、项目实例（`project::instance`）和文件系统
//! 工具（`file`）等底层模块。

use crate::app::agent::shell::git_std_command;
use std::fmt::Write;
use std::path::Path;

/// CLI 版本号，从 Cargo 构建元数据中自动提取
///
/// 该常量用于在项目信息显示中标识当前 VibeWindow 版本，
/// 通过 `env!("CARGO_PKG_VERSION")` 宏在编译时注入。
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 在指定项目目录中创建新的 CLI 会话
///
/// 该函数尝试在给定的项目工作树中创建一个新会话。如果创建失败，
/// 则返回一个基于降序 ID 生成的回退标识符。
///
/// # 参数
///
/// * `project_dir` - 项目工作树的根目录路径
/// * `title` - 可选的会话标题；如果为 `None`，则由会话系统分配默认标题
///
/// # 返回值
///
/// 返回新创建会话的 ID 字符串。如果会话创建成功，返回实际会话 ID；
/// 如果创建过程中出现错误，返回格式为 `{prefix}_cli` 的回退 ID。
///
/// # 异步行为
///
/// 该函数是异步的，需要通过 `.await` 调用。内部会通过项目实例提供器
/// 确保正确的上下文环境，并执行会话创建逻辑。
///
/// # 错误处理
///
/// - 项目实例提供失败时，使用降序 ID 回退
/// - 会话创建失败时，同样使用降序 ID 回退
/// - 回退 ID 生成失败时，硬编码为 `"cli"`
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
///
/// let project_path = Path::new("/path/to/project");
/// let session_id = create_cli_session(project_path, Some("My Session".to_string())).await;
/// println!("Created session: {}", session_id);
/// ```
pub(crate) async fn create_cli_session(project_dir: &Path, title: Option<String>) -> String {
    let directory = project_dir.to_string_lossy().to_string();

    // 准备回退 ID：使用降序生成器创建，失败时硬编码为 "cli"
    let fallback = crate::app::agent::id::descending(crate::app::agent::id::Prefix::Session, None)
        .unwrap_or_else(|_| "cli".to_string());

    // 通过项目实例提供器创建会话
    // 该模式确保在正确的项目上下文中执行会话创建逻辑
    let created =
        crate::app::agent::project::instance::provide(project_dir.to_path_buf(), None, move || {
            let directory = directory.clone();
            let title = title.clone();
            Box::pin(async move {
                crate::app::agent::session::session::create_next(
                    crate::app::agent::session::session::CreateInput {
                        parent_id: None,
                        title,
                        directory,
                        permission: None,
                    },
                )
                .await
            })
        })
        .await;

    // 根据创建结果返回会话 ID 或回退值
    // Ok(Ok(info)) 表示项目实例和会话创建都成功
    // 其他情况（实例失败或会话失败）均使用回退 ID
    match created {
        Ok(Ok(info)) => info.id,
        _ => fallback,
    }
}

/// 从用户输入生成初始 CLI 会话标题
///
/// 该函数处理用户输入字符串，规范化空白字符并截断至适当长度，
/// 生成适合作为会话初始标题的字符串。
///
/// # 参数
///
/// * `input` - 用户的原始输入字符串，可能包含多余的空白字符
///
/// # 返回值
///
/// 返回处理后的标题字符串：
/// - 如果输入为空或仅包含空白，返回 `"CLI session"`
/// - 否则返回规范化且截断后的标题（最多 50 个字符）
/// - 如果截断发生，标题末尾添加 `"..."` 后缀
///
/// # 处理逻辑
///
/// 1. 将所有连续空白字符（空格、制表符、换行等）压缩为单个空格
/// 2. 去除首尾空白
/// 3. 如果结果为空，使用默认标题
/// 4. 否则取前 50 个字符，必要时添加省略号
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::agent::loop_::cli::session::initial_cli_session_title_for_input;
///
/// let title = initial_cli_session_title_for_input("  Help me   refactor the code  ");
/// assert_eq!(title, "Help me refactor the code");
///
/// let long_title = initial_cli_session_title_for_input("This is a very long user input that exceeds fifty characters limit");
/// assert!(long_title.ends_with("..."));
/// ```
pub(crate) fn initial_cli_session_title_for_input(input: &str) -> String {
    // 将所有连续的空白字符压缩为单个空格
    let normalized = input.split_whitespace().collect::<Vec<_>>().join(" ");

    // 去除首尾可能残留的空白
    let trimmed = normalized.trim();

    // 空输入返回默认标题
    if trimmed.is_empty() {
        return "CLI session".to_string();
    }

    // 截取前 50 个字符作为标题
    let mut chars = trimmed.chars();
    let title: String = chars.by_ref().take(50).collect();

    // 如果还有剩余字符，说明发生了截断，添加省略号
    // 注意：在添加省略号前先去除可能的单词边界空白
    if chars.next().is_some() { format!("{}...", title.trim_end()) } else { title }
}

/// 根据用户输入内容异步刷新会话标题
///
/// 该函数尝试基于用户的首次输入内容生成更具描述性的会话标题。
/// 如果会话当前使用空标题或默认标题，则先用简化标题占位，
/// 然后异步生成 AI 驱动的智能标题。
///
/// # 参数
///
/// * `session_id` - 目标会话的唯一标识符
/// * `first_user_content` - 用户在会话中的首次输入内容
/// * `preferred_model` - 可选的首选模型标识符，用于标题生成
///
/// # 异步行为
///
/// 该函数是异步的，内部执行两个阶段：
/// 1. **同步阶段**：如果会话标题为空或默认值，立即更新为基于输入的简化标题
/// 2. **异步阶段**：调用 AI 模型生成智能标题，成功后更新会话
///
/// # 错误处理
///
/// - 会话更新失败时静默忽略（使用 `let _ = ...`）
/// - 标题生成失败时提前返回，保留第一阶段设置的简化标题
/// - 该函数设计为"尽力而为"模式，不传播错误
///
/// # 设计考量
///
/// 该函数采用两阶段更新策略：
/// - 第一阶段确保用户立即看到有意义的标题，即使 AI 生成失败
/// - 第二阶段提供更智能的标题，提升会话可识别性
///
/// # 示例
///
/// ```ignore
/// maybe_refresh_cli_session_title(
///     "session_123",
///     "Help me implement a binary search tree",
///     Some("gpt-4".to_string()),
/// ).await;
/// ```
pub(crate) async fn maybe_refresh_cli_session_title(
    session_id: &str,
    first_user_content: &str,
    preferred_model: Option<String>,
) {
    // 生成基于输入的简化标题作为回退
    let fallback = initial_cli_session_title_for_input(first_user_content);
    let fallback_for_update = fallback.clone();

    // 第一阶段：如果当前标题为空或是默认值，立即更新为简化标题
    // 这确保用户在 AI 生成完成前就能看到有意义的标题
    let _ = crate::app::agent::session::session::update_any(session_id, move |s| {
        if s.title.trim().is_empty()
            || crate::app::agent::session::session::is_default_title(&s.title)
        {
            s.title = fallback_for_update;
        }
    })
    .await;

    // 第二阶段：异步生成 AI 驱动的智能标题
    let generated = crate::app::agent::session::title::generate_from_content(
        session_id.to_string(),
        first_user_content.to_string(),
        preferred_model,
        None,
    )
    .await;

    // 如果生成失败，保留第一阶段的简化标题
    let Ok(title) = generated else { return };

    // 生成成功，更新会话标题为 AI 生成的智能标题
    let _ = crate::app::agent::session::session::update_any(session_id, move |s| {
        s.title = title;
    })
    .await;
}

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
/// // 可能返回: "/projects/myapp:main • VibeWindow 1.0.0"
/// ```
pub(crate) fn build_project_info(worktree: &Path) -> String {
    let base = worktree.display().to_string();
    let branch = current_branch(worktree);

    // 根据是否存在分支信息构建基础字符串
    let mut out = if let Some(branch) = branch { format!("{base}:{branch}") } else { base };

    // 追加 VibeWindow 版本标识
    // write! 宏返回 Result，但此处忽略错误（字符串写入不应失败）
    let _ = write!(out, " • VibeWindow {CLI_VERSION}");

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
