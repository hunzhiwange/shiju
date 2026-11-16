//! # 通道提示构建模块
//!
//! 本模块负责构建各种类型的系统提示，用于指导 AI 代理在不同通道环境下的行为。
//! 主要功能包括：
//!
//! - 构建特定通道的系统提示（针对不同消息通道如 CLI、Telegram、Discord 等）
//! - 构建运行时工具可见性提示（告知代理当前可用的工具列表）
//! - 构建记忆上下文（从长期记忆中检索相关信息）
//! - 构建完整的系统提示（包括工具、安全规则、技能、工作空间等）
//! - 加载工作空间引导文件（OpenClaw 格式的身份定义文件）
//!
//! ## 提示构建流程
//!
//! 系统提示的构建遵循 OpenClaw 框架结构：
//! 1. **工具列表** — 可用工具及其描述
//! 2. **安全规则** — 操作护栏和限制
//! 3. **技能** — 技能指令和工具元数据
//! 4. **工作空间** — 当前工作目录
//! 5. **身份文件** — AGENTS、SOUL、TOOLS、IDENTITY、USER、MEMORY
//! 6. **日期时间** — 当前时区信息（用于缓存稳定性）
//! 7. **运行时信息** — 主机名、操作系统、模型名称

use std::path::Path;
use model_types::ChatMessage;
use agent_skills as skills;
use tool_types::Tool;
use agent_loop::instructions::build_tool_instructions_from_specs;

/// Returns the model request passed to the session processor for a channel message.
///
/// Explicit channel selections use complete provider/model references. `auto` remains
/// a policy request so the processor selects once and reports the concrete model.
pub use channel_support::prompt::channel_model_request_model;

/// Formats safe Auto-selection metadata for channel diagnostics.
///
/// The decision deliberately has no prompt field; callers must not append request
/// content to this trace.
pub use channel_support::prompt::model_selection_trace;

/// Builds the transient protocol messages shared by external model entrypoints.
///
/// AGENTS is a system-role repository constraint placed after the user-role
/// skill directory. Neither message is persisted in conversation history.
/// Full skill bodies remain exclusive to successful Skill tool invocations.
pub fn build_entrypoint_protocol_messages(instructions: &[session_engine::instruction::Resolved], skills: &[skills::Skill], skill_tool_enabled: bool, context_window_tokens: usize) -> Vec<ChatMessage> {
    let injection = session_engine::context_injection::assemble_agents_context(instructions, &[]);
    let agents_messages = injection.transient_messages.into_iter().map(|message| match message.role {
        session_engine::context_injection::ContextRole::System => ChatMessage::system(message.content),
    }).collect();
    channel_support::prompt::build_entrypoint_protocol_messages(agents_messages, skills, skill_tool_enabled, context_window_tokens)
}

/// 构建通道特定的系统提示
///
/// 根据通道类型和配置，为基础提示添加通道相关的指令和上下文信息。
/// 不同通道可能需要不同的行为规范，例如 CLI 通道可以直接显示执行细节，
/// 而其他通道（如 Telegram、Discord）通常需要隐藏内部工具细节。
///
/// # 参数
///
/// * `base_prompt` - 基础系统提示内容
/// * `channel_name` - 通道名称（如 "cli"、"telegram"、"discord" 等）
/// * `reply_target` - 回复目标标识符（通常是用户或会话 ID）
/// * `expose_internal_tool_details` - 是否暴露内部工具执行细节
///
/// # 返回值
///
/// 返回构建完成的系统提示字符串，包含：
/// - 基础提示内容
/// - 通道投递指令（如果存在）
/// - 执行可见性规则（非 CLI 通道）
/// - 通道上下文信息（如果有回复目标）
///
/// # 示例
///
/// ```ignore
/// let prompt = build_channel_system_prompt(
///     "You are a helpful assistant.",
///     "telegram",
///     "user_123",
///     false,
/// );
/// ```
pub use channel_support::prompt::build_channel_system_prompt;

/// 构建运行时工具可见性提示
///
/// 根据当前运行时策略，生成一个明确的工具可用性列表，告知代理
/// 在当前轮次中可以调用哪些工具。这个列表是当前消息的权威策略快照。
///
/// # 参数
///
/// * `tools_registry` - 工具注册表（所有可用工具的集合）
/// * `excluded_tools` - 被运行时策略排除的工具名称列表
/// * `native_tools` - 是否使用原生工具调用（provider 的 function-calling）
///
/// # 返回值
///
/// 返回格式化的提示字符串，包含：
/// - 允许的工具列表（按字母排序）
/// - 被排除的工具列表
/// - 工具调用协议说明（原生调用或 XML 协议）
///
/// # 示例
///
/// ```ignore
/// let tools: Vec<Box<dyn Tool>> = vec![...];
/// let prompt = build_runtime_tool_visibility_prompt(
///     &tools,
///     &["dangerous_tool".to_string()],
///     true,
/// );
/// ```
pub fn build_runtime_tool_visibility_prompt(tools_registry: &[Box<dyn Tool>], excluded_tools: &[String], native_tools: bool) -> String {
    channel_support::prompt::build_runtime_tool_visibility_prompt(tools_registry, excluded_tools, native_tools, build_tool_instructions_from_specs)
}

/// 判断是否应跳过某个记忆上下文条目
///
/// 某些记忆条目不适合注入到上下文中，例如：
/// - 代理自动保存的条目（避免循环依赖）
/// - 以 "_history" 结尾的键（历史记录类条目）
/// - 内容过长的条目（超过最大字符限制）
///
/// # 参数
///
/// * `key` - 记忆条目的键名
/// * `content` - 记忆条目的内容
///
/// # 返回值
///
/// 如果应该跳过该条目返回 `true`，否则返回 `false`
pub use channel_support::prompt::should_skip_memory_context_entry;

/// 构建记忆上下文
///
/// 从长期记忆存储中检索与用户消息相关的条目，并构建成提示格式。
/// 检索过程会考虑相关性评分、条目数量限制和字符数限制。
///
/// # 参数
///
/// * `mem` - 记忆存储接口的实现
/// * `user_msg` - 用户消息（用于语义检索）
/// * `min_relevance_score` - 最小相关性评分阈值
///
/// # 返回值
///
/// 返回格式化的记忆上下文字符串，包含相关的记忆条目。
/// 如果没有相关记忆或检索失败，返回空字符串。
///
/// # 异步说明
///
/// 此函数是异步的，因为记忆检索可能涉及 I/O 操作（如数据库查询、向量搜索等）。
///
/// # 示例
///
/// ```ignore
/// let memory_store: Box<dyn Memory> = ...;
/// let context = build_memory_context(
///     memory_store.as_ref(),
///     "用户询问了关于项目配置的问题",
///     0.5,
/// ).await;
/// ```
pub use channel_support::prompt::build_memory_context;

/// 加载 OpenClaw 格式的引导文件到提示中
///
/// OpenClaw 框架使用一组标准的 Markdown 文件来定义代理的身份、行为和上下文：
/// - `AGENTS.md` — 代理工程协议和工作指南
/// - `SOUL.md` — 核心身份和价值观
/// - `TOOLS.md` — 工具使用指南
/// - `IDENTITY.md` — 身份定义
/// - `USER.md` — 用户特定配置
/// - `HEARTBEAT.md` — 心跳状态与周期任务
/// - `MEMORY.md` — 精选的长期记忆
///
/// # 参数
///
/// * `prompt` - 要追加内容的目标提示字符串（可变引用）
/// * `workspace_dir` - 工作空间目录路径
/// * `max_chars_per_file` - 每个文件的最大字符数限制
///
/// # 注意
///
/// 这些文件会在提示中声明"已注入"，提示代理不要尝试再次读取它们。
/// 日常记忆文件（`memory/*.md`）不会在此加载，而是通过 `memory_recall` / `memory_search` 工具按需访问。
pub use channel_support::prompt::load_openclaw_bootstrap_files;

pub use channel_support::prompt::build_workspace_identity_context;

/// 构建完整的系统提示
///
/// 加载工作空间身份文件并构建系统提示。默认遵循 `OpenClaw` 框架结构：
///
/// 1. **工具** — 工具列表及描述
/// 2. **安全** — 操作护栏提醒
/// 3. **技能** — 完整的技能指令和工具元数据
/// 4. **工作空间** — 工作目录
/// 5. **身份文件** — AGENTS、SOUL、TOOLS、IDENTITY、USER、MEMORY
/// 6. **日期时间** — 时区信息（用于缓存稳定性）
/// 7. **运行时** — 主机、操作系统、模型
///
/// 当 `identity_config` 设置为 AIEOS 格式时，引导文件部分将被替换为
/// 从文件或内联 JSON 加载的 AIEOS 身份数据。
///
/// 日常记忆文件（`memory/*.md`）**不会**在此注入 — 它们通过
/// `memory_recall` / `memory_search` 工具按需访问。
///
/// # 参数
///
/// * `workspace_dir` - 工作空间目录路径
/// * `model_name` - 当前使用的模型名称
/// * `tools` - 工具列表，每项为 (名称, 描述) 元组
/// * `skills` - 技能列表
/// * `identity_config` - 身份配置（可选），支持 OpenClaw 或 AIEOS 格式
/// * `bootstrap_max_chars` - 引导文件的最大字符数限制（可选）
///
/// # 返回值
///
/// 返回构建完成的系统提示字符串。如果所有部分都为空，
/// 则返回默认的基础提示。
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
///
/// let prompt = build_system_prompt(
///     Path::new("/workspace"),
///     "gpt-4",
///     &[("read", "读取文件"), ("write", "写入文件")],
///     &[],
///     None,
///     Some(10000),
/// );
/// ```
pub fn build_system_prompt(
    workspace_dir: &std::path::Path,
    model_name: &str,
    tools: &[(&str, &str)],
    skills: &[skills::Skill],
    identity_config: Option<&agent_config::runtime_schema::IdentityConfig>,
    bootstrap_max_chars: Option<usize>,
    timezone: chrono_tz::Tz,
) -> String {
    // 使用默认模式调用完整版本的构建函数
    build_system_prompt_with_mode(
        workspace_dir,
        model_name,
        tools,
        skills,
        identity_config,
        bootstrap_max_chars,
        false,                                                      // 不使用原生工具调用
        agent_config::runtime_schema::SkillsPromptInjectionMode::Full, // 完整技能提示
        timezone,
    )
}

/// 构建完整的系统提示（带模式控制）
///
/// 这是 `build_system_prompt` 的完整版本，提供更多配置选项。
/// 允许控制工具调用模式和技能提示注入模式。
///
/// # 参数
///
/// * `workspace_dir` - 工作空间目录路径
/// * `model_name` - 当前使用的模型名称
/// * `tools` - 工具列表，每项为 (名称, 描述) 元组
/// * `skills` - 技能列表
/// * `identity_config` - 身份配置（可选），支持 OpenClaw 或 AIEOS 格式
/// * `bootstrap_max_chars` - 引导文件的最大字符数限制（可选）
/// * `native_tools` - 是否使用原生工具调用（provider 的 function-calling）
/// * `skills_prompt_mode` - 技能提示注入模式（Full 或 Compact）
///
/// # 返回值
///
/// 返回构建完成的系统提示字符串。
///
/// # 提示结构
///
/// 生成的提示按以下顺序组织：
/// 1. 工具章节 — 可用工具列表
/// 2. 执行风格 — 优先行动并压缩分析与过程播报
/// 3. 安全章节 — 操作护栏
/// 4. 技能章节 — 技能定义（根据模式）
/// 5. 工作空间章节 — 当前目录
/// 6. 项目上下文章节 — 引导文件或 AIEOS 身份
/// 7. 日期时间章节 — 当前时间
/// 8. 运行时章节 — 主机信息
/// 9. 通道能力章节 — 消息通道特性
pub fn build_system_prompt_with_mode(workspace_dir: &Path, model_name: &str, tools: &[(&str, &str)], skills: &[skills::Skill], identity_config: Option<&agent_config::runtime_schema::IdentityConfig>, bootstrap_max_chars: Option<usize>, native_tools: bool, skills_prompt_mode: agent_config::runtime_schema::SkillsPromptInjectionMode, timezone: chrono_tz::Tz) -> String {
    channel_support::prompt::build_system_prompt_with_mode(workspace_dir, model_name, tools, skills, identity_config, bootstrap_max_chars, native_tools, skills_prompt_mode, timezone, agent_loop::prompt::multi_agent::recursive_orchestration_policy())
}

/// 将单个工作空间文件注入到提示中
///
/// 读取指定的工作空间文件，并将其内容追加到提示字符串中。
/// 如果文件过长会进行截断，如果文件不存在会添加缺失标记。
///
/// # 参数
///
/// * `prompt` - 要追加内容的目标提示字符串（可变引用）
/// * `workspace_dir` - 工作空间目录路径
/// * `filename` - 要注入的文件名
/// * `max_chars` - 文件内容的最大字符数限制
///
/// # 处理逻辑
///
/// - **文件存在且非空**：读取内容，如果超过字符限制则截断
/// - **文件存在但为空**：跳过，不添加任何内容
/// - **文件不存在**：添加 "[File not found: {filename}]" 标记
///
/// # 截断说明
///
/// 使用字符边界安全的截断方式，确保不会在 UTF-8 多字节字符中间截断。
/// 截断后会添加提示信息，告知使用 `read` 工具查看完整文件。
///
/// # 示例
///
/// ```ignore
/// let mut prompt = String::new();
/// inject_workspace_file(
///     &mut prompt,
///     Path::new("/workspace"),
///     "AGENTS.md",
///     10000,
/// );
/// ```
pub use channel_support::prompt::inject_workspace_file;
