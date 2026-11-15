//! 通道模型引用与投递提示策略。
use crate::format::channel_delivery_instructions;

pub fn channel_model_request_model(provider: &str, model: &str) -> String {
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

pub fn build_channel_system_prompt(
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

use agent_skills as skills;
use agent_memory::{self as memory, Memory};
use agent_util::truncate_with_ellipsis;
const BOOTSTRAP_MAX_CHARS: usize = 20_000;
pub const MEMORY_CONTEXT_MAX_ENTRIES: usize = 4;
pub const MEMORY_CONTEXT_ENTRY_MAX_CHARS: usize = 800;
pub const MEMORY_CONTEXT_MAX_CHARS: usize = 4_000;

pub fn model_selection_trace(
    decision: &model_selection::ModelSelectionDecision,
) -> String {
    format!(
        "model={}/{}; source={:?}; profile={:?}",
        decision.model.provider, decision.model.model, decision.source, decision.profile
    )
}

pub fn should_skip_memory_context_entry(key: &str, content: &str) -> bool {
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

pub async fn build_memory_context(
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

pub fn load_openclaw_bootstrap_files(
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

pub fn build_workspace_identity_context(
    workspace_dir: &std::path::Path,
    identity_config: Option<&agent_config::runtime_schema::IdentityConfig>,
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

pub fn inject_workspace_file(
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

pub fn build_system_prompt_with_mode(
    workspace_dir: &std::path::Path,
    model_name: &str,
    tools: &[(&str, &str)],
    skills: &[skills::Skill],
    identity_config: Option<&agent_config::runtime_schema::IdentityConfig>,
    bootstrap_max_chars: Option<usize>,
    native_tools: bool,
    skills_prompt_mode: agent_config::runtime_schema::SkillsPromptInjectionMode,
    timezone: chrono_tz::Tz,
    recursive_policy: &str,
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
        prompt.push_str(recursive_policy);
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
    let now = agent_util::locale::configured_datetime(timezone);
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

use tool_types::Tool;
use model_types::ChatMessage;
pub fn build_runtime_tool_visibility_prompt(
    tools_registry: &[Box<dyn Tool>],
    excluded_tools: &[String],
    native_tools: bool,
    build_tool_instructions_from_specs: fn(&[tool_types::ToolSpec]) -> String,
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

pub fn build_entrypoint_protocol_messages(
    agents_messages: Vec<ChatMessage>,
    skills: &[skills::Skill],
    skill_tool_enabled: bool,
    context_window_tokens: usize,
) -> Vec<ChatMessage> {
    let mut messages = Vec::new();
    let mut skill_state = skills::session_state::SkillListingSessionState::default();
    if let Some(reminder) =
        skill_state.prepare("main", skills, skill_tool_enabled, context_window_tokens)
    {
        messages.push(ChatMessage::user(reminder.content));
    }
    messages.extend(agents_messages);
    messages
}

pub fn filtered_tool_specs_for_runtime(
    tools_registry: &[Box<dyn Tool>],
    excluded_tools: &[String],
) -> Vec<tool_types::ToolSpec> {
    tools_registry
        .iter()
        .map(|tool| tool.spec())
        .filter(|spec| !excluded_tools.iter().any(|excluded| excluded == &spec.id))
        .collect()
}
