//! 新 TUI 的弹层状态模型。
//!
//! 这一层只定义 overlay 的内部表示与焦点关系，不直接包含 renderer、快捷键或网络请求。
//! 当前先覆盖 confirm/search/question/todo/task/palette/error 七类稳定宿主。

use vw_shared::question;
use vw_shared::todo::Todo;
use vw_tui::{
    V3ApprovalChoice, V3ApprovalDecision, V3ApprovalKind, V3ApprovalRequest, V3UserInputOption,
    V3UserInputQuestion, V3UserInputRequest,
};

use super::agent_view::AgentViewState;
use super::background_task::BackgroundTaskPanel;
use super::integration::{IntegrationKind, IntegrationPanel};
use super::permission::{PermissionPreviewKind, PermissionQueue};
use super::schedule_panel::SchedulePanel;
use super::settings_panel::{SettingsPanel, SettingsValues};
use super::ui_message::{UiMessageId, UiStepState, UiTokenUsage, UiTurnTerminal};
use super::workflow_panel::WorkflowPanel;

/// 当前焦点位于 prompt 还是 overlay。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum OverlayFocus {
    #[default]
    Prompt,
    Overlay,
}

/// overlay 顶层状态。
///
/// 采用简单栈结构承接后续 phase 的 modal manager；当前不引入额外路由层。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct OverlayState {
    pub(crate) stack: Vec<UiOverlay>,
    pub(crate) focus: OverlayFocus,
}

impl OverlayState {
    /// 压入一个新的 overlay，并把焦点切到 overlay 区域。
    pub(crate) fn push(&mut self, overlay: UiOverlay) {
        self.stack.push(overlay);
        self.focus = OverlayFocus::Overlay;
    }

    /// 弹出顶部 overlay；如果栈空则把焦点还给 prompt。
    pub(crate) fn pop(&mut self) -> Option<UiOverlay> {
        let overlay = self.stack.pop();
        if self.stack.is_empty() {
            self.focus = OverlayFocus::Prompt;
        }
        overlay
    }

    /// 返回当前活动 overlay。
    pub(crate) fn active(&self) -> Option<&UiOverlay> {
        self.stack.last()
    }

    /// 清空所有 overlay，并恢复 prompt 焦点。
    pub(crate) fn clear(&mut self) {
        self.stack.clear();
        self.focus = OverlayFocus::Prompt;
    }
}

/// overlay 的逻辑种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiOverlayKind {
    Confirm,
    Search,
    Question,
    Todo,
    Task,
    CommandPalette,
    Report,
    Error,
    Mcp,
    Memory,
    Settings,
    Permissions,
    Integration,
    BackgroundTasks,
    Agents,
    Workflows,
    Schedules,
}

/// 顶层 overlay 枚举。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UiOverlay {
    Confirm(UiConfirmOverlay),
    Search(UiSearchOverlay),
    Question(UiQuestionOverlay),
    Todo(UiTodoOverlay),
    Task(UiTaskOverlay),
    CommandPalette(UiCommandPaletteOverlay),
    Report(UiReportOverlay),
    Error(UiErrorOverlay),
    Mcp(UiMcpOverlay),
    Memory(UiMemoryOverlay),
    Settings(SettingsPanel),
    Permissions(UiPermissionOverlay),
    Integration(IntegrationPanel),
    BackgroundTasks(BackgroundTaskPanel),
    Agents(AgentViewState),
    Workflows(WorkflowPanel),
    Schedules(SchedulePanel),
}

impl UiOverlay {
    /// 返回 overlay 的逻辑分类。
    pub(crate) fn kind(&self) -> UiOverlayKind {
        match self {
            Self::Confirm(_) => UiOverlayKind::Confirm,
            Self::Search(_) => UiOverlayKind::Search,
            Self::Question(_) => UiOverlayKind::Question,
            Self::Todo(_) => UiOverlayKind::Todo,
            Self::Task(_) => UiOverlayKind::Task,
            Self::CommandPalette(_) => UiOverlayKind::CommandPalette,
            Self::Report(_) => UiOverlayKind::Report,
            Self::Error(_) => UiOverlayKind::Error,
            Self::Mcp(_) => UiOverlayKind::Mcp,
            Self::Memory(_) => UiOverlayKind::Memory,
            Self::Settings(_) => UiOverlayKind::Settings,
            Self::Permissions(_) => UiOverlayKind::Permissions,
            Self::Integration(_) => UiOverlayKind::Integration,
            Self::BackgroundTasks(_) => UiOverlayKind::BackgroundTasks,
            Self::Agents(_) => UiOverlayKind::Agents,
            Self::Workflows(_) => UiOverlayKind::Workflows,
            Self::Schedules(_) => UiOverlayKind::Schedules,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UiPermissionOverlay {
    pub(crate) queue: PermissionQueue,
    pub(crate) notice: Option<String>,
}

impl UiPermissionOverlay {
    /// 将当前队首权限转换为只读安全弹层；不会在渲染层制造授权结果。
    pub(crate) fn current_request(&self) -> Option<V3ApprovalRequest> {
        let request = self.queue.current()?;
        let kind = match request.preview {
            PermissionPreviewKind::Shell => V3ApprovalKind::Command,
            PermissionPreviewKind::File => V3ApprovalKind::FileChange,
            PermissionPreviewKind::Network => V3ApprovalKind::Network,
            PermissionPreviewKind::Plan
            | PermissionPreviewKind::Skill
            | PermissionPreviewKind::Fallback => V3ApprovalKind::Permissions,
        };
        let mut detail = request
            .explanation
            .clone()
            .unwrap_or_else(|| request.patterns.join("\n"));
        if detail.trim().is_empty() {
            detail = request.permission.clone();
        }
        let mut choices = vec![V3ApprovalChoice::new(
            "allow-once",
            "Allow once",
            V3ApprovalDecision::Allow,
        )];
        if request.always_available() {
            choices.push(V3ApprovalChoice::new(
                "allow-session",
                "Allow for session",
                V3ApprovalDecision::AllowForSession,
            ));
        }
        choices.extend([
            V3ApprovalChoice::new("deny", "Deny", V3ApprovalDecision::Deny),
            V3ApprovalChoice::new("cancel", "Cancel", V3ApprovalDecision::Cancel),
        ]);
        Some(V3ApprovalRequest::new(
            request.id.clone(),
            kind,
            format!("Permission: {}", request.permission),
            detail,
            choices,
        ))
    }
}

/// 确认框携带的类型化意图。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum UiConfirmIntent {
    #[default]
    Generic,
    ExitTui,
    ClearSession,
    ResetSession {
        message_id: String,
    },
    CompactSession {
        message_id: String,
    },
    IntegrationDelete {
        kind: IntegrationKind,
        id: String,
    },
    ScheduleDelete {
        id: String,
    },
    EnableFullAccess {
        values: Box<SettingsValues>,
    },
}

/// 确认对话框。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiConfirmOverlay {
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) confirm_label: String,
    pub(crate) cancel_label: String,
    pub(crate) destructive: bool,
    pub(crate) intent: UiConfirmIntent,
}

/// 搜索命中的一条结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiSearchMatch {
    pub(crate) message_id: Option<UiMessageId>,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) preview: String,
}

/// 搜索弹层。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UiSearchOverlay {
    pub(crate) query: String,
    pub(crate) matches: Vec<UiSearchMatch>,
    pub(crate) selected_index: Option<usize>,
    pub(crate) case_sensitive: bool,
}

/// question 里的单个可选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiQuestionOption {
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) preview: Option<String>,
}

/// question 里的单个问题定义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiQuestionPrompt {
    pub(crate) header: String,
    pub(crate) question: String,
    pub(crate) options: Vec<UiQuestionOption>,
    pub(crate) multiple: bool,
    pub(crate) allow_custom_input: bool,
}

/// question 对应的工具调用上下文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiQuestionToolContext {
    pub(crate) message_id: String,
    pub(crate) call_id: String,
}

/// question 在宿主里的展示表面类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiQuestionSurfaceKind {
    Question,
    ToolFallback,
    PermissionRequest,
}

/// question 弹层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiQuestionOverlay {
    pub(crate) request_id: String,
    pub(crate) session_id: String,
    pub(crate) prompts: Vec<UiQuestionPrompt>,
    pub(crate) answers: Vec<Vec<String>>,
    pub(crate) tool: Option<UiQuestionToolContext>,
    pub(crate) selected_index: usize,
}

impl UiQuestionOverlay {
    /// 将共享层 question 请求转为新 TUI 内部弹层结构。
    pub(crate) fn from_request(request: &question::Request) -> Self {
        let prompts = request
            .questions
            .iter()
            .map(|info| UiQuestionPrompt {
                header: info.header.clone(),
                question: info.question.clone(),
                options: info
                    .options
                    .iter()
                    .map(|option| UiQuestionOption {
                        label: option.label.clone(),
                        description: option.description.clone(),
                        preview: option.preview.clone(),
                    })
                    .collect(),
                multiple: info.multiple.unwrap_or(false),
                allow_custom_input: info.custom.unwrap_or(false),
            })
            .collect::<Vec<_>>();

        Self {
            request_id: request.id.clone(),
            session_id: request.session_id.clone(),
            answers: vec![Vec::new(); prompts.len()],
            prompts,
            tool: request.tool.as_ref().map(|tool| UiQuestionToolContext {
                message_id: tool.message_id.clone(),
                call_id: tool.call_id.clone(),
            }),
            selected_index: 0,
        }
    }

    /// 转为渲染 crate 的只读提问结构，使用稳定索引保持回答顺序。
    pub(crate) fn to_v3_request(&self) -> V3UserInputRequest {
        let questions = self
            .prompts
            .iter()
            .enumerate()
            .map(|(index, prompt)| {
                let mut question = V3UserInputQuestion::new(
                    index.to_string(),
                    prompt.header.clone(),
                    prompt.question.clone(),
                    prompt
                        .options
                        .iter()
                        .map(|option| {
                            V3UserInputOption::new(
                                option.label.clone(),
                                option.description.clone(),
                            )
                        })
                        .collect(),
                );
                question.allow_other = prompt.allow_custom_input;
                question
            })
            .collect();
        V3UserInputRequest::new(self.request_id.clone(), questions)
    }

    /// 当前 question 是否来自工具侧的通用退化问题面。
    pub(crate) fn is_tool_backed(&self) -> bool {
        self.tool.is_some()
    }

    /// 当前 question 应使用哪种表面类型呈现。
    pub(crate) fn surface_kind(&self) -> UiQuestionSurfaceKind {
        if self.is_permission_request() {
            UiQuestionSurfaceKind::PermissionRequest
        } else if self.is_tool_backed() {
            UiQuestionSurfaceKind::ToolFallback
        } else {
            UiQuestionSurfaceKind::Question
        }
    }

    /// 当前 question 是否看起来像工具审批请求。
    pub(crate) fn is_permission_request(&self) -> bool {
        self.is_tool_backed()
            && self.prompts.iter().any(|prompt| {
                contains_permission_marker(prompt.header.as_str())
                    || contains_permission_marker(prompt.question.as_str())
                    || prompt.options.iter().any(|option| {
                        option_looks_like_permission(option.label.as_str())
                            || option_looks_like_permission(option.description.as_str())
                    })
            })
    }

    /// question modal 在当前宿主下的展示标题。
    pub(crate) fn modal_title(&self) -> &'static str {
        match self.surface_kind() {
            UiQuestionSurfaceKind::Question => "提问",
            UiQuestionSurfaceKind::ToolFallback => "工具提问回退",
            UiQuestionSurfaceKind::PermissionRequest => "权限请求",
        }
    }

    /// 当前表面的统一请求名词。
    pub(crate) fn request_label(&self) -> &'static str {
        match self.surface_kind() {
            UiQuestionSurfaceKind::Question => "提问",
            UiQuestionSurfaceKind::ToolFallback => "工具提问",
            UiQuestionSurfaceKind::PermissionRequest => "权限请求",
        }
    }

    /// 当前表面在提交失败时的错误标题。
    pub(crate) fn reply_error_title(&self) -> &'static str {
        match self.surface_kind() {
            UiQuestionSurfaceKind::Question => "提问回复失败",
            UiQuestionSurfaceKind::ToolFallback => "工具提问回复失败",
            UiQuestionSurfaceKind::PermissionRequest => "权限请求回复失败",
        }
    }

    /// 当前表面在拒绝失败时的错误标题。
    pub(crate) fn reject_error_title(&self) -> &'static str {
        match self.surface_kind() {
            UiQuestionSurfaceKind::Question => "提问拒绝失败",
            UiQuestionSurfaceKind::ToolFallback => "工具提问拒绝失败",
            UiQuestionSurfaceKind::PermissionRequest => "权限请求拒绝失败",
        }
    }

    /// 当前表面在空提交时的提示标题。
    pub(crate) fn empty_submission_title(&self) -> &'static str {
        match self.surface_kind() {
            UiQuestionSurfaceKind::Question => "提交提问回复",
            UiQuestionSurfaceKind::ToolFallback => "提交工具提问回复",
            UiQuestionSurfaceKind::PermissionRequest => "提交权限请求回复",
        }
    }

    /// 当前表面在空提交时的提示文案。
    pub(crate) fn empty_submission_message(&self) -> &'static str {
        match self.surface_kind() {
            UiQuestionSurfaceKind::Question => "请至少提供一个回答后再提交。",
            UiQuestionSurfaceKind::ToolFallback => {
                "请至少提供一个回答后再提交该工具回退问题，或按 Ctrl+R 明确拒绝。"
            }
            UiQuestionSurfaceKind::PermissionRequest => {
                "请先选择一个授权选项后再提交，或按 Ctrl+R 明确拒绝。"
            }
        }
    }
}

fn contains_permission_marker(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    ["approval", "approve", "permission", "allow", "deny", "reject"]
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn option_looks_like_permission(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    ["allow", "approve", "deny", "reject", "always"]
        .iter()
        .any(|marker| normalized.contains(marker))
}

/// todo 条目的内部视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiTodoItem {
    pub(crate) id: String,
    pub(crate) content: String,
    pub(crate) status: String,
    pub(crate) priority: String,
}

impl From<&Todo> for UiTodoItem {
    fn from(value: &Todo) -> Self {
        Self {
            id: value.id.clone(),
            content: value.content.clone(),
            status: value.status.clone(),
            priority: value.priority.clone(),
        }
    }
}

/// todo 弹层。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UiTodoOverlay {
    pub(crate) session_id: Option<String>,
    pub(crate) items: Vec<UiTodoItem>,
    pub(crate) selected_index: usize,
    pub(crate) dirty: bool,
}

impl UiTodoOverlay {
    /// 基于共享层 todo 列表创建内部弹层视图。
    pub(crate) fn from_todos(session_id: Option<&str>, todos: &[Todo]) -> Self {
        Self {
            session_id: session_id.map(ToOwned::to_owned),
            items: todos.iter().map(UiTodoItem::from).collect(),
            selected_index: 0,
            dirty: false,
        }
    }
}

/// task 面板中的单条 step 摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiTaskStepItem {
    pub(crate) message_id: UiMessageId,
    pub(crate) step_index: u32,
    pub(crate) state: UiStepState,
    pub(crate) started_ms: u64,
    pub(crate) finished_ms: Option<u64>,
    pub(crate) model: Option<String>,
    pub(crate) finish_reason: Option<String>,
    pub(crate) usage: UiTokenUsage,
}

/// task / step 摘要面板。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UiTaskOverlay {
    pub(crate) session_id: Option<String>,
    pub(crate) turn_terminal: UiTurnTerminal,
    pub(crate) pending_questions: usize,
    pub(crate) todo_count: usize,
    pub(crate) sync_error: Option<String>,
    pub(crate) steps: Vec<UiTaskStepItem>,
    pub(crate) selected_index: usize,
}

/// palette 中的一条命令项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiPaletteItem {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) detail: Option<String>,
    pub(crate) source: String,
    pub(crate) argument_hint: Option<String>,
    pub(crate) available: bool,
    pub(crate) unavailable_reason: Option<String>,
    pub(crate) replacement: String,
}

/// 命令面板弹层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiCommandPaletteOverlay {
    pub(crate) query: String,
    pub(crate) items: Vec<UiPaletteItem>,
    pub(crate) selected_index: Option<usize>,
    pub(crate) first_visible_index: usize,
    pub(crate) visible_rows: usize,
    pub(crate) notice: Option<String>,
}

impl Default for UiCommandPaletteOverlay {
    fn default() -> Self {
        Self {
            query: String::new(),
            items: Vec::new(),
            selected_index: None,
            first_visible_index: 0,
            visible_rows: 8,
            notice: None,
        }
    }
}

/// 只读命令报告弹层。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UiReportOverlay {
    pub(crate) title: String,
    pub(crate) lines: Vec<String>,
    pub(crate) capability_notice: Option<String>,
    pub(crate) scroll_offset: usize,
}

/// 错误弹层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiErrorOverlay {
    pub(crate) title: String,
    pub(crate) message: String,
    pub(crate) recoverable: bool,
}

/// MCP 服务器的传输协议类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum McpServerTransport {
    Stdio,
    Sse,
    Http,
}

impl McpServerTransport {
    /// 返回传输协议的显示标签。
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Sse => "sse",
            Self::Http => "http",
        }
    }
}

/// MCP 服务器的 UI 视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiMcpServerInfo {
    pub(crate) name: String,
    pub(crate) transport: McpServerTransport,
    /// stdio 时为命令路径，sse/http 时为 URL。
    pub(crate) address: String,
}

/// MCP 服务器列表面板。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UiMcpOverlay {
    pub(crate) servers: Vec<UiMcpServerInfo>,
    pub(crate) selected_index: usize,
    /// 配置来源路径（project 或 global），用于显示。
    pub(crate) config_source: String,
}

/// 单个内存文件条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiMemoryEntry {
    /// 作用域标签："project" / "global"。
    pub(crate) scope: String,
    /// 文件名（不含路径）。
    pub(crate) filename: String,
    /// 完整文件路径，用于显示。
    pub(crate) path: String,
    /// 文件前 N 行预览内容。
    pub(crate) preview_lines: Vec<String>,
    /// 文件总行数。
    pub(crate) total_lines: usize,
}

/// 内存文件列表面板。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UiMemoryOverlay {
    pub(crate) entries: Vec<UiMemoryEntry>,
    pub(crate) selected_index: usize,
}
