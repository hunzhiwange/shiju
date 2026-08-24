//! TUI 的统一状态容器。
//!
//! 本模块负责承接 Phase 2 里的状态层闭环，不直接处理 terminal I/O、gateway 请求
//! 或 renderer 绘制细节。这里收口三类稳定职责：
//! - `TuiState`：messages、scroll、status、tasks、prompt、overlays、session metadata
//! - snapshot 边界：`ChatSession` 与内部状态之间的双向转换
//! - reducer/selectors 的共享数据结构与辅助方法
//!
//! 当前实现刻意保持保守：
//! - `calls` 仍作为持久化边界上的原始 JSON 保留，不在此 slice 里强行解释
//! - `message_ids` 与 `think_timing` 通过 session metadata 一并保留，避免 round-trip
//!   时丢失共享层已有信息
//! - 当前 runtime pipeline 已覆盖 delta/step/terminal，并把 delta 中约定俗成的
//!   tool/thinking transport 片段规整为独立 `UiMessage`
//! - question/todo 之外的更高阶 grouped/collapsed 视图仍留给后续 slice，不在这里
//!   过早引入 renderer 专用状态

use std::path::PathBuf;

use serde_json::Value;
use vw_shared::session::ui_types::{
    self as session_ui, ChatRole, ChatSession, ChatSessionMeta, ChatSessionStep,
};

use super::model::{
    MessageActionState, OverlayState, PromptState, TranscriptModeState, UiAssistantMessage,
    UiAttachment, UiAttachmentKind, UiCompactBoundary, UiMemoryEntry, UiMessage, UiMessageBase,
    UiMessageId, UiMessageOrigin, UiProgressMessage, UiQuestionOverlay, UiStep, UiStepState,
    UiSystemMessage, UiSystemMessageLevel, UiThinkingBlock, UiThinkingPhase, UiThinkingTiming,
    UiTodoOverlay, UiTokenUsage, UiToolResult, UiTurnTerminal, UiUnknownMessage, UiUserMessage,
};
use crate::cli::session::GitWorkspaceStatus;

pub(crate) mod reducer;
mod runtime_pipeline;
pub(crate) mod selectors;

use self::selectors::{TuiSearchTextCache, TuiTranscriptLayoutCache, TuiTranscriptMeasurements};
use self::selectors::{TuiTranscriptProjectionCache, derive_transcript_projection_cache};

const EXTENDED_MESSAGE_PREFIX: &str = "vibewindow:tui:";
const REASONING_MARKER: &str = "[reasoning]";

fn split_reasoning_marker(raw: &str) -> Option<(&str, &str)> {
    for (marker_start, _) in raw.match_indices(REASONING_MARKER) {
        if marker_start > 0 && !raw[..marker_start].ends_with('\n') {
            continue;
        }

        let suffix = &raw[marker_start + REASONING_MARKER.len()..];
        let reasoning = if let Some(reasoning) = suffix.strip_prefix("\r\n") {
            reasoning
        } else if let Some(reasoning) = suffix.strip_prefix('\n') {
            reasoning
        } else if suffix.is_empty() {
            suffix
        } else {
            continue;
        };
        let visible = raw[..marker_start].trim_end_matches(['\r', '\n']);
        return Some((visible, reasoning));
    }
    None
}

fn reasoning_visible_content(content: Option<String>) -> Option<String> {
    content.map(|content| {
        split_reasoning_marker(&content).map(|(visible, _)| visible.to_string()).unwrap_or(content)
    })
}

pub(crate) use reducer::{
    TuiAction, TuiTerminalUpdate, TuiToolCallUpdate, TuiToolResultUpdate, reduce_tui_state,
};
pub(crate) use runtime_pipeline::apply_runtime_event;
/// 滚动窗口的最小状态。
///
/// 当前滚动状态保留“消息索引锚点”，同时记录 viewport 行高与宽度，
/// 让 selectors 能按真实 wrap/height cache 推导稳定窗口。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiScrollState {
    pub(crate) top_message: usize,
    pub(crate) viewport_messages: usize,
    pub(crate) viewport_height: u16,
    pub(crate) viewport_width: u16,
    pub(crate) overscan: usize,
    pub(crate) follow_tail: bool,
    pub(crate) sticky_message: Option<usize>,
    pub(crate) last_seen_message: Option<usize>,
}

impl Default for TuiScrollState {
    fn default() -> Self {
        Self {
            top_message: 0,
            viewport_messages: 0,
            viewport_height: 0,
            viewport_width: 0,
            overscan: 2,
            follow_tail: true,
            sticky_message: None,
            last_seen_message: None,
        }
    }
}

impl TuiScrollState {
    /// 将滚动位置钳制到当前消息数量允许的范围内。
    pub(crate) fn clamp(&mut self, message_count: usize) {
        self.top_message = self.top_message.min(message_count.saturating_sub(1));
        self.refresh_sticky(message_count);
    }

    /// 让滚动位置对齐到末尾消息。
    pub(crate) fn snap_to_tail(&mut self, message_count: usize) {
        self.top_message = message_count.saturating_sub(1);
        self.refresh_sticky(message_count);
    }

    /// 同步当前 scrollable 宿主反馈出的视口能力。
    pub(crate) fn sync_viewport(&mut self, viewport_height: u16, viewport_width: u16) {
        // 兼容现有 footer/测试摘要；真实窗口裁剪改由 selectors 基于行高推导。
        self.viewport_messages = viewport_height as usize;
        self.viewport_height = viewport_height;
        self.viewport_width = viewport_width;
    }

    /// 将滚动位置对齐到 grouped transcript 的稳定锚点上。
    pub(crate) fn clamp_to_anchors(&mut self, anchors: &[usize]) {
        if anchors.is_empty() {
            self.top_message = 0;
            self.sticky_message = None;
            return;
        }

        let anchor_index = anchor_index_for_message(anchors, self.top_message);
        self.top_message = anchors[anchor_index];
        self.refresh_sticky_from_anchors(anchors, anchor_index);
    }

    /// 让滚动位置对齐到最后一个 grouped transcript 锚点。
    pub(crate) fn snap_to_tail_anchors(&mut self, anchors: &[usize]) {
        if let Some(anchor) = anchors.last().copied() {
            self.top_message = anchor;
            self.refresh_sticky_from_anchors(anchors, anchors.len().saturating_sub(1));
        } else {
            self.top_message = 0;
            self.sticky_message = None;
        }
    }

    fn refresh_sticky(&mut self, message_count: usize) {
        self.sticky_message = if self.follow_tail || message_count == 0 || self.top_message == 0 {
            None
        } else {
            Some(self.top_message.saturating_sub(1))
        };
    }

    fn refresh_sticky_from_anchors(&mut self, anchors: &[usize], anchor_index: usize) {
        self.sticky_message = if self.follow_tail || anchors.is_empty() || anchor_index == 0 {
            None
        } else {
            Some(anchors[anchor_index.saturating_sub(1)])
        };
    }

    fn refresh_seen_tail(&mut self, message_count: usize) {
        if message_count == 0 {
            self.last_seen_message = None;
            return;
        }

        let tail_message = message_count.saturating_sub(1);
        self.last_seen_message = if self.follow_tail {
            Some(tail_message)
        } else {
            Some(self.last_seen_message.unwrap_or(tail_message).min(tail_message))
        };
    }
}

fn anchor_index_for_message(anchors: &[usize], message_index: usize) -> usize {
    anchors.iter().rposition(|anchor| *anchor <= message_index).unwrap_or_default()
}

/// 状态线的基础输入。
///
/// 这里不直接存放渲染后的文案，只保留后续 status selector 会稳定消费的源数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiStatusState {
    pub(crate) session_title: String,
    pub(crate) provider_name: Option<String>,
    pub(crate) model_name: Option<String>,
    pub(crate) turn_terminal: UiTurnTerminal,
    pub(crate) last_error: Option<String>,
    pub(crate) verbose: bool,
}

impl Default for TuiStatusState {
    fn default() -> Self {
        Self {
            session_title: String::new(),
            provider_name: None,
            model_name: None,
            turn_terminal: UiTurnTerminal::Pending,
            last_error: None,
            verbose: false,
        }
    }
}

/// question/todo 相关的任务状态。
///
/// Phase 2 先让状态层直接承接已经在 overlay 模型中定义好的内部镜像类型，
/// 后续 reducer/renderer 可直接复用，不需要再次接触共享层原始载荷。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TuiTaskState {
    pub(crate) pending_questions: Vec<UiQuestionOverlay>,
    /// 当前 Workflow pause 的本地 typed question ID；不包含恢复 token。
    pub(crate) workflow_pause_request_id: Option<String>,
    pub(crate) todo_overlay: Option<UiTodoOverlay>,
    pub(crate) sync_error: Option<String>,
}

/// 单条持久化消息在内部状态里的附属元数据。
///
/// `UiMessage` 本身更偏向渲染模型，这里保留共享快照 round-trip 所需的补充信息：
/// - `raw_message_id`：与 `ChatSession.message_ids` 对齐的原始 ID
/// - `think_timing`：当前共享快照仍挂在 assistant 文本消息上的时间片数据
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TuiPersistedMessage {
    pub(crate) raw_message_id: Option<String>,
    pub(crate) think_timing: Vec<UiThinkingTiming>,
    pub(crate) tool_payload: Option<String>,
    pub(crate) assistant_payload: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedToolMessage {
    tool_name: String,
    command: Option<String>,
    text: String,
    is_error: bool,
}

fn parse_tool_message_payload(raw: &str) -> ParsedToolMessage {
    let trimmed = raw.trim();
    let (tool_name, payload_text) = match trimmed.split_once('\n') {
        Some((header, payload)) => {
            let parsed_name = header
                .trim()
                .strip_prefix("tool ")
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .unwrap_or("tool");
            (parsed_name.to_string(), payload.trim().to_string())
        }
        None => ("tool".to_string(), trimmed.to_string()),
    };

    if let Ok(value) = serde_json::from_str::<Value>(&payload_text) {
        let status = value.get("status").and_then(Value::as_str).unwrap_or("completed");
        let is_error = matches!(status, "error" | "denied");
        let command = value
            .get("input")
            .or_else(|| value.get("title"))
            .and_then(|input| match input {
                Value::Null => None,
                Value::String(text) => {
                    let text = text.trim();
                    (!text.is_empty()).then(|| text.to_string())
                }
                input => Some(input.to_string()),
            });
        let text = if is_error {
            value
                .get("error")
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
                .or_else(|| {
                    value
                        .get("output")
                        .and_then(Value::as_str)
                        .filter(|text| !text.trim().is_empty())
                })
                .unwrap_or(payload_text.as_str())
        } else {
            value
                .get("output")
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
                .unwrap_or(payload_text.as_str())
        };

        return ParsedToolMessage { tool_name, command, text: text.to_string(), is_error };
    }

    ParsedToolMessage { tool_name, command: None, text: payload_text, is_error: false }
}

fn serialize_tool_result_message(message: &UiToolResult) -> String {
    let mut payload = if message.is_error {
        serde_json::json!({
            "status": "error",
            "error": message.content,
        })
    } else {
        serde_json::json!({
            "status": "completed",
            "output": message.content,
        })
    };
    if let Some(command) = message.command.as_deref().filter(|command| !command.trim().is_empty()) {
        payload
            .as_object_mut()
            .expect("tool result payload is an object")
            .insert("input".to_string(), Value::String(command.to_string()));
    }

    format!("tool {}\n{}\n", message.tool_name, payload)
}

/// 会话预览的内部镜像结构。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiSessionPreview {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) updated_ms: u64,
    pub(crate) message_count: usize,
    pub(crate) call_count: usize,
    pub(crate) last_content: Option<String>,
}

impl From<ChatSessionMeta> for TuiSessionPreview {
    fn from(value: ChatSessionMeta) -> Self {
        Self {
            id: value.id,
            title: value.title,
            updated_ms: value.updated_ms,
            message_count: value.message_count,
            call_count: value.call_count,
            last_content: reasoning_visible_content(value.last_content),
        }
    }
}

impl From<&ChatSessionMeta> for TuiSessionPreview {
    fn from(value: &ChatSessionMeta) -> Self {
        Self {
            id: value.id.clone(),
            title: value.title.clone(),
            updated_ms: value.updated_ms,
            message_count: value.message_count,
            call_count: value.call_count,
            last_content: reasoning_visible_content(value.last_content.clone()),
        }
    }
}

impl TuiSessionPreview {
    /// 从完整快照推导一份轻量预览。
    pub(crate) fn from_chat_session(session: &ChatSession) -> Self {
        Self {
            id: session.id.clone(),
            title: session.title.clone(),
            updated_ms: session.updated_ms,
            message_count: session.messages.len(),
            call_count: session.calls.len(),
            last_content: reasoning_visible_content(
                session.messages.last().map(|message| message.content.clone()),
            ),
        }
    }
}

/// 当前会话的持久化元信息。
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct TuiSessionState {
    pub(crate) session_id: Option<String>,
    pub(crate) title: String,
    pub(crate) created_ms: u64,
    pub(crate) updated_ms: u64,
    pub(crate) scope: Option<String>,
    /// session_ui snapshot 对应的持久化文件路径，不是当前 workspace root。
    pub(crate) path: Option<PathBuf>,
    pub(crate) preview: Option<TuiSessionPreview>,
    pub(crate) persisted_calls: Vec<Value>,
    pub(crate) persisted_messages: Vec<TuiPersistedMessage>,
}

/// 当前工作区与本地项目上下文的内部镜像。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TuiProjectContextState {
    pub(crate) workspace_root: Option<PathBuf>,
    pub(crate) info: String,
    pub(crate) git_status: GitWorkspaceStatus,
    pub(crate) memory_evidence: Option<UiMemoryEntry>,
}

/// TUI 可消费的轻量模型目录项。
///
/// 这里不直接复用 provider resolver 的完整模型结构，避免把配置态与 UI suggestion
/// 表面强耦合在一起。当前只保留 slash command 选择模型所需的最小字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiModelCatalogEntry {
    pub(crate) provider_id: String,
    pub(crate) provider_name: String,
    pub(crate) model_id: String,
    pub(crate) model_name: String,
}

impl TuiModelCatalogEntry {
    /// 返回可直接提交给 `/model` 的稳定模型标识。
    pub(crate) fn qualified_id(&self) -> String {
        format!("{}/{}", self.provider_id, self.model_id)
    }

    /// 供 prompt footer 展示的附加说明。
    pub(crate) fn suggestion_detail(&self) -> String {
        if self.model_name.trim().is_empty() || self.model_name == self.model_id {
            format!("供应商: {}", self.provider_name)
        } else {
            format!("{} · {}", self.provider_name, self.model_name)
        }
    }

    /// 按 provider/model 的常见检索字段做不区分大小写的包含匹配。
    pub(crate) fn matches_query(&self, query: &str) -> bool {
        let query = query.trim();
        if query.is_empty() {
            return true;
        }

        let query = query.to_ascii_lowercase();
        self.provider_id.to_ascii_lowercase().contains(&query)
            || self.provider_name.to_ascii_lowercase().contains(&query)
            || self.model_id.to_ascii_lowercase().contains(&query)
            || self.model_name.to_ascii_lowercase().contains(&query)
            || self.qualified_id().to_ascii_lowercase().contains(&query)
    }
}

/// runtime pipeline 在一次流式 turn 内部维护的瞬时状态。
///
/// 这些字段不会进入 snapshot，也不会跨 turn 保留；
/// 它们仅用于把 transport 层分块文本稳定规整为 `UiMessage`。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TuiRuntimeState {
    pub(crate) active_plan_run_request_id: Option<String>,
    pub(crate) active_plan_run_task_id: Option<String>,
    pub(crate) thinking_open: bool,
    pub(crate) thinking_depth: u32,
    pub(crate) thinking_redacted: bool,
    pub(crate) event_time_ms: Option<u64>,
    pub(crate) last_event_time_ms: Option<u64>,
}

/// TUI 的顶层状态容器。
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct TuiState {
    pub(crate) messages: Vec<UiMessage>,
    pub(crate) unseen_baseline_id: Option<UiMessageId>,
    pub(crate) search_index: TuiSearchTextCache,
    pub(crate) transcript: TuiTranscriptProjectionCache,
    pub(crate) transcript_layout: TuiTranscriptLayoutCache,
    pub(crate) scroll: TuiScrollState,
    pub(crate) status: TuiStatusState,
    pub(crate) tasks: TuiTaskState,
    pub(crate) prompt: PromptState,
    pub(crate) overlays: OverlayState,
    pub(crate) project: TuiProjectContextState,
    pub(crate) session: TuiSessionState,
    pub(crate) runtime: TuiRuntimeState,
    pub(crate) model_catalog: Vec<TuiModelCatalogEntry>,
    pub(crate) transcript_mode: TranscriptModeState,
    pub(crate) message_actions: MessageActionState,
    pub(crate) session_options: super::model::SessionOptions,
    pub(crate) session_capabilities: super::model::SessionCapabilityMatrix,
    pub(crate) settings_warnings: Vec<String>,
    pub(crate) integration_catalog: Vec<super::model::integration::IntegrationItem>,
    pub(crate) integration_warnings: Vec<String>,
    pub(crate) orchestration: super::runtime::orchestration::OrchestrationSnapshot,
    pub(crate) theme_name: String,
    pub(crate) agent_view: super::model::agent_view::AgentViewState,
    pub(crate) notifications: super::model::notification::NotificationStore,
}

impl TuiState {
    /// 从共享层 `ChatSession` 快照恢复出内部状态。
    ///
    /// 当前恢复策略遵循“持久化两层化”：
    /// - 文本消息进入 `UiMessage`
    /// - `message_ids`/`think_timing`/`calls` 保存在 session metadata，供 round-trip 使用
    /// - `steps` 转成 `UiMessage::Step`，为后续 grouped/collapsed 渲染铺底
    pub(crate) fn from_chat_session(session: &ChatSession) -> Self {
        let mut state = Self::default();
        let active_terminal = terminal_from_snapshot(session);
        let last_assistant_index = session
            .messages
            .iter()
            .enumerate()
            .filter_map(|(index, message)| (message.role == ChatRole::Assistant).then_some(index))
            .next_back();

        state.session.session_id = Some(session.id.clone());
        state.session.title = session.title.clone();
        state.session.created_ms = session.created_ms;
        state.session.updated_ms = session.updated_ms;
        state.session.preview = Some(TuiSessionPreview::from_chat_session(session));
        state.session.persisted_calls = session.calls.clone();

        state.status.session_title = session.title.clone();
        state.status.turn_terminal = active_terminal.clone();
        state.status.model_name = session.steps.last().and_then(|step| step.model.clone());

        for (index, message) in session.messages.iter().enumerate() {
            let raw_message_id = session.message_ids.get(index).cloned().flatten();
            let base = snapshot_message_base(session.id.as_str(), index, raw_message_id.clone());
            let mut pending_reasoning = None;
            let mut assistant_payload = None;

            let ui_message = match message.role {
                ChatRole::User => {
                    UiMessage::User(UiUserMessage { base, text: message.content.clone() })
                }
                ChatRole::Assistant => {
                    let text = if let Some((visible, reasoning)) =
                        split_reasoning_marker(&message.content)
                    {
                        assistant_payload = Some(message.content.clone());
                        if !reasoning.is_empty() {
                            pending_reasoning = Some(UiMessage::Thinking(UiThinkingBlock {
                                base: UiMessageBase::new(UiMessageId::local(format!(
                                    "snapshot-{index}-reasoning"
                                )))
                                .with_session_id(session.id.as_str()),
                                summary: reducer::thinking_summary(reasoning),
                                content: reasoning.to_string(),
                                timing: message
                                    .think_timing
                                    .iter()
                                    .map(UiThinkingTiming::from)
                                    .collect(),
                                collapsed: true,
                                phase: UiThinkingPhase::Closed,
                            }));
                        }
                        visible.to_string()
                    } else {
                        message.content.clone()
                    };
                    UiMessage::Assistant(UiAssistantMessage {
                        base,
                        text,
                        usage: UiTokenUsage::default(),
                        step_count: if Some(index) == last_assistant_index {
                            session.steps.len()
                        } else {
                            0
                        },
                        terminal: if Some(index) == last_assistant_index {
                            active_terminal.clone()
                        } else {
                            UiTurnTerminal::Done { finish_reason: None }
                        },
                        model: if Some(index) == last_assistant_index {
                            state.status.model_name.clone()
                        } else {
                            None
                        },
                    })
                }
                ChatRole::Tool => {
                    let parsed = parse_tool_message_payload(&message.content);
                    UiMessage::ToolResult(UiToolResult {
                        base,
                        call_id: None,
                        tool_name: parsed.tool_name,
                        command: parsed.command,
                        content: parsed.text,
                        is_error: parsed.is_error,
                    })
                }
                ChatRole::System => {
                    deserialize_extended_message(message.content.as_str(), base.clone())
                        .unwrap_or_else(|| {
                            UiMessage::System(UiSystemMessage {
                                base,
                                text: message.content.clone(),
                                level: UiSystemMessageLevel::Info,
                            })
                        })
                }
            };

            state.messages.push(ui_message);
            if let Some(reasoning) = pending_reasoning {
                state.messages.push(reasoning);
            }
            state.session.persisted_messages.push(TuiPersistedMessage {
                raw_message_id,
                think_timing: message.think_timing.iter().map(UiThinkingTiming::from).collect(),
                tool_payload: (message.role == ChatRole::Tool).then(|| message.content.clone()),
                assistant_payload,
            });
        }

        for step in &session.steps {
            state
                .messages
                .push(UiMessage::Step(ui_step_from_snapshot_step(session.id.as_str(), step)));
        }

        state.refresh_search_index();
        state.refresh_session_preview();
        state.refresh_transcript_projection();
        state.clamp_scroll();
        state
    }

    /// 将内部状态重新压回共享层 `ChatSession` 快照。
    ///
    /// 当前回写共享层已经稳定定义的持久化交集：
    /// - user/assistant/system/tool 文本消息
    /// - `message_ids` 与 `think_timing`
    /// - 原始 `calls` JSON
    /// - step 统计信息
    ///
    /// 其他更细粒度的 thinking/tool-call UI 消息会继续留在内部状态里，等待后续子任务
    /// 决定持久化策略。
    pub(crate) fn to_chat_session(&self) -> ChatSession {
        let mut messages = Vec::new();
        let mut message_ids = Vec::new();
        let mut steps = Vec::new();
        let mut persisted_index = 0usize;

        for message in &self.messages {
            if let Some(chat_message) = snapshot_chat_message_from_ui_message(
                message,
                self.session.persisted_messages.get(persisted_index),
            ) {
                message_ids.push(
                    self.session
                        .persisted_messages
                        .get(persisted_index)
                        .and_then(|metadata| metadata.raw_message_id.clone())
                        .or_else(|| raw_message_id_from_ui_message(message)),
                );
                messages.push(chat_message);
                persisted_index = persisted_index.saturating_add(1);
                continue;
            }

            if let UiMessage::Step(step) = message {
                // `ChatSessionStep.index` 在 sqlite 中是 `(session_id, step_index)` 主键的一部分。
                // 但 TUI live runtime 的 step 编号会在每个 turn 内从 1 重新开始，所以这里要在
                // 序列化阶段把整段 transcript 里的 step 重新编号成 session 内唯一序号，避免
                // 多 turn 持久化时因为重复 step_index 导致整笔事务回滚。
                let mut snapshot_step = snapshot_step_from_ui_step(step);
                snapshot_step.index =
                    u32::try_from(steps.len().saturating_add(1)).unwrap_or(u32::MAX);
                steps.push(snapshot_step);
            }
        }

        ChatSession {
            id: self.session.session_id.clone().unwrap_or_default(),
            title: self.session.title.clone(),
            messages,
            message_ids,
            calls: self.session.persisted_calls.clone(),
            steps,
            created_ms: self.session.created_ms,
            updated_ms: self.session.updated_ms,
        }
    }

    /// 向状态尾部追加一条新消息，并同步 session preview。
    pub(crate) fn append_message(&mut self, mut message: UiMessage) {
        attach_session_id(self.session.session_id.as_deref(), &mut message);

        if let Some(metadata) = persisted_message_from_ui_message(&message) {
            self.session.persisted_messages.push(metadata);
        }

        self.messages.push(message);
        let message_index = self.messages.len().saturating_sub(1);
        self.refresh_search_index_for_message(message_index);
        self.refresh_transcript_projection();
        self.refresh_session_preview();
        self.clamp_scroll();
    }

    /// 在历史头部插入消息，同时按稳定消息 ID 保持手动滚动锚点。
    pub(crate) fn prepend_messages_preserving_anchor(&mut self, mut messages: Vec<UiMessage>) {
        if messages.is_empty() {
            return;
        }
        let anchor_id = (!self.scroll.follow_tail)
            .then(|| {
                self.messages.get(self.scroll.top_message).map(|message| message.base().id.clone())
            })
            .flatten();
        for message in &mut messages {
            attach_session_id(self.session.session_id.as_deref(), message);
        }
        messages.append(&mut self.messages);
        self.messages = messages;
        self.refresh_search_index();
        self.refresh_transcript_projection();
        if let Some(anchor_id) = anchor_id
            && let Some(index) =
                self.messages.iter().position(|message| message.base().id == anchor_id)
        {
            self.scroll.top_message = index;
        }
        self.refresh_session_preview();
        self.clamp_scroll();
    }

    /// 基于当前消息序列刷新可搜索文本缓存。
    pub(crate) fn refresh_search_index(&mut self) {
        self.search_index.rebuild(&self.messages);
    }

    /// 只刷新一条消息对应的 searchable text cache entry。
    pub(crate) fn refresh_search_index_for_message(&mut self, message_index: usize) {
        self.search_index.refresh_message(&self.messages, message_index);
    }

    /// 基于当前消息序列刷新 grouped transcript 投影视图缓存。
    ///
    /// 当前缓存只收口顶层 transcript item 边界与消息锚点，
    /// 为 S4-2a 的 virtual window 与 scroll anchor 提供统一输入，
    /// 后续 wrap/height cache 会在此基础上继续扩展。
    pub(crate) fn refresh_transcript_projection(&mut self) {
        self.transcript = derive_transcript_projection_cache(&self.messages);
        self.transcript_layout.reconcile_projection(&self.messages, &self.transcript);
        self.refresh_transcript_layout_for_current_width();
    }

    /// 为当前 viewport width 预热对应的 transcript layout bucket。
    pub(crate) fn refresh_transcript_layout_for_current_width(&mut self) {
        let content_width = self.scroll.viewport_width;
        self.transcript_layout.rebuild_width(
            &self.messages,
            &self.transcript,
            content_width,
            self.status.verbose,
        );
    }

    /// 合并 renderer 对当前可见窗口的真实行高测量结果。
    pub(crate) fn apply_transcript_measurements(
        &mut self,
        measurements: &TuiTranscriptMeasurements,
    ) {
        self.transcript_layout.rebuild_width(
            &self.messages,
            &self.transcript,
            measurements.content_width,
            measurements.verbose,
        );
        self.transcript_layout.apply_measurements(measurements);
    }

    /// 只刷新一条消息所属 transcript item 的 wrap/height cache。
    pub(crate) fn refresh_transcript_layout_for_message(&mut self, message_index: usize) {
        self.transcript_layout.refresh_message(&self.messages, &self.transcript, message_index);
    }

    /// 依据当前消息与会话元信息刷新预览数据。
    pub(crate) fn refresh_session_preview(&mut self) {
        let Some(session_id) = self.session.session_id.as_ref() else {
            self.session.preview = None;
            self.status.session_title = self.session.title.clone();
            return;
        };

        self.status.session_title = self.session.title.clone();

        self.session.preview = Some(TuiSessionPreview {
            id: session_id.clone(),
            title: self.session.title.clone(),
            updated_ms: self.session.updated_ms,
            message_count: self.session.persisted_messages.len(),
            call_count: self.session.persisted_calls.len(),
            last_content: self.messages.iter().rev().find_map(last_content_from_ui_message),
        });
    }

    /// 清空当前 transcript 与持久化镜像，但保留 workspace/session 上下文。
    pub(crate) fn clear_messages(&mut self) {
        self.messages.clear();
        self.unseen_baseline_id = None;
        self.search_index = TuiSearchTextCache::default();
        self.transcript = TuiTranscriptProjectionCache::default();
        self.transcript_layout.clear();
        self.scroll.top_message = 0;
        self.scroll.follow_tail = true;
        self.scroll.sticky_message = None;
        self.scroll.last_seen_message = None;
        self.status.turn_terminal = UiTurnTerminal::Pending;
        self.status.last_error = None;
        self.tasks.pending_questions.clear();
        self.tasks.todo_overlay = None;
        self.tasks.sync_error = None;
        self.overlays.clear();
        self.runtime = TuiRuntimeState::default();
        self.session.persisted_messages.clear();
        self.session.persisted_calls.clear();
        self.refresh_session_preview();
    }

    /// 将滚动位置钳制到当前消息数量允许的范围内。
    pub(crate) fn clamp_scroll(&mut self) {
        let anchors = self.transcript.anchors();
        if self.scroll.follow_tail {
            self.scroll.snap_to_tail_anchors(anchors);
        } else {
            self.scroll.clamp_to_anchors(anchors);
        }
        self.scroll.refresh_seen_tail(self.messages.len());
        if self.scroll.follow_tail {
            self.unseen_baseline_id = None;
        }
    }
}

pub(super) fn message_base_mut(message: &mut UiMessage) -> &mut UiMessageBase {
    match message {
        UiMessage::User(message) => &mut message.base,
        UiMessage::Assistant(message) => &mut message.base,
        UiMessage::ToolCall(message) => &mut message.base,
        UiMessage::ToolResult(message) => &mut message.base,
        UiMessage::Thinking(message) => &mut message.base,
        UiMessage::Step(message) => &mut message.base,
        UiMessage::System(message) => &mut message.base,
        UiMessage::Error(message) => &mut message.base,
        UiMessage::Attachment(message) => &mut message.base,
        UiMessage::Progress(message) => &mut message.base,
        UiMessage::CompactBoundary(message) => &mut message.base,
        UiMessage::Unknown(message) => &mut message.base,
    }
}

pub(super) fn is_persistable_chat_message(message: &UiMessage) -> bool {
    match message {
        UiMessage::User(_)
        | UiMessage::Assistant(_)
        | UiMessage::ToolResult(_)
        | UiMessage::Attachment(_)
        | UiMessage::Progress(_)
        | UiMessage::CompactBoundary(_)
        | UiMessage::Unknown(_) => true,
        UiMessage::System(message) => !is_ui_local_message_id(message.base.id.as_str()),
        _ => false,
    }
}

pub(super) fn persisted_slot_index_for_message_index(
    messages: &[UiMessage],
    message_index: usize,
) -> Option<usize> {
    if message_index >= messages.len() || !is_persistable_chat_message(&messages[message_index]) {
        return None;
    }

    let mut persisted_index = 0usize;
    for (index, message) in messages.iter().enumerate() {
        if !is_persistable_chat_message(message) {
            continue;
        }
        if index == message_index {
            return Some(persisted_index);
        }
        persisted_index = persisted_index.saturating_add(1);
    }
    None
}

pub(super) fn raw_message_id_from_ui_message(message: &UiMessage) -> Option<String> {
    message.id().as_str().strip_prefix("gateway:").map(ToOwned::to_owned)
}

fn attach_session_id(session_id: Option<&str>, message: &mut UiMessage) {
    let Some(session_id) = session_id else {
        return;
    };
    let base = message_base_mut(message);
    if base.session_id.is_none() {
        base.session_id = Some(session_id.to_string());
    }
}

fn snapshot_message_base(
    session_id: &str,
    index: usize,
    raw_message_id: Option<String>,
) -> UiMessageBase {
    let base = match raw_message_id {
        Some(message_id) => UiMessageBase::new(UiMessageId::gateway(message_id)),
        None => UiMessageBase::new(UiMessageId::local(format!("snapshot-{index}"))),
    };

    base.with_session_id(session_id)
}

fn ui_step_from_snapshot_step(session_id: &str, step: &ChatSessionStep) -> UiStep {
    let mut base = UiMessageBase::new(UiMessageId::local(format!("snapshot-step-{}", step.index)))
        .with_session_id(session_id)
        .with_created_ms(step.started_ms);

    if let Some(path) = step.start_snapshot_path.as_deref() {
        base.parent_id = Some(UiMessageId::local(path.to_string()));
    }

    UiStep {
        base,
        step_index: step.index,
        started_ms: step.started_ms,
        finished_ms: step.finished_ms,
        usage: UiTokenUsage::from(&step.usage),
        finish_reason: step.finish_reason.clone(),
        model: step.model.clone(),
        state: step_state_from_snapshot(step),
    }
}

fn step_state_from_snapshot(step: &ChatSessionStep) -> UiStepState {
    if step.finished_ms.is_none() {
        return UiStepState::Running;
    }

    let finish_reason = normalize_optional_string(step.finish_reason.clone());
    if contains_marker(finish_reason.as_deref(), &["cancelled", "canceled", "aborted"]) {
        return UiStepState::Cancelled;
    }
    if contains_marker(finish_reason.as_deref(), &["error", "failed", "failure"]) {
        return UiStepState::Failed;
    }
    UiStepState::Complete
}

fn terminal_from_snapshot(session: &ChatSession) -> UiTurnTerminal {
    if let Some(step) = session.steps.last() {
        if step.finished_ms.is_none() {
            return UiTurnTerminal::Streaming;
        }

        let finish_reason = normalize_optional_string(step.finish_reason.clone());
        if contains_marker(finish_reason.as_deref(), &["timeout", "timed out", "deadline exceeded"])
        {
            return UiTurnTerminal::TimedOut {
                message: finish_reason.unwrap_or_else(|| "session timed out".to_string()),
            };
        }
        if contains_marker(
            finish_reason.as_deref(),
            &["cancelled", "canceled", "interrupted", "aborted"],
        ) {
            return UiTurnTerminal::Cancelled { reason: finish_reason };
        }
        if contains_marker(finish_reason.as_deref(), &["error", "failed", "failure"]) {
            return UiTurnTerminal::Error {
                message: finish_reason.unwrap_or_else(|| "session failed".to_string()),
            };
        }
        return UiTurnTerminal::Done { finish_reason };
    }

    match session.messages.last().map(|message| message.role) {
        Some(ChatRole::Assistant | ChatRole::System | ChatRole::Tool) => {
            UiTurnTerminal::Done { finish_reason: None }
        }
        Some(ChatRole::User) | None => UiTurnTerminal::Pending,
    }
}

fn persisted_message_from_ui_message(message: &UiMessage) -> Option<TuiPersistedMessage> {
    is_persistable_chat_message(message).then(|| TuiPersistedMessage {
        raw_message_id: raw_message_id_from_ui_message(message),
        think_timing: Vec::new(),
        tool_payload: match message {
            UiMessage::ToolResult(message) => Some(serialize_tool_result_message(message)),
            _ => None,
        },
        assistant_payload: None,
    })
}

fn snapshot_chat_message_from_ui_message(
    message: &UiMessage,
    metadata: Option<&TuiPersistedMessage>,
) -> Option<session_ui::ChatMessage> {
    if !is_persistable_chat_message(message) {
        return None;
    }

    match message {
        UiMessage::User(message) => Some(session_ui::ChatMessage {
            role: ChatRole::User,
            content: message.text.clone(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }),
        UiMessage::Assistant(message) => Some(session_ui::ChatMessage {
            role: ChatRole::Assistant,
            content: metadata
                .and_then(|metadata| metadata.assistant_payload.clone())
                .unwrap_or_else(|| message.text.clone()),
            model: None,
            think_timing: metadata
                .map(|metadata| {
                    metadata.think_timing.iter().map(snapshot_think_timing).collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            processing_timing: None,
        }),
        UiMessage::ToolResult(message) => Some(session_ui::ChatMessage {
            role: ChatRole::Tool,
            content: metadata
                .and_then(|metadata| metadata.tool_payload.clone())
                .unwrap_or_else(|| serialize_tool_result_message(message)),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }),
        UiMessage::System(message) => Some(session_ui::ChatMessage {
            role: ChatRole::System,
            content: message.text.clone(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }),
        UiMessage::Attachment(_)
        | UiMessage::Progress(_)
        | UiMessage::CompactBoundary(_)
        | UiMessage::Unknown(_) => Some(session_ui::ChatMessage {
            role: ChatRole::System,
            content: serialize_extended_message(message),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }),
        _ => None,
    }
}

fn snapshot_step_from_ui_step(step: &UiStep) -> ChatSessionStep {
    ChatSessionStep {
        index: step.step_index,
        turn_id: None,
        started_ms: step.started_ms,
        finished_ms: step.finished_ms,
        start_snapshot_path: step.base.parent_id.as_ref().map(|parent| parent.as_str().to_string()),
        finish_snapshot_path: None,
        usage: snapshot_token_usage(&step.usage),
        cost_usd: None,
        finish_reason: step.finish_reason.clone(),
        model: step.model.clone(),
        tool_timings: Vec::new(),
    }
}

fn snapshot_token_usage(usage: &UiTokenUsage) -> session_ui::TokenUsage {
    session_ui::TokenUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cached_tokens: usage.cached_tokens,
        reasoning_tokens: usage.reasoning_tokens,
    }
}

fn snapshot_think_timing(timing: &UiThinkingTiming) -> session_ui::ThinkTiming {
    session_ui::ThinkTiming {
        start_ms: timing.start_ms,
        end_ms: timing.end_ms,
        last_update_ms: timing.last_update_ms,
    }
}

fn last_content_from_ui_message(message: &UiMessage) -> Option<String> {
    match message {
        UiMessage::User(message) => Some(message.text.clone()),
        UiMessage::Assistant(message) => Some(message.text.clone()),
        UiMessage::ToolResult(message) => Some(if message.content.trim().is_empty() {
            message.tool_name.clone()
        } else {
            format!("{}: {}", message.tool_name, message.content)
        }),
        UiMessage::System(message) if !is_ui_local_message_id(message.base.id.as_str()) => {
            Some(message.text.clone())
        }
        UiMessage::Attachment(message) => Some(message.name.clone()),
        UiMessage::Progress(message) => Some(message.label.clone()),
        UiMessage::CompactBoundary(message) => Some(message.summary.clone()),
        UiMessage::Unknown(message) => Some(message.summary.clone()),
        _ => None,
    }
}

fn serialize_extended_message(message: &UiMessage) -> String {
    let (base, content) = match message {
        UiMessage::Attachment(message) => (
            &message.base,
            serde_json::json!({
                "type": "attachment",
                "kind": match message.kind {
                    UiAttachmentKind::Image => "image",
                    UiAttachmentKind::File => "file",
                    UiAttachmentKind::Link => "link",
                    UiAttachmentKind::PastedText => "pasted_text",
                    UiAttachmentKind::Resource => "resource",
                },
                "name": message.name,
                "media_type": message.media_type,
                "uri": message.uri,
                "alt_text": message.alt_text,
                "width_px": message.width_px,
                "height_px": message.height_px,
                "size_bytes": message.size_bytes,
                "source": message.source,
                "preview": message.preview,
            }),
        ),
        UiMessage::Progress(message) => (
            &message.base,
            serde_json::json!({
                "type": "progress",
                "label": message.label,
                "current": message.current,
                "total": message.total,
                "completed": message.completed,
            }),
        ),
        UiMessage::CompactBoundary(message) => (
            &message.base,
            serde_json::json!({
                "type": "compact_boundary",
                "summary": message.summary,
                "compacted_messages": message.compacted_messages,
            }),
        ),
        UiMessage::Unknown(message) => (
            &message.base,
            serde_json::json!({
                "type": "unknown",
                "external_kind": message.external_kind,
                "summary": message.summary,
            }),
        ),
        _ => return String::new(),
    };

    let origin = match base.origin.as_ref() {
        Some(UiMessageOrigin::Gateway { event_type }) => {
            serde_json::json!({"type": "gateway", "event_type": event_type})
        }
        Some(UiMessageOrigin::Command { name }) => {
            serde_json::json!({"type": "command", "name": name})
        }
        Some(UiMessageOrigin::Local) => serde_json::json!({"type": "local"}),
        None => Value::Null,
    };
    format!(
        "{EXTENDED_MESSAGE_PREFIX}{}",
        serde_json::json!({
            "parent_id": base.parent_id.as_ref().map(UiMessageId::as_str),
            "created_ms": base.created_ms,
            "origin": origin,
            "content": content,
        })
    )
}

fn deserialize_extended_message(raw: &str, mut base: UiMessageBase) -> Option<UiMessage> {
    let payload = raw.strip_prefix(EXTENDED_MESSAGE_PREFIX)?;
    let envelope = serde_json::from_str::<Value>(payload).ok()?;
    if let Some(parent_id) = envelope.get("parent_id").and_then(Value::as_str) {
        base.parent_id = Some(UiMessageId::new(parent_id));
    }
    base.created_ms = envelope.get("created_ms").and_then(Value::as_u64);
    base.origin = deserialize_message_origin(envelope.get("origin"));

    let content = envelope.get("content")?;
    let content_type = content.get("type").and_then(Value::as_str)?;
    match content_type {
        "attachment" => Some(UiMessage::Attachment(UiAttachment {
            base,
            kind: match content.get("kind").and_then(Value::as_str) {
                Some("image") => UiAttachmentKind::Image,
                Some("link") => UiAttachmentKind::Link,
                Some("pasted_text") => UiAttachmentKind::PastedText,
                Some("resource") => UiAttachmentKind::Resource,
                _ => UiAttachmentKind::File,
            },
            name: content.get("name").and_then(Value::as_str)?.to_string(),
            media_type: json_optional_string(content.get("media_type")),
            uri: content.get("uri").and_then(Value::as_str).unwrap_or_default().to_string(),
            alt_text: json_optional_string(content.get("alt_text")),
            width_px: content
                .get("width_px")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok()),
            height_px: content
                .get("height_px")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok()),
            size_bytes: content.get("size_bytes").and_then(Value::as_u64),
            source: json_optional_string(content.get("source")),
            preview: json_optional_string(content.get("preview")),
        })),
        "progress" => Some(UiMessage::Progress(UiProgressMessage {
            base,
            label: content.get("label").and_then(Value::as_str)?.to_string(),
            current: content.get("current").and_then(Value::as_u64),
            total: content.get("total").and_then(Value::as_u64),
            completed: content.get("completed").and_then(Value::as_bool).unwrap_or(false),
        })),
        "compact_boundary" => Some(UiMessage::CompactBoundary(UiCompactBoundary {
            base,
            summary: content.get("summary").and_then(Value::as_str)?.to_string(),
            compacted_messages: content
                .get("compacted_messages")
                .and_then(Value::as_u64)
                .and_then(|count| usize::try_from(count).ok())
                .unwrap_or_default(),
        })),
        "unknown" => Some(UiMessage::Unknown(UiUnknownMessage {
            base,
            external_kind: content
                .get("external_kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            summary: content.get("summary").and_then(Value::as_str)?.to_string(),
        })),
        _ => None,
    }
}

fn deserialize_message_origin(value: Option<&Value>) -> Option<UiMessageOrigin> {
    let value = value?;
    match value.get("type").and_then(Value::as_str) {
        Some("gateway") => Some(UiMessageOrigin::Gateway {
            event_type: value
                .get("event_type")
                .and_then(Value::as_str)
                .unwrap_or("chat.content")
                .to_string(),
        }),
        Some("command") => Some(UiMessageOrigin::Command {
            name: value.get("name").and_then(Value::as_str).unwrap_or("unknown").to_string(),
        }),
        Some("local") => Some(UiMessageOrigin::Local),
        _ => None,
    }
}

fn json_optional_string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(ToOwned::to_owned)
}

fn is_ui_local_message_id(message_id: &str) -> bool {
    message_id.starts_with("local:")
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn contains_marker(value: Option<&str>, markers: &[&str]) -> bool {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };

    let normalized = value.to_ascii_lowercase();
    markers.iter().any(|marker| normalized.contains(marker))
}
