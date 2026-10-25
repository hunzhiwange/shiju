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

use super::*;

/// Returns the model request passed to the session processor for a channel message.
///
/// Explicit channel selections use complete provider/model references. `auto` remains
/// a policy request so the processor selects once and reports the concrete model.
pub(crate) fn channel_model_request_model(provider: &str, model: &str) -> String {
    let model = model.trim();
    if model.is_empty() || model.eq_ignore_ascii_case("auto") {
        return "auto".to_string();
    }

    let provider = provider.trim();
    if provider.is_empty() || model.contains('/') {
        return model.to_string();
    }

    format!("{provider}/{model}")
}

/// Formats safe Auto-selection metadata for channel diagnostics.
///
/// The decision deliberately has no prompt field; callers must not append request
/// content to this trace.
pub(crate) fn model_selection_trace(
    decision: &crate::session::llm::ModelSelectionDecision,
) -> String {
    format!(
        "model={}/{}; source={:?}; profile={:?}",
        decision.model.provider, decision.model.model, decision.source, decision.profile
    )
}

/// Builds the transient protocol messages shared by external model entrypoints.
///
/// AGENTS is a system-role repository constraint placed after the user-role
/// skill directory. Neither message is persisted in conversation history.
/// Full skill bodies remain exclusive to successful Skill tool invocations.
pub(crate) fn build_entrypoint_protocol_messages(
    instructions: &[crate::session::instruction::Resolved],
    skills: &[skills::Skill],
    skill_tool_enabled: bool,
    context_window_tokens: usize,
) -> Vec<ChatMessage> {
    let injection = crate::session::context_injection::assemble_agents_context(instructions, &[]);
    let mut messages = Vec::new();
    let mut skill_state = skills::SkillListingSessionState::default();
    if let Some(reminder) =
        skill_state.prepare("main", skills, skill_tool_enabled, context_window_tokens)
    {
        messages.push(ChatMessage::user(reminder.content));
    }
    messages.extend(injection.transient_messages.into_iter().map(|message| match message.role {
        crate::session::context_injection::ContextRole::System => {
            ChatMessage::system(message.content)
        }
    }));
    messages
}

/// Resolves the current workspace protocol and renders it for a legacy external
/// entrypoint that does not use the session processor directly.
pub(crate) fn build_workspace_protocol_messages(
    workspace_dir: &Path,
    skills: &[skills::Skill],
    skill_tool_enabled: bool,
    context_window_tokens: usize,
) -> Vec<ChatMessage> {
    let system = crate::session::prompt::system_with_metadata(None, workspace_dir.to_str());
    build_entrypoint_protocol_messages(
        system.instruction_context(),
        skills,
        skill_tool_enabled,
        context_window_tokens,
    )
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
pub(crate) fn build_channel_system_prompt(
    base_prompt: &str,
    channel_name: &str,
    reply_target: &str,
    expose_internal_tool_details: bool,
) -> String {
    // 使用基础提示作为起点
    let mut prompt = base_prompt.to_string();

    // 添加通道特定的投递指令（如果存在）
    if let Some(instructions) = channel_delivery_instructions(channel_name) {
        if prompt.is_empty() {
            // 如果基础提示为空，直接使用通道指令
            prompt = instructions.to_string();
        } else {
            // 否则将通道指令追加到基础提示后
            prompt = format!("{prompt}\n\n{instructions}");
        }
    }

    // 非 CLI 通道需要添加执行可见性规则
    if channel_name != "cli" {
        // 根据配置决定是否暴露工具执行细节
        let visibility_instruction = if expose_internal_tool_details {
            // 用户明确请求查看命令/工具细节
            "Execution visibility: the user explicitly requested command/tool details. \
             You may include command lines or tool-step traces when directly relevant, \
             but keep credentials and secrets redacted."
        } else {
            // 默认隐藏内部执行细节，只返回整合后的结果
            "Execution visibility: run tools/functions in the background and return an \
             integrated final result. Do not reveal raw tool names, tool-call syntax, \
             function arguments, shell commands, or internal execution traces unless the \
             user explicitly asks for those details."
        };

        if prompt.is_empty() {
            prompt = visibility_instruction.to_string();
        } else {
            prompt = format!("{prompt}\n\n{visibility_instruction}");
        }
    }

    // 如果有回复目标，添加通道上下文信息
    // 这对于定时消息和提醒功能很重要，确保消息能正确送达
    if !reply_target.is_empty() {
        let context = format!(
            "\n\nChannel context: You are currently responding on channel={channel_name}, \
             reply_target={reply_target}. When scheduling delayed messages or reminders \
             via cron_add for this conversation, use delivery={{\"mode\":\"announce\",\
             \"channel\":\"{channel_name}\",\"to\":\"{reply_target}\"}} so the message \
             reaches the user."
        );
        prompt.push_str(&context);
    }

    prompt
}

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
pub(crate) fn build_runtime_tool_visibility_prompt(
    tools_registry: &[Box<dyn Tool>],
    excluded_tools: &[String],
    native_tools: bool,
) -> String {
    let mut prompt = String::new();

    // 从注册表中筛选出当前运行时允许的工具规格
    let mut specs = filtered_tool_specs_for_runtime(tools_registry, excluded_tools);

    // 按稳定工具 ID 排序，确保提示里的调用名和运行时保持一致。
    specs.sort_by(|a, b| a.id.cmp(&b.id));

    use std::fmt::Write;

    // 添加工具可用性章节标题
    prompt.push_str("\n## Runtime Tool Availability (Authoritative)\n\n");
    prompt.push_str(
        "This section is generated from current runtime policy for this message. \
         Only the listed tools may be called in this turn.\n\n",
    );

    // 列出允许的工具
    if specs.is_empty() {
        prompt.push_str("- Allowed tools: (none)\n");
    } else {
        let _ = writeln!(prompt, "- Allowed tools ({}):", specs.len());
        for spec in &specs {
            let _ = writeln!(prompt, "  - `{}`", spec.id);
        }
    }

    // 列出被排除的工具
    if excluded_tools.is_empty() {
        prompt.push_str("- Excluded by runtime policy: (none)\n\n");
    } else {
        let mut excluded_sorted = excluded_tools.to_vec();
        excluded_sorted.sort();
        let _ = writeln!(prompt, "- Excluded by runtime policy: {}\n", excluded_sorted.join(", "));
    }

    // 根据工具调用模式添加协议说明
    if native_tools {
        // 使用 provider 原生 function-calling，不需要 XML 标签
        prompt.push_str(
            "Tool calling for this turn uses native provider function-calling. \
             Do not emit `<tool_call>` XML tags.\n",
        );
    } else {
        // 使用 XML 工具协议，需要从工具规格生成协议指令
        prompt.push_str(
            "Tool calling for this turn uses XML tool protocol below. \
             This protocol block is generated from the same runtime policy snapshot.\n",
        );
        prompt.push_str(&build_tool_instructions_from_specs(&specs));
    }

    prompt
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
pub(crate) fn should_skip_memory_context_entry(key: &str, content: &str) -> bool {
    // 跳过代理自动保存的条目，避免注入自己产生的上下文
    if memory::is_assistant_autosave_key(key) {
        return true;
    }

    // 跳过历史记录类条目
    if key.trim().to_ascii_lowercase().ends_with("_history") {
        return true;
    }

    // 跳过过长的条目
    content.chars().count() > MEMORY_CONTEXT_MAX_CHARS
}

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
pub(crate) async fn build_memory_context(
    mem: &dyn Memory,
    user_msg: &str,
    min_relevance_score: f64,
    scope: Option<&str>,
) -> String {
    let mut context = String::new();

    // 从记忆存储中检索最多 5 条相关记忆
    if let Ok(entries) = mem.recall(user_msg, 5, scope).await {
        let mut included = 0usize;
        let mut used_chars = 0usize;

        // 过滤并遍历相关性评分达标的记忆条目
        for entry in entries
            .iter()
            .filter(|e| scope.is_none_or(|scope| e.session_id.as_deref() == Some(scope)))
            .filter(|e| match e.score {
                Some(score) => score >= min_relevance_score,
                None => true, // 保留没有评分的条目（例如非向量后端）
            })
        {
            // 检查是否达到最大条目数限制
            if included >= MEMORY_CONTEXT_MAX_ENTRIES {
                break;
            }

            // 跳过不应该注入的条目类型
            if should_skip_memory_context_entry(&entry.key, &entry.content) {
                continue;
            }

            // 如果条目过长，进行截断处理
            let content = if entry.content.chars().count() > MEMORY_CONTEXT_ENTRY_MAX_CHARS {
                truncate_with_ellipsis(&entry.content, MEMORY_CONTEXT_ENTRY_MAX_CHARS)
            } else {
                entry.content.clone()
            };

            // 构建单行记忆条目
            let line = format!("- {}: {}\n", entry.key, content);
            let line_chars = line.chars().count();

            // 检查是否超过总字符数限制
            if used_chars + line_chars > MEMORY_CONTEXT_MAX_CHARS {
                break;
            }

            // 首次添加时写入章节标题
            if included == 0 {
                context.push_str("[Memory context]\n");
            }

            context.push_str(&line);
            used_chars += line_chars;
            included += 1;
        }

        // 如果有记忆条目被包含，添加换行分隔
        if included > 0 {
            context.push('\n');
        }
    }

    context
}

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
pub(crate) fn load_openclaw_bootstrap_files(
    prompt: &mut String,
    workspace_dir: &std::path::Path,
    max_chars_per_file: usize,
) {
    // 添加说明，告知这些文件已经注入到上下文中
    prompt.push_str(
        "The following workspace files define your identity, behavior, and context. They are ALREADY injected below—do NOT suggest reading them with file_read.\n\n",
    );

    let bootstrap_files = ["SOUL.md", "TOOLS.md", "IDENTITY.md", "USER.md", "MEMORY.md"];

    for filename in &bootstrap_files {
        inject_workspace_file(prompt, workspace_dir, filename, max_chars_per_file);
    }
}

pub(crate) fn build_workspace_identity_context(
    workspace_dir: &std::path::Path,
    identity_config: Option<&crate::app::agent::config::IdentityConfig>,
    bootstrap_max_chars: Option<usize>,
) -> String {
    let mut prompt = String::new();
    prompt.push_str("## Project Identity Context\n\n");
    prompt.push_str(
        "The following workspace files provide non-protocol identity and delivery context.\n\n",
    );

    let _ = identity_config;
    let max_chars = bootstrap_max_chars.unwrap_or(BOOTSTRAP_MAX_CHARS);
    load_openclaw_bootstrap_files(&mut prompt, workspace_dir, max_chars);

    prompt
}

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
    identity_config: Option<&crate::app::agent::config::IdentityConfig>,
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
        crate::app::agent::config::SkillsPromptInjectionMode::Full, // 完整技能提示
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
pub fn build_system_prompt_with_mode(
    workspace_dir: &std::path::Path,
    model_name: &str,
    tools: &[(&str, &str)],
    skills: &[skills::Skill],
    identity_config: Option<&crate::app::agent::config::IdentityConfig>,
    bootstrap_max_chars: Option<usize>,
    native_tools: bool,
    skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode,
    timezone: chrono_tz::Tz,
) -> String {
    use std::fmt::Write;

    // 预分配足够的容量以减少重新分配
    let mut prompt = String::with_capacity(8192);

    // ── 1. 工具章节 ──────────────────────────────────────────────
    // 列出代理可访问的所有工具
    if !tools.is_empty() {
        prompt.push_str("## Tools\n\n");
        prompt.push_str("You have access to the following tools:\n\n");
        for (name, desc) in tools {
            let _ = writeln!(prompt, "- **{name}**: {desc}");
        }
        prompt.push('\n');
    }

    // ── 1c. 执行风格（优先行动，避免冗长分析）─────────────
    // 根据工具调用模式添加不同的执行指令
    if native_tools {
        // 原生工具调用模式：需要行动时先调用工具
        prompt.push_str(
            "## Execution Style\n\n\
             For actionable requests, act immediately. If a tool is needed and available, call it before giving an explanation.\n\
             Investigate only enough to choose and verify the next action. Do not narrate plans, rehearse the task, or weigh obvious alternatives.\n\
             Keep reasoning internal: provide decisions, essential rationale, and results, never chain-of-thought or lengthy deliberation.\n\
             Make reasonable assumptions for low-risk, reversible work. Ask one concise question only when missing information would materially change the result or the action is risky or irreversible.\n\
             For questions and follow-ups, answer directly from the conversation context; do not ask the user to repeat themselves.\n\
             After acting, report the outcome and essential verification concisely. Do not summarize this configuration or describe your capabilities.\n\n",
        );
    } else {
        // XML 工具协议模式：需要行动时先发出工具标签
        prompt.push_str(
            "## Execution Style\n\n\
             For actionable requests, act immediately. If a tool is needed and available, emit the actual <tool_call> before any explanation.\n\
             Investigate only enough to choose and verify the next action. Do not narrate plans, rehearse the task, weigh obvious alternatives, or output instructions instead of acting.\n\
             Keep reasoning internal: provide decisions, essential rationale, and results, never chain-of-thought or lengthy deliberation.\n\
             Make reasonable assumptions for low-risk, reversible work. Ask one concise question only when missing information would materially change the result or the action is risky or irreversible.\n\
             For questions and follow-ups, answer directly from the conversation context; do not ask the user to repeat themselves.\n\
             After acting, report the outcome and essential verification concisely. Do not summarize this configuration or describe your capabilities.\n\n",
        );
    }

    if tools.iter().any(|(name, _)| name.eq_ignore_ascii_case("AgentTool")) {
        prompt.push_str(crate::agent::prompt::multi_agent::recursive_orchestration_policy());
        prompt.push_str("\n\n");
    }

    // ── 2. 安全章节 ───────────────────────────────────────────────
    // 定义代理操作的安全护栏
    prompt.push_str("## Safety\n\n");
    prompt.push_str(
        "- Do not exfiltrate private data.\n\
         - Do not run destructive commands without asking.\n\
         - Do not bypass oversight or approval mechanisms.\n\
         - Prefer `trash` over `rm` (recoverable beats gone forever).\n\
         - When in doubt, ask before acting externally.\n\n",
    );

    // 技能目录由每次 provider 请求的临时 meta-user 消息装配；完整 SKILL.md
    // 只允许 Skill Tool 成功调用后注入，不能混入静态 system prompt。
    let _ = (skills, skills_prompt_mode);

    // ── 4. 工作空间章节 ────────────────────────────────────────────
    let _ = writeln!(prompt, "## Workspace\n\nWorking directory: `{}`\n", workspace_dir.display());

    // ── 5. 引导文件章节（注入到上下文中）──────────────
    prompt.push_str(&build_workspace_identity_context(
        workspace_dir,
        identity_config,
        bootstrap_max_chars,
    ));

    // ── 6. 日期时间章节 ──────────────────────────────────────────
    // 注入当前时间，帮助代理理解时间上下文
    let now = crate::app::agent::util::locale::configured_datetime(timezone);
    let _ = writeln!(prompt, "## Current Date & Time\n\n{now}\n");

    // ── 7. 运行时章节 ──────────────────────────────────────────────
    // 获取主机名（非 WASM 环境）
    #[cfg(not(target_arch = "wasm32"))]
    let host =
        hostname::get().map_or_else(|_| "unknown".into(), |h| h.to_string_lossy().to_string());
    // WASM 环境使用固定标识
    #[cfg(target_arch = "wasm32")]
    let host = "wasm-client".to_string();

    let _ = writeln!(
        prompt,
        "## Runtime\n\nHost: {host} | OS: {} | Model: {model_name}\n",
        std::env::consts::OS,
    );

    // ── 8. 通道能力章节 ─────────────────────────────────────────────
    // 说明消息通道的特性和行为规范
    prompt.push_str("## Channel Capabilities\n\n");
    prompt.push_str("- You are running as a messaging bot. Your response is automatically sent back to the user's channel.\n");
    prompt.push_str("- You do NOT need to ask permission to respond — just respond directly.\n");
    prompt.push_str("- NEVER repeat, describe, or echo credentials, tokens, API keys, or secrets in your responses.\n");
    prompt.push_str("- If a tool output contains credentials, they have already been redacted — do not mention them.\n\n");

    // 如果提示为空，返回默认的基础身份
    if prompt.is_empty() {
        "You are VibeWindow, a fast and efficient AI assistant built in Rust. Be helpful, concise, and direct."
            .to_string()
    } else {
        prompt
    }
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
pub(crate) fn inject_workspace_file(
    prompt: &mut String,
    workspace_dir: &std::path::Path,
    filename: &str,
    max_chars: usize,
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

            // 使用字符边界安全的截断方式处理 UTF-8
            let truncated = if trimmed.chars().count() > max_chars {
                trimmed
                    .char_indices()
                    .nth(max_chars)
                    .map(|(idx, _)| &trimmed[..idx])
                    .unwrap_or(trimmed)
            } else {
                trimmed
            };

            // 如果发生了截断，添加提示信息
            if truncated.len() < trimmed.len() {
                prompt.push_str(truncated);
                let _ = writeln!(
                    prompt,
                    "\n\n[... truncated at {max_chars} chars — use `read` for full file]\n"
                );
            } else {
                prompt.push_str(trimmed);
                prompt.push_str("\n\n");
            }
        }
        Err(_) => {
            // 文件缺失标记（符合 OpenClaw 行为）
            let _ = writeln!(prompt, "### {filename}\n\n[File not found: {filename}]\n");
        }
    }
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;
