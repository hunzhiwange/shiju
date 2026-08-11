//! # 系统提示词构建模块
//!
//! 本模块负责为 VibeWindow 代理构建完整的系统提示词（System Prompt）。
//!
//! ## 核心功能
//!
//! - **模块化提示词构建**：通过 `PromptSection` trait 定义可插拔的提示词片段
//! - **上下文注入**：将工作区文件和工具定义等动态注入提示词
//! - **身份管理**：支持标准身份文件的加载与渲染
//!
//! ## 主要组件
//!
//! - [`PromptContext`] - 提示词构建上下文，包含所有必要信息
//! - [`SystemPromptBuilder`] - 系统提示词构建器，支持链式添加片段
//! - [`PromptSection`] - 提示词片段 trait，定义片段名称和构建逻辑
//!
//! ## 内置提示词片段
//!
//! | 片段 | 用途 |
//! |------|------|
//! | `ToolsSection` | 工具列表及参数定义 |
//! | `ActionSection` | 行为指导（原生工具/分发器模式） |
//! | `SafetySection` | 安全约束与最佳实践 |
//! | `WorkspaceSection` | 工作区目录结构与 Git 状态 |
//! | `IdentitySection` | 项目身份文件 |
//! | `DateTimeSection` | 当前日期时间 |
//! | `RuntimeSection` | 运行时环境信息（主机、操作系统、模型） |
//! | `ChannelMediaSection` | 通道媒体标记说明 |
//!
//! ## 使用示例
//!
//! ```ignore
//! use crate::app::agent::agent::prompt::{SystemPromptBuilder, PromptContext};
//!
//! let builder = SystemPromptBuilder::with_defaults();
//! let ctx = PromptContext {
//!     workspace_dir: Path::new("."),
//!     model_name: "gpt-4",
//!     tools: &tools,
//!     skills: &skills,
//!     skills_prompt_mode: SkillsPromptInjectionMode::Full,
//!     identity_config: None,
//!     dispatcher_instructions: "",
//!     timezone: chrono_tz::Asia::Shanghai,
//! };
//! let prompt = builder.build(&ctx)?;
//! ```

pub(crate) mod multi_agent;

use crate::app::agent::config::IdentityConfig;
use crate::app::agent::skills::Skill;
use crate::app::agent::tools::Tool;
use anyhow::Result;
use chrono_tz::Tz;
use std::fmt::Write;
use std::path::Path;

/// 单个工作区文件注入时的最大字符数限制
///
/// 超过此限制的文件内容将被截断，并添加提示信息引导代理使用 `read` 工具获取完整内容。
const BOOTSTRAP_MAX_CHARS: usize = 20_000;

/// 工作区身份文件列表
///
/// 这些文件定义了代理的身份、行为规范和上下文信息。
/// 系统会按顺序尝试加载这些文件并注入到系统提示词中。
///
/// # 文件用途说明
///
/// - `AGENTS.md` - 代理工程协议与编码规范
/// - `SOUL.md` - 代理核心价值观与行为准则
/// - `TOOLS.md` - 工具使用指南与最佳实践
/// - `IDENTITY.md` - 项目身份与角色定义
/// - `USER.md` - 用户偏好与定制化配置
/// - `HEARTBEAT.md` - 心跳状态与周期性任务
/// - `BOOTSTRAP.md` - 启动引导与初始化指令
/// - `MEMORY.md` - 长期记忆与知识存储
pub const WORKSPACE_IDENTITY_FILES: [&str; 8] = [
    "AGENTS.md",
    "SOUL.md",
    "TOOLS.md",
    "IDENTITY.md",
    "USER.md",
    "HEARTBEAT.md",
    "BOOTSTRAP.md",
    "MEMORY.md",
];

/// 提示词构建上下文
///
/// 包含构建系统提示词所需的所有信息，包括工作区路径、模型名称、
/// 可用工具、技能列表、身份配置等。
///
/// # 字段说明
///
/// - `workspace_dir` - 当前工作区目录路径
/// - `model_name` - 当前使用的模型名称（用于运行时信息展示）
/// - `tools` - 可用工具列表，将注入到工具部分
/// - `skills` - 可用技能列表，根据注入模式渲染
/// - `skills_prompt_mode` - 技能注入模式（完整/摘要/禁用）
/// - `identity_config` - 可选的身份配置
/// - `dispatcher_instructions` - 分发器指令（非空时切换行为模式）
pub struct PromptContext<'a> {
    /// 当前工作区目录路径
    pub workspace_dir: &'a Path,
    /// 当前使用的模型名称
    pub model_name: &'a str,
    /// 可用工具列表
    pub tools: &'a [Box<dyn Tool>],
    /// 可用技能列表
    pub skills: &'a [Skill],
    /// 技能注入模式
    pub skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode,
    /// 可选的身份配置
    pub identity_config: Option<&'a IdentityConfig>,
    /// 分发器指令（非空时启用分发器模式）
    pub dispatcher_instructions: &'a str,
    /// 已验证的聊天 IANA 时区
    pub timezone: Tz,
}

/// 提示词片段 trait
///
/// 定义系统提示词中可插拔片段的接口。每个片段负责生成特定类型的提示内容，
/// 如工具列表、安全规则、工作区信息等。
///
/// # 实现要求
///
/// 实现此 trait 的类型必须：
/// - 是线程安全的（`Send + Sync`）
/// - 提供 `name()` 方法返回片段标识符
/// - 提供 `build()` 方法根据上下文生成提示内容
///
/// # 示例
///
/// ```ignore
/// struct CustomSection;
///
/// impl PromptSection for CustomSection {
///     fn name(&self) -> &str {
///         "custom"
///     }
///
///     fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
///         Ok("## Custom Section\n\nCustom content here.".to_string())
///     }
/// }
/// ```
pub trait PromptSection: Send + Sync {
    /// 返回片段的标识符名称
    ///
    /// 用于日志记录和调试，应使用小写蛇形命名（如 `"tools"`、`"workspace"`）。
    fn name(&self) -> &str;

    /// 根据上下文构建提示词片段内容
    ///
    /// # 参数
    ///
    /// - `ctx` - 提示词构建上下文，包含所有必要信息
    ///
    /// # 返回值
    ///
    /// - `Ok(String)` - 成功生成的提示词内容
    /// - `Err(...)` - 构建过程中的错误
    ///
    /// # 说明
    ///
    /// 返回空字符串表示该片段在当前上下文中不需要生成内容，
    /// `SystemPromptBuilder` 会自动跳过空片段。
    fn build(&self, ctx: &PromptContext<'_>) -> Result<String>;
}

/// 系统提示词构建器
///
/// 提供流畅的 API 用于组装多个提示词片段，生成完整的系统提示词。
///
/// # 使用方式
///
/// 1. 使用 [`with_defaults()`](Self::with_defaults) 创建包含所有默认片段的构建器
/// 2. 使用 [`add_section()`](Self::add_section) 添加自定义片段
/// 3. 使用 [`build()`](Self::build) 生成最终提示词
///
/// # 默认片段顺序
///
/// 1. `ToolsSection` - 工具定义
/// 2. `ActionSection` - 行为指导
/// 3. `SafetySection` - 安全规则
/// 4. `WorkspaceSection` - 工作区信息
/// 5. `IdentitySection` - 身份文件
/// 6. `DateTimeSection` - 日期时间
/// 7. `RuntimeSection` - 运行时信息
/// 8. `ChannelMediaSection` - 通道媒体标记说明
#[derive(Default)]
pub struct SystemPromptBuilder {
    /// 提示词片段列表，按顺序构建
    sections: Vec<Box<dyn PromptSection>>,
}

impl SystemPromptBuilder {
    /// 创建包含所有默认片段的构建器
    ///
    /// 返回一个预配置了所有内置片段的构建器实例，
    /// 适用于大多数标准场景。
    ///
    /// # 返回值
    ///
    /// 配置了默认片段的 `SystemPromptBuilder` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let builder = SystemPromptBuilder::with_defaults();
    /// let prompt = builder.build(&ctx)?;
    /// ```
    pub fn with_defaults() -> Self {
        Self {
            sections: vec![
                Box::new(ToolsSection),
                Box::new(ActionSection),
                Box::new(SafetySection),
                Box::new(WorkspaceSection),
                Box::new(IdentitySection),
                Box::new(DateTimeSection),
                Box::new(RuntimeSection),
                Box::new(ChannelMediaSection),
            ],
        }
    }

    /// 添加自定义提示词片段
    ///
    /// 支持链式调用，可在默认片段基础上添加额外的自定义内容。
    ///
    /// # 参数
    ///
    /// - `section` - 实现 `PromptSection` trait 的片段实例
    ///
    /// # 返回值
    ///
    /// 添加片段后的构建器实例（支持链式调用）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let builder = SystemPromptBuilder::with_defaults()
    ///     .add_section(Box::new(MyCustomSection));
    /// ```
    pub fn add_section(mut self, section: Box<dyn PromptSection>) -> Self {
        self.sections.push(section);
        self
    }

    /// 构建最终系统提示词
    ///
    /// 按顺序调用所有片段的 `build()` 方法，将非空结果拼接成完整的系统提示词。
    /// 片段之间使用双换行符分隔。
    ///
    /// # 参数
    ///
    /// - `ctx` - 提示词构建上下文
    ///
    /// # 返回值
    ///
    /// - `Ok(String)` - 完整的系统提示词
    /// - `Err(...)` - 任一片片构建失败时的错误
    ///
    /// # 处理逻辑
    ///
    /// 1. 遍历所有已注册的片段
    /// 2. 调用每个片段的 `build()` 方法
    /// 3. 跳过空内容片段（`trim().is_empty()`）
    /// 4. 移除片段末尾空白，添加双换行分隔符
    pub fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let mut output = String::new();
        for section in &self.sections {
            let part = section.build(ctx)?;
            // 跳过空片段
            if part.trim().is_empty() {
                continue;
            }
            // 移除末尾空白并添加双换行分隔
            output.push_str(part.trim_end());
            output.push_str("\n\n");
        }
        Ok(output)
    }
}

/// 身份片段 - 项目上下文与身份定义
///
/// 加载并渲染工作区身份文件。
pub struct IdentitySection;

/// 工具片段 - 可用工具列表与参数定义
///
/// 枚举所有可用工具及其名称、描述和参数 schema。
pub struct ToolsSection;

/// 行动片段 - 代理行为指导
///
/// 根据是否使用分发器模式生成不同的行为指令。
pub struct ActionSection;

/// 安全片段 - 安全约束与最佳实践
///
/// 定义代理必须遵守的安全规则和行为边界。
pub struct SafetySection;

/// 已废弃的技能片段占位符。
///
/// 技能元数据由临时元用户提醒装配，完整正文只由 Skill Tool 成功调用后注入。
pub struct SkillsSection;

/// 工作区片段 - 目录结构与 Git 状态
///
/// 展示当前工作区的目录内容、Git 仓库状态等信息。
pub struct WorkspaceSection;

/// 运行时片段 - 环境信息
///
/// 显示主机名、操作系统和当前使用的模型。
pub struct RuntimeSection;

/// 日期时间片段 - 当前时间信息
///
/// 提供当前的日期时间和时区信息。
pub struct DateTimeSection;

/// 通道媒体片段 - 媒体标记说明
///
/// 说明来自各通道的媒体消息格式。
pub struct ChannelMediaSection;

impl PromptSection for IdentitySection {
    fn name(&self) -> &str {
        "identity"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let mut prompt = String::from("## Project Context\n\n");
        let _ = ctx.identity_config;

        prompt.push_str(
            "The following workspace files define your identity, behavior, and context. \
             You MUST follow these instructions at all times.\n\n",
        );

        for file in WORKSPACE_IDENTITY_FILES {
            inject_workspace_file(&mut prompt, ctx.workspace_dir, file, BOOTSTRAP_MAX_CHARS, false);
        }

        Ok(prompt)
    }
}

impl PromptSection for ToolsSection {
    fn name(&self) -> &str {
        "tools"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let mut out = String::from("## Tools\n\n");

        // 无工具时返回空字符串，构建器会自动跳过
        if ctx.tools.is_empty() {
            return Ok(String::new());
        }

        out.push_str("You have access to the following tools:\n\n");

        // 遍历并格式化每个工具的信息
        for tool in ctx.tools {
            let spec = tool.spec();
            let _ = writeln!(
                out,
                "- **{}**: {}\n  Parameters: `{}`",
                spec.id, spec.description, spec.input_schema
            );
        }

        // 如果存在分发器指令，追加到工具部分
        if !ctx.dispatcher_instructions.is_empty() {
            out.push('\n');
            out.push_str(ctx.dispatcher_instructions);
        } else if out.ends_with('\n') {
            out.pop();
        }

        if out.ends_with('\n') {
            out.pop();
        }

        Ok(out)
    }
}

impl PromptSection for ActionSection {
    fn name(&self) -> &str {
        "action"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        // 根据是否使用分发器选择不同的行为指导
        let native_tools = ctx.dispatcher_instructions.is_empty();

        let mut prompt = if native_tools {
            // 原生工具模式：可执行任务优先行动，避免冗长分析和过程播报
            "## Execution Style\n\n\
             For actionable requests, act immediately. If a tool is needed and available, call it before giving an explanation.\n\
             Investigate only enough to choose and verify the next action. Do not narrate plans, rehearse the task, or weigh obvious alternatives.\n\
             Keep reasoning internal: provide decisions, essential rationale, and results, never chain-of-thought or lengthy deliberation.\n\
             Make reasonable assumptions for low-risk, reversible work. Ask one concise question only when missing information would materially change the result or the action is risky or irreversible.\n\
             For questions and follow-ups, answer directly from the conversation context; do not ask the user to repeat themselves.\n\
             After acting, report the outcome and essential verification concisely. Do not summarize this configuration or describe your capabilities."
                .to_string()
        } else {
            // 分发器模式：可执行任务优先发出工具标签，避免用计划代替行动
            "## Execution Style\n\n\
             For actionable requests, act immediately. If a tool is needed and available, emit the actual <tool_call> before any explanation.\n\
             Investigate only enough to choose and verify the next action. Do not narrate plans, rehearse the task, weigh obvious alternatives, or output instructions instead of acting.\n\
             Keep reasoning internal: provide decisions, essential rationale, and results, never chain-of-thought or lengthy deliberation.\n\
             Make reasonable assumptions for low-risk, reversible work. Ask one concise question only when missing information would materially change the result or the action is risky or irreversible.\n\
             For questions and follow-ups, answer directly from the conversation context; do not ask the user to repeat themselves.\n\
             After acting, report the outcome and essential verification concisely. Do not summarize this configuration or describe your capabilities."
                .to_string()
        };

        if ctx.tools.iter().any(|tool| tool.name().eq_ignore_ascii_case("AgentTool")) {
            prompt.push_str("\n\n");
            prompt.push_str(multi_agent::recursive_orchestration_policy());
        }

        Ok(prompt)
    }
}

impl PromptSection for SafetySection {
    fn name(&self) -> &str {
        "safety"
    }

    fn build(&self, _ctx: &PromptContext<'_>) -> Result<String> {
        // 安全规则：保护隐私、防止破坏性操作、保持透明
        Ok("## Safety\n\n- Do not exfiltrate private data.\n- Do not run destructive commands without asking.\n- Do not bypass oversight or approval mechanisms.\n- Prefer `trash` over `rm`.\n- When in doubt, ask before acting externally.".into())
    }
}

impl PromptSection for SkillsSection {
    fn name(&self) -> &str {
        "skills"
    }

    fn build(&self, _ctx: &PromptContext<'_>) -> Result<String> {
        Ok(String::new())
    }
}

impl PromptSection for WorkspaceSection {
    fn name(&self) -> &str {
        "workspace"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let cwd = ctx.workspace_dir;

        // 检测是否为 Git 仓库
        let is_git = is_git_repo(cwd);
        let git_status = if is_git { " (Git Repository)" } else { "" };

        // 构建工作区信息头部
        let mut out =
            format!("## Workspace\n\nWorking directory: `{}`{}\n\n", cwd.display(), git_status);

        out.push_str("Files in current directory:\n<directories>\n");

        // 读取并格式化目录内容
        if let Ok(entries) = std::fs::read_dir(cwd) {
            let mut items = Vec::new();

            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

                // 过滤隐藏文件（但保留 .github 目录）
                if name.starts_with('.') && name != ".github" {
                    continue;
                }

                // 目录添加尾部斜杠标记
                let is_dir = path.is_dir();
                let marker = if is_dir { "/" } else { "" };
                items.push(format!("  {}{}", name, marker));
            }

            // 按名称排序
            items.sort();

            // 最多显示 50 个条目，超出时显示省略提示
            for item in items.iter().take(50) {
                out.push_str(item);
                out.push('\n');
            }

            if items.len() > 50 {
                out.push_str("  ... (more files hidden)\n");
            }
        }

        out.push_str("</directories>");

        Ok(out)
    }
}

/// 检测指定路径是否为 Git 仓库
///
/// 检查当前目录或任意父目录是否存在 `.git` 文件夹。
///
/// # 参数
///
/// - `path` - 要检测的路径
///
/// # 返回值
///
/// - `true` - 路径位于 Git 仓库内
/// - `false` - 路径不在 Git 仓库内
fn is_git_repo(path: &std::path::Path) -> bool {
    // 检查当前目录或任一祖先目录是否包含 .git
    path.join(".git").exists() || path.ancestors().any(|p| p.join(".git").exists())
}

impl PromptSection for RuntimeSection {
    fn name(&self) -> &str {
        "runtime"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        // 获取主机名（原生平台）
        #[cfg(not(target_arch = "wasm32"))]
        let host =
            hostname::get().map_or_else(|_| "unknown".into(), |h| h.to_string_lossy().to_string());

        // WASM 平台使用固定标识
        #[cfg(target_arch = "wasm32")]
        let host = "wasm-client".to_string();

        // 格式化运行时信息
        Ok(format!(
            "## Runtime\n\nHost: {host} | OS: {} | Model: {}",
            std::env::consts::OS,
            ctx.model_name
        ))
    }
}

impl PromptSection for DateTimeSection {
    fn name(&self) -> &str {
        "datetime"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        let now = crate::app::agent::util::locale::configured_datetime(ctx.timezone);
        Ok(format!("## Current Date & Time\n\n{now}"))
    }
}

impl PromptSection for ChannelMediaSection {
    fn name(&self) -> &str {
        "channel_media"
    }

    fn build(&self, _ctx: &PromptContext<'_>) -> Result<String> {
        // 媒体标记说明：帮助代理正确理解和处理来自各通道的媒体消息
        Ok("## Channel Media Markers\n\n\
            Messages from channels may contain media markers:\n\
            - `[Voice] <text>` — The user sent a voice/audio message that has already been transcribed to text. Respond to the transcribed content directly.\n\
            - `[IMAGE:<path>]` — An image attachment, processed by the vision pipeline.\n\
            - `[Document: <name>] <path>` — A file attachment saved to the workspace."
            .into())
    }
}

/// 将单个工作区文件注入到提示词中
///
/// 读取指定文件内容并格式化追加到提示词字符串中。
/// 支持内容截断和缺失文件标记。
///
/// # 参数
///
/// - `prompt` - 目标提示词字符串（可变引用）
/// - `workspace_dir` - 工作区根目录
/// - `filename` - 要注入的文件名
/// - `max_chars` - 最大字符数限制，超出时截断
/// - `optional` - 是否为可选文件（可选文件缺失时不显示标记）
///
/// # 处理逻辑
///
/// 1. 尝试读取文件内容
/// 2. 空文件：直接跳过
/// 3. 超长文件：截断并添加提示信息
/// 4. 文件缺失：
///    - 可选文件：静默跳过
///    - 必需文件：添加 `[File not found: ...]` 标记
fn inject_workspace_file(
    prompt: &mut String,
    workspace_dir: &std::path::Path,
    filename: &str,
    max_chars: usize,
    optional: bool,
) {
    use std::fmt::Write;

    let path = workspace_dir.join(filename);

    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let trimmed = content.trim();

            // 跳过空文件
            if trimmed.is_empty() {
                return;
            }

            // 添加文件标题
            let _ = writeln!(prompt, "### {filename}\n");

            // 超长内容截断处理
            let truncated = if trimmed.len() > max_chars {
                crate::app::agent::util::truncate_with_ellipsis(trimmed, max_chars)
            } else {
                trimmed.to_string()
            };

            // 如果发生了截断，添加提示信息
            if truncated.len() < trimmed.len() {
                prompt.push_str(&truncated);
                let _ = writeln!(
                    prompt,
                    "\n\n[... truncated at {max_chars} chars — use `read` for full file]\n"
                );
            } else {
                // 未截断，直接添加原文
                prompt.push_str(trimmed);
                prompt.push_str("\n\n");
            }
        }
        Err(_) => {
            // 必需文件缺失时添加标记（与 OpenClaw 行为一致）
            if !optional {
                let _ = writeln!(prompt, "### {filename}\n\n[File not found: {filename}]\n");
            }
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
