use vw_api_types::external_session::ToolKind;

/// 工具结果预览的最大行数
pub const PREVIEW_LINES: usize = 12;
/// 工具结果预览的最大字符数
pub const PREVIEW_CHARS: usize = 1200;
/// 工具参数预览的最大字符数（标题 / detail 已提炼出要点，参数只需看个开头；
/// 大会话里 Bash 参数预览曾占 payload 的五分之一）
pub const INPUT_PREVIEW_CHARS: usize = 400;
/// 思考正文预览的最大字符数
pub const THINKING_PREVIEW_CHARS: usize = 400;
/// 工具标题的最大字符数
pub const TITLE_CHARS: usize = 200;
/// Text 块超过该字符数时只下发预览 + 引用（注入的 AGENTS.md / developer / task-notification 等）
pub const INLINE_TEXT_MAX_CHARS: usize = 8 * 1024;
/// 压缩摘要等长事件说明的预览字符数
pub const EVENT_PREVIEW_CHARS: usize = 400;

/// 工具调用来自哪家 Agent；同名工具在不同 Agent 下归类可能不同。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSource {
    Claude,
    Codex,
    Gemini,
    OpenCode,
    /// Pi 与 OpenClaw（同构）
    Pi,
    /// Hermes / Grok Build 等：按名称关键字推断
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedTool {
    pub kind: ToolKind,
    /// MCP 服务器名（kind = Mcp 时）
    pub server: Option<String>,
}

impl NormalizedTool {
    fn of(kind: ToolKind) -> Self {
        Self { kind, server: None }
    }

    fn mcp(server: impl Into<String>) -> Self {
        Self {
            kind: ToolKind::Mcp,
            server: Some(server.into()),
        }
    }
}

/// 按 §3.3 的映射表把原始工具名归一化为 [`ToolKind`]。
///
/// `namespace` 只有 Codex `function_call.namespace` 会传（如 `mcp__cua_repl`）。
pub fn normalize_tool(
    source: ToolSource,
    raw_name: &str,
    namespace: Option<&str>,
) -> NormalizedTool {
    // MCP：Codex 看 namespace，其余看 `mcp__<server>__<tool>` 名称
    if let Some(server) = namespace.and_then(|ns| ns.strip_prefix("mcp__")) {
        return NormalizedTool::mcp(server.trim_end_matches('_'));
    }
    if let Some((server, _)) = split_mcp_name(raw_name) {
        return NormalizedTool::mcp(server);
    }

    use ToolKind::*;
    let kind = match source {
        ToolSource::Claude => match raw_name {
            "Bash" => Shell,
            "Read" => Read,
            "Grep" | "Glob" | "ToolSearch" | "ListAgents" => Search,
            "Edit" | "MultiEdit" | "NotebookEdit" => Edit,
            "Write" => Write,
            "WebFetch" | "WebSearch" => Web,
            "Agent" | "Task" | "SendMessage" | "TaskStop" => Agent,
            "AskUserQuestion" => Ask,
            "TodoWrite" => Todo,
            _ => Other,
        },
        ToolSource::Codex => match raw_name {
            "exec" | "exec_command" | "shell" | "local_shell" | "write_stdin"
            | "CommandExecution" => Shell,
            "read_file" => Read,
            // apply_patch / FileChange 默认按 Edit；全是新增文件时由解析器改为 Write
            "apply_patch" | "FileChange" => Edit,
            "write_file" => Write,
            "web_search" | "web_search_call" | "open_page" | "WebSearch" => Web,
            "McpToolCall" => Mcp,
            "spawn_agent" | "send_message" | "agent_message" | "SubAgentActivity"
            | "create_thread" => Agent,
            "request_user_input_async" => Ask,
            "update_plan" => Todo,
            name if name.starts_with("web.") => Web,
            _ => Other,
        },
        ToolSource::Gemini => match raw_name {
            "run_shell_command" => Shell,
            "read_file" | "read_many_files" => Read,
            "grep_search" | "glob" | "list_directory" | "search_file_content" => Search,
            "replace" | "edit" => Edit,
            "write_file" => Write,
            "web_fetch" | "google_web_search" => Web,
            "ask_user" => Ask,
            "write_todos" => Todo,
            // 待核实：Gemini CLI 的 MCP 工具名形如 `server/tool`
            name if name.contains('/') => {
                let server = name.split('/').next().unwrap_or_default();
                return NormalizedTool::mcp(server);
            }
            _ => Other,
        },
        ToolSource::OpenCode => match raw_name {
            "bash" => Shell,
            "read" => Read,
            "grep" | "glob" | "list" | "ast_grep_search" | "codesearch" => Search,
            "edit" | "patch" => Edit,
            "write" => Write,
            "webfetch" | "websearch" => Web,
            "task" | "background_output" | "background_cancel" => Agent,
            "question" => Ask,
            "todowrite" | "todoread" => Todo,
            "invalid" => Other,
            // 待核实：OpenCode 的 MCP 工具名形如 `server_tool`（内置表之外含 `_` 的名字）
            name if name.contains('_') => {
                let server = name.split('_').next().unwrap_or_default();
                return NormalizedTool::mcp(server);
            }
            _ => Other,
        },
        ToolSource::Pi => match raw_name {
            "bash" | "bashExecution" => Shell,
            "read" => Read,
            "grep" | "find" | "ls" => Search,
            "edit" => Edit,
            "write" => Write,
            "fetch" => Web,
            _ => Other,
        },
        ToolSource::Generic => generic_kind(raw_name),
    };
    NormalizedTool::of(kind)
}

/// Hermes / Grok Build：没有固定工具表，按名称关键字推断。
fn generic_kind(raw_name: &str) -> ToolKind {
    let name = raw_name.to_ascii_lowercase();
    let has = |keys: &[&str]| keys.iter().any(|k| name.contains(k));
    if name.starts_with("web") {
        ToolKind::Web
    } else if has(&["shell", "bash", "exec", "terminal"]) {
        ToolKind::Shell
    } else if name.starts_with("read") {
        ToolKind::Read
    } else if has(&["grep", "glob", "search"]) {
        ToolKind::Search
    } else if has(&["edit", "patch"]) {
        ToolKind::Edit
    } else if name.starts_with("write") {
        ToolKind::Write
    } else {
        ToolKind::Other
    }
}

/// 拆 `mcp__<server>__<tool>`；不是这种形式返回 `None`。
pub fn split_mcp_name(raw_name: &str) -> Option<(&str, &str)> {
    let rest = raw_name.strip_prefix("mcp__")?;
    let (server, tool) = rest.split_once("__")?;
    (!server.is_empty() && !tool.is_empty()).then_some((server, tool))
}

/// Shell 命令按首个程序名细化：`rg`/`grep`/`fd`/`find`/`ls` 开头算搜索，其余仍是 Shell。
pub fn refine_shell_kind(command: &str) -> ToolKind {
    let program = command
        .split_whitespace()
        .next()
        .map(|p| p.rsplit('/').next().unwrap_or(p))
        .unwrap_or_default();
    match program {
        "rg" | "grep" | "fd" | "find" | "ls" => ToolKind::Search,
        _ => ToolKind::Shell,
    }
}

/// Codex `CommandExecution.parsed_cmd[].type` → kind（`unknown` 返回 `None`，保持 Shell）
pub fn kind_from_codex_parsed_cmd(parsed_type: &str) -> Option<ToolKind> {
    match parsed_type {
        "read" => Some(ToolKind::Read),
        "search" | "list" | "list_files" => Some(ToolKind::Search),
        _ => None,
    }
}

