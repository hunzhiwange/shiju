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

use crate::app::agent::project::instance;
use crate::app::agent::provider::provider;
use crate::app::agent::util::log;
use std::collections::BTreeSet;
use std::path::Path;
use time::format_description;

/// 统一的会话系统提示模板
///
/// 所有模型共用；指令优先级为：用户本轮要求 > 项目协议（含 AGENTS.md）> 本文件默认指引。
const PROMPT_UNIFIED: &str = include_str!("prompt/unified.txt");

pub(crate) const UNIFIED_PROMPT_MARKER: &str =
    "You are VibeWindow, a general-purpose autonomous assistant";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct RuntimePromptModes {
    pub(crate) goal: bool,
    pub(crate) workflow: bool,
}

impl RuntimePromptModes {
    pub(crate) fn from_options(options: &serde_json::Value) -> Self {
        Self {
            goal: options.get("goal_mode").and_then(serde_json::Value::as_bool).unwrap_or(false),
            workflow: options
                .get("workflow_mode")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        }
    }
}

fn has_tool(tool_ids: &BTreeSet<String>, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| tool_ids.contains(*candidate))
}

fn runtime_capability_instructions(
    available_tool_ids: &[String],
    rustcodegraph_skill_available: bool,
) -> String {
    let tool_ids = available_tool_ids
        .iter()
        .map(|tool| tool.trim().to_ascii_lowercase())
        .filter(|tool| !tool.is_empty())
        .collect::<BTreeSet<_>>();
    let mut sections = Vec::new();

    let mut file_rules = Vec::new();
    if has_tool(&tool_ids, &["file_read"]) {
        file_rules.push(
            "Use `file_read` to inspect text. If the result is truncated or paginated, continue reading the required content using offset/limit.",
        );
    }
    if has_tool(&tool_ids, &["glob"]) || has_tool(&tool_ids, &["grep"]) {
        let search_rule = match (has_tool(&tool_ids, &["glob"]), has_tool(&tool_ids, &["grep"])) {
            (true, true) => {
                "Use `glob` to find paths and `grep` to search contents. After finding matches, read the required context."
            }
            (true, false) => "Use `glob` to find files by path pattern.",
            (false, true) => {
                "Use `grep` to search file contents. After finding matches, read the required context."
            }
            (false, false) => unreachable!(),
        };
        file_rules.push(search_rule);
    }
    if has_tool(&tool_ids, &["apply_patch"]) {
        file_rules.push(
            "When modifying existing text, prefer `apply_patch` for focused patches and preserve unrelated changes.",
        );
    }
    if has_tool(&tool_ids, &["file_edit"]) {
        file_rules.push(
            "Read the file before invoking `file_edit`. By default, `old_string` must be unique; for multiple replacements, provide more specific context or explicitly set `replace_all`.",
        );
    }
    if has_tool(&tool_ids, &["file_write"]) {
        file_rules.push(
            "Use `file_write` only for new files or when a complete replacement is genuinely necessary. Read an existing file before overwriting it.",
        );
    }
    if !file_rules.is_empty() {
        sections.push(format!("## Files and Search\n- {}", file_rules.join("\n- ")));
    }

    if has_tool(&tool_ids, &["bash", "shell"]) {
        sections.push(
            "## Command Execution\n- Use `bash` for project CLIs, builds, tests, and Git. Check the exit code of every result; when a command fails, investigate the cause first and do not treat failed output as valid evidence.\n- Do not use shell commands such as `cat`, `find`, or `grep` to bypass available specialized file tools unless project protocols explicitly require it."
                .to_string(),
        );
    }

    let skill_available = has_tool(&tool_ids, &["skill"]);
    if skill_available {
        sections.push(
            "## Skills\n- Available skills are listed by name and description in a temporary `<system-reminder>`. When a skill matches the task or the user names one, invoke the Skill Tool first. After it succeeds, follow the injected skill instructions in full; do not guess paths or preload instructions."
                .to_string(),
        );
    }

    if skill_available && rustcodegraph_skill_available {
        sections.push(
            "## RustCodeGraph\n- The RustCodeGraph skill is available. For code navigation, first invoke the Skill Tool to load it and check the index as the skill requires.\n- When the index is available, prefer `explore`; use `query` / `node` for precise lookup, `callers` / `callees` / `impact` for call and impact analysis, and `affected` for test selection.\n- After a branch switch or bulk file change, or whenever status/query output reports pending, stale, or degraded index state, invoke the RustCodeGraph `sync` operation once and retry the original query. Normal saved edits are watched automatically, so do not sync before every query.\n- Do not immediately reread source context already obtained from the graph. Fall back to ordinary file search only for unindexed content, a stale index, or documents and configuration not covered by the graph."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["todoread"]) && has_tool(&tool_ids, &["todowrite"]) {
        sections.push(
            "## Task Tracking\n- For non-trivial tasks, first use `TodoWrite` to create a concise task list and keep its state synchronized with `TodoRead`. Do not create a task list for simple questions and answers.\n- Update each item immediately upon completion and continue. Do not claim the task is complete while unfinished items remain. When the user asks you to continue, resume from the next unfinished item."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["agenttool", "agent", "agent_tool", "delegate"]) {
        sections.push(
            "## Subagents\n- Delegate only concrete, independent, bounded subtasks. Do not delegate the entire original task or duplicate the parent task.\n- Independent subtasks may run concurrently in the background using `AgentTool`'s `launch` / `spawn`; use `run` only when the result is needed immediately. Record child-session IDs, continue local work, and `wait` for, verify, and integrate results before the final response. Use `stop` to end tasks that no longer provide value."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["process"]) {
        sections.push(
            "## Background Tasks\n- Run long-running commands in the background with `process` and record the task ID. Continue independent work in the meantime; do not busy-wait with `sleep`.\n- Before the final response, read the output of tasks that remain relevant, confirm their exit status, and terminate background tasks that no longer provide value. Running does not mean validation passed."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["batch"]) {
        sections.push(
            "## Parallel Tool Calls\n- Use `batch` only to combine calls that are independent, concurrency-safe, and do not contend for the same state. Run calls separately when they have sequential dependencies, approval requirements, or write conflicts."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["askuserquestion", "question"]) {
        sections.push(
            "## Asking the User Questions\n- Use the question tool only when a critical ambiguity, missing credentials, or a high-risk decision still blocks progress after you inspect the context. Complete all unblocked work first and ask only one specific question."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["tool_search"]) {
        sections.push(
            "## Tool Discovery\n- When you need a capability that is not currently shown, search for it with `tool_search` first. Search results remain filtered by the current model, client, and permissions."
                .to_string(),
        );
    }

    if has_tool(
        &tool_ids,
        &[
            "websearch",
            "web_search",
            "websearchtool",
            "webfetch",
            "web_fetch",
            "browser",
            "browseropen",
            "http_request",
        ],
    ) {
        sections.push(
            "## Network and Browser\n- Use the currently available search, fetch, or browser tools for current information, external pages, and user-requested verification. Distinguish search summaries from page evidence and cite relevant sources in the final response.\n- The browser does not implicitly provide page state. Obtain the current snapshot or interactive state before acting, and revalidate the target page after a refresh or reconstruction."
                .to_string(),
        );
    }

    if has_tool(&tool_ids, &["list_mcp_resources"])
        || has_tool(&tool_ids, &["list_mcp_resource_templates"])
    {
        sections.push(
            "## MCP Resources\n- When external systems provide structured resources or templates through MCP, first discover and read the corresponding resources. Do not substitute web search for authoritative context that is directly accessible."
                .to_string(),
        );
    }

    sections.join("\n\n")
}

fn runtime_mode_instructions(modes: RuntimePromptModes) -> String {
    let mut rules = Vec::new();
    if modes.goal {
        rules.push(
            "Goal mode is enabled: treat the original request as an ongoing goal. Continue until the completion criteria and necessary validation are satisfied; do not claim completion merely because a turn ended, part of the task succeeded, or you provided a verbal summary. Only external dependencies, permissions, or security risks that you cannot resolve yourself constitute a genuine blocker.",
        );
    }
    if modes.workflow {
        rules.push(
            "Workflow mode is enabled: the current session is part of a planning, batch-execution, or goal-acceptance workflow. Strictly complete the phase and task boundaries specified by the current prompt; do not skip dependency batches, take over other tasks, or change ownership of the outer plan.",
        );
    }
    if rules.is_empty() {
        String::new()
    } else {
        format!("## Current Runtime Mode\n- {}", rules.join("\n- "))
    }
}

pub(crate) fn render_unified_prompt_with_modes(
    model_id: &str,
    working_directory: &str,
    available_tool_ids: &[String],
    rustcodegraph_skill_available: bool,
    runtime_modes: RuntimePromptModes,
) -> String {
    let model_id = model_id.trim();
    let model_id = if model_id.is_empty() { "unknown" } else { model_id };
    let working_directory = working_directory.trim();
    let working_directory = if working_directory.is_empty() { "." } else { working_directory };
    PROMPT_UNIFIED
        .replace("{{model_id}}", model_id)
        .replace("{{working_directory}}", working_directory)
        .replace(
            "{{runtime_capabilities}}",
            &runtime_capability_instructions(available_tool_ids, rustcodegraph_skill_available),
        )
        .replace("{{runtime_modes}}", &runtime_mode_instructions(runtime_modes))
        .trim()
        .to_string()
}

pub(crate) fn render_unified_prompt(
    model_id: &str,
    working_directory: &str,
    available_tool_ids: &[String],
    rustcodegraph_skill_available: bool,
) -> String {
    render_unified_prompt_with_modes(
        model_id,
        working_directory,
        available_tool_ids,
        rustcodegraph_skill_available,
        RuntimePromptModes::default(),
    )
}

pub(crate) fn render_unified_prompt_from_ref(
    model: Option<&str>,
    root: Option<&str>,
    available_tool_ids: &[String],
    rustcodegraph_skill_available: bool,
) -> String {
    render_unified_prompt_from_ref_with_modes(
        model,
        root,
        available_tool_ids,
        rustcodegraph_skill_available,
        RuntimePromptModes::default(),
    )
}

pub(crate) fn render_unified_prompt_from_ref_with_modes(
    model: Option<&str>,
    root: Option<&str>,
    available_tool_ids: &[String],
    rustcodegraph_skill_available: bool,
    runtime_modes: RuntimePromptModes,
) -> String {
    render_unified_prompt_with_modes(
        model.unwrap_or("unknown"),
        &resolve_cwd(root),
        available_tool_ids,
        rustcodegraph_skill_available,
        runtime_modes,
    )
}

pub(crate) fn render_unified_prompt_for_model(
    model: &provider::Model,
    root: Option<&str>,
    available_tool_ids: &[String],
    rustcodegraph_skill_available: bool,
) -> String {
    render_unified_prompt(
        &format!("{}/{}", model.provider_id, model.api.id),
        &resolve_cwd(root),
        available_tool_ids,
        rustcodegraph_skill_available,
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
pub fn instructions() -> String {
    render_unified_prompt_from_ref(None, None, &[], false)
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
pub(crate) fn provider_with_context(
    model: &provider::Model,
    root: Option<&str>,
    available_tool_ids: &[String],
    rustcodegraph_skill_available: bool,
) -> Vec<String> {
    let id = model.api.id.as_str();

    // 记录系统提示选择日志，用于调试和审计
    log::create(None)
        .tag("modelID", id)
        .tag("providerID", &model.provider_id)
        .info("system_prompt::provider called", None);

    vec![render_unified_prompt_for_model(
        model,
        root,
        available_tool_ids,
        rustcodegraph_skill_available,
    )]
}

pub fn provider(model: &provider::Model) -> Vec<String> {
    provider_with_context(model, None, &[], false)
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
fn yes_no_en(v: bool) -> &'static str {
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
fn platform() -> &'static str {
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
fn today_date_string() -> String {
    let now = time::OffsetDateTime::now_utc();

    // 定义日期格式：简写周几、简写月份、日期、年份
    let fmt = format_description::parse("[weekday repr:short] [month repr:short] [day] [year]")
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
fn environment_text(
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
fn resolve_cwd(root: Option<&str>) -> String {
    // 优先使用传入的 root 参数（过滤空白字符串）
    if let Some(root) = root.filter(|s| !s.trim().is_empty()) {
        return root.trim().to_string();
    }

    // 尝试从项目实例获取目录
    let cwd = instance::directory();
    if !cwd.trim().is_empty() {
        return cwd;
    }

    // 回退到系统当前目录
    std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
}

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
fn detect_git_from_dir(dir: &str) -> bool {
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
fn resolve_is_git(cwd: &str) -> bool {
    // 检查项目配置中的 VCS 设置
    instance::project()
        .as_ref()
        .and_then(|p| p.vcs.clone())
        .is_some_and(|v| v == crate::app::agent::project::Vcs::Git)
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
#[cfg(test)]
#[path = "system_tests.rs"]
mod system_tests;
