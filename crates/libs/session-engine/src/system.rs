//! # 系统提示与环境信息模块
//!
//! 本模块负责生成代理会话的系统提示和环境信息。
//!
//! ## 主要功能
//!
//! - 为所有模型提供统一的系统提示模板
//! - 生成包含工作目录、平台信息、日期等的环境信息文本
//! - 检测 Git 仓库状态
//!
//! ## 模块结构
//!
//! - `PROMPT_UNIFIED`：统一系统提示模板
//! - `provider()`：返回统一提示
//! - `environment_from_ref()`：生成环境信息
//!
//! ## 使用示例
//!
//! ```ignore
//! let prompt = system::instructions();
//! let prompts = system::provider(&model);
//! let env = system::environment_from_ref(Some("openai/gpt-4"), None).await;
//! ```

use agent_project::instance;
use agent_provider::provider;
use agent_log as log;
// Capability sets are handled by session-engine::system_prompt.
use std::path::Path;
use time::format_description;

/// 统一的会话系统提示模板
///
/// 所有模型共用；指令优先级为：用户本轮要求 > 项目协议（含 AGENTS.md）> 本文件默认指引。

pub use crate::system_prompt::{UNIFIED_PROMPT_MARKER, RuntimePromptModes};
pub fn render_unified_prompt_with_modes(
    model_id: &str,
    working_directory: &str,
    available_tool_ids: &[String],
    runtime_modes: RuntimePromptModes,
    recursive_policy: &str,
) -> String { crate::system_prompt::render_unified_prompt_with_modes(model_id, working_directory, available_tool_ids, runtime_modes, recursive_policy) }
pub fn render_unified_prompt(
    model_id: &str,
    working_directory: &str,
    available_tool_ids: &[String],
    recursive_policy: &str,
) -> String { crate::system_prompt::render_unified_prompt(model_id, working_directory, available_tool_ids, recursive_policy) }
pub fn render_unified_prompt_from_ref(
    model: Option<&str>,
    root: Option<&str>,
    available_tool_ids: &[String],
    recursive_policy: &str,
) -> String { crate::system_prompt::render_unified_prompt_from_ref(model, root, available_tool_ids, recursive_policy) }
pub fn render_unified_prompt_for_model(
    model: &provider::Model,
    root: Option<&str>,
    available_tool_ids: &[String],
    recursive_policy: &str,
) -> String {
    render_unified_prompt(
        &format!("{}/{}", model.provider_id, model.api.id),
        &resolve_cwd(root),
        available_tool_ids,
        recursive_policy,
    )
}

/// 获取默认的系统指令
///
/// 返回统一系统提示，作为会话的基础指令集
///
/// # 返回值
///
/// 返回格式化后的系统指令字符串
///
/// # 示例
///
/// ```ignore
/// let instructions = system::instructions();
/// println!("系统指令: {}", instructions);
/// ```
pub fn instructions( recursive_policy: &str,
) -> String {
    render_unified_prompt_from_ref(None, None, &[], recursive_policy)
}

/// 返回统一的系统提示
///
/// 不再按模型 ID 分流；任意模型均使用同一份 `PROMPT_UNIFIED`。
///
/// # 参数
///
/// - `model`: 模型信息引用，包含模型 ID 和提供方信息（仅用于日志）
///
/// # 返回值
///
/// 返回包含统一系统提示的向量
///
/// # 示例
///
/// ```ignore
/// let model = provider::get_model("openai", "gpt-4").await?;
/// let prompts = system::provider(&model);
/// ```
pub fn provider_with_context(
    model: &provider::Model,
    root: Option<&str>,
    available_tool_ids: &[String],
    recursive_policy: &str,
) -> Vec<String> {
    let id = model.api.id.as_str();

    // 记录系统提示选择日志，用于调试和审计
    log::create(None)
        .tag("modelID", id)
        .tag("providerID", &model.provider_id)
        .debug("system_prompt::provider called", None);

    vec![render_unified_prompt_for_model(model, root, available_tool_ids, recursive_policy)]
}

pub fn provider(model: &provider::Model,
    recursive_policy: &str,
) -> Vec<String> {
    provider_with_context(model, None, &[], recursive_policy)
}

/// Converts a Boolean value to "Yes" or "No".
///
/// Used to provide a user-friendly English representation in generated environment information.
///
/// # 参数
///
/// - `v`: Boolean value to convert
///
/// # 返回值
///
/// - `true` → `"Yes"`
/// - `false` → `"No"`
pub fn yes_no_en(v: bool) -> &'static str {
    if v { "Yes" } else { "No" }
}

/// 获取当前平台标识符
///
/// 返回标准化的平台名称字符串，用于环境信息展示
///
/// # 返回值
///
/// - macOS → `"darwin"`
/// - Windows → `"win32"`
/// - 其他系统 → 使用标准 OS 名称（如 `"linux"`）
///
/// # 示例
///
/// ```ignore
/// let platform = platform();
/// println!("当前平台: {}", platform); // 例如: "darwin"
/// ```
pub fn platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "win32"
    } else {
        std::env::consts::OS
    }
}

/// 获取今日日期的格式化字符串
///
/// 返回当前 UTC 日期的格式化字符串，格式为：
/// `周几 月份 日期 年份`（例如: `Thu Mar 19 2026`）
///
/// # 返回值
///
/// 返回格式化后的日期字符串，如果格式化失败则返回 ISO 日期格式
///
/// # 示例
///
/// ```ignore
/// let date = today_date_string();
/// println!("今日: {}", date); // 例如: "Thu Mar 19 2026"
/// ```
pub fn today_date_string() -> String {
    let now = time::OffsetDateTime::now_utc();

    // 定义日期格式：简写周几、简写月份、日期、年份
    let fmt = format_description::parse_borrowed::<2>(
        "[weekday repr:short] [month repr:short] [day] [year]",
    )
    .unwrap_or_default();

    // 格式化日期，失败时回退到 ISO 日期格式
    now.format(&fmt).unwrap_or_else(|_| now.date().to_string())
}

/// 生成环境信息文本
///
/// 创建包含模型信息、工作目录、Git 状态、平台和日期的
/// 结构化环境信息字符串，用于注入到系统提示中
///
/// # 参数
///
/// - `model_name`: 模型名称
/// - `provider_id`: 提供方 ID
/// - `cwd`: 当前工作目录路径
/// - `is_git`: 是否为 Git 仓库
/// - `dirs`: 目录信息字符串
///
/// # 返回值
///
/// 返回格式化的环境信息文本，结构如下：
/// ```text
/// You are using the <model_name> model. Exact model ID: <provider_id>/<model_name>
/// Here is some information about the runtime environment:
/// <env>
///   Working directory: <cwd>
///   Is Git repository: <Yes/No>
///   Platform: <platform>
///   Today's date: <date>
/// </env>
/// <directories>
///   <dirs>
/// </directories>
/// ```
pub fn environment_text(
    model_name: &str,
    provider_id: &str,
    cwd: &str,
    is_git: bool,
    dirs: &str,
) -> String {
    [
        format!(
            "You are using the {} model. Exact model ID: {}/{}",
            model_name, provider_id, model_name
        ),
        "Here is some information about the runtime environment:".to_string(),
        "<env>".to_string(),
        format!("  Working directory: {}", cwd),
        format!("  Is Git repository: {}", yes_no_en(is_git)),
        format!("  Platform: {}", platform()),
        format!("  Today's date: {}", today_date_string()),
        "</env>".to_string(),
        "<directories>".to_string(),
        format!("  {}", dirs),
        "</directories>".to_string(),
    ]
    .join("\n")
}

/// 解析当前工作目录
///
/// 按以下优先级确定工作目录：
/// 1. 传入的 `root` 参数（如果非空）
/// 2. 项目实例配置的目录
/// 3. 系统当前目录
///
/// # 参数
///
/// - `root`: 可选的根目录路径，优先级最高
///
/// # 返回值
///
/// 返回解析后的工作目录路径字符串
///
/// # 示例
///
/// ```ignore
/// let cwd = resolve_cwd(Some("/path/to/project"));
/// let cwd = resolve_cwd(None); // 使用项目配置或系统当前目录
/// ```
pub use crate::system_prompt::resolve_cwd;

/// 从目录检测是否为 Git 仓库
///
/// 从指定目录开始，向上遍历父目录，查找 `.git` 目录
///
/// # 参数
///
/// - `dir`: 起始目录路径
///
/// # 返回值
///
/// - `true`: 找到 `.git` 目录
/// - `false`: 未找到或目录为空
///
/// # 算法
///
/// 从起始目录开始，检查是否存在 `.git` 子目录。
/// 如果不存在，则移动到父目录继续检查，直到到达文件系统根目录
pub fn detect_git_from_dir(dir: &str) -> bool {
    let mut cur = Path::new(dir.trim());

    // 空路径直接返回 false
    if cur.as_os_str().is_empty() {
        return false;
    }

    // 向上遍历查找 .git 目录
    loop {
        if cur.join(".git").exists() {
            return true;
        }
        match cur.parent() {
            Some(parent) => cur = parent,
            None => break, // 到达文件系统根目录，停止搜索
        }
    }
    false
}

/// 解析是否为 Git 仓库
///
/// 综合项目配置和文件系统检测结果判断
///
/// # 参数
///
/// - `cwd`: 当前工作目录路径
///
/// # 返回值
///
/// - `true`: 项目配置为 Git 或文件系统检测到 Git 仓库
/// - `false`: 否则
///
/// # 检测逻辑
///
/// 1. 首先检查项目配置中是否明确标记为 Git VCS
/// 2. 如果配置未明确，则通过文件系统检测 `.git` 目录
pub fn resolve_is_git(cwd: &str) -> bool {
    // 检查项目配置中的 VCS 设置
    instance::project()
        .as_ref()
        .and_then(|p| p.vcs.clone())
        .is_some_and(|v| v == agent_project::Vcs::Git)
        // 如果配置未明确，则通过文件系统检测
        || detect_git_from_dir(cwd)
}

/// 从模型引用生成环境信息
///
/// 根据提供的模型标识符和可选的根目录，生成完整的环境信息
///
/// # 参数
///
/// - `model`: 可选的模型标识符（格式: `provider_id/model_id`）
/// - `root`: 可选的项目根目录路径
///
/// # 返回值
///
/// 返回包含环境信息字符串的向量
///
/// # 处理流程
///
/// 1. 解析工作目录
/// 2. 检测 Git 仓库状态
/// 3. 解析模型信息（如果提供）
/// 4. 查询模型详细信息
/// 5. 生成环境信息文本
///
/// # 示例
///
/// ```ignore
/// let env = environment_from_ref(Some("openai/gpt-4"), Some("/project")).await;
/// let env = environment_from_ref(None, None).await;  // 使用默认配置
/// ```
pub async fn environment_from_ref(model: Option<&str>, root: Option<&str>) -> String {
    // 解析工作目录
    let cwd = resolve_cwd(root);

    // 检测 Git 仓库状态
    let is_git = resolve_is_git(&cwd);

    // 目录信息（当前为空，预留扩展）
    let dirs = String::new();

    // 如果未提供模型标识符，返回基础环境信息
    let Some(model) = model else {
        return environment_text("unknown", "unknown", &cwd, is_git, &dirs);
    };

    // 解析模型标识符（格式: provider_id/model_id）
    let parsed = provider::parse_model(model);

    // 如果解析失败，使用原始字符串作为模型名
    if parsed.provider_id.is_empty() || parsed.model_id.is_empty() {
        return environment_text(model, "unknown", &cwd, is_git, &dirs);
    }

    // 尝试获取模型详细信息
    match provider::get_model(&parsed.provider_id, &parsed.model_id).await {
        Ok(m) => {
            // 成功获取模型信息，使用完整的模型 ID
            environment_text(m.api.id.as_str(), m.provider_id.as_str(), &cwd, is_git, &dirs)
        }
        Err(_) => {
            // 获取失败，使用解析后的信息
            environment_text(
                parsed.model_id.as_str(),
                parsed.provider_id.as_str(),
                &cwd,
                is_git,
                &dirs,
            )
        }
    }
}


/// Capture team identity and scheduling capacity at the model request boundary.
pub async fn team_runtime_instructions<H: crate::session_io::SessionIo>(session_id: &str, enabled: bool) -> String {
    if !enabled {
        return String::new();
    }
    let parent =
        H::get(session_id).await.ok().and_then(|session| session.parent_id);
    let role = parent.as_deref().map_or_else(
        || "primary agent".to_string(),
        |id| format!("child agent; parent session: {id}"),
    );
    let remaining = crate::child_agent::coordinator::global_child_agent_coordinator()
        .team_capacity(session_id)
        .available_permits();
    format!(
        "## Current Team Runtime\nSession ID: {session_id}; role: {role}. Remaining shared child execution slots: {remaining} (snapshot; scheduling may change). The primary agent does not consume a child slot. Use list_agents for current team relationships. All agents share the working directory and must respect their own filtered tools and permissions."
    )
}
