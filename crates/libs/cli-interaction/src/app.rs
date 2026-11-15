//! 将 Gateway 会话运行时、TUI 状态与活动聊天界面连接起来的组装根。
//!
//! 子模块按启动、终端循环、会话、面板和活动运行器职责组织；
//! Gateway 事件到界面事件的语义转换留在 `bridge`，不在主循环中重复。

use cli_interaction_model::snapshots::{CronDetailView, CronJobSnapshot, OrchestrationSnapshot};
use cli_gateway_transport::stream_adapter::terminal_from_error_message;
use anyhow::{Result, anyhow};
use crossterm::event::{self, Event, KeyEventKind};
use dialoguer::Editor;
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::text::Line;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use vw_gateway_client::vw_api_types::task::TaskPlanRunRequest;
use vw_gateway_client::vw_api_types::workflow::{WorkflowResumeRequest, WorkflowRunResponse};
use vw_gateway_client::{
    GatewayChatStreamEvent, GatewayChatStreamRequest, GatewayStreamAttemptResult,
    GatewayWorkflowStreamEvent, PendingPermissionReplyDto,
};
use vw_shared::session::ui_types::{ChatSession, ChatSessionMeta};
use vw_tui::{
    ChatWidget, ChatWidgetConfig, CustomTerminal, TerminalMode, TranscriptPager,
    TranscriptReflowState, V3Action, V3Attachment, V3AttachmentKind, V3DisplayEvent, V3Event,
    V3SelectionItem, V3StepStatus, insert_history_lines, run_terminal,
};

use super::bridge::GatewayV3Bridge;
use super::commands::{
    CommandRoute, HOOK_CATALOG_SURFACE, OrchestrationPanelKind, SCHEDULE_SELECTION_SURFACE,
    SESSION_COMPACT_SURFACE, SESSION_RESET_SURFACE, SESSION_RESUME_SURFACE,
    SETTINGS_ACCESS_SURFACE, SETTINGS_ACP_SURFACE, SETTINGS_DELEGATE_SURFACE,
    SETTINGS_EFFORT_SURFACE, SETTINGS_MODEL_SURFACE, SETTINGS_PERMISSION_SURFACE,
    SETTINGS_PLAN_SURFACE, SETTINGS_SUBAGENT_SURFACE, SETTINGS_SURFACE, SettingsTarget,
    TASK_SELECTION_SURFACE, WORKFLOW_SELECTION_SURFACE, WORKSPACE_FILES_SURFACE,
    command_palette_items_with_integrations, help_text, session_resume_items,
};
use super::controller::{TuiController, TuiControllerCommand, TuiOverlayCommand};
use cli_interaction_model::background_task::{TaskControlAction, TaskDetailView};
use cli_interaction_model::integration::{IntegrationItem, IntegrationKind};
use cli_interaction_model::permission::{PermissionDecision, PermissionQueue, PermissionRequest};
use cli_interaction_model::settings_panel::{SettingKey, SettingsPanel, SettingsValues};
use cli_interaction_model::{
    PermissionMode, PromptSubmission, ReasoningEffort, UiAttachment, UiAttachmentKind,
    UiConfirmIntent, UiConfirmOverlay, UiMessage, UiMessageBase, UiMessageId, UiMessageOrigin,
    UiOverlay, UiPermissionOverlay, UiStepState, UiTodoOverlay, UiTurnTerminal, UiUserMessage,
};
use cli_gateway_runtime::capabilities::load_session_capabilities_blocking;
use cli_gateway_runtime::cron::{
    CronMutation, load_owned_cron_detail_blocking,
    mutate_owned_cron_and_refresh_blocking,
};
use cli_gateway_runtime::diagnostics::{GatewayDiagnosticsReport, load_gateway_diagnostics_blocking};
use cli_gateway_runtime::file_search::search_files_blocking;
use cli_gateway_transport::gateway::{GatewayUiRuntime, normalize_optional_str_ref};
use cli_gateway_runtime::hooks::{
    HookAuditView, HookCatalogItem, HookCatalogView, HookMutationRefresh, load_hook_audit_blocking,
    load_hook_catalog_blocking, mutate_hook_and_refresh_blocking,
};
use cli_gateway_runtime::integrations::{
    MemoryMutation, delete_integration_blocking, load_integrations_blocking,
    mutate_memory_and_refresh_blocking, query_memory_blocking, set_integration_enabled_blocking,
};
use cli_gateway_transport::interaction::InteractionSnapshot;
use cli_gateway_runtime::orchestration::{
    control_owned_task_blocking, load_orchestration_blocking,
    load_owned_task_detail_blocking, load_owned_workflow_detail_blocking,
};
use cli_gateway_runtime::plan_run::{
    plan_run_request, stop_owned_plan_run_blocking, stream_plan_run_resumable_blocking,
};
use cli_gateway_runtime::session_diff::load_session_diff_blocking;
use cli_interaction_model::{UiRuntimeEvent, UiRuntimeTerminalEvent};

use cli_gateway_runtime::workflow_run::{
    OwnedWorkflowRunRequest, WorkflowRunOwnership, resume_workflow_run_blocking,
    stream_workflow_run_blocking, workflow_run_request,
};
use cli_gateway_runtime::workspace_files::{
    WorkspaceFileListing, WorkspaceFileText, load_workspace_files_blocking,
    read_workspace_file_blocking,
};
use cli_interaction_state::{TuiAction, TuiState, reduce_tui_state};

type V3Terminal = CustomTerminal<CrosstermBackend<io::Stdout>>;
const SESSION_HISTORY_DISPLAY_LIMIT: usize = 100;
pub(crate) const HOST_MESSAGE_BUDGET: usize = 64;
pub(crate) const STREAM_EVENT_BUDGET: usize = 64;
pub(crate) const TERMINAL_EVENT_BUDGET: usize = 64;

enum StreamMessage {
    ChatEvent(GatewayChatStreamEvent),
    ChatPublished { preview: ChatSessionMeta, recovered: Option<Box<ChatSession>> },
    WorkflowEvent(GatewayWorkflowStreamEvent),
    RuntimeEvent(UiRuntimeEvent),
    Finished(Result<GatewayStreamAttemptResult, String>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActiveStreamKind {
    Chat,
    PlanRun,
    Workflow,
}

struct ActiveStream {
    kind: ActiveStreamKind,
    request_id: String,
    receiver: Receiver<StreamMessage>,
    cancel: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveHookMutation {
    plugin_id: String,
    hook_hash: String,
    trusted: bool,
}

/// 外部编辑器调用边界；输入当前草稿，返回替换文本或用户取消。
pub(crate) type EditorAction = dyn Fn(&str) -> Result<Option<String>, String> + Send + Sync;

enum HostMessage {
    SessionTitleLoaded {
        directory: PathBuf,
        session_id: String,
        request_id: String,
        title: String,
    },
    SessionBootstrapFinished(Result<Box<TuiState>, String>),
    DirectorySwitchFinished {
        generation: u64,
        source_directory: PathBuf,
        recovery_draft: String,
        result: Result<Box<DirectorySwitchBootstrap>, String>,
    },
    EditorFinished(Result<Option<String>, String>),
    StopFinished {
        request_id: String,
        result: Result<bool, String>,
    },
    PlanRunStopFinished {
        request_id: String,
        task_id: String,
        result: Result<bool, String>,
    },
    InteractionsRefreshed(Result<InteractionSnapshot, String>),
    PermissionReplyFinished {
        request_id: String,
        result: Result<(), String>,
    },
    QuestionReplyFinished {
        request_id: String,
        result: Result<(), String>,
    },
    WorkflowResumeFinished {
        pause_request_id: String,
        result: Result<Box<WorkflowRunResponse>, String>,
    },
    FileSearchFinished {
        generation: u64,
        query: String,
        result: Result<Vec<vw_tui::V3FileSearchMatch>, String>,
    },
    SessionDiffFinished {
        generation: u64,
        session_id: Option<String>,
        result: Result<vw_tui::V3FileDiff, String>,
    },
    DoctorFinished {
        generation: u64,
        directory: PathBuf,
        result: Result<GatewayDiagnosticsReport, String>,
    },
    CronDetailFinished {
        generation: u64,
        directory: PathBuf,
        job_id: String,
        result: Result<Box<CronDetailView>, String>,
    },
    TaskDetailFinished {
        generation: u64,
        directory: PathBuf,
        task_id: String,
        result: Result<Box<TaskDetailView>, String>,
    },
    WorkspaceFilesFinished {
        generation: u64,
        directory: PathBuf,
        requested_path: String,
        result: Result<WorkspaceFileListing, String>,
    },
    WorkspaceFileReadFinished {
        generation: u64,
        directory: PathBuf,
        requested_path: String,
        result: Result<WorkspaceFileText, String>,
    },
    HookCatalogFinished {
        generation: u64,
        directory: PathBuf,
        result: Result<HookCatalogView, String>,
    },
    HookAuditFinished {
        generation: u64,
        directory: PathBuf,
        result: Result<HookAuditView, String>,
    },
    HookMutationFinished {
        generation: u64,
        directory: PathBuf,
        plugin_id: String,
        hook_hash: String,
        trusted: bool,
        result: Result<HookMutationRefresh, String>,
    },
    MemorySearchFinished {
        generation: u64,
        directory: PathBuf,
        query: Option<String>,
        result: Result<Vec<IntegrationItem>, String>,
    },
    MemoryMutationFinished {
        generation: u64,
        directory: PathBuf,
        query: Option<String>,
        action: &'static str,
        key: String,
        result: Result<Vec<IntegrationItem>, String>,
    },
}

/// 事件循环执行一次操作后的继续或退出指示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerControl {
    Continue,
    Quit,
}

/// 在渲染 crate 之外统一管理活跃流、transcript 和本地副作生命周期。
pub(crate) struct ActiveRunner {
    active_stream: Option<ActiveStream>,
    chat_submission: Option<(String, Vec<V3Attachment>)>,
    title_request_id: Option<String>,
    workflow_run: Option<WorkflowRunOwnership>,
    transcript_pager: Option<TranscriptPager>,
    session_bootstrap_in_flight: bool,
    directory_switch_generation: u64,
    directory_switch_source: Option<PathBuf>,
    editor_in_flight: bool,
    interaction_in_flight: bool,
    file_search_generation: u64,
    file_search_query: Option<String>,
    session_diff_generation: u64,
    doctor_generation: u64,
    cron_generation: u64,
    task_detail_generation: u64,
    workspace_files_generation: u64,
    hooks_generation: u64,
    memory_generation: u64,
    memory_mutation_generation: Option<u64>,
    memory_snapshot_directory: Option<PathBuf>,
    memory_snapshot_query: Option<String>,
    hook_catalog_directory: Option<PathBuf>,
    hook_catalog: Option<HookCatalogView>,
    hook_mutation: Option<ActiveHookMutation>,
    editor: Arc<EditorAction>,
    host_sender: mpsc::Sender<HostMessage>,
    host_receiver: Receiver<HostMessage>,
}

fn push_runner_error(app: &mut V3GatewayApp, message: impl Into<String>) {
    app.widget_mut().push_history(V3DisplayEvent::Error { message: message.into() });
}

/// Gateway 驱动的 TUI 组装根，持有运行时、状态、控制器与渲染部件。
#[derive(Debug)]
pub(crate) struct DirectorySwitchBootstrap {
    runtime: GatewayUiRuntime,
    state: TuiState,
}

impl DirectorySwitchBootstrap {
    pub(crate) fn new(runtime: GatewayUiRuntime, state: TuiState) -> Self {
        Self { runtime, state }
    }
}

pub(crate) struct V3GatewayApp {
    runtime: GatewayUiRuntime,
    state: TuiState,
    controller: TuiController,
    widget: ChatWidget,
    bridge: GatewayV3Bridge,
    terminal_mode: TerminalMode,
    transcript_reflow: TranscriptReflowState,
    interaction_refresh_requested: bool,
    pending_first_response: bool,
}

mod runner;
mod runner_commands;
mod runner_actions;
mod runner_panels;
mod runner_host;
mod runner_stream;
mod application;
mod interactions;
mod sessions;
mod settings;
mod integrations;
mod orchestration;
mod transcript;
mod view;
mod startup;
mod terminal;
mod workers;
mod request_ids;

use view::build_session_widget;
use view::setting_item;
use view::read_only_item;
use view::selectable_item;
use view::effort_id;
use view::permission_mode_id;
use view::display_event_from_message;
pub use startup::run_tui;
pub(crate) use startup::initial_tui_state;
pub(crate) use startup::restore_or_create_session_state;
pub(crate) use startup::bootstrap_directory_switch;
use startup::new_draft_id;
pub(crate) use terminal::apply_ready_terminal_events;
use terminal::run_event_loop;
use workers::spawn_stream;
use workers::spawn_session_title_read;
use workers::spawn_file_search;
use workers::spawn_session_diff;
use workers::spawn_doctor;
use workers::spawn_cron_detail;
use workers::spawn_task_detail;
use workers::spawn_workspace_files;
use workers::spawn_workspace_file_read;
use workers::spawn_hook_catalog;
use workers::spawn_hook_audit;
use workers::spawn_hook_mutation;
use workers::spawn_memory_search;
use workers::spawn_memory_mutation;
use workers::spawn_plan_run;
use workers::spawn_workflow_run;
use workers::spawn_stop;
use workers::spawn_plan_run_stop;
use workers::spawn_interaction_refresh;
use workers::spawn_session_bootstrap;
use workers::spawn_directory_switch;
use workers::spawn_permission_reply;
use workers::spawn_question_reply;
use workers::spawn_workflow_resume;
use request_ids::next_stream_request_id;
use request_ids::next_plan_run_request_id;
use request_ids::next_workflow_run_request_id;

#[cfg(test)]
#[path = "session_title_tests.rs"]
mod session_title_tests;


use cli_interaction_model::paste::PromptAttachmentKind;
