//! TUI v2 顶层编排入口。
//!
//! 本模块负责 Phase 3 里最靠近运行边界的两类职责：
//! - 组装 `GatewayUiRuntime`、`TuiState`、`TuiController` 与 `TuiRenderer`
//! - 把 gateway 流事件经 runtime/state 标准化后落进 `UiMessage` 管线
//! - 宿主管理 terminal lifecycle，包括 raw mode、alt screen 与退出清理
//!
//! 当前实现已覆盖到 Phase 6 的基础 overlay 宿主：
//! - 作为 CLI interactive 默认入口
//! - 先接真实 gateway submit 流，不继续扩张 message row/virtual list
//! - grouped/collapsed 仍留给后续 slice；search/task overlay 已接入当前宿主

use std::collections::hash_map::DefaultHasher;
#[cfg(unix)]
use std::fs::OpenOptions;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::io::IsTerminal;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use anyhow::{Result, anyhow};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use serde_json::json;
use vw_agent::provider::provider;
use vw_gateway_client::GatewayChatStreamRequest;
use vw_gateway_client::vw_api_types::id::SessionId;
use vw_shared::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionMeta, TokenUsage,
};
use vw_shared::todo::Todo;

use super::controller::{
    TuiController, TuiControllerCommand, TuiOverlayCommand, build_prompt_submission,
};
use super::input::{
    TuiSlashCommandInvocation, TuiSlashCommandOutcome, execute_slash_command, parse_slash_command,
};
use super::model::{
    AcpBackendOption, DelegateAgentOption, McpServerTransport, PermissionMode, PromptSubmission,
    QueueDrainBoundary, QueuedPromptCommand, QueuedPromptCommandKind, ReasoningEffort,
    TranscriptEffectIntent, UiConfirmOverlay, UiErrorOverlay, UiMcpOverlay, UiMcpServerInfo,
    UiMemoryEntry, UiMemoryOverlay, UiMessage, UiMessageBase, UiMessageId, UiOverlay,
    UiQuestionOverlay, UiSystemMessage, UiSystemMessageLevel, UiTaskOverlay, UiTaskStepItem,
    UiTodoOverlay,
};
use super::paste::PromptEditorIntent;
use super::render::{TuiRenderFeedback, TuiRenderer};
use super::runtime::gateway::GatewayUiRuntime;
use super::runtime::gateway::normalize_optional_str_ref;
use super::runtime::stream_adapter::{UiRuntimeEvent, UiRuntimeTerminalEvent};
use super::state::{
    TuiAction, TuiModelCatalogEntry, TuiState, TuiStickyPromptSummary, TuiUnseenRangeSummary,
    TuiVisibleTranscriptWindow, TuiWindowSummary, apply_runtime_event, reduce_tui_state,
    select_visible_grouped_transcript_window,
};
use super::terminal_capabilities::{
    TerminalCapabilities, TerminalModePlan, TerminalModeSession, TuiBackground,
};
use crate::app::agent::config::Config;
use crate::app::agent::session::processor as legacy_processor;
use crate::cli::processor::{
    SessionProcessorComparableResult, SessionProcessorComparableTerminal,
    run_session_processor_comparable_for_cli,
};
use crate::cli::session::{build_project_info, collect_git_workspace_status};
use crate::cli::setup::CliSetup;

#[cfg(unix)]
use std::fs::File;

/// Unix 平台下直接写入控制终端，避免 stdout 被重定向时污染 fullscreen TUI。
#[cfg(unix)]
type CliBackendWriter = File;

/// 非 Unix 平台下沿用标准输出作为 Ratatui 后端。
#[cfg(not(unix))]
type CliBackendWriter = std::io::Stdout;

type TuiTerminal = Terminal<CrosstermBackend<CliBackendWriter>>;

const BUSY_HOST_POLL_RATE: Duration = Duration::from_millis(60);
const CANCEL_REQUESTED_STATUS: &str = "已请求取消；当前输出会在收到下一次运行时事件后停止。";
const QUESTION_CUSTOM_ANSWER_PREFIX: &str = "__custom__:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiRunMode {
    Standard,
}

impl TuiRunMode {
    pub(crate) fn badge_label(self) -> &'static str {
        match self {
            Self::Standard => "TUI",
        }
    }

    pub(crate) fn shadow_compare_enabled(self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TodoSessionAccessAction {
    OpenPanel,
    Refresh,
    Save,
}

impl TodoSessionAccessAction {
    pub(crate) fn overlay_title(self) -> &'static str {
        match self {
            Self::OpenPanel => "待办面板不可用",
            Self::Refresh => "待办刷新失败",
            Self::Save => "待办保存失败",
        }
    }

    pub(crate) fn action_label(self) -> &'static str {
        match self {
            Self::OpenPanel => "打开待办面板",
            Self::Refresh => "刷新当前会话的待办列表",
            Self::Save => "保存当前会话的待办列表",
        }
    }
}

pub(crate) fn todo_session_unavailable_overlay(action: TodoSessionAccessAction) -> UiErrorOverlay {
    UiErrorOverlay {
        title: action.overlay_title().to_string(),
        message: format!(
            "当前 TUI 宿主还没有绑定活动会话，因此无法{}。\n请先新建或恢复一个会话，再重试。",
            action.action_label()
        ),
        recoverable: true,
    }
}

pub(crate) fn is_session_unavailable_error(message: &str) -> bool {
    let normalized = message.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    [
        "session id is required",
        "session is required",
        "non-empty session id",
        "missing session",
        "session unavailable",
        "session not found",
        "no active session",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

/// 从工作区目录或 home 下的全局配置文件加载 MCP 服务器列表。
///
/// 优先查找 `<workspace_root>/.vwacprc.json`，若不存在则尝试
/// active home config dir's `acp/config.json`。解析 `mcpServers` 字段。
fn build_mcp_overlay(workspace_root: Option<&std::path::Path>) -> UiMcpOverlay {
    // 候选配置文件路径：project 优先，fallback 到 global
    let candidates: Vec<(std::path::PathBuf, &str)> = {
        let mut v = Vec::new();
        if let Some(root) = workspace_root {
            v.push((root.join(".vwacprc.json"), "project"));
        }
        if let Some(home) = dirs_home() {
            v.push((
                vw_config_types::paths::home_config_dir(home).join("acp").join("config.json"),
                "global",
            ));
        }
        v
    };

    for (path, label) in &candidates {
        if !path.exists() {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(path)
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(servers) = json.get("mcpServers")
        {
            let servers_info = parse_mcp_servers_json(servers);
            return UiMcpOverlay {
                servers: servers_info,
                selected_index: 0,
                config_source: format!("{} ({})", path.display(), label),
            };
        }
    }

    UiMcpOverlay {
        servers: Vec::new(),
        selected_index: 0,
        config_source: "未找到 MCP 配置文件".to_string(),
    }
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

/// 将 `mcpServers` JSON 值解析为 `UiMcpServerInfo` 列表。
/// 支持对象映射格式和数组格式。
pub(crate) fn parse_mcp_servers_json(value: &serde_json::Value) -> Vec<UiMcpServerInfo> {
    match value {
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(name, server)| mcp_server_entry_to_info(name.clone(), server))
            .collect(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(|server| {
                let name =
                    server.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                mcp_server_entry_to_info(name, server)
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn mcp_server_entry_to_info(
    name: String,
    server: &serde_json::Value,
) -> UiMcpServerInfo {
    let transport_type = server.get("type").and_then(|v| v.as_str()).unwrap_or("stdio");

    let (transport, address) = match transport_type {
        "http" => {
            let url = server.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
            (McpServerTransport::Http, url)
        }
        "sse" => {
            let url = server.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
            (McpServerTransport::Sse, url)
        }
        _ => {
            let command = server.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
            (McpServerTransport::Stdio, command)
        }
    };

    UiMcpServerInfo { name, transport, address }
}

/// 内存预览最大行数。
const MEMORY_PREVIEW_MAX_LINES: usize = 30;

/// 扫描 project 和 global memory markdown 文件，构建内存面板数据。
///
/// 扫描路径（按优先级）：
/// - `<workspace_root>/AGENTS.md`（项目代理配置）
/// - `<workspace_root>/.vibewindow/memory/*.md`（项目内存）
/// - active home config dir's `memory/*.md`（全局内存）
fn build_memory_overlay(workspace_root: Option<&std::path::Path>) -> UiMemoryOverlay {
    let mut entries: Vec<UiMemoryEntry> = Vec::new();

    // 扫描项目根 AGENTS.md
    if let Some(root) = workspace_root {
        let agents_path = root.join("AGENTS.md");
        if agents_path.exists()
            && let Some(entry) = read_memory_file(&agents_path, "project")
        {
            entries.push(entry);
        }

        // 扫描 <workspace_root>/.vibewindow/memory/*.md
        let project_memory_dir = root.join(".vibewindow").join("memory");
        collect_memory_dir_entries(&project_memory_dir, "project", &mut entries);
    }

    // 扫描 active home config dir's memory/*.md
    if let Some(home) = dirs_home() {
        let global_memory_dir = vw_config_types::paths::home_config_dir(home).join("memory");
        collect_memory_dir_entries(&global_memory_dir, "global", &mut entries);
    }

    UiMemoryOverlay { entries, selected_index: 0 }
}

/// 读取一个 memory 文件，返回 UiMemoryEntry（前 MEMORY_PREVIEW_MAX_LINES 行）。
fn read_memory_file(path: &std::path::Path, scope: &str) -> Option<UiMemoryEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let all_lines: Vec<&str> = content.lines().collect();
    let total_lines = all_lines.len();
    let preview_lines: Vec<String> =
        all_lines.iter().take(MEMORY_PREVIEW_MAX_LINES).map(|l| l.to_string()).collect();
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
    Some(UiMemoryEntry {
        scope: scope.to_string(),
        filename,
        path: path.display().to_string(),
        preview_lines,
        total_lines,
    })
}

/// 收集目录下所有 `.md` 文件为 memory entries。
fn collect_memory_dir_entries(
    dir: &std::path::Path,
    scope: &str,
    entries: &mut Vec<UiMemoryEntry>,
) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<std::path::PathBuf> = read_dir
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    paths.sort();
    for path in paths {
        if let Some(entry) = read_memory_file(&path, scope) {
            entries.push(entry);
        }
    }
}

/// 构建退出确认弹层的 body 文本，附带当前会话状态摘要。
pub(crate) fn build_exit_confirm_body(state: &TuiState) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("确认离开 TUI 并返回 shell 吗？".to_string());
    lines.push(String::new());

    // 会话信息摘要
    if let Some(session_id) = &state.session.session_id {
        let short_id = if session_id.len() > 20 {
            format!("{}…", &session_id[..20])
        } else {
            session_id.clone()
        };
        lines.push(format!("  会话 ID  : {short_id}"));
    }

    let msg_count = state
        .messages
        .iter()
        .filter(|m| {
            use super::model::UiMessage;
            matches!(m, UiMessage::User(_) | UiMessage::Assistant(_))
        })
        .count();
    if msg_count > 0 {
        lines.push(format!("  消息数   : {msg_count} 条"));
    }

    if state.prompt.is_busy() {
        lines.push(String::new());
        lines.push("  ⚠ 当前有请求正在进行，退出将中止本次输出。".to_string());
    }

    lines.push(String::new());
    lines.push("Enter 确认退出  Esc 取消".to_string());
    lines.join("\n")
}

fn is_permission_surface_error(message: &str) -> bool {
    let normalized = message.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    [
        "requires approval",
        "require approval",
        "approval required",
        "permission request",
        "approval prompt",
        "non-cli approval",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn is_permission_event_type(event_type: Option<&str>) -> bool {
    event_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .is_some_and(|value| value.contains("permission") || value.contains("approval"))
}

pub(crate) fn runtime_event_fallback_overlay(event: &UiRuntimeEvent) -> Option<UiErrorOverlay> {
    match event {
        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::TimedOut { message, .. }) => {
            Some(UiErrorOverlay {
                title: "输出超时".to_string(),
                message: format!("当前轮次在 gateway 返回稳定终态前已超时。\n{}", message.trim()),
                recoverable: true,
            })
        }
        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error(message)) => {
            Some(runtime_terminal_error_overlay(message.as_str()))
        }
        UiRuntimeEvent::Unknown { event_type } => {
            Some(runtime_unknown_event_overlay(event_type.as_deref()))
        }
        UiRuntimeEvent::Delta(_)
        | UiRuntimeEvent::PlanRunTaskStarted { .. }
        | UiRuntimeEvent::ContextLoaded { .. }
        | UiRuntimeEvent::Message(_)
        | UiRuntimeEvent::StepStart { .. }
        | UiRuntimeEvent::StepFinish { .. }
        | UiRuntimeEvent::PostToolRound
        | UiRuntimeEvent::TaskStateChanged { .. }
        | UiRuntimeEvent::SessionMetadataChanged { .. }
        | UiRuntimeEvent::UsageUpdated { .. }
        | UiRuntimeEvent::Terminal(
            UiRuntimeTerminalEvent::Done { .. } | UiRuntimeTerminalEvent::Cancelled { .. },
        ) => None,
    }
}

fn runtime_unknown_event_overlay(event_type: Option<&str>) -> UiErrorOverlay {
    let event_type =
        event_type.map(str::trim).filter(|value| !value.is_empty()).unwrap_or("unknown");

    if is_permission_event_type(Some(event_type)) {
        return UiErrorOverlay {
            title: "权限事件回退".to_string(),
            message: format!(
                "gateway 发出了一条权限或授权事件，但 TUI 无法将它解码到专用权限界面。请按 F2 打开待处理请求；如果没有出现，尝试升级协议后重试。\n事件类型: {}",
                event_type
            ),
            recoverable: true,
        };
    }

    UiErrorOverlay {
        title: "运行时事件回退".to_string(),
        message: format!(
            "gateway 发出了一条当前不支持或无法解码的运行时事件。会话内容仍可查看，但本轮可能不完整。\n事件类型: {}",
            event_type
        ),
        recoverable: true,
    }
}

fn runtime_terminal_error_overlay(message: &str) -> UiErrorOverlay {
    let normalized = message.trim();
    if is_session_unavailable_error(normalized) {
        return UiErrorOverlay {
            title: "会话不可用".to_string(),
            message: format!(
                "当前运行时已经失去可用的会话绑定，因此本轮无法继续。\n{}",
                if normalized.is_empty() {
                    "请先新建或恢复一个会话，再重试。"
                } else {
                    normalized
                }
            ),
            recoverable: true,
        };
    }

    if is_permission_surface_error(normalized) {
        return UiErrorOverlay {
            title: "权限请求失败".to_string(),
            message: format!(
                "当前轮次触发了授权或权限边界，但 TUI 无法将它解码到专用权限界面。请按 F2 打开待处理请求；如果没有出现，先恢复会话后重试。\n{}",
                if normalized.is_empty() {
                    "授权或权限请求处理失败"
                } else {
                    normalized
                }
            ),
            recoverable: true,
        };
    }

    UiErrorOverlay {
        title: "输出失败".to_string(),
        message: format!(
            "当前轮次因运行时或网络错误而结束。\n{}",
            if normalized.is_empty() { "gateway 输出失败" } else { normalized }
        ),
        recoverable: true,
    }
}

pub(crate) fn latest_session_preview(previews: &[ChatSessionMeta]) -> Option<&ChatSessionMeta> {
    previews.iter().max_by(|left, right| {
        left.updated_ms
            .cmp(&right.updated_ms)
            .then(left.message_count.cmp(&right.message_count))
            .then(left.call_count.cmp(&right.call_count))
            .then(left.id.cmp(&right.id))
    })
}

pub(crate) fn snapshot_from_preview(preview: &ChatSessionMeta) -> ChatSession {
    ChatSession {
        id: preview.id.clone(),
        title: preview.title.clone(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: preview.updated_ms,
        updated_ms: preview.updated_ms,
    }
}

pub(crate) fn empty_session_snapshot(session_id: &str) -> ChatSession {
    ChatSession {
        id: session_id.to_string(),
        title: String::new(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 0,
        updated_ms: 0,
    }
}

pub(crate) fn select_restore_session_id(
    explicit_session_id: Option<&str>,
    previews: &[ChatSessionMeta],
) -> Option<String> {
    normalize_optional_str_ref(explicit_session_id)
        .map(ToOwned::to_owned)
        .or_else(|| latest_session_preview(previews).map(|preview| preview.id.clone()))
}

/// tui_v2 的顶层应用宿主。
///
/// 当前阶段该结构只维护 fullscreen skeleton 所需的最小状态：
/// - `runtime`：保留 gateway endpoint、目录和后续 session seed
/// - `state`：统一的 reducer/store 状态容器
/// - `controller`：terminal event -> state action
/// - `renderer`：state/selectors -> frame
pub(crate) struct TuiApp {
    run_mode: TuiRunMode,
    runtime: GatewayUiRuntime,
    state: TuiState,
    controller: TuiController,
    renderer: TuiRenderer,
    deferred_prompt_draft: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiAppCommandOutcome {
    Continue,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionRestoreOutcome {
    Snapshot,
    MetadataOnly,
    Missing,
}

/// 视口外流式 delta 的重绘判定快照。
///
/// 这里对齐 claude code 的 OffscreenFreeze 思路，但保持 Ratatui 宿主边界：
/// 如果一次 delta 只改动了视口外尾部消息，就直接跳过本次 draw，避免长会话在
/// 流式输出时反复重绘不可见历史区。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiDeltaRedrawSnapshot {
    message_count: usize,
    window_summary: TuiWindowSummary,
    sticky_prompt: Option<TuiStickyPromptSummary>,
    unseen_range: Option<TuiUnseenRangeSummary>,
    visible_content_hash: u64,
}

impl TuiDeltaRedrawSnapshot {
    pub(crate) fn capture(state: &TuiState) -> Self {
        let visible_window = select_visible_grouped_transcript_window(state);
        Self {
            message_count: state.messages.len(),
            window_summary: visible_window.window_summary(),
            sticky_prompt: visible_window.sticky_prompt().cloned(),
            unseen_range: visible_window.unseen_range(),
            visible_content_hash: hash_visible_window_messages(state, &visible_window),
        }
    }

    fn contains_tail(&self) -> bool {
        self.message_count > 0 && self.window_summary.covered_message_end >= self.message_count
    }
}

fn hash_visible_window_messages(
    state: &TuiState,
    visible_window: &TuiVisibleTranscriptWindow<'_>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    visible_window.covered_message_start.hash(&mut hasher);
    visible_window.covered_message_end.hash(&mut hasher);

    for message in state.messages.iter().skip(visible_window.covered_message_start).take(
        visible_window.covered_message_end.saturating_sub(visible_window.covered_message_start),
    ) {
        format!("{message:?}").hash(&mut hasher);
    }

    hasher.finish()
}

pub(crate) fn should_redraw_after_runtime_delta(
    before: &TuiDeltaRedrawSnapshot,
    after: &TuiState,
) -> bool {
    let after_snapshot = TuiDeltaRedrawSnapshot::capture(after);

    if before.window_summary.follows_tail() || after_snapshot.window_summary.follows_tail() {
        return true;
    }

    if before.message_count != after_snapshot.message_count {
        return true;
    }

    if before.window_summary != after_snapshot.window_summary
        || before.sticky_prompt != after_snapshot.sticky_prompt
        || before.unseen_range != after_snapshot.unseen_range
        || before.visible_content_hash != after_snapshot.visible_content_hash
    {
        return true;
    }

    before.contains_tail() || after_snapshot.contains_tail() || tail_message_visible(after)
}

fn tail_message_visible(state: &TuiState) -> bool {
    if state.messages.is_empty() {
        return false;
    }

    let viewport_messages = state.scroll.viewport_messages.max(1);
    let last_message = state.messages.len().saturating_sub(1);
    last_message >= state.scroll.top_message
        && last_message < state.scroll.top_message.saturating_add(viewport_messages)
}

pub(crate) fn dequeue_queued_prompt_command(
    state: &mut TuiState,
    deferred_prompt_draft: &mut Option<String>,
) -> Option<TuiControllerCommand> {
    if state.prompt.is_busy() {
        return None;
    }

    loop {
        let session_id = state.session.session_id.clone();
        let Some(command) =
            state.prompt.drain_queued_for(QueueDrainBoundary::Turn, session_id.as_deref())
        else {
            restore_deferred_prompt_draft(state, deferred_prompt_draft);
            return None;
        };

        capture_deferred_prompt_draft(state, deferred_prompt_draft);

        if let Some(next_command) = queued_command_to_controller_command(state, command) {
            return Some(next_command);
        }
    }
}

pub(crate) fn restore_deferred_prompt_draft(
    state: &mut TuiState,
    deferred_prompt_draft: &mut Option<String>,
) {
    let Some(draft) = deferred_prompt_draft.take() else {
        return;
    };

    if state.prompt.is_busy() {
        *deferred_prompt_draft = Some(draft);
        return;
    }

    if state.prompt.value.is_empty() {
        reduce_tui_state(state, TuiAction::PromptValueSet(draft));
    }
}

fn capture_deferred_prompt_draft(state: &mut TuiState, deferred_prompt_draft: &mut Option<String>) {
    if state.prompt.value.is_empty() {
        return;
    }

    *deferred_prompt_draft = Some(state.prompt.value.clone());
    reduce_tui_state(state, TuiAction::PromptValueSet(String::new()));
}

pub(crate) fn queued_command_to_controller_command(
    state: &TuiState,
    command: QueuedPromptCommand,
) -> Option<TuiControllerCommand> {
    match command.kind {
        QueuedPromptCommandKind::Submit => {
            let execution = command.execution;
            build_prompt_submission(state, command.raw.as_str())
                .map(|mut submission| {
                    submission.execution = execution;
                    submission
                })
                .map(TuiControllerCommand::SubmitPrompt)
        }
        QueuedPromptCommandKind::SlashCommand => Some(TuiControllerCommand::ExecuteSlashCommand(
            parse_queued_slash_command(state, command.raw.as_str()),
        )),
    }
}

fn parse_queued_slash_command(state: &TuiState, raw: &str) -> TuiSlashCommandInvocation {
    parse_slash_command(state, raw).unwrap_or(TuiSlashCommandInvocation {
        raw: raw.trim().to_string(),
        token: raw.trim().trim_start_matches('/').to_string(),
        argument: None,
        kind: None,
        action: None,
        execution_policy: None,
    })
}

pub(crate) fn question_overlay_submission_answers(overlay: &UiQuestionOverlay) -> Vec<Vec<String>> {
    overlay
        .answers
        .iter()
        .map(|answers| {
            answers
                .iter()
                .map(|answer| {
                    answer
                        .strip_prefix(QUESTION_CUSTOM_ANSWER_PREFIX)
                        .unwrap_or(answer.as_str())
                        .to_string()
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(crate) fn todo_overlay_items_as_shared_todos(overlay: &UiTodoOverlay) -> Vec<Todo> {
    overlay
        .items
        .iter()
        .map(|item| Todo {
            content: item.content.clone(),
            status: item.status.clone(),
            priority: item.priority.clone(),
            id: item.id.clone(),
        })
        .collect()
}

#[path = "app_command.rs"]
mod app_command;
#[cfg(test)]
pub(crate) use app_command::{apply_settings_values_to_state, full_access_confirmation};
#[path = "app_session.rs"]
mod app_session;
#[path = "app_stream.rs"]
mod app_stream;

impl TuiApp {
    /// 基于 CLI 配置与 setup 组装一份可运行的 tui_v2 应用。
    pub(crate) fn bootstrap(
        config: &Config,
        setup: &CliSetup,
        run_mode: TuiRunMode,
    ) -> Result<Self> {
        let runtime = GatewayUiRuntime::for_workspace(config).map_err(|err| anyhow!(err))?;
        let preflight =
            runtime.ensure_local_gateway_ready_blocking().map_err(|err| anyhow!(err))?;
        let theme_setting = config.app_ui.system_settings.app_theme.trim().to_ascii_lowercase();
        let background = if theme_setting.contains("light") {
            TuiBackground::Light
        } else if theme_setting.contains("dark") {
            TuiBackground::Dark
        } else {
            TuiBackground::Unknown
        };
        let capabilities = TerminalCapabilities::detect(background);
        let mut app = Self {
            run_mode,
            runtime,
            state: TuiState::default(),
            controller: TuiController::default(),
            renderer: TuiRenderer::for_capabilities(capabilities, theme_setting.contains("dalton")),
            deferred_prompt_draft: None,
        };
        app.state.theme_name =
            if theme_setting.contains("light") { "Light".to_string() } else { "Dark".to_string() };
        app.bootstrap_session_state_blocking(config, setup);
        if preflight.started_gateway() {
            app.push_app_system_message(
                format!(
                    "Local gateway auto-started at {} before entering TUI.",
                    app.runtime.endpoint().describe()
                ),
                UiSystemMessageLevel::Success,
            );
        }
        app.refresh_project_context_blocking();
        app.refresh_task_state_blocking();
        Ok(app)
    }

    /// 返回当前内部状态，便于测试验证骨架编排是否正确。
    pub(crate) fn state(&self) -> &TuiState {
        &self.state
    }

    /// 运行 fullscreen skeleton 主循环。
    ///
    /// 当前循环只覆盖最小交互：
    /// - tick 与 resize 触发重绘
    /// - prompt 输入与真实 gateway turn
    /// - overlay modal 打开/关闭
    /// - Ctrl+C / 空 prompt 下的 Esc 退出
    pub(crate) fn run(mut self) -> Result<()> {
        let mut terminal = TuiTerminalLifecycle::enter()?;
        let mut feedback = self.draw(terminal.terminal_mut())?;
        self.state.apply_transcript_measurements(&feedback.transcript_measurements);
        self.controller.sync_layout(&mut self.state, &feedback);

        loop {
            if let Some(command) =
                dequeue_queued_prompt_command(&mut self.state, &mut self.deferred_prompt_draft)
            {
                if matches!(
                    self.handle_command(terminal.terminal_mut(), command)?,
                    TuiAppCommandOutcome::Quit
                ) {
                    break;
                }

                feedback = self.draw(terminal.terminal_mut())?;
                self.state.apply_transcript_measurements(&feedback.transcript_measurements);
                self.controller.sync_layout(&mut self.state, &feedback);
                continue;
            }

            let event = self.controller.next_event()?;
            let command = self.controller.handle_event(&mut self.state, event);
            if matches!(
                self.handle_command(terminal.terminal_mut(), command)?,
                TuiAppCommandOutcome::Quit
            ) {
                break;
            }

            feedback = self.draw(terminal.terminal_mut())?;
            self.state.apply_transcript_measurements(&feedback.transcript_measurements);
            self.controller.sync_layout(&mut self.state, &feedback);
        }

        Ok(())
    }

    fn draw(&mut self, terminal: &mut TuiTerminal) -> Result<TuiRenderFeedback> {
        let mut feedback = TuiRenderFeedback::default();
        let endpoint_label = self.runtime.endpoint().describe();
        let spinner_frame = self.controller.spinner_frame();

        terminal.draw(|frame| {
            feedback = self.renderer.render_frame(
                frame,
                &self.state,
                self.run_mode.badge_label(),
                endpoint_label.as_str(),
                spinner_frame,
            );
        })?;

        Ok(feedback)
    }
}

/// 提供 CLI TUI 的显式入口。
pub(crate) fn run_tui_v2(config: &Config, setup: &CliSetup, run_mode: TuiRunMode) -> Result<()> {
    TuiApp::bootstrap(config, setup, run_mode)?.run()
}

pub(crate) fn default_session_title(directory: &Path) -> String {
    directory
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(|name| format!("TUI {}", name.trim()))
        .unwrap_or_else(|| "TUI 会话".to_string())
}

pub(crate) fn bootstrap_message_base() -> UiMessageBase {
    UiMessageBase::new(UiMessageId::local("ui-bootstrap-system"))
}

pub(crate) fn bootstrap_system_message(run_mode: TuiRunMode) -> String {
    match run_mode {
        TuiRunMode::Standard => "TUI 已启动。直接在输入区键入内容并按 Enter，即可发起真实的 gateway 轮次；按 F1 查看帮助，按 F2/F3/F4 打开问题、待办和任务面板。".to_string(),
    }
}

pub(crate) fn provider_name_from_model(model_name: &str) -> Option<String> {
    normalize_optional_str_ref(Some(model_name))
        .and_then(|model_name| model_name.split_once('/').map(|(provider_name, _)| provider_name))
        .map(ToOwned::to_owned)
}

fn load_model_catalog_blocking() -> Vec<TuiModelCatalogEntry> {
    let mut entries = block_on_sync(provider::list())
        .into_values()
        .flat_map(|provider_info| {
            let provider_id = provider_info.id;
            let provider_name =
                normalize_catalog_text(provider_info.name.as_str(), provider_id.as_str());
            let mut models = provider_info.models.into_values().collect::<Vec<_>>();
            models.sort_by(|left, right| {
                left.name.cmp(&right.name).then_with(|| left.id.cmp(&right.id))
            });

            models.into_iter().map(move |model| TuiModelCatalogEntry {
                provider_id: provider_id.clone(),
                provider_name: provider_name.clone(),
                model_id: model.id.clone(),
                model_name: normalize_catalog_text(model.name.as_str(), model.id.as_str()),
            })
        })
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        left.provider_name
            .cmp(&right.provider_name)
            .then_with(|| left.provider_id.cmp(&right.provider_id))
            .then_with(|| left.model_name.cmp(&right.model_name))
            .then_with(|| left.model_id.cmp(&right.model_id))
    });
    entries.dedup_by(|left, right| {
        left.provider_id == right.provider_id && left.model_id == right.model_id
    });
    entries
}

pub(crate) fn normalize_catalog_text(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() { fallback.to_string() } else { value.to_string() }
}

pub(crate) fn delegate_agent_options(
    config: &Config,
    selected: Option<&str>,
) -> Vec<DelegateAgentOption> {
    let mut agents = config
        .agents
        .iter()
        .filter_map(|(key, agent)| {
            let key = key.trim();
            let selected = selected == Some(key);
            let safe_key = !key.is_empty()
                && key != "main"
                && key.chars().all(|ch| !ch.is_whitespace() && !ch.is_control());
            if !safe_key || ((!agent.enabled || agent.hidden) && !selected) {
                return None;
            }
            let label = agent
                .label
                .as_deref()
                .map(str::trim)
                .filter(|label| !label.is_empty())
                .unwrap_or(key);
            Some(DelegateAgentOption {
                key: Some(key.to_string()),
                label: crate::cli::tui_v2::terminal_capabilities::sanitize_terminal_text(label),
            })
        })
        .collect::<Vec<_>>();
    agents.sort_by(|left, right| left.key.cmp(&right.key));
    agents.insert(0, DelegateAgentOption::main());
    agents
}

fn block_on_sync<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
        Err(_) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tui_v2 model catalog runtime should initialize");
            runtime.block_on(future)
        }
    }
}

pub(crate) fn gateway_stream_request_from_state(
    state: &TuiState,
    submission: &PromptSubmission,
) -> GatewayChatStreamRequest {
    let snapshot = state.to_chat_session();
    let acp_agent = submission.execution.acp_backend.clone();
    let full_access_enabled = submission.execution.full_access_enabled;
    let subagent_mode_enabled = submission.execution.subagent_mode_enabled;
    let mut allowed_tools = (!full_access_enabled
        && state.session_options.permission_mode == PermissionMode::Plan)
        .then(|| vec!["todowrite".to_string()]);
    if subagent_mode_enabled {
        let tools = allowed_tools.get_or_insert_with(Vec::new);
        tools.retain(|tool| !tool.eq_ignore_ascii_case("AgentTool"));
        tools.push("AgentTool".to_string());
        tools.sort_unstable();
    }
    let mut options = acp_agent
        .as_ref()
        .map(|acp_agent| {
        let cwd = normalize_optional_str_ref(submission.root.as_deref())
            .map(ToOwned::to_owned)
            .or_else(|| state.project.workspace_root.as_ref().map(|path| path.display().to_string()))
            .unwrap_or_default();
        let mut options = json!({
            "acp_test": true,
            "acp_agent": acp_agent,
            "cwd": cwd,
            "acp_force_new_session": submission.execution.acp_force_new_session,
            "acp_permission_mode": if full_access_enabled { "approve-all" } else { "approve-reads" },
        });
        if full_access_enabled {
            options["full_access"] = json!(true);
        }
        options
    })
        .or_else(|| full_access_enabled.then(|| json!({"full_access": true})));
    if subagent_mode_enabled {
        let options = options.get_or_insert_with(|| json!({}));
        options["allowed_tools"] = json!(allowed_tools);
    }
    let system = subagent_mode_enabled.then(|| vec![SUBAGENT_MODE_SYSTEM_PROMPT.to_string()]);
    let acp_allowed_tools = acp_agent.is_some().then(|| allowed_tools.clone()).flatten();
    GatewayChatStreamRequest {
        session_id: normalize_optional_str_ref(submission.session_id.as_deref())
            .map(SessionId::from),
        messages: snapshot.messages.iter().map(gateway_message_from_chat_message).collect(),
        system,
        model: submission.model.clone(),
        reasoning_effort: Some(match state.session_options.effort {
            ReasoningEffort::Minimal => vw_gateway_client::vw_api_types::ReasoningEffortDto::None,
            ReasoningEffort::Low => vw_gateway_client::vw_api_types::ReasoningEffortDto::Low,
            ReasoningEffort::Medium => vw_gateway_client::vw_api_types::ReasoningEffortDto::Medium,
            ReasoningEffort::High => vw_gateway_client::vw_api_types::ReasoningEffortDto::High,
            ReasoningEffort::XHigh => vw_gateway_client::vw_api_types::ReasoningEffortDto::Xhigh,
        }),
        agent: submission.execution.delegate_agent.clone(),
        allowed_tools,
        acp_agent,
        acp_allowed_tools,
        options,
    }
}

const SUBAGENT_MODE_SYSTEM_PROMPT: &str = "## AgentTool-First Mode\n\n\
- Prefer AgentTool for meaningful independent workstreams when delegation improves speed, focus, or verification.\n\
- The parent agent may work directly when delegation would not help, and remains responsible for coordinating, verifying evidence, and synthesizing results.\n\
- If an AgentTool call fails, report the exact blocker and continue safely when the task can still be completed without that delegation.\n\
- Continue to follow all safety, permission, scope, and validation requirements.";

pub(crate) fn gateway_message_from_chat_message(message: &ChatMessage) -> serde_json::Value {
    json!({
        "role": gateway_role(message.role),
        "content": message.content,
    })
}

fn legacy_shadow_request_from_state(
    state: &TuiState,
    submission: &PromptSubmission,
) -> Option<legacy_processor::Request> {
    let session_id = normalize_optional_str_ref(submission.session_id.as_deref())?.to_string();
    let root = normalize_optional_str_ref(submission.root.as_deref())
        .map(ToOwned::to_owned)
        .or_else(|| state.project.workspace_root.as_ref().map(|path| path.display().to_string()));
    let snapshot = state.to_chat_session();

    Some(legacy_processor::Request {
        stream: submission.stream_id.unwrap_or_default(),
        session: session_id,
        query: submission.text.clone(),
        root,
        model: submission.model.clone(),
        options: json!({}),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: snapshot.messages,
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: vw_agent::session::runtime::SessionToolset::default(),
    })
}

pub(crate) fn comparable_usage_from_runtime_terminal(
    terminal: &UiRuntimeTerminalEvent,
) -> TokenUsage {
    match terminal {
        UiRuntimeTerminalEvent::Done { usage, .. }
        | UiRuntimeTerminalEvent::Cancelled { usage, .. }
        | UiRuntimeTerminalEvent::TimedOut { usage, .. } => usage
            .as_ref()
            .map(|usage| TokenUsage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cached_tokens: usage.cached_tokens,
                reasoning_tokens: usage.reasoning_tokens,
            })
            .unwrap_or_default(),
        UiRuntimeTerminalEvent::Error(_) => TokenUsage::default(),
    }
}

pub(crate) fn comparable_terminal_from_runtime_terminal(
    terminal: &UiRuntimeTerminalEvent,
) -> SessionProcessorComparableTerminal {
    match terminal {
        UiRuntimeTerminalEvent::Done { finish_reason, message_id, parent_message_id, .. } => {
            SessionProcessorComparableTerminal::Done {
                finish_reason: finish_reason.clone(),
                message_id: message_id.clone(),
                parent_message_id: parent_message_id.clone(),
            }
        }
        UiRuntimeTerminalEvent::Cancelled { reason, message_id, parent_message_id, .. } => {
            SessionProcessorComparableTerminal::Cancelled {
                reason: reason.clone(),
                message_id: message_id.clone(),
                parent_message_id: parent_message_id.clone(),
            }
        }
        UiRuntimeTerminalEvent::TimedOut { message, message_id, parent_message_id, .. } => {
            SessionProcessorComparableTerminal::TimedOut {
                message: message.clone(),
                message_id: message_id.clone(),
                parent_message_id: parent_message_id.clone(),
            }
        }
        UiRuntimeTerminalEvent::Error(message) => {
            SessionProcessorComparableTerminal::Error(message.clone())
        }
    }
}

fn run_legacy_shadow_compare_blocking(
    request: legacy_processor::Request,
) -> Result<SessionProcessorComparableResult> {
    thread::spawn(move || -> Result<SessionProcessorComparableResult> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| anyhow!("shadow compare runtime init failed: {err}"))?;
        runtime.block_on(run_session_processor_comparable_for_cli(request, None))
    })
    .join()
    .map_err(|_| anyhow!("legacy shadow compare worker panicked"))?
}

pub(crate) fn compare_shadow_results(
    gateway: &SessionProcessorComparableResult,
    legacy: &SessionProcessorComparableResult,
) -> std::result::Result<(), String> {
    let mut diffs = Vec::new();

    if comparable_terminal_kind(gateway.terminal()) != comparable_terminal_kind(legacy.terminal()) {
        diffs.push(format!(
            "terminal {} != {}",
            comparable_terminal_kind(gateway.terminal()),
            comparable_terminal_kind(legacy.terminal())
        ));
    }

    if normalized_compare_output(gateway.output.as_str())
        != normalized_compare_output(legacy.output.as_str())
    {
        diffs.push(format!(
            "output {} != {}",
            compare_output_summary(gateway.output.as_str()),
            compare_output_summary(legacy.output.as_str())
        ));
    }

    if gateway.usage != legacy.usage {
        diffs.push(format!(
            "usage {} != {}",
            usage_summary(&gateway.usage),
            usage_summary(&legacy.usage)
        ));
    }

    if gateway.step_finishes != legacy.step_finishes {
        diffs.push(format!("steps {} != {}", gateway.step_finishes, legacy.step_finishes));
    }

    if diffs.is_empty() { Ok(()) } else { Err(diffs.join("; ")) }
}

fn comparable_terminal_kind(terminal: &SessionProcessorComparableTerminal) -> &'static str {
    match terminal {
        SessionProcessorComparableTerminal::Done { .. } => "done",
        SessionProcessorComparableTerminal::Cancelled { .. } => "cancelled",
        SessionProcessorComparableTerminal::TimedOut { .. } => "timeout",
        SessionProcessorComparableTerminal::Error(_) => "error",
    }
}

fn normalized_compare_output(output: &str) -> &str {
    output.trim_end_matches(['\n', '\r'])
}

fn compare_output_summary(output: &str) -> String {
    let normalized = normalized_compare_output(output);
    let preview = normalized.chars().take(24).collect::<String>();
    if preview.is_empty() {
        "0 chars".to_string()
    } else if normalized.chars().count() > 24 {
        format!("{} chars ({preview}...)", normalized.chars().count())
    } else {
        format!("{} chars ({preview})", normalized.chars().count())
    }
}

fn usage_summary(usage: &TokenUsage) -> String {
    format!(
        "in={} out={} cached={} reasoning={}",
        usage.input_tokens, usage.output_tokens, usage.cached_tokens, usage.reasoning_tokens
    )
}

pub(crate) fn shadow_compare_success_message(result: &SessionProcessorComparableResult) -> String {
    format!(
        "Shadow compare matched legacy on terminal/output/usage/steps (terminal={} steps={} output={}).",
        comparable_terminal_kind(result.terminal()),
        result.step_finishes,
        compare_output_summary(result.output.as_str())
    )
}

trait ComparableTerminalAccess {
    fn terminal(&self) -> &SessionProcessorComparableTerminal;
}

impl ComparableTerminalAccess for SessionProcessorComparableResult {
    fn terminal(&self) -> &SessionProcessorComparableTerminal {
        &self.terminal
    }
}

pub(crate) fn gateway_role(role: ChatRole) -> &'static str {
    match role {
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::System => "system",
        ChatRole::Tool => "tool",
    }
}

/// fullscreen TUI 的 terminal lifecycle 宿主。
///
/// 这里复用 legacy TUI 已验证的 raw mode/alt screen 进入与退出顺序，
/// 但把职责收口到独立类型里，避免 controller 或 renderer 反向持有 terminal 句柄。
struct TuiTerminalLifecycle {
    terminal: TuiTerminal,
    mode_session: TerminalModeSession,
    raw_mode_enabled: bool,
    fullscreen: bool,
}

impl TuiTerminalLifecycle {
    fn enter() -> Result<Self> {
        #[cfg(unix)]
        let screen: CliBackendWriter =
            OpenOptions::new().read(true).write(true).open("/dev/tty")?;

        #[cfg(not(unix))]
        let screen: CliBackendWriter = std::io::stdout();

        let backend = CrosstermBackend::new(screen);
        let mut terminal = Terminal::new(backend)?;
        let mode_plan = TerminalModePlan::for_environment(
            std::io::stdin().is_terminal(),
            std::env::var("TERM").ok().as_deref(),
            std::env::var("TERM_PROGRAM").ok().as_deref(),
        );
        let fullscreen = mode_plan.is_fullscreen();
        let mut mode_session = TerminalModeSession::new(mode_plan);

        if fullscreen {
            enable_raw_mode()?;
        }
        if let Err(error) = mode_session.enter(terminal.backend_mut()) {
            if fullscreen {
                let _ = disable_raw_mode();
            }
            return Err(error.into());
        }
        if fullscreen && let Err(error) = terminal.hide_cursor() {
            let _ = mode_session.leave(terminal.backend_mut());
            let _ = disable_raw_mode();
            return Err(error.into());
        }

        Ok(Self { terminal, mode_session, raw_mode_enabled: fullscreen, fullscreen })
    }

    fn terminal_mut(&mut self) -> &mut TuiTerminal {
        &mut self.terminal
    }
}

impl Drop for TuiTerminalLifecycle {
    fn drop(&mut self) {
        let mut restore_failed = false;
        if self.fullscreen && self.terminal.show_cursor().is_err() {
            restore_failed = true;
        }
        if self.mode_session.suspend(self.terminal.backend_mut()).is_err() {
            restore_failed = true;
        }
        if self.raw_mode_enabled && disable_raw_mode().is_err() {
            restore_failed = true;
        }
        if restore_failed {
            eprintln!("TUI terminal restore failed");
        }
    }
}
