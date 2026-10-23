//! 将 Gateway 会话运行时、TUI 状态与活动聊天界面连接起来的组装根。
//!
//! 本模块负责终端生命周期、会话恢复、流式请求启动和事件循环；
//! Gateway 事件到界面事件的语义转换留在 `bridge`，不在主循环中重复。

use crate::app::agent::config::Config;
use crate::cli::setup::CliSetup;
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use crossterm::event::{self, Event, KeyEventKind};
use dialoguer::Editor;
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::text::Line;
use serde_json::json;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use vw_gateway_client::vw_api_types::id::SessionId;
use vw_gateway_client::vw_api_types::task::TaskPlanRunRequest;
use vw_gateway_client::vw_api_types::workflow::{WorkflowResumeRequest, WorkflowRunResponse};
use vw_gateway_client::{
    GatewayChatStreamEvent, GatewayChatStreamRequest, GatewayStreamAttemptResult,
    GatewayWorkflowStreamEvent, PendingPermissionReplyDto,
};
use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};
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
use super::model::background_task::{TaskControlAction, TaskDetailView};
use super::model::integration::{IntegrationItem, IntegrationKind};
use super::model::permission::{PermissionDecision, PermissionQueue, PermissionRequest};
use super::model::settings_panel::{SettingKey, SettingsPanel, SettingsValues};
use super::model::{
    PermissionMode, PromptSubmission, ReasoningEffort, UiAttachment, UiAttachmentKind,
    UiConfirmIntent, UiConfirmOverlay, UiMessage, UiMessageBase, UiMessageId, UiMessageOrigin,
    UiOverlay, UiPermissionOverlay, UiStepState, UiTodoOverlay, UiTurnTerminal, UiUserMessage,
};
use super::paste::{PromptAttachment, PromptAttachmentKind};
use super::runtime::capabilities::load_session_capabilities_blocking;
use super::runtime::cron::{
    CronDetailView, CronJobSnapshot, CronMutation, load_owned_cron_detail_blocking,
    mutate_owned_cron_and_refresh_blocking,
};
use super::runtime::diagnostics::{GatewayDiagnosticsReport, load_gateway_diagnostics_blocking};
use super::runtime::file_search::search_files_blocking;
use super::runtime::gateway::{GatewayUiRuntime, normalize_optional_str_ref};
use super::runtime::hooks::{
    HookAuditView, HookCatalogItem, HookCatalogView, HookMutationRefresh, load_hook_audit_blocking,
    load_hook_catalog_blocking, mutate_hook_and_refresh_blocking,
};
use super::runtime::integrations::{
    MemoryMutation, delete_integration_blocking, load_integrations_blocking,
    mutate_memory_and_refresh_blocking, query_memory_blocking, set_integration_enabled_blocking,
};
use super::runtime::interaction::InteractionSnapshot;
use super::runtime::orchestration::{
    OrchestrationSnapshot, control_owned_task_blocking, load_orchestration_blocking,
    load_owned_task_detail_blocking, load_owned_workflow_detail_blocking,
};
use super::runtime::plan_run::{
    plan_run_request, stop_owned_plan_run_blocking, stream_plan_run_resumable_blocking,
};
use super::runtime::session_diff::load_session_diff_blocking;
use super::runtime::stream_adapter::{UiRuntimeEvent, UiRuntimeTerminalEvent};
use super::runtime::workflow_run::{
    OwnedWorkflowRunRequest, WorkflowRunOwnership, resume_workflow_run_blocking,
    stream_workflow_run_blocking, workflow_run_request,
};
use super::runtime::workspace_files::{
    WorkspaceFileListing, WorkspaceFileText, load_workspace_files_blocking,
    read_workspace_file_blocking,
};
use super::state::{TuiAction, TuiState, reduce_tui_state};

type V3Terminal = CustomTerminal<CrosstermBackend<io::Stdout>>;
const SESSION_HISTORY_DISPLAY_LIMIT: usize = 100;
pub(crate) const HOST_MESSAGE_BUDGET: usize = 64;
pub(crate) const STREAM_EVENT_BUDGET: usize = 64;
pub(crate) const TERMINAL_EVENT_BUDGET: usize = 64;

enum StreamMessage {
    ChatEvent(GatewayChatStreamEvent),
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

impl ActiveRunner {
    fn new() -> Self {
        Self::new_with_editor(Arc::new(|draft| {
            Editor::new().edit(draft).map_err(|error| error.to_string())
        }))
    }

    /// 用可注入的编辑器动作创建 runner，便于将外部进程与事件循环解耦。
    pub(crate) fn new_with_editor(editor: Arc<EditorAction>) -> Self {
        let (host_sender, host_receiver) = mpsc::channel();
        Self {
            active_stream: None,
            workflow_run: None,
            transcript_pager: None,
            session_bootstrap_in_flight: false,
            directory_switch_generation: 0,
            directory_switch_source: None,
            editor_in_flight: false,
            interaction_in_flight: false,
            file_search_generation: 0,
            file_search_query: None,
            session_diff_generation: 0,
            doctor_generation: 0,
            cron_generation: 0,
            task_detail_generation: 0,
            workspace_files_generation: 0,
            hooks_generation: 0,
            memory_generation: 0,
            memory_mutation_generation: None,
            memory_snapshot_directory: None,
            memory_snapshot_query: None,
            hook_catalog_directory: None,
            hook_catalog: None,
            hook_mutation: None,
            editor,
            host_sender,
            host_receiver,
        }
    }

    /// 将界面操作转为宿主命令并执行，错误会投影为界面消息而非中止循环。
    pub(crate) fn execute_action(
        &mut self,
        app: &mut V3GatewayApp,
        action: V3Action,
    ) -> RunnerControl {
        let recovery = match &action {
            V3Action::Submit { text, attachments } | V3Action::Queue { text, attachments } => {
                Some((text.clone(), attachments.clone()))
            }
            _ => None,
        };
        if app.state().session.session_id.is_none()
            && matches!(action, V3Action::Submit { .. } | V3Action::Queue { .. })
        {
            if let Some((text, attachments)) = recovery {
                app.widget_mut().restore_submission(text.as_str(), attachments);
            }
            push_runner_error(app, "session history is still loading; retry in a moment");
            return RunnerControl::Continue;
        }
        if self.active_stream.is_some() && matches!(action, V3Action::Submit { .. }) {
            if let Some((text, attachments)) = recovery {
                app.widget_mut().restore_submission(text.as_str(), attachments);
            }
            push_runner_error(
                app,
                "a gateway stream is already active; queue the follow-up instead",
            );
            return RunnerControl::Continue;
        }
        if self.directory_switch_source.is_some()
            && matches!(action, V3Action::Submit { .. } | V3Action::Queue { .. })
        {
            if let Some((text, attachments)) = recovery {
                app.widget_mut().restore_submission(text.as_str(), attachments);
            }
            push_runner_error(app, "a directory switch is already in progress");
            return RunnerControl::Continue;
        }
        let command = match app.handle_action(action) {
            Ok(command) => command,
            Err(error) => {
                if let Some((text, attachments)) = recovery {
                    app.widget_mut().restore_submission(text.as_str(), attachments);
                }
                push_runner_error(app, error);
                app.show_next_interaction();
                return RunnerControl::Continue;
            }
        };
        self.execute_command(app, command)
    }

    fn execute_command(
        &mut self,
        app: &mut V3GatewayApp,
        command: TuiControllerCommand,
    ) -> RunnerControl {
        match command {
            TuiControllerCommand::Continue => RunnerControl::Continue,
            TuiControllerCommand::FileSearchQueryChanged { query } => {
                self.start_file_search(app, query);
                RunnerControl::Continue
            }
            TuiControllerCommand::Quit => RunnerControl::Quit,
            TuiControllerCommand::CancelActiveSubmission => {
                if let Some(stream) = self.active_stream.as_ref() {
                    match stream.kind {
                        ActiveStreamKind::Chat => {
                            stream.cancel.store(true, Ordering::Release);
                            app.accept_gateway_event(GatewayChatStreamEvent::Error(
                                "cancelled by user".to_string(),
                            ));
                            spawn_stop(
                                app.runtime().clone(),
                                stream.request_id.clone(),
                                self.host_sender.clone(),
                            );
                        }
                        ActiveStreamKind::PlanRun => {
                            let owned_request_id =
                                app.state().runtime.active_plan_run_request_id.clone();
                            let owned_task_id = app.state().runtime.active_plan_run_task_id.clone();
                            let Some(task_id) = owned_task_id else {
                                push_runner_error(
                                    app,
                                    "cannot stop plan-run before its owned task ID is known",
                                );
                                return RunnerControl::Continue;
                            };
                            if owned_request_id.as_deref() != Some(stream.request_id.as_str()) {
                                push_runner_error(
                                    app,
                                    "active plan-run request ownership does not match",
                                );
                                return RunnerControl::Continue;
                            }
                            let request_id = stream.request_id.clone();
                            stream.cancel.store(true, Ordering::Release);
                            spawn_plan_run_stop(
                                app.runtime().clone(),
                                request_id,
                                task_id,
                                self.host_sender.clone(),
                            );
                            app.accept_runtime_event(UiRuntimeEvent::Terminal(
                                UiRuntimeTerminalEvent::Cancelled {
                                    reason: Some("计划运行已由用户停止".to_string()),
                                    usage: None,
                                    message_id: None,
                                    parent_message_id: None,
                                },
                            ));
                        }
                        ActiveStreamKind::Workflow => {
                            stream.cancel.store(true, Ordering::Release);
                            if let Some(ownership) = self.workflow_run.as_mut()
                                && ownership.request_id() == stream.request_id
                                && let Some(terminal) = ownership.project_local_terminal(
                                    UiRuntimeTerminalEvent::Cancelled {
                                        reason: Some("Workflow stream 已由用户停止".to_string()),
                                        usage: None,
                                        message_id: None,
                                        parent_message_id: None,
                                    },
                                )
                            {
                                app.accept_runtime_event(terminal);
                            }
                        }
                    }
                    RunnerControl::Continue
                } else {
                    RunnerControl::Quit
                }
            }
            TuiControllerCommand::QueuePrompt { text, attachments } => {
                app.widget_mut().queue_input(text, attachments);
                self.start_next_queued_prompt(app);
                RunnerControl::Continue
            }
            TuiControllerCommand::OpenTranscript => {
                let width = crossterm::terminal::size().map(|(width, _)| width).unwrap_or(80);
                self.transcript_pager = Some(app.transcript_pager(width));
                RunnerControl::Continue
            }
            TuiControllerCommand::OpenExternalEditor => {
                self.launch_editor(app.widget().draft().to_string());
                RunnerControl::Continue
            }
            TuiControllerCommand::Command(command) => match command {
                CommandRoute::Help => {
                    app.widget_mut().push_history(V3DisplayEvent::System { text: help_text() });
                    RunnerControl::Continue
                }
                CommandRoute::Status => {
                    let text = app.command_status_text();
                    app.widget_mut().push_history(V3DisplayEvent::System { text });
                    RunnerControl::Continue
                }
                CommandRoute::RunDoctor => {
                    self.start_doctor(app);
                    RunnerControl::Continue
                }
                CommandRoute::ShowDiff => {
                    self.start_session_diff(app);
                    RunnerControl::Continue
                }
                CommandRoute::OpenWorkspaceFiles { path } => {
                    self.start_workspace_files(app, path);
                    RunnerControl::Continue
                }
                CommandRoute::ReadWorkspaceFile { path } => {
                    self.start_workspace_file_read(app, path);
                    RunnerControl::Continue
                }
                CommandRoute::OpenHookCatalog => {
                    if self.hook_mutation.is_some() {
                        push_runner_error(app, "a Hook mutation is already in progress");
                    } else {
                        self.start_hook_catalog(app);
                    }
                    RunnerControl::Continue
                }
                CommandRoute::ShowHookAudit => {
                    if self.hook_mutation.is_some() {
                        push_runner_error(app, "a Hook mutation is already in progress");
                    } else {
                        self.start_hook_audit(app);
                    }
                    RunnerControl::Continue
                }
                CommandRoute::ConfirmHookMutation { plugin_id, hook_hash, trusted } => {
                    self.open_hook_mutation_confirmation(
                        app,
                        plugin_id.as_str(),
                        hook_hash.as_str(),
                        trusted,
                    );
                    RunnerControl::Continue
                }
                CommandRoute::ChangeDirectory { path } => {
                    if self.memory_mutation_generation.is_some() {
                        push_runner_error(
                            app,
                            "cannot switch directories during a memory mutation",
                        );
                        RunnerControl::Continue
                    } else {
                        self.start_directory_switch(app, path)
                    }
                }
                CommandRoute::Exit => RunnerControl::Quit,
                CommandRoute::OpenSessionPicker => {
                    self.execute_session_operation(app, |app| app.open_session_picker())
                }
                CommandRoute::ContinueSession => {
                    self.execute_session_operation(app, |app| app.resume_latest_session())
                }
                CommandRoute::NewSession => {
                    self.execute_session_operation(app, |app| app.create_new_session())
                }
                CommandRoute::ResumeSession { session_id } => self
                    .execute_session_operation(app, |app| app.resume_session(session_id.as_str())),
                CommandRoute::RenameSession { title: None } => {
                    app.prime_session_command("/rename ", "请输入当前会话的新标题，然后按 Enter。")
                }
                CommandRoute::RenameSession { title: Some(title) } => {
                    self.execute_session_operation(app, |app| app.rename_session(title.as_str()))
                }
                CommandRoute::ForkSession { message_id } => self
                    .execute_session_operation(app, |app| app.fork_session(message_id.as_deref())),
                CommandRoute::ResetSession { message_id: None } => self
                    .execute_session_operation(app, |app| {
                        app.open_session_message_picker(SESSION_RESET_SURFACE, "选择回退消息")
                    }),
                CommandRoute::ResetSession { message_id: Some(message_id) } => self
                    .execute_session_operation(app, |app| {
                        app.open_reset_confirmation(message_id.as_str())
                    }),
                CommandRoute::CompactSession { message_id: None } => self
                    .execute_session_operation(app, |app| {
                        app.open_session_message_picker(SESSION_COMPACT_SURFACE, "选择压缩起点")
                    }),
                CommandRoute::CompactSession { message_id: Some(message_id) } => self
                    .execute_session_operation(app, |app| {
                        app.open_compact_confirmation(message_id.as_str())
                    }),
                CommandRoute::ArchiveSession => {
                    self.execute_session_operation(app, V3GatewayApp::open_archive_confirmation)
                }
                CommandRoute::OpenSettings => {
                    app.open_settings();
                    RunnerControl::Continue
                }
                CommandRoute::OpenModelPicker => {
                    app.open_model_picker();
                    RunnerControl::Continue
                }
                CommandRoute::OpenEffortPicker => {
                    app.open_effort_picker();
                    RunnerControl::Continue
                }
                CommandRoute::OpenPermissionPicker { plan_only } => {
                    app.open_permission_picker(plan_only);
                    RunnerControl::Continue
                }
                CommandRoute::OpenSettingsTarget(target) => {
                    app.open_settings_target(target);
                    RunnerControl::Continue
                }
                CommandRoute::ApplyModel(model) => {
                    app.apply_model(model);
                    RunnerControl::Continue
                }
                CommandRoute::ApplyEffort(effort) => {
                    app.apply_effort(effort);
                    RunnerControl::Continue
                }
                CommandRoute::ApplyPermissionMode(mode) => {
                    app.apply_permission_mode(mode);
                    RunnerControl::Continue
                }
                CommandRoute::ApplyAcpBackend(backend) => {
                    app.apply_acp_backend(backend);
                    RunnerControl::Continue
                }
                CommandRoute::ApplyDelegateAgent(agent) => {
                    app.apply_delegate_agent(agent);
                    RunnerControl::Continue
                }
                CommandRoute::ApplyFullAccess(enabled) => {
                    app.apply_full_access(enabled);
                    RunnerControl::Continue
                }
                CommandRoute::ApplySubagentMode(enabled) => {
                    app.apply_subagent_mode(enabled);
                    RunnerControl::Continue
                }
                CommandRoute::QueryMemory { query } => {
                    if self.memory_mutation_generation.is_some() {
                        push_runner_error(app, "a memory mutation is already in progress");
                    } else {
                        self.start_memory_search(app, query);
                    }
                    RunnerControl::Continue
                }
                CommandRoute::ConfirmMemoryStore { key, content } => {
                    self.open_memory_store_confirmation(app, key.as_str(), content);
                    RunnerControl::Continue
                }
                CommandRoute::ConfirmMemoryDelete { key } => {
                    self.open_memory_delete_confirmation(app, key.as_str());
                    RunnerControl::Continue
                }
                CommandRoute::OpenIntegrations(kind) => {
                    app.open_integrations(kind);
                    RunnerControl::Continue
                }
                CommandRoute::ExpandIntegrationPrompt { kind, id, arguments } => {
                    match app.expand_integration_prompt(kind, id.as_str(), arguments.as_str()) {
                        Ok(prompt) => self.execute_action(
                            app,
                            V3Action::Submit { text: prompt, attachments: Vec::new() },
                        ),
                        Err(error) => {
                            push_runner_error(app, error);
                            RunnerControl::Continue
                        }
                    }
                }
                CommandRoute::ConfirmIntegrationToggle { kind, id, enabled } => {
                    app.open_integration_toggle_confirmation(kind, id.as_str(), enabled);
                    RunnerControl::Continue
                }
                CommandRoute::ConfirmIntegrationDelete { kind, id } => {
                    app.open_integration_delete_confirmation(kind, id.as_str());
                    RunnerControl::Continue
                }
                CommandRoute::OpenOrchestration(kind) => {
                    app.open_orchestration(kind);
                    RunnerControl::Continue
                }
                CommandRoute::OpenWorkflowDetail { uuid } => {
                    match load_owned_workflow_detail_blocking(
                        app.runtime(),
                        &app.state().orchestration,
                        uuid.as_str(),
                    ) {
                        Ok(detail) => app
                            .widget_mut()
                            .push_history(V3DisplayEvent::System { text: detail.render_text() }),
                        Err(error) => push_runner_error(app, error),
                    }
                    RunnerControl::Continue
                }
                CommandRoute::OpenCronDetail { job_id } => {
                    self.start_cron_detail(app, job_id);
                    RunnerControl::Continue
                }
                CommandRoute::OpenTaskDetail { task_id } => {
                    self.start_task_detail(app, task_id);
                    RunnerControl::Continue
                }
                CommandRoute::ConfirmCronToggle { job_id, enabled } => {
                    app.open_cron_toggle_confirmation(job_id.as_str(), enabled);
                    RunnerControl::Continue
                }
                CommandRoute::ConfirmCronDelete { job_id } => {
                    app.open_cron_delete_confirmation(job_id.as_str());
                    RunnerControl::Continue
                }
                CommandRoute::StartWorkflowRun { uuid, query } => {
                    self.start_workflow_run(app, uuid, query)
                }
                CommandRoute::ConfirmTaskControl { id, action } => {
                    app.open_task_control_confirmation(id.as_str(), action);
                    RunnerControl::Continue
                }
                CommandRoute::StartPlanRun { query } => {
                    self.start_plan_run(app, query, "/plan ", "请输入计划任务，然后按 Enter。")
                }
                CommandRoute::StartGoalRun { query } => {
                    self.start_plan_run(app, query, "/goal ", "请输入持续目标，然后按 Enter。")
                }
                CommandRoute::Unavailable { command_id, reason } => {
                    push_runner_error(app, format!("/{command_id} unavailable: {reason}"));
                    RunnerControl::Continue
                }
            },
            TuiControllerCommand::Overlay(command) => {
                self.execute_overlay_command(app, command);
                RunnerControl::Continue
            }
            command @ TuiControllerCommand::SubmitPrompt(_) => {
                if self.active_stream.is_some() {
                    push_runner_error(
                        app,
                        "a gateway stream is already active; queue the follow-up instead",
                    );
                    return RunnerControl::Continue;
                }
                let Some(request) = app.request_for_command(&command) else {
                    push_runner_error(app, "TUI submission did not produce a request");
                    return RunnerControl::Continue;
                };
                if let TuiControllerCommand::SubmitPrompt(submission) = &command {
                    app.record_submission_attachments(submission);
                }
                let request_id = next_stream_request_id();
                self.active_stream = Some(spawn_stream(app.runtime().clone(), request, request_id));
                RunnerControl::Continue
            }
        }
    }

    fn start_plan_run(
        &mut self,
        app: &mut V3GatewayApp,
        query: String,
        empty_draft: &str,
        empty_hint: &str,
    ) -> RunnerControl {
        if query.trim().is_empty() {
            return app.prime_session_command(empty_draft, empty_hint);
        }
        if self.active_stream.is_some() {
            push_runner_error(app, "a gateway stream is already active");
            return RunnerControl::Continue;
        }
        let request = match app.plan_run_request(query.as_str()) {
            Ok(request) => request,
            Err(error) => {
                push_runner_error(app, error);
                return RunnerControl::Continue;
            }
        };
        let request_id = next_plan_run_request_id();
        app.begin_plan_run(request_id.as_str(), query.as_str());
        self.active_stream = Some(spawn_plan_run(app.runtime().clone(), request, request_id));
        RunnerControl::Continue
    }

    fn start_workflow_run(
        &mut self,
        app: &mut V3GatewayApp,
        uuid: String,
        query: String,
    ) -> RunnerControl {
        if self.active_stream.is_some() {
            push_runner_error(app, "a gateway stream is already active");
            return RunnerControl::Continue;
        }
        let request = match workflow_run_request(
            &app.state().orchestration,
            app.state().session.session_id.as_deref(),
            uuid.as_str(),
            query.as_str(),
        ) {
            Ok(request) => request,
            Err(error) => {
                push_runner_error(app, error);
                return RunnerControl::Continue;
            }
        };
        let request_id = next_workflow_run_request_id();
        app.begin_workflow_run(request_id.as_str(), query.as_str());
        let session_id = app
            .state()
            .session
            .session_id
            .clone()
            .expect("workflow request validated a stable session id");
        self.workflow_run = Some(WorkflowRunOwnership::new_for_session(
            request_id.clone(),
            request.uuid.clone(),
            session_id,
        ));
        self.active_stream = Some(spawn_workflow_run(app.runtime().clone(), request, request_id));
        RunnerControl::Continue
    }

    fn execute_session_operation(
        &mut self,
        app: &mut V3GatewayApp,
        operation: impl FnOnce(&mut V3GatewayApp) -> Result<(), String>,
    ) -> RunnerControl {
        let previous_session_id = app.state().session.session_id.clone();
        let had_workflow_question = app.state().tasks.workflow_pause_request_id.is_some();
        let result = if self.hook_mutation.is_some() {
            Err("cannot switch directories while a Hook mutation is active".to_string())
        } else if self.directory_switch_source.is_some() {
            Err("cannot switch sessions while a directory switch is active".to_string())
        } else if self.active_stream.is_some() {
            Err("cannot switch sessions while a gateway stream is active".to_string())
        } else if app.widget().queued_input_count() > 0 {
            Err("cannot switch sessions while queued prompts are pending".to_string())
        } else {
            operation(app)
        };
        match result {
            Ok(()) => {
                if previous_session_id != app.state().session.session_id
                    || (had_workflow_question
                        && app.state().tasks.workflow_pause_request_id.is_none())
                {
                    self.workflow_run = None;
                }
            }
            Err(error) => push_runner_error(app, error),
        }
        RunnerControl::Continue
    }

    fn start_directory_switch(&mut self, app: &mut V3GatewayApp, path: String) -> RunnerControl {
        if path.trim().is_empty() {
            return app
                .prime_session_command("/cd ", "请输入 Gateway 已知的工作目录，然后按 Enter。");
        }
        let result = if self.directory_switch_source.is_some() {
            Err("a directory switch is already in progress".to_string())
        } else if self.active_stream.is_some() {
            Err("cannot switch directories while a gateway stream is active".to_string())
        } else if app.widget().queued_input_count() > 0 {
            Err("cannot switch directories while queued prompts are pending".to_string())
        } else {
            Ok(())
        };
        if let Err(error) = result {
            push_runner_error(app, error);
            return RunnerControl::Continue;
        }

        self.directory_switch_generation = self.directory_switch_generation.wrapping_add(1);
        let source_directory = app.runtime().directory().to_path_buf();
        let recovery_draft = format!("/cd {}", path.trim());
        self.directory_switch_source = Some(source_directory.clone());
        app.widget_mut().push_history(V3DisplayEvent::System {
            text: format!("正在切换工作目录：{}", path.trim()),
        });
        spawn_directory_switch(
            app.runtime().clone(),
            PathBuf::from(path.trim()),
            self.directory_switch_generation,
            source_directory,
            recovery_draft,
            self.host_sender.clone(),
        );
        RunnerControl::Continue
    }

    fn execute_overlay_command(&mut self, app: &mut V3GatewayApp, command: TuiOverlayCommand) {
        if let TuiOverlayCommand::ConfirmDecision { request_id, intent, confirmed } = command {
            let result = app.dismiss_confirmation(request_id.as_str()).and_then(|()| {
                if confirmed { self.execute_confirmed_intent(app, intent) } else { Ok(()) }
            });
            if let Err(error) = result {
                push_runner_error(app, error);
            }
            app.show_next_interaction();
            return;
        }
        match &command {
            TuiOverlayCommand::QuestionRejected(question)
                if app.state().tasks.workflow_pause_request_id.as_deref()
                    == Some(question.request_id.as_str()) =>
            {
                app.replace_workflow_pause_question(None);
                return;
            }
            TuiOverlayCommand::QuestionSubmitted(question)
                if app.state().tasks.workflow_pause_request_id.as_deref()
                    == Some(question.request_id.as_str()) =>
            {
                if self.interaction_in_flight {
                    app.record_interaction_error("an interaction request is already in flight");
                    app.show_next_interaction();
                    return;
                }
                if self.active_stream.is_some() {
                    app.record_interaction_error(
                        "cannot resume a Workflow while another gateway stream is active",
                    );
                    app.show_next_interaction();
                    return;
                }
                let request = self
                    .workflow_run
                    .as_ref()
                    .ok_or_else(|| "workflow resume has no owned run state".to_string())
                    .and_then(|ownership| {
                        ownership
                            .resume_request(app.state().session.session_id.as_deref(), question)
                    });
                let request = match request {
                    Ok(request) => request,
                    Err(error) => {
                        app.record_interaction_error(error);
                        app.show_next_interaction();
                        return;
                    }
                };
                self.interaction_in_flight = true;
                app.begin_workflow_resume();
                spawn_workflow_resume(
                    app.runtime().clone(),
                    question.request_id.clone(),
                    request,
                    self.host_sender.clone(),
                );
                return;
            }
            _ => {}
        }
        if self.interaction_in_flight {
            app.record_interaction_error("an interaction request is already in flight");
            app.show_next_interaction();
            return;
        }
        self.interaction_in_flight = true;
        match command {
            TuiOverlayCommand::ConfirmDecision { .. } => unreachable!("handled above"),
            TuiOverlayCommand::PermissionDecision { request_id, decision } => {
                let (reply, message) = match decision {
                    PermissionDecision::Once => (PendingPermissionReplyDto::Once, None),
                    PermissionDecision::Always => (PendingPermissionReplyDto::Always, None),
                    PermissionDecision::Reject { reason } => {
                        (PendingPermissionReplyDto::Reject, reason)
                    }
                };
                spawn_permission_reply(
                    app.runtime().clone(),
                    request_id,
                    reply,
                    message,
                    self.host_sender.clone(),
                );
            }
            TuiOverlayCommand::QuestionSubmitted(question) => {
                spawn_question_reply(
                    app.runtime().clone(),
                    question.request_id,
                    Some(question.answers),
                    self.host_sender.clone(),
                );
            }
            TuiOverlayCommand::QuestionRejected(question) => {
                spawn_question_reply(
                    app.runtime().clone(),
                    question.request_id,
                    None,
                    self.host_sender.clone(),
                );
            }
        }
    }

    fn execute_confirmed_intent(
        &mut self,
        app: &mut V3GatewayApp,
        intent: UiConfirmIntent,
    ) -> Result<(), String> {
        match intent {
            UiConfirmIntent::ApplySessionSettings { values } => {
                app.apply_settings_values(*values, true)
            }
            UiConfirmIntent::ResetSession { message_id } => {
                self.ensure_session_mutation_idle(app)?;
                app.reset_session(message_id.as_str())
            }
            UiConfirmIntent::CompactSession { message_id } => {
                self.ensure_session_mutation_idle(app)?;
                app.compact_session(message_id.as_str())
            }
            UiConfirmIntent::ArchiveSession => {
                self.ensure_session_mutation_idle(app)?;
                app.archive_session()
            }
            UiConfirmIntent::IntegrationToggle { kind, id, enabled } => {
                app.mutate_integration(kind, id.as_str(), Some(enabled))
            }
            UiConfirmIntent::IntegrationDelete { kind, id } => {
                app.mutate_integration(kind, id.as_str(), None)
            }
            UiConfirmIntent::HookMutation { plugin_id, hook_hash, trusted } => {
                self.start_hook_mutation(app, plugin_id.as_str(), hook_hash.as_str(), trusted)
            }
            UiConfirmIntent::MemoryStore { key, content } => {
                self.start_memory_mutation(app, MemoryMutation::Store { key, content })
            }
            UiConfirmIntent::MemoryDelete { key } => {
                self.require_memory_key(app, key.as_str())?;
                self.start_memory_mutation(app, MemoryMutation::Delete { key })
            }
            UiConfirmIntent::TaskControl { id, action } => {
                app.control_owned_task(id.as_str(), action)
            }
            UiConfirmIntent::ScheduleToggle { id, enabled } => {
                app.mutate_owned_cron(id.as_str(), CronMutation::SetEnabled(enabled))
            }
            UiConfirmIntent::ScheduleDelete { id } => {
                app.mutate_owned_cron(id.as_str(), CronMutation::Delete)
            }
            _ => Err("active confirmation is not a session mutation intent".to_string()),
        }
    }

    fn ensure_session_mutation_idle(&self, app: &V3GatewayApp) -> Result<(), String> {
        if self.directory_switch_source.is_some() {
            return Err("cannot mutate a session while a directory switch is active".to_string());
        }
        if self.active_stream.is_some() {
            return Err("cannot mutate a session while a gateway stream is active".to_string());
        }
        if app.widget().queued_input_count() > 0 {
            return Err("cannot mutate a session while queued prompts are pending".to_string());
        }
        Ok(())
    }

    fn launch_editor(&mut self, draft: String) {
        if self.editor_in_flight {
            return;
        }
        self.editor_in_flight = true;
        let editor = Arc::clone(&self.editor);
        let sender = self.host_sender.clone();
        std::thread::spawn(move || {
            let result = editor(draft.as_str());
            let _ = sender.send(HostMessage::EditorFinished(result));
        });
    }

    fn start_file_search(&mut self, app: &V3GatewayApp, query: Option<String>) {
        self.file_search_generation = self.file_search_generation.wrapping_add(1);
        self.file_search_query.clone_from(&query);
        let Some(query) = query else {
            return;
        };
        spawn_file_search(
            app.runtime().clone(),
            self.file_search_generation,
            query,
            self.host_sender.clone(),
        );
    }

    fn start_session_diff(&mut self, app: &V3GatewayApp) {
        self.session_diff_generation = self.session_diff_generation.wrapping_add(1);
        spawn_session_diff(
            app.runtime().clone(),
            self.session_diff_generation,
            self.host_sender.clone(),
        );
    }

    fn start_doctor(&mut self, app: &mut V3GatewayApp) {
        self.doctor_generation = self.doctor_generation.wrapping_add(1);
        app.widget_mut().push_history(V3DisplayEvent::System {
            text: "正在运行 Gateway 诊断…".to_string(),
        });
        spawn_doctor(app.runtime().clone(), self.doctor_generation, self.host_sender.clone());
    }

    fn start_cron_detail(&mut self, app: &V3GatewayApp, job_id: String) {
        self.cron_generation = self.cron_generation.wrapping_add(1);
        spawn_cron_detail(
            app.runtime().clone(),
            app.state().orchestration.cron_jobs.clone(),
            self.cron_generation,
            job_id,
            self.host_sender.clone(),
        );
    }

    fn start_task_detail(&mut self, app: &V3GatewayApp, task_id: String) {
        self.task_detail_generation = self.task_detail_generation.wrapping_add(1);
        spawn_task_detail(
            app.runtime().clone(),
            app.state().orchestration.clone(),
            self.task_detail_generation,
            task_id,
            self.host_sender.clone(),
        );
    }

    fn start_workspace_files(&mut self, app: &V3GatewayApp, path: String) {
        self.workspace_files_generation = self.workspace_files_generation.wrapping_add(1);
        spawn_workspace_files(
            app.runtime().clone(),
            self.workspace_files_generation,
            path,
            self.host_sender.clone(),
        );
    }

    fn start_workspace_file_read(&mut self, app: &V3GatewayApp, path: String) {
        self.workspace_files_generation = self.workspace_files_generation.wrapping_add(1);
        spawn_workspace_file_read(
            app.runtime().clone(),
            self.workspace_files_generation,
            path,
            self.host_sender.clone(),
        );
    }

    fn start_hook_catalog(&mut self, app: &V3GatewayApp) {
        self.hooks_generation = self.hooks_generation.wrapping_add(1);
        spawn_hook_catalog(app.runtime().clone(), self.hooks_generation, self.host_sender.clone());
    }

    fn start_hook_audit(&mut self, app: &V3GatewayApp) {
        self.hooks_generation = self.hooks_generation.wrapping_add(1);
        spawn_hook_audit(app.runtime().clone(), self.hooks_generation, self.host_sender.clone());
    }

    fn start_memory_search(&mut self, app: &V3GatewayApp, query: Option<String>) {
        self.memory_generation = self.memory_generation.wrapping_add(1);
        spawn_memory_search(
            app.runtime().clone(),
            self.memory_generation,
            query,
            self.host_sender.clone(),
        );
    }

    fn open_memory_store_confirmation(&self, app: &mut V3GatewayApp, key: &str, content: String) {
        let key = key.trim();
        let result = if self.memory_mutation_generation.is_some() {
            Err("a memory mutation is already in progress".to_string())
        } else {
            Ok(UiConfirmOverlay {
                request_id: format!("memory-store-{key}"),
                title: "确认写入 core 记忆".to_string(),
                body: format!("写入 core 记忆 Key: {key}\n服务端同名记忆可能被覆盖。"),
                confirm_label: "写入".to_string(),
                cancel_label: "取消".to_string(),
                destructive: true,
                intent: UiConfirmIntent::MemoryStore { key: key.to_string(), content },
            })
        };
        app.show_integration_confirmation(result);
    }

    fn open_memory_delete_confirmation(&self, app: &mut V3GatewayApp, key: &str) {
        let result = if self.memory_mutation_generation.is_some() {
            Err("a memory mutation is already in progress".to_string())
        } else {
            self.require_memory_key(app, key).map(|()| UiConfirmOverlay {
                request_id: format!("memory-delete-{}", key.trim()),
                title: "确认删除记忆".to_string(),
                body: format!("删除当前目录记忆 Key: {}", key.trim()),
                confirm_label: "删除".to_string(),
                cancel_label: "保留记忆".to_string(),
                destructive: true,
                intent: UiConfirmIntent::MemoryDelete { key: key.trim().to_string() },
            })
        };
        app.show_integration_confirmation(result);
    }

    fn require_memory_key(&self, app: &V3GatewayApp, key: &str) -> Result<(), String> {
        if self.memory_snapshot_directory.as_deref() != Some(app.runtime().directory()) {
            return Err(
                "memory deletion requires a query snapshot from the current directory".to_string()
            );
        }
        let key = key.trim();
        app.state()
            .integration_catalog
            .iter()
            .any(|item| item.kind == IntegrationKind::Memory && item.id == key)
            .then_some(())
            .ok_or_else(|| {
                "memory deletion requires an exact key from the current memory snapshot".to_string()
            })
    }

    fn start_memory_mutation(
        &mut self,
        app: &V3GatewayApp,
        mutation: MemoryMutation,
    ) -> Result<(), String> {
        if self.directory_switch_source.is_some() {
            return Err("cannot mutate memory while a directory switch is active".to_string());
        }
        if self.memory_mutation_generation.is_some() {
            return Err("a memory mutation is already in progress".to_string());
        }
        if matches!(mutation, MemoryMutation::Delete { .. }) {
            self.require_memory_key(app, mutation.key())?;
        }
        let query = (self.memory_snapshot_directory.as_deref() == Some(app.runtime().directory()))
            .then(|| self.memory_snapshot_query.clone())
            .flatten();
        self.memory_generation = self.memory_generation.wrapping_add(1);
        self.memory_mutation_generation = Some(self.memory_generation);
        spawn_memory_mutation(
            app.runtime().clone(),
            self.memory_generation,
            mutation,
            query,
            self.host_sender.clone(),
        );
        Ok(())
    }

    fn open_hook_mutation_confirmation(
        &self,
        app: &mut V3GatewayApp,
        plugin_id: &str,
        hook_hash: &str,
        trusted: bool,
    ) {
        let result = self.current_hook(app, plugin_id, hook_hash).and_then(|hook| {
            if hook.trusted == trusted {
                return Err(format!(
                    "Hook {plugin_id} {hook_hash} is already {}",
                    if trusted { "trusted" } else { "revoked" }
                ));
            }
            let action = if trusted { "信任" } else { "撤销信任" };
            Ok(UiConfirmOverlay {
                request_id: format!(
                    "hook-{}-{plugin_id}-{hook_hash}",
                    if trusted { "trust" } else { "revoke" }
                ),
                title: format!("确认{action} Hook"),
                body: format!(
                    "{action}当前目录声明\nPlugin: {}\nEvent: {}\nMode: {}\nHash: {}",
                    hook.plugin_id, hook.event, hook.mode, hook.hook_hash
                ),
                confirm_label: action.to_string(),
                cancel_label: "取消".to_string(),
                destructive: true,
                intent: UiConfirmIntent::HookMutation {
                    plugin_id: hook.plugin_id.clone(),
                    hook_hash: hook.hook_hash.clone(),
                    trusted,
                },
            })
        });
        app.show_integration_confirmation(result);
    }

    fn start_hook_mutation(
        &mut self,
        app: &V3GatewayApp,
        plugin_id: &str,
        hook_hash: &str,
        trusted: bool,
    ) -> Result<(), String> {
        if self.hook_mutation.is_some() {
            return Err("a Hook mutation is already in progress".to_string());
        }
        let hook = self.current_hook(app, plugin_id, hook_hash)?;
        if hook.trusted == trusted {
            return Err("Hook catalog changed before confirmation was applied".to_string());
        }
        self.hooks_generation = self.hooks_generation.wrapping_add(1);
        self.hook_mutation = Some(ActiveHookMutation {
            plugin_id: plugin_id.to_string(),
            hook_hash: hook_hash.to_string(),
            trusted,
        });
        spawn_hook_mutation(
            app.runtime().clone(),
            self.hooks_generation,
            plugin_id.to_string(),
            hook_hash.to_string(),
            trusted,
            self.host_sender.clone(),
        );
        Ok(())
    }

    fn current_hook(
        &self,
        app: &V3GatewayApp,
        plugin_id: &str,
        hook_hash: &str,
    ) -> Result<HookCatalogItem, String> {
        if self.hook_catalog_directory.as_deref() != Some(app.runtime().directory()) {
            return Err("Hook mutation requires the current directory catalog snapshot".to_string());
        }
        self.hook_catalog
            .as_ref()
            .and_then(|catalog| catalog.find(plugin_id, hook_hash))
            .cloned()
            .ok_or_else(|| {
                "Hook mutation requires an exact plugin ID/hash from the current catalog"
                    .to_string()
            })
    }

    /// 非阻塞汇集本地副作与 Gateway 流消息，并在流结束后尝试启动排队输入。
    pub(crate) fn drain(&mut self, app: &mut V3GatewayApp) -> bool {
        if app.take_interaction_refresh_requested() {
            self.request_interaction_refresh(app);
        }
        let host_work_remaining = self.drain_host_messages(app);
        let stream_work_remaining = self.drain_active_stream(app);
        if app.take_interaction_refresh_requested() {
            self.request_interaction_refresh(app);
        }
        host_work_remaining || stream_work_remaining
    }

    /// 在终端首帧可绘制后，后台恢复最近会话并转换完整状态。
    pub(crate) fn request_session_bootstrap(&mut self, app: &mut V3GatewayApp) {
        if self.session_bootstrap_in_flight {
            return;
        }
        self.session_bootstrap_in_flight = true;
        app.widget_mut().push_history(V3DisplayEvent::System {
            text: "正在加载最近会话…".to_string(),
        });
        spawn_session_bootstrap(app.runtime().clone(), self.host_sender.clone());
    }

    /// 在后台加载当前会话交互；并发事件会保留刷新标记供本轮结束后重试。
    pub(crate) fn request_interaction_refresh(&mut self, app: &mut V3GatewayApp) {
        if self.interaction_in_flight {
            app.request_interaction_refresh();
            return;
        }
        let Some(session_id) = app.state().session.session_id.clone() else {
            app.record_interaction_error("cannot refresh interactions without a session id");
            return;
        };
        self.interaction_in_flight = true;
        spawn_interaction_refresh(app.runtime().clone(), session_id, self.host_sender.clone());
    }

    fn drain_host_messages(&mut self, app: &mut V3GatewayApp) -> bool {
        for _ in 0..HOST_MESSAGE_BUDGET {
            match self.host_receiver.try_recv() {
                Ok(HostMessage::SessionBootstrapFinished(result)) => {
                    self.session_bootstrap_in_flight = false;
                    match result {
                        Ok(state) => app.commit_initial_session_state(*state),
                        Err(error) => {
                            push_runner_error(
                                app,
                                format!("failed to load session history: {error}"),
                            );
                        }
                    }
                }
                Ok(HostMessage::DirectorySwitchFinished {
                    generation,
                    source_directory,
                    recovery_draft,
                    result,
                }) => {
                    if generation != self.directory_switch_generation {
                        continue;
                    }
                    self.directory_switch_source = None;
                    if app.runtime().directory() != source_directory {
                        continue;
                    }
                    match result {
                        Ok(bootstrap) => {
                            self.file_search_generation =
                                self.file_search_generation.wrapping_add(1);
                            self.file_search_query = None;
                            self.session_diff_generation =
                                self.session_diff_generation.wrapping_add(1);
                            self.doctor_generation = self.doctor_generation.wrapping_add(1);
                            self.cron_generation = self.cron_generation.wrapping_add(1);
                            self.task_detail_generation =
                                self.task_detail_generation.wrapping_add(1);
                            self.workspace_files_generation =
                                self.workspace_files_generation.wrapping_add(1);
                            self.hooks_generation = self.hooks_generation.wrapping_add(1);
                            self.memory_generation = self.memory_generation.wrapping_add(1);
                            self.memory_mutation_generation = None;
                            self.memory_snapshot_directory = None;
                            self.memory_snapshot_query = None;
                            self.hook_catalog_directory = None;
                            self.hook_catalog = None;
                            self.workflow_run = None;
                            let directory = bootstrap.runtime.directory().display().to_string();
                            app.commit_directory_switch(*bootstrap);
                            app.widget_mut().push_history(V3DisplayEvent::System {
                                text: format!("工作目录已切换到 {directory}"),
                            });
                        }
                        Err(error) => {
                            if app.widget().draft().is_empty() {
                                app.widget_mut().replace_draft(recovery_draft.as_str());
                            }
                            push_runner_error(
                                app,
                                format!("failed to switch working directory: {error}"),
                            );
                        }
                    }
                }
                Ok(HostMessage::EditorFinished(result)) => {
                    self.editor_in_flight = false;
                    match result {
                        Ok(Some(text)) => app.widget_mut().replace_draft(text.as_str()),
                        Ok(None) => {}
                        Err(error) => {
                            push_runner_error(app, format!("external editor failed: {error}"));
                        }
                    }
                }
                Ok(HostMessage::StopFinished { request_id, result }) => match result {
                    Ok(true) => {}
                    Ok(false) => push_runner_error(
                        app,
                        format!("gateway did not stop active stream {request_id}"),
                    ),
                    Err(error) => push_runner_error(
                        app,
                        format!("failed to stop active stream {request_id}: {error}"),
                    ),
                },
                Ok(HostMessage::PlanRunStopFinished { request_id, task_id, result }) => {
                    match result {
                        Ok(true) => {}
                        Ok(false) => push_runner_error(
                            app,
                            format!(
                                "gateway did not stop owned plan-run request {request_id} task {task_id}"
                            ),
                        ),
                        Err(error) => push_runner_error(
                            app,
                            format!(
                                "failed to stop owned plan-run request {request_id} task {task_id}: {error}"
                            ),
                        ),
                    }
                }
                Ok(HostMessage::InteractionsRefreshed(result)) => {
                    self.interaction_in_flight = false;
                    match result {
                        Ok(snapshot) => app.apply_interaction_snapshot(snapshot),
                        Err(error) => app.record_interaction_error(format!(
                            "failed to refresh pending interactions: {error}"
                        )),
                    }
                }
                Ok(HostMessage::PermissionReplyFinished { request_id, result }) => {
                    self.interaction_in_flight = false;
                    match result {
                        Ok(()) => app.complete_permission(request_id.as_str()),
                        Err(error) => {
                            app.record_interaction_error(format!(
                                "failed to reply to permission {request_id}: {error}"
                            ));
                            app.show_next_interaction();
                        }
                    }
                }
                Ok(HostMessage::QuestionReplyFinished { request_id, result }) => {
                    self.interaction_in_flight = false;
                    match result {
                        Ok(()) => app.complete_question(request_id.as_str()),
                        Err(error) => {
                            app.record_interaction_error(format!(
                                "failed to reply to question {request_id}: {error}"
                            ));
                            app.show_next_interaction();
                        }
                    }
                }
                Ok(HostMessage::WorkflowResumeFinished { pause_request_id, result }) => {
                    self.interaction_in_flight = false;
                    match result {
                        Ok(response) => {
                            let projected = self
                                .workflow_run
                                .as_mut()
                                .ok_or_else(|| "workflow resume has no owned run state".to_string())
                                .and_then(|ownership| {
                                    if ownership.pause_request_id().as_deref()
                                        != Some(pause_request_id.as_str())
                                    {
                                        return Err(
                                            "workflow resume result does not match the current pause"
                                                .to_string(),
                                        );
                                    }
                                    let events = ownership.project_resume_response(*response)?;
                                    let question = ownership.pause_question()?;
                                    Ok((events, question))
                                });
                            match projected {
                                Ok((events, question)) => {
                                    for event in events {
                                        app.accept_runtime_event(event);
                                    }
                                    app.replace_workflow_pause_question(question);
                                    if let Err(error) = app.persist_session_snapshot() {
                                        push_runner_error(
                                            app,
                                            format!("session save failed: {error}"),
                                        );
                                    }
                                }
                                Err(error) => app.record_interaction_error(error),
                            }
                        }
                        Err(error) => app.record_interaction_error(format!(
                            "failed to resume Workflow pause {pause_request_id}: {error}"
                        )),
                    }
                    app.show_next_interaction();
                }
                Ok(HostMessage::FileSearchFinished { generation, query, result }) => {
                    if generation != self.file_search_generation
                        || self.file_search_query.as_deref() != Some(query.as_str())
                    {
                        continue;
                    }
                    match result {
                        Ok(matches) => {
                            app.widget_mut().set_file_search_matches(query.as_str(), matches);
                        }
                        Err(error) => app.widget_mut().set_file_search_error(
                            query.as_str(),
                            format!("file search unavailable: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::SessionDiffFinished { generation, session_id, result }) => {
                    if generation != self.session_diff_generation
                        || session_id.as_deref() != app.runtime().session_id()
                    {
                        continue;
                    }
                    match result {
                        Ok(diff) if diff.files.is_empty() => {
                            app.widget_mut().push_history(V3DisplayEvent::System {
                                text: "当前会话没有文件变化。".to_string(),
                            });
                        }
                        Ok(diff) => {
                            app.widget_mut().push_history(V3DisplayEvent::FileDiff { diff });
                        }
                        Err(error) => {
                            push_runner_error(app, format!("failed to load session diff: {error}"));
                        }
                    }
                }
                Ok(HostMessage::DoctorFinished { generation, directory, result }) => {
                    if generation != self.doctor_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(report) => {
                            app.widget_mut().push_history(V3DisplayEvent::System {
                                text: report.render_text(),
                            });
                        }
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to load Gateway diagnostics: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::CronDetailFinished { generation, directory, job_id, result }) => {
                    if generation != self.cron_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(detail) => app
                            .widget_mut()
                            .push_history(V3DisplayEvent::System { text: detail.render_text() }),
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to load Cron detail {job_id}: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::TaskDetailFinished { generation, directory, task_id, result }) => {
                    if generation != self.task_detail_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(detail) => app
                            .widget_mut()
                            .push_history(V3DisplayEvent::System { text: detail.render_text() }),
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to load Scheduler task detail {task_id}: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::WorkspaceFilesFinished {
                    generation,
                    directory,
                    requested_path,
                    result,
                }) => {
                    if generation != self.workspace_files_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(listing) if listing.path() == requested_path => {
                            app.widget_mut().show_selection_overlay(
                                WORKSPACE_FILES_SURFACE,
                                format!("浏览文件：{}", listing.path()),
                                listing.into_items(),
                            );
                        }
                        Ok(_) => push_runner_error(
                            app,
                            "workspace file listing did not match the current request",
                        ),
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to browse workspace files: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::WorkspaceFileReadFinished {
                    generation,
                    directory,
                    requested_path,
                    result,
                }) => {
                    if generation != self.workspace_files_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(text) if text.path() == requested_path => {
                            app.widget_mut()
                                .push_history(V3DisplayEvent::System { text: text.render_text() });
                        }
                        Ok(_) => push_runner_error(
                            app,
                            "workspace file response did not match the current request",
                        ),
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to read workspace file: {error}"),
                        ),
                    }
                }
                Ok(HostMessage::HookCatalogFinished { generation, directory, result }) => {
                    if generation != self.hooks_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(catalog) => {
                            self.hook_catalog_directory = Some(directory);
                            self.hook_catalog = Some(catalog.clone());
                            app.widget_mut().show_selection_overlay(
                                HOOK_CATALOG_SURFACE,
                                "Hook 目录",
                                catalog.into_selection_items(),
                            );
                        }
                        Err(error) => {
                            push_runner_error(app, format!("failed to load Hook catalog: {error}"));
                        }
                    }
                }
                Ok(HostMessage::HookAuditFinished { generation, directory, result }) => {
                    if generation != self.hooks_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(audit) => app
                            .widget_mut()
                            .push_history(V3DisplayEvent::System { text: audit.render_text() }),
                        Err(error) => {
                            push_runner_error(app, format!("failed to load Hook audit: {error}"));
                        }
                    }
                }
                Ok(HostMessage::HookMutationFinished {
                    generation,
                    directory,
                    plugin_id,
                    hook_hash,
                    trusted,
                    result,
                }) => {
                    if generation != self.hooks_generation {
                        continue;
                    }
                    let Some(active) = self.hook_mutation.take() else {
                        continue;
                    };
                    if active.plugin_id != plugin_id
                        || active.hook_hash != hook_hash
                        || active.trusted != trusted
                    {
                        push_runner_error(
                            app,
                            "Hook mutation response did not match the active request",
                        );
                        continue;
                    }
                    if directory.as_path() != app.runtime().directory() {
                        continue;
                    }
                    match result {
                        Ok(refresh) => {
                            self.hook_catalog_directory = Some(directory);
                            self.hook_catalog = Some(refresh.catalog.clone());
                            app.widget_mut().push_history(V3DisplayEvent::System {
                                text: format!(
                                    "Hook {}: {plugin_id} {hook_hash}",
                                    if trusted { "已信任" } else { "已撤销信任" }
                                ),
                            });
                            app.widget_mut().push_history(V3DisplayEvent::System {
                                text: refresh.audit.render_text(),
                            });
                            app.widget_mut().show_selection_overlay(
                                HOOK_CATALOG_SURFACE,
                                "Hook 目录",
                                refresh.catalog.into_selection_items(),
                            );
                        }
                        Err(error) => push_runner_error(
                            app,
                            format!(
                                "failed to {} Hook {plugin_id} {hook_hash}: {error}",
                                if trusted { "trust" } else { "revoke" }
                            ),
                        ),
                    }
                }
                Ok(HostMessage::MemorySearchFinished { generation, directory, query, result }) => {
                    if generation != self.memory_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    match result {
                        Ok(items) => {
                            self.memory_snapshot_directory = Some(directory);
                            self.memory_snapshot_query.clone_from(&query);
                            app.replace_memory_items(items);
                            app.open_integrations(IntegrationKind::Memory);
                        }
                        Err(error) => push_runner_error(
                            app,
                            format!(
                                "failed to {} memory: {error}",
                                if query.is_some() { "query" } else { "refresh" }
                            ),
                        ),
                    }
                }
                Ok(HostMessage::MemoryMutationFinished {
                    generation,
                    directory,
                    query,
                    action,
                    key,
                    result,
                }) => {
                    if generation != self.memory_generation
                        || directory.as_path() != app.runtime().directory()
                    {
                        continue;
                    }
                    self.memory_mutation_generation = None;
                    match result {
                        Ok(items) => {
                            self.memory_snapshot_directory = Some(directory);
                            self.memory_snapshot_query.clone_from(&query);
                            app.replace_memory_items(items);
                            app.widget_mut().push_history(V3DisplayEvent::System {
                                text: format!(
                                    "记忆已{}: {key}",
                                    if action == "store" { "写入" } else { "删除" }
                                ),
                            });
                            app.open_integrations(IntegrationKind::Memory);
                        }
                        Err(error) => push_runner_error(
                            app,
                            format!("failed to {action} memory {key}: {error}"),
                        ),
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return false,
            }
        }
        true
    }

    fn drain_active_stream(&mut self, app: &mut V3GatewayApp) -> bool {
        let Some(stream) = self.active_stream.as_ref() else {
            self.start_next_queued_prompt(app);
            return false;
        };
        let mut terminal = None;
        let mut processed = 0;

        for _ in 0..STREAM_EVENT_BUDGET {
            match stream.receiver.try_recv() {
                Ok(StreamMessage::ChatEvent(event)) => {
                    processed += 1;
                    app.accept_gateway_event(event);
                }
                Ok(StreamMessage::WorkflowEvent(event)) => {
                    processed += 1;
                    let Some(ownership) = self.workflow_run.as_mut() else {
                        push_runner_error(app, "workflow stream event has no owned run state");
                        continue;
                    };
                    if ownership.request_id() != stream.request_id {
                        push_runner_error(app, "workflow stream request ownership does not match");
                        continue;
                    }
                    let events = ownership.project_event(event);
                    let pause_question = ownership.pause_question();
                    for event in events {
                        app.accept_runtime_event(event);
                    }
                    match pause_question {
                        Ok(question) => app.replace_workflow_pause_question(question),
                        Err(error) => push_runner_error(app, error),
                    }
                }
                Ok(StreamMessage::RuntimeEvent(event)) => {
                    processed += 1;
                    app.accept_runtime_event(event);
                }
                Ok(StreamMessage::Finished(result)) => {
                    processed += 1;
                    terminal = Some(result);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    terminal = Some(Err("gateway stream worker disconnected".to_string()));
                    break;
                }
            }
        }

        let Some(result) = terminal else {
            return processed == STREAM_EVENT_BUDGET;
        };
        let kind = stream.kind;
        let needs_plan_terminal = kind == ActiveStreamKind::PlanRun
            && app.state().runtime.active_plan_run_request_id.is_some();
        let needs_workflow_terminal = kind == ActiveStreamKind::Workflow
            && self.workflow_run.as_ref().is_some_and(|ownership| {
                ownership.request_id() == stream.request_id && ownership.terminal_events() == 0
            });
        match result {
            Ok(
                GatewayStreamAttemptResult::Terminated | GatewayStreamAttemptResult::CallerStopped,
            ) => {}
            Ok(GatewayStreamAttemptResult::Incomplete) => {
                push_runner_error(app, "gateway stream ended before a terminal event");
            }
            Err(error) => push_runner_error(app, format!("gateway stream failed: {error}")),
        }
        if needs_plan_terminal {
            app.accept_runtime_event(UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error(
                "plan-run stream ended before a terminal event".to_string(),
            )));
        }
        if needs_workflow_terminal
            && let Some(ownership) = self.workflow_run.as_mut()
            && let Some(terminal) = ownership.project_local_terminal(UiRuntimeTerminalEvent::Error(
                "workflow stream ended before a terminal event".to_string(),
            ))
        {
            app.accept_runtime_event(terminal);
        }
        if let Err(error) = app.persist_session_snapshot() {
            push_runner_error(app, format!("session save failed: {error}"));
        }
        self.active_stream = None;
        self.start_next_queued_prompt(app);
        false
    }

    #[cfg(test)]
    pub(crate) fn install_test_stream(
        &mut self,
        events: impl IntoIterator<Item = GatewayChatStreamEvent>,
    ) {
        let (sender, receiver) = mpsc::channel();
        for event in events {
            sender.send(StreamMessage::ChatEvent(event)).expect("test stream receiver");
        }
        self.active_stream = Some(ActiveStream {
            kind: ActiveStreamKind::Chat,
            request_id: "test-stream".to_string(),
            receiver,
            cancel: Arc::new(AtomicBool::new(false)),
        });
    }

    fn start_next_queued_prompt(&mut self, app: &mut V3GatewayApp) {
        if self.active_stream.is_some() {
            return;
        }
        let Some((text, attachments)) = app.widget_mut().pop_queued_input() else {
            return;
        };
        let _ = self.execute_action(app, V3Action::Submit { text, attachments });
    }

    fn refresh_pager(&mut self, app: &V3GatewayApp, width: u16) {
        if let Some(pager) = self.transcript_pager.as_mut() {
            pager.replace_committed(app.transcript_lines(width, true));
        }
    }

    fn handle_pager_key(&mut self, key: crossterm::event::KeyEvent) {
        let Some(pager) = self.transcript_pager.as_mut() else {
            return;
        };
        pager.handle_key_event(key);
        if pager.is_done() {
            self.transcript_pager = None;
        }
    }

    #[cfg(test)]
    /// 返回当前是否有未结束的 Gateway 流。
    pub(crate) fn has_active_stream(&self) -> bool {
        self.active_stream.is_some()
    }

    #[cfg(test)]
    /// 返回活跃流的请求 ID，供停止与诊断路径关联。
    pub(crate) fn active_request_id(&self) -> Option<&str> {
        self.active_stream.as_ref().map(|stream| stream.request_id.as_str())
    }

    #[cfg(test)]
    pub(crate) fn active_stream_kind(&self) -> Option<&'static str> {
        self.active_stream.as_ref().map(|stream| match stream.kind {
            ActiveStreamKind::Chat => "chat",
            ActiveStreamKind::PlanRun => "plan-run",
            ActiveStreamKind::Workflow => "workflow",
        })
    }

    #[cfg(test)]
    pub(crate) fn last_workflow_run(&self) -> Option<&WorkflowRunOwnership> {
        self.workflow_run.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn last_workflow_request_id(&self) -> Option<&str> {
        self.workflow_run.as_ref().map(WorkflowRunOwnership::request_id)
    }

    /// 返回完整 transcript 阅读器是否已打开。
    pub(crate) fn has_transcript_pager(&self) -> bool {
        self.transcript_pager.is_some()
    }

    /// 返回外部编辑器动作是否仍在执行。
    pub(crate) fn editor_in_flight(&self) -> bool {
        self.editor_in_flight
    }

    /// 返回交互刷新或回复工作线程是否尚未结束。
    pub(crate) fn interaction_in_flight(&self) -> bool {
        self.interaction_in_flight
    }

    #[cfg(test)]
    pub(crate) fn directory_switch_in_flight(&self) -> bool {
        self.directory_switch_source.is_some()
    }

    #[cfg(test)]
    pub(crate) fn hook_catalog_contains(
        &self,
        plugin_id: &str,
        hook_hash: &str,
        trusted: bool,
    ) -> bool {
        self.hook_catalog
            .as_ref()
            .and_then(|catalog| catalog.find(plugin_id, hook_hash))
            .is_some_and(|hook| hook.trusted == trusted)
    }

    #[cfg(test)]
    pub(crate) fn hook_mutation_in_flight(&self) -> bool {
        self.hook_mutation.is_some()
    }

    #[cfg(test)]
    pub(crate) fn memory_mutation_in_flight(&self) -> bool {
        self.memory_mutation_generation.is_some()
    }
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
}

impl V3GatewayApp {
    /// 以内联终端模式创建应用，并用会话配置补齐缺失的模型名。
    pub(crate) fn new(runtime: GatewayUiRuntime, mut state: TuiState) -> Self {
        if state.status.model_name.is_none() {
            state.status.model_name = state.session_options.model.clone();
        }
        Self::new_with_terminal_mode(runtime, state, TerminalMode::Inline)
    }

    /// 按指定终端模式创建应用，同时回放已恢复的历史消息。
    pub(crate) fn new_with_terminal_mode(
        runtime: GatewayUiRuntime,
        state: TuiState,
        terminal_mode: TerminalMode,
    ) -> Self {
        let mut widget = build_session_widget(&runtime, &state, None);
        for warning in &state.settings_warnings {
            widget.push_history(V3DisplayEvent::System { text: warning.clone() });
        }
        Self {
            runtime,
            state,
            controller: TuiController,
            widget,
            bridge: GatewayV3Bridge::default(),
            terminal_mode,
            transcript_reflow: TranscriptReflowState::default(),
            interaction_refresh_requested: false,
        }
    }

    /// 接收一个 Gateway 流事件，原子地更新状态、聊天部件与会话头。
    pub(crate) fn accept_gateway_event(&mut self, event: GatewayChatStreamEvent) -> Vec<V3Event> {
        let previous_display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        let events = self.bridge.adapt_gateway_event(&mut self.state, event);
        if self.bridge.take_interaction_refresh_requested() {
            self.interaction_refresh_requested = true;
        }
        for event in events.iter().cloned() {
            self.widget.handle_v3_event(event);
        }
        self.sync_runtime_session_seed();
        let display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        if display != previous_display {
            self.widget.set_session_header(display);
        }
        events
    }

    /// 接收 plan-run 标准事件，并把新进度消息与流式文本一起投影到活动 transcript。
    pub(crate) fn accept_runtime_event(&mut self, event: UiRuntimeEvent) -> Vec<V3Event> {
        let previous_display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        let before_len = self.state.messages.len();
        let events = self.bridge.adapt_runtime_event(&mut self.state, event);
        for message in self.state.messages.iter().skip(before_len) {
            if matches!(message, UiMessage::Progress(_)) {
                self.widget.push_history(display_event_from_message(message));
            }
        }
        for event in events.iter().cloned() {
            self.widget.handle_v3_event(event);
        }
        self.sync_runtime_session_seed();
        let display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        if display != previous_display {
            self.widget.set_session_header(display);
        }
        events
    }

    fn plan_run_request(&self, query: &str) -> Result<TaskPlanRunRequest, String> {
        plan_run_request(
            query,
            self.runtime.directory(),
            self.state.session_options.model.clone(),
            self.state.session_options.delegate_agent.clone(),
            self.state.session_options.acp_backend.clone(),
            self.state.session.session_id.clone(),
        )
    }

    fn begin_plan_run(&mut self, request_id: &str, query: &str) {
        self.state.runtime.active_plan_run_request_id = Some(request_id.to_string());
        self.state.runtime.active_plan_run_task_id = None;
        self.state.status.turn_terminal = UiTurnTerminal::Streaming;
        self.state.status.last_error = None;
        let base = UiMessageBase::new(UiMessageId::local(format!("plan-run-user-{request_id}")))
            .with_origin(UiMessageOrigin::Local);
        self.state.append_message(UiMessage::User(UiUserMessage { base, text: query.to_string() }));
        self.widget.push_history(V3DisplayEvent::User {
            text: query.to_string(),
            attachments: Vec::new(),
        });
    }

    fn begin_workflow_run(&mut self, request_id: &str, query: &str) {
        self.state.status.turn_terminal = UiTurnTerminal::Streaming;
        self.state.status.last_error = None;
        let base =
            UiMessageBase::new(UiMessageId::local(format!("workflow-run-user-{request_id}")))
                .with_origin(UiMessageOrigin::Local);
        self.state.append_message(UiMessage::User(UiUserMessage { base, text: query.to_string() }));
        self.widget.push_history(V3DisplayEvent::User {
            text: query.to_string(),
            attachments: Vec::new(),
        });
    }

    fn begin_workflow_resume(&mut self) {
        self.state.status.turn_terminal = UiTurnTerminal::Streaming;
        self.state.status.last_error = None;
    }

    fn replace_workflow_pause_question(
        &mut self,
        question: Option<super::model::UiQuestionOverlay>,
    ) {
        reduce_tui_state(&mut self.state, TuiAction::WorkflowPauseQuestionReplaced(question));
        self.show_next_interaction();
    }

    fn request_interaction_refresh(&mut self) {
        self.interaction_refresh_requested = true;
    }

    fn take_interaction_refresh_requested(&mut self) -> bool {
        std::mem::take(&mut self.interaction_refresh_requested)
    }

    fn apply_interaction_snapshot(&mut self, snapshot: InteractionSnapshot) {
        if self.state.session.session_id.as_deref() != Some(snapshot.session_id.as_str()) {
            return;
        }
        let mut queue = PermissionQueue::default();
        queue.replace(snapshot.permissions.into_iter().map(PermissionRequest::from_dto).collect());
        self.state.overlays.stack.retain(|overlay| !matches!(overlay, UiOverlay::Permissions(_)));
        if queue.len() > 0 {
            self.state
                .overlays
                .push(UiOverlay::Permissions(UiPermissionOverlay { queue, notice: None }));
        }
        reduce_tui_state(
            &mut self.state,
            TuiAction::QuestionsReplaced(
                snapshot
                    .questions
                    .iter()
                    .map(super::model::UiQuestionOverlay::from_request)
                    .collect::<Vec<_>>(),
            ),
        );
        reduce_tui_state(
            &mut self.state,
            TuiAction::TodoOverlayReplaced(Some(UiTodoOverlay::from_todos(
                Some(snapshot.session_id.as_str()),
                snapshot.todos.as_slice(),
            ))),
        );
        reduce_tui_state(&mut self.state, TuiAction::TaskSyncErrorSet(None));
        self.show_next_interaction();
    }

    fn complete_permission(&mut self, request_id: &str) {
        for overlay in &mut self.state.overlays.stack {
            let UiOverlay::Permissions(permission) = overlay else {
                continue;
            };
            if permission.queue.current().is_some_and(|request| request.id == request_id) {
                permission.queue.pop_current();
            }
        }
        self.state.overlays.stack.retain(|overlay| {
            !matches!(overlay, UiOverlay::Permissions(permission) if permission.queue.len() == 0)
        });
        reduce_tui_state(&mut self.state, TuiAction::TaskSyncErrorSet(None));
        self.show_next_interaction();
    }

    fn complete_question(&mut self, request_id: &str) {
        self.state.tasks.pending_questions.retain(|question| question.request_id != request_id);
        reduce_tui_state(&mut self.state, TuiAction::TaskSyncErrorSet(None));
        self.show_next_interaction();
    }

    fn record_interaction_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        reduce_tui_state(&mut self.state, TuiAction::TaskSyncErrorSet(Some(message.clone())));
        push_runner_error(self, message);
    }

    /// 以会话确认、权限、提问的顺序重建安全 overlay；没有请求时才清除。
    pub(crate) fn show_next_interaction(&mut self) {
        let confirmation = self.state.overlays.stack.iter().rev().find_map(|overlay| {
            let UiOverlay::Confirm(confirmation) = overlay else {
                return None;
            };
            Some(confirmation.to_v3_request())
        });
        let permission = self.state.overlays.stack.iter().rev().find_map(|overlay| {
            let UiOverlay::Permissions(permission) = overlay else {
                return None;
            };
            permission.current_request()
        });
        if let Some(confirmation) = confirmation {
            self.widget.show_approval(confirmation);
        } else if let Some(permission) = permission {
            self.widget.show_approval(permission);
        } else if let Some(question) = self.state.tasks.pending_questions.first() {
            self.widget.show_user_input(question.to_v3_request());
        } else {
            self.widget.clear_overlay();
        }
    }

    /// 将当前视图状态转换为会话快照并同步保存。
    pub(crate) fn persist_session_snapshot(&mut self) -> Result<(), String> {
        self.sync_runtime_session_seed();
        let observed_now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or_default();
        let updated_ms = observed_now_ms.max(self.state.session.updated_ms.saturating_add(1));
        let mut snapshot = self.state.to_chat_session();
        snapshot.updated_ms = updated_ms;
        self.runtime.session_ui_save_blocking(&snapshot)?;
        reduce_tui_state(&mut self.state, TuiAction::SessionUpdatedMsSet(updated_ms));
        Ok(())
    }

    /// 加载非归档会话并打开可搜索选择层。
    pub(crate) fn open_session_picker(&mut self) -> Result<(), String> {
        let candidates = self.runtime.session_resume_candidates_blocking()?;
        if candidates.is_empty() {
            return Err("no non-archived sessions are available to resume".to_string());
        }
        self.widget.show_selection_overlay(
            SESSION_RESUME_SURFACE,
            "恢复会话",
            session_resume_items(candidates.as_slice()),
        );
        Ok(())
    }

    /// 恢复当前目录下最近的非归档会话。
    pub(crate) fn resume_latest_session(&mut self) -> Result<(), String> {
        let candidates = self.runtime.session_resume_candidates_blocking()?;
        let session_id = candidates
            .first()
            .map(|candidate| candidate.id.clone())
            .ok_or_else(|| "no non-archived sessions are available to continue".to_string())?;
        self.resume_session(session_id.as_str())
    }

    /// 在完整快照加载成功后一次性切换状态、界面与 runtime seed。
    pub(crate) fn resume_session(&mut self, session_id: &str) -> Result<(), String> {
        let snapshot = self.runtime.session_ui_load_blocking(session_id)?;
        self.commit_session_switch(snapshot);
        Ok(())
    }

    /// 在 Gateway 创建成功后切换到新的空会话。
    pub(crate) fn create_new_session(&mut self) -> Result<(), String> {
        let title = default_session_title(self.runtime.directory());
        let created = self.runtime.session_create_blocking(Some(title.as_str()))?;
        let mut snapshot = empty_session_snapshot(created.id.as_str());
        snapshot.title = created.title;
        self.commit_session_switch(snapshot);
        Ok(())
    }

    fn settings_values(&self) -> SettingsValues {
        SettingsValues {
            model: self.state.session_options.model.clone(),
            delegate_agent: self.state.session_options.delegate_agent.clone(),
            acp_backend: self.state.session_options.acp_backend.clone(),
            full_access_enabled: self.state.session_options.explicit_full_access_enabled(),
            subagent_mode_enabled: self.state.session_options.subagent_mode_enabled(),
            workflow_mode: self.state.session_options.workflow_mode,
            goal_mode: self.state.session_options.goal_mode,
            effort: self.state.session_options.effort,
            fast: self.state.session_options.fast,
            theme: self.state.theme_name.clone(),
            permission_mode: self.state.session_options.permission_mode,
            vim_mode: super::vim::VimMode::Disabled,
        }
    }

    fn open_settings(&mut self) {
        let panel = SettingsPanel::for_capabilities(
            self.settings_values(),
            &self.state.session_capabilities,
        );
        let items = panel
            .rows
            .iter()
            .filter_map(|row| {
                let id = match row.key {
                    SettingKey::Model => "model",
                    SettingKey::DelegateAgent => "delegate",
                    SettingKey::AcpBackend => "acp",
                    SettingKey::FullAccess => "full_access",
                    SettingKey::SubagentMode => "subagents",
                    SettingKey::Effort => "effort",
                    SettingKey::Permissions => "permissions",
                    _ => return None,
                };
                let current = panel.value_label(row.key);
                Some(V3SelectionItem {
                    id: id.to_string(),
                    label: row.label.to_string(),
                    description: Some(
                        row.unavailable_reason
                            .clone()
                            .unwrap_or_else(|| format!("当前：{current}")),
                    ),
                    disabled: row.unavailable_reason.is_some(),
                    disabled_reason: row.unavailable_reason.clone(),
                    ..V3SelectionItem::default()
                })
            })
            .collect();
        self.widget.show_selection_overlay(SETTINGS_SURFACE, "会话设置", items);
    }

    fn open_settings_target(&mut self, target: SettingsTarget) {
        match target {
            SettingsTarget::Model => self.open_model_picker(),
            SettingsTarget::Effort => self.open_effort_picker(),
            SettingsTarget::Permissions => self.open_permission_picker(false),
            SettingsTarget::AcpBackend => self.open_acp_picker(),
            SettingsTarget::DelegateAgent => self.open_delegate_picker(),
            SettingsTarget::FullAccess => self.open_access_picker(),
            SettingsTarget::SubagentMode => self.open_subagent_picker(),
        }
    }

    fn open_model_picker(&mut self) {
        if self.state.session_capabilities.models.is_empty() {
            push_runner_error(self, "当前 Gateway 没有声明可选模型");
            return;
        }
        let current = self.state.session_options.model.as_deref();
        let items = self
            .state
            .session_capabilities
            .models
            .iter()
            .map(|model| setting_item(model, model, current == Some(model.as_str())))
            .collect();
        self.widget.show_selection_overlay(SETTINGS_MODEL_SURFACE, "选择模型", items);
    }

    fn open_effort_picker(&mut self) {
        if self.state.session_capabilities.efforts.is_empty() {
            push_runner_error(self, "当前 provider 没有声明推理强度能力");
            return;
        }
        let current = self.state.session_options.effort;
        let items = self
            .state
            .session_capabilities
            .efforts
            .iter()
            .copied()
            .map(|effort| {
                let id = effort_id(effort);
                setting_item(id, id, effort == current)
            })
            .collect();
        self.widget.show_selection_overlay(SETTINGS_EFFORT_SURFACE, "选择推理强度", items);
    }

    fn open_permission_picker(&mut self, plan_only: bool) {
        let current = self.state.session_options.permission_mode;
        let items = self
            .state
            .session_capabilities
            .permission_modes
            .iter()
            .copied()
            .filter(|mode| {
                !plan_only || matches!(mode, PermissionMode::Default | PermissionMode::Plan)
            })
            .map(|mode| {
                let id = permission_mode_id(mode);
                setting_item(id, id, mode == current)
            })
            .collect();
        let surface = if plan_only { SETTINGS_PLAN_SURFACE } else { SETTINGS_PERMISSION_SURFACE };
        self.widget.show_selection_overlay(surface, "选择权限模式", items);
    }

    fn open_acp_picker(&mut self) {
        let current = self.state.session_options.acp_backend.as_deref();
        let items = self
            .state
            .session_capabilities
            .acp_backends
            .iter()
            .map(|backend| {
                let id = backend.key.as_deref().unwrap_or("default");
                setting_item(id, backend.label.as_str(), current == backend.key.as_deref())
            })
            .collect();
        self.widget.show_selection_overlay(SETTINGS_ACP_SURFACE, "选择 ACP 后端", items);
    }

    fn open_delegate_picker(&mut self) {
        let current = self.state.session_options.delegate_agent.as_deref();
        let items = self
            .state
            .session_capabilities
            .delegate_agents
            .iter()
            .map(|agent| {
                let id = agent.key.as_deref().unwrap_or("main");
                setting_item(id, agent.label.as_str(), current == agent.key.as_deref())
            })
            .collect();
        self.widget.show_selection_overlay(SETTINGS_DELEGATE_SURFACE, "选择委托 Agent", items);
    }

    fn open_access_picker(&mut self) {
        let full = self.state.session_options.full_access_enabled();
        self.widget.show_selection_overlay(
            SETTINGS_ACCESS_SURFACE,
            "选择访问范围",
            vec![
                setting_item("standard", "工作区写入", !full),
                setting_item("full", "完整访问（危险）", full),
            ],
        );
    }

    fn open_subagent_picker(&mut self) {
        let enabled = self.state.session_options.subagent_mode_enabled();
        self.widget.show_selection_overlay(
            SETTINGS_SUBAGENT_SURFACE,
            "选择子智能体模式",
            vec![
                setting_item("disabled", "关闭", !enabled),
                setting_item("enabled", "开启", enabled),
            ],
        );
    }

    fn apply_model(&mut self, model: String) {
        let mut values = self.settings_values();
        values.model = Some(model);
        self.apply_settings_or_report(values);
    }

    fn apply_effort(&mut self, effort: ReasoningEffort) {
        let mut values = self.settings_values();
        values.effort = effort;
        self.apply_settings_or_report(values);
    }

    fn apply_permission_mode(&mut self, mode: PermissionMode) {
        let mut values = self.settings_values();
        values.permission_mode = mode;
        self.apply_settings_or_report(values);
    }

    fn apply_acp_backend(&mut self, backend: Option<String>) {
        let mut values = self.settings_values();
        values.acp_backend = backend;
        self.apply_settings_or_report(values);
    }

    fn apply_delegate_agent(&mut self, agent: Option<String>) {
        let mut values = self.settings_values();
        values.delegate_agent = agent;
        self.apply_settings_or_report(values);
    }

    fn apply_full_access(&mut self, enabled: bool) {
        let mut values = self.settings_values();
        values.full_access_enabled = enabled;
        self.apply_settings_or_report(values);
    }

    fn apply_subagent_mode(&mut self, enabled: bool) {
        let mut values = self.settings_values();
        values.subagent_mode_enabled = enabled;
        self.apply_settings_or_report(values);
    }

    fn apply_settings_or_report(&mut self, values: SettingsValues) {
        if let Err(error) = self.apply_settings_values(values, false) {
            push_runner_error(self, error);
        }
    }

    fn apply_settings_values(
        &mut self,
        values: SettingsValues,
        dangerous_confirmed: bool,
    ) -> Result<(), String> {
        self.validate_settings_values(&values)?;
        let enables_full_access =
            values.full_access_enabled && !self.state.session_options.full_access_enabled();
        let enables_bypass = values.permission_mode == PermissionMode::Bypass
            && !(self.state.session_options.permission_mode == PermissionMode::Bypass
                && self.state.session_options.bypass_confirmed);
        if !dangerous_confirmed && (enables_full_access || enables_bypass) {
            let mut risks = Vec::new();
            if enables_full_access {
                risks.push("完整文件系统与命令访问");
            }
            if enables_bypass {
                risks.push("绕过常规权限模式");
            }
            self.push_session_confirmation(UiConfirmOverlay {
                request_id: "settings-confirm:apply".to_string(),
                title: "确认危险会话设置".to_string(),
                body: format!("将为后续请求启用：{}。当前请求不会被修改。", risks.join("、")),
                confirm_label: "确认启用".to_string(),
                cancel_label: "保持安全设置".to_string(),
                destructive: true,
                intent: UiConfirmIntent::ApplySessionSettings { values: Box::new(values) },
            });
            return Ok(());
        }

        self.state.session_options.model.clone_from(&values.model);
        self.state.status.model_name.clone_from(&values.model);
        self.state.session_options.effort = values.effort;
        self.state.session_options.delegate_agent.clone_from(&values.delegate_agent);
        self.state.session_options.select_acp_backend(values.acp_backend.clone());
        self.state.session_options.set_full_access(values.full_access_enabled);
        self.state.session_options.set_subagent_mode(values.subagent_mode_enabled);
        self.state.session_options.set_permission_mode(values.permission_mode, dangerous_confirmed);
        let display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        self.widget.set_session_header(display);
        self.widget.push_history(V3DisplayEvent::System {
            text: "会话设置已更新，将从下一条请求开始生效。".to_string(),
        });
        Ok(())
    }

    fn validate_settings_values(&self, values: &SettingsValues) -> Result<(), String> {
        let capabilities = &self.state.session_capabilities;
        if values.model != self.state.session_options.model
            && let Some(model) = values.model.as_ref()
            && !capabilities.models.iter().any(|candidate| candidate == model)
        {
            return Err(format!("Gateway 未声明模型能力：{model}"));
        }
        if values.effort != self.state.session_options.effort
            && !capabilities.efforts.contains(&values.effort)
        {
            return Err(format!("Gateway 未声明推理强度：{}", effort_id(values.effort)));
        }
        if values.permission_mode != self.state.session_options.permission_mode
            && !capabilities.permission_modes.contains(&values.permission_mode)
        {
            return Err(format!(
                "Gateway 未声明权限模式：{}",
                permission_mode_id(values.permission_mode)
            ));
        }
        if values.full_access_enabled
            && !self.state.session_options.explicit_full_access_enabled()
            && !capabilities.permission_modes.contains(&PermissionMode::Bypass)
        {
            return Err("Gateway 安全配置未声明完整访问能力".to_string());
        }
        if values.acp_backend != self.state.session_options.acp_backend
            && !capabilities
                .acp_backends
                .iter()
                .any(|candidate| candidate.key == values.acp_backend)
        {
            return Err("Gateway 未声明所选 ACP 后端".to_string());
        }
        if values.delegate_agent != self.state.session_options.delegate_agent
            && !capabilities
                .delegate_agents
                .iter()
                .any(|candidate| candidate.key == values.delegate_agent)
        {
            return Err("Gateway 配置未声明所选委托 Agent".to_string());
        }
        Ok(())
    }

    fn prime_session_command(&mut self, draft: &str, hint: &str) -> RunnerControl {
        self.widget.replace_draft(draft);
        self.widget.push_history(V3DisplayEvent::System { text: hint.to_string() });
        RunnerControl::Continue
    }

    fn open_session_message_picker(&mut self, surface: &str, title: &str) -> Result<(), String> {
        let items = self
            .state
            .session
            .persisted_messages
            .iter()
            .filter_map(|metadata| metadata.raw_message_id.as_deref())
            .map(|message_id| V3SelectionItem {
                id: message_id.to_string(),
                label: message_id.to_string(),
                description: Some("持久化 Gateway 消息".to_string()),
                search_value: Some(message_id.to_string()),
                ..V3SelectionItem::default()
            })
            .collect::<Vec<_>>();
        if items.is_empty() {
            return Err("current session has no persisted message ids".to_string());
        }
        self.widget.show_selection_overlay(surface, title, items);
        Ok(())
    }

    /// 重命名成功后重新加载当前会话的 Gateway canonical snapshot。
    pub(crate) fn rename_session(&mut self, title: &str) -> Result<(), String> {
        self.runtime.session_rename_blocking(title)?;
        self.refresh_current_session_snapshot()?;
        self.widget.push_history(V3DisplayEvent::System {
            text: format!("会话已重命名为：{}", title.trim()),
        });
        Ok(())
    }

    /// 使用 Gateway 返回的新会话 ID 加载分叉快照并一次性重绑活动会话。
    pub(crate) fn fork_session(&mut self, message_id: Option<&str>) -> Result<(), String> {
        let message_id = message_id
            .map(|message_id| self.require_persisted_message_id(message_id))
            .transpose()?;
        let forked_id = self.runtime.session_fork_blocking(message_id)?;
        let snapshot = self.runtime.session_ui_load_blocking(forked_id.as_str())?;
        self.commit_session_switch(snapshot);
        self.widget.push_history(V3DisplayEvent::System {
            text: format!("已分叉并切换到会话 {forked_id}"),
        });
        Ok(())
    }

    fn open_reset_confirmation(&mut self, message_id: &str) -> Result<(), String> {
        let message_id = self.require_persisted_message_id(message_id)?.to_string();
        self.push_session_confirmation(UiConfirmOverlay {
            request_id: format!("session-confirm:reset:{message_id}"),
            title: "确认回退会话".to_string(),
            body: format!(
                "将会话和生成的代码变更回退到消息 {message_id}。此操作不可在 TUI 内撤销。"
            ),
            confirm_label: "确认回退".to_string(),
            cancel_label: "取消".to_string(),
            destructive: true,
            intent: UiConfirmIntent::ResetSession { message_id },
        });
        Ok(())
    }

    fn open_compact_confirmation(&mut self, message_id: &str) -> Result<(), String> {
        let message_id = self.require_persisted_message_id(message_id)?.to_string();
        self.push_session_confirmation(UiConfirmOverlay {
            request_id: format!("session-confirm:compact:{message_id}"),
            title: "确认压缩会话".to_string(),
            body: format!("从持久化消息 {message_id} 开始压缩当前会话上下文。"),
            confirm_label: "确认压缩".to_string(),
            cancel_label: "取消".to_string(),
            destructive: true,
            intent: UiConfirmIntent::CompactSession { message_id },
        });
        Ok(())
    }

    fn open_archive_confirmation(&mut self) -> Result<(), String> {
        let session_id = self.current_session_id()?.to_string();
        self.push_session_confirmation(UiConfirmOverlay {
            request_id: format!("session-confirm:archive:{session_id}"),
            title: "确认归档会话".to_string(),
            body: format!("归档当前会话 {session_id}；它将不再出现在默认恢复列表中。"),
            confirm_label: "确认归档".to_string(),
            cancel_label: "取消".to_string(),
            destructive: true,
            intent: UiConfirmIntent::ArchiveSession,
        });
        Ok(())
    }

    fn push_session_confirmation(&mut self, confirmation: UiConfirmOverlay) {
        let request = confirmation.to_v3_request();
        reduce_tui_state(
            &mut self.state,
            TuiAction::OverlayPushed(UiOverlay::Confirm(confirmation)),
        );
        self.widget.show_approval(request);
    }

    fn dismiss_confirmation(&mut self, request_id: &str) -> Result<(), String> {
        let Some(UiOverlay::Confirm(confirmation)) = self.state.overlays.active() else {
            return Err("no session confirmation is active".to_string());
        };
        if confirmation.request_id != request_id {
            return Err("session confirmation does not match the active intent".to_string());
        }
        reduce_tui_state(&mut self.state, TuiAction::OverlayPopped);
        Ok(())
    }

    fn reset_session(&mut self, message_id: &str) -> Result<(), String> {
        let message_id = self.require_persisted_message_id(message_id)?.to_string();
        self.runtime.session_reset_blocking(message_id.as_str())?;
        self.refresh_current_session_snapshot()?;
        self.widget.push_history(V3DisplayEvent::System {
            text: format!("会话已回退到消息 {message_id}"),
        });
        Ok(())
    }

    fn compact_session(&mut self, message_id: &str) -> Result<(), String> {
        let message_id = self.require_persisted_message_id(message_id)?.to_string();
        let compacted = self.runtime.session_compact_blocking(message_id.as_str())?;
        self.refresh_current_session_snapshot()?;
        let outcome = if compacted { "已压缩" } else { "无需压缩" };
        self.widget.push_history(V3DisplayEvent::System {
            text: format!("会话上下文{outcome}（起点 {message_id}）"),
        });
        Ok(())
    }

    fn archive_session(&mut self) -> Result<(), String> {
        let session_id = self.current_session_id()?.to_string();
        self.runtime.session_archive_blocking()?;
        self.refresh_current_session_snapshot()?;
        self.widget.push_history(V3DisplayEvent::System {
            text: format!("会话 {session_id} 已归档"),
        });
        Ok(())
    }

    fn refresh_current_session_snapshot(&mut self) -> Result<(), String> {
        let session_id = self.current_session_id()?.to_string();
        let snapshot = self.runtime.session_ui_load_blocking(session_id.as_str())?;
        self.commit_session_switch(snapshot);
        Ok(())
    }

    fn current_session_id(&self) -> Result<&str, String> {
        self.state
            .session
            .session_id
            .as_deref()
            .and_then(|session_id| normalize_optional_str_ref(Some(session_id)))
            .ok_or_else(|| "session mutation requires an active session id".to_string())
    }

    fn require_persisted_message_id<'a>(&self, message_id: &'a str) -> Result<&'a str, String> {
        let message_id = normalize_optional_str_ref(Some(message_id))
            .ok_or_else(|| "session mutation requires a non-empty message id".to_string())?;
        if self
            .state
            .session
            .persisted_messages
            .iter()
            .any(|metadata| metadata.raw_message_id.as_deref() == Some(message_id))
        {
            Ok(message_id)
        } else {
            Err(format!("message id {message_id} is not present in the current canonical snapshot"))
        }
    }

    /// 将界面操作路由为控制器命令；提交成功时先追加本地用户历史。
    pub(crate) fn handle_action(
        &mut self,
        action: V3Action,
    ) -> Result<TuiControllerCommand, String> {
        let visible_attachments = match &action {
            V3Action::Submit { attachments, .. } => attachments.clone(),
            _ => Vec::new(),
        };
        let command =
            self.bridge.controller_command(&mut self.state, &mut self.controller, action)?;
        if let TuiControllerCommand::SubmitPrompt(submission) = &command {
            self.widget.push_history(vw_tui::V3DisplayEvent::User {
                text: submission.text.clone(),
                attachments: visible_attachments,
            });
        }
        Ok(command)
    }

    /// 仅为提交命令构造 Gateway 流式请求。
    pub(crate) fn request_for_command(
        &self,
        command: &TuiControllerCommand,
    ) -> Option<GatewayChatStreamRequest> {
        GatewayV3Bridge::request_for_command(&self.state, command)
    }

    fn record_submission_attachments(&mut self, submission: &PromptSubmission) {
        let parent_id = self.state.messages.iter().rev().find_map(|message| match message {
            UiMessage::User(message) => Some(message.base.id.clone()),
            _ => None,
        });
        for attachment in &submission.attachments {
            let mut base = UiMessageBase::new(UiMessageId::local(format!(
                "attachment-{}-{}",
                submission.stream_id.unwrap_or_default(),
                attachment.id,
            )))
            .with_origin(UiMessageOrigin::Local);
            base.parent_id = parent_id.clone();
            if let Some(session_id) = submission.session_id.as_ref() {
                base = base.with_session_id(session_id.clone());
            }
            if let Some(created_ms) = submission.stream_id {
                base = base.with_created_ms(created_ms);
            }
            let mut message = UiAttachment::new(
                base,
                match attachment.kind {
                    PromptAttachmentKind::File => UiAttachmentKind::File,
                    PromptAttachmentKind::Image => UiAttachmentKind::Image,
                    PromptAttachmentKind::PastedText => UiAttachmentKind::PastedText,
                    PromptAttachmentKind::Resource => UiAttachmentKind::Resource,
                },
                attachment.label.clone(),
                attachment.uri.clone().unwrap_or_default(),
            );
            if let Some(media_type) = attachment.media_type.as_ref() {
                message = message.with_media_type(media_type.clone());
            }
            if let Some(text) = attachment.text.as_ref() {
                message = message.with_preview(text.clone());
            }
            if !attachment.bytes.is_empty() {
                message = message.with_size_bytes(attachment.bytes.len() as u64);
            }
            self.state.append_message(UiMessage::Attachment(message));
        }
    }

    /// 借用 Gateway 运行时，不转移其所有权。
    pub(crate) fn runtime(&self) -> &GatewayUiRuntime {
        &self.runtime
    }

    /// 借用当前 TUI 状态快照。
    pub(crate) fn state(&self) -> &TuiState {
        &self.state
    }

    /// 可变借用 TUI 状态；调用方需继续维护 reducer 不变量。
    pub(crate) fn state_mut(&mut self) -> &mut TuiState {
        &mut self.state
    }

    /// 借用聊天渲染部件。
    pub(crate) fn widget(&self) -> &ChatWidget {
        &self.widget
    }

    /// 可变借用聊天渲染部件，供宿主同步草稿与历史。
    pub(crate) fn widget_mut(&mut self) -> &mut ChatWidget {
        &mut self.widget
    }

    fn open_integrations(&mut self, kind: IntegrationKind) {
        let mut items = self
            .state
            .integration_catalog
            .iter()
            .filter(|item| item.kind == kind)
            .map(|item| V3SelectionItem {
                id: item.stable_key(),
                label: item.title.clone(),
                description: Some(format!(
                    "{} · {} · 来源 {}",
                    if item.enabled { "enabled" } else { "disabled" },
                    item.detail,
                    item.source,
                )),
                search_value: Some(format!("{} {} {}", item.id, item.title, item.source)),
                disabled: true,
                disabled_reason: Some("只读目录；管理操作使用带稳定 ID 的斜杠命令".to_string()),
                ..V3SelectionItem::default()
            })
            .collect::<Vec<_>>();
        if items.is_empty() {
            items.push(V3SelectionItem {
                id: "empty".to_string(),
                label: "无可用条目".to_string(),
                description: self.state.integration_warnings.first().cloned(),
                disabled: true,
                ..V3SelectionItem::default()
            });
        }
        self.widget.show_selection_overlay(
            format!("integration_{}", kind.command_name()),
            format!("{} 目录", kind.command_name()),
            items,
        );
    }

    fn replace_memory_items(&mut self, mut items: Vec<IntegrationItem>) {
        self.state.integration_catalog.retain(|item| item.kind != IntegrationKind::Memory);
        self.state.integration_catalog.append(&mut items);
    }

    fn expand_integration_prompt(
        &self,
        kind: IntegrationKind,
        id: &str,
        arguments: &str,
    ) -> Result<String, String> {
        let item = self.require_integration(kind, id)?;
        if !item.enabled {
            return Err(format!("integration {} is disabled", item.stable_key()));
        }
        let prefix = item
            .prompt_prefix
            .as_deref()
            .ok_or_else(|| "integration does not expose prompt expansion".to_string())?;
        Ok(if arguments.trim().is_empty() {
            prefix.to_string()
        } else {
            format!("{prefix}\n\n{}", arguments.trim())
        })
    }

    fn open_integration_toggle_confirmation(
        &mut self,
        kind: IntegrationKind,
        id: &str,
        enabled: bool,
    ) {
        let result = self.require_integration(kind, id).and_then(|item| {
            if !item.can_toggle {
                return Err("selected integration cannot be toggled".to_string());
            }
            Ok(UiConfirmOverlay {
                request_id: format!("integration-toggle-{}-{id}", kind.command_name()),
                title: "确认更改集成状态".to_string(),
                body: format!("{} {}", if enabled { "启用" } else { "禁用" }, item.stable_key()),
                confirm_label: "确认".to_string(),
                cancel_label: "取消".to_string(),
                destructive: false,
                intent: UiConfirmIntent::IntegrationToggle { kind, id: id.to_string(), enabled },
            })
        });
        self.show_integration_confirmation(result);
    }

    pub(crate) fn open_integration_delete_confirmation(&mut self, kind: IntegrationKind, id: &str) {
        let result = self.require_integration(kind, id).and_then(|item| {
            if !item.can_delete {
                return Err("selected integration cannot be deleted".to_string());
            }
            Ok(UiConfirmOverlay {
                request_id: format!("integration-delete-{}-{id}", kind.command_name()),
                title: "确认删除集成".to_string(),
                body: format!("删除 {}", item.stable_key()),
                confirm_label: "删除".to_string(),
                cancel_label: "取消".to_string(),
                destructive: true,
                intent: UiConfirmIntent::IntegrationDelete { kind, id: id.to_string() },
            })
        });
        self.show_integration_confirmation(result);
    }

    fn show_integration_confirmation(&mut self, result: Result<UiConfirmOverlay, String>) {
        match result {
            Ok(overlay) => {
                reduce_tui_state(
                    &mut self.state,
                    TuiAction::OverlayPushed(UiOverlay::Confirm(overlay)),
                );
                self.show_next_interaction();
            }
            Err(error) => push_runner_error(self, error),
        }
    }

    fn mutate_integration(
        &mut self,
        kind: IntegrationKind,
        id: &str,
        enabled: Option<bool>,
    ) -> Result<(), String> {
        let item = self.require_integration(kind, id)?.clone();
        match enabled {
            Some(enabled) => set_integration_enabled_blocking(&self.runtime, &item, enabled)?,
            None => delete_integration_blocking(&self.runtime, &item)?,
        }
        let loaded = load_integrations_blocking(&self.runtime);
        self.state.integration_catalog = loaded.items;
        self.state.integration_warnings = loaded.warnings;
        self.widget.set_command_palette_items(command_palette_items_with_integrations(
            &self.state.integration_catalog,
        ));
        Ok(())
    }

    fn require_integration(
        &self,
        kind: IntegrationKind,
        id: &str,
    ) -> Result<&super::model::integration::IntegrationItem, String> {
        let id = id.trim();
        if id.is_empty() {
            return Err("integration operation requires a stable id".to_string());
        }
        self.state
            .integration_catalog
            .iter()
            .find(|item| item.kind == kind && item.id == id)
            .ok_or_else(|| {
                format!("integration {}:{id} is not in the Gateway catalog", kind.command_name())
            })
    }

    fn open_orchestration(&mut self, kind: OrchestrationPanelKind) {
        let (surface, title, mut items, warning): (
            &str,
            &str,
            Vec<V3SelectionItem>,
            Option<String>,
        ) = match kind {
            OrchestrationPanelKind::Workflows => (
                WORKFLOW_SELECTION_SURFACE,
                "工作流",
                self.state
                    .orchestration
                    .workflows
                    .items
                    .iter()
                    .map(|item| selectable_item(&item.uuid, &item.name, &item.description))
                    .collect::<Vec<_>>(),
                self.state.orchestration.workflows.error.clone(),
            ),
            OrchestrationPanelKind::Schedules => (
                SCHEDULE_SELECTION_SURFACE,
                "定时任务",
                self.state
                    .orchestration
                    .schedules
                    .items
                    .iter()
                    .map(|item| {
                        selectable_item(
                            &item.id,
                            &item.name,
                            &format!(
                                "{} · {} · next {}",
                                if item.enabled { "enabled" } else { "disabled" },
                                item.schedule,
                                item.next_run,
                            ),
                        )
                    })
                    .collect::<Vec<_>>(),
                self.state.orchestration.schedules.error.clone(),
            ),
            OrchestrationPanelKind::Agents => (
                "orchestration",
                "Agents",
                self.state
                    .orchestration
                    .agents
                    .members
                    .iter()
                    .map(|item| {
                        read_only_item(
                            &item.id,
                            &item.name,
                            &format!(
                                "{:?} · team {}",
                                item.status,
                                item.team_id.as_deref().unwrap_or("-")
                            ),
                        )
                    })
                    .collect::<Vec<_>>(),
                self.state.orchestration.agents.error.clone(),
            ),
            OrchestrationPanelKind::Tasks => (
                TASK_SELECTION_SURFACE,
                "后台任务",
                self.state
                    .orchestration
                    .tasks
                    .tasks
                    .iter()
                    .map(|item| {
                        selectable_item(
                            &item.id,
                            &item.title,
                            &format!("{:?} · {}", item.status, item.detail),
                        )
                    })
                    .collect::<Vec<_>>(),
                self.state.orchestration.tasks.error.clone(),
            ),
        };
        if items.is_empty() {
            items.push(V3SelectionItem {
                id: "empty".to_string(),
                label: "无可用条目".to_string(),
                description: warning,
                disabled: true,
                ..V3SelectionItem::default()
            });
        }
        self.widget.show_selection_overlay(surface, title, items);
    }

    fn open_task_control_confirmation(&mut self, id: &str, action: TaskControlAction) {
        let result = self
            .state
            .orchestration
            .tasks
            .tasks
            .iter()
            .find(|task| task.id == id.trim())
            .ok_or_else(|| {
                "task control requires an ID owned by the current project snapshot".to_string()
            })
            .and_then(|task| {
                let scope =
                    self.state.orchestration.scope.as_ref().ok_or_else(|| {
                        "task control requires a resolved project scope".to_string()
                    })?;
                if task.session_id != scope.project_scope().project_id().as_ref() {
                    return Err("task control snapshot does not match the resolved project scope"
                        .to_string());
                }
                if !task.canonical_status.allows(action) {
                    return Err(format!(
                        "task canonical state does not allow {}",
                        action.command_name()
                    ));
                }
                let label = action.display_label();
                Ok(UiConfirmOverlay {
                    request_id: format!("task-{}-{}", action.command_name(), task.id),
                    title: format!("确认{label}任务"),
                    body: format!("{label}当前项目任务 {}", task.id),
                    confirm_label: format!("{label}任务"),
                    cancel_label: if action.destructive() {
                        "保留任务".to_string()
                    } else {
                        "取消".to_string()
                    },
                    destructive: action.destructive(),
                    intent: UiConfirmIntent::TaskControl { id: task.id.clone(), action },
                })
            });
        self.show_integration_confirmation(result);
    }

    fn open_cron_toggle_confirmation(&mut self, id: &str, enabled: bool) {
        let result = self.require_cron_job(id).map(|_| UiConfirmOverlay {
            request_id: format!("cron-{}-{id}", if enabled { "enable" } else { "disable" }),
            title: "确认更改定时任务状态".to_string(),
            body: format!("{}当前定时任务 {id}", if enabled { "启用" } else { "禁用" }),
            confirm_label: (if enabled { "启用" } else { "禁用" }).to_string(),
            cancel_label: "取消".to_string(),
            destructive: false,
            intent: UiConfirmIntent::ScheduleToggle { id: id.to_string(), enabled },
        });
        self.show_integration_confirmation(result);
    }

    fn open_cron_delete_confirmation(&mut self, id: &str) {
        let result = self.require_cron_job(id).map(|_| UiConfirmOverlay {
            request_id: format!("cron-delete-{id}"),
            title: "确认删除定时任务".to_string(),
            body: format!("删除当前定时任务 {id}"),
            confirm_label: "删除".to_string(),
            cancel_label: "保留任务".to_string(),
            destructive: true,
            intent: UiConfirmIntent::ScheduleDelete { id: id.to_string() },
        });
        self.show_integration_confirmation(result);
    }

    fn require_cron_job(&self, id: &str) -> Result<&CronJobSnapshot, String> {
        let id = id.trim();
        self.state.orchestration.cron_jobs.iter().find(|job| job.owns_id(id)).ok_or_else(|| {
            "Cron mutation requires an ID from the current Cron snapshot".to_string()
        })
    }

    fn mutate_owned_cron(&mut self, id: &str, mutation: CronMutation) -> Result<(), String> {
        let refreshed = mutate_owned_cron_and_refresh_blocking(
            &self.runtime,
            &self.state.orchestration.cron_jobs,
            id,
            mutation,
        )?;
        self.state.orchestration.schedules = refreshed.schedules;
        self.state.orchestration.cron_jobs = refreshed.cron_jobs;
        Ok(())
    }

    fn control_owned_task(&mut self, id: &str, action: TaskControlAction) -> Result<(), String> {
        control_owned_task_blocking(&self.runtime, &self.state.orchestration, id, action)?;
        self.state.orchestration = load_orchestration_blocking(&self.runtime);
        Ok(())
    }

    /// 返回创建应用时选定的终端模式。
    pub(crate) fn terminal_mode(&self) -> TerminalMode {
        self.terminal_mode
    }

    fn command_status_text(&self) -> String {
        let display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        format!(
            "Gateway: {}\nSession: {}\nModel: {}\nPermissions: {}",
            display.gateway_endpoint,
            display.session_id.as_deref().unwrap_or("not created"),
            display.model,
            display.permissions,
        )
    }

    /// 使用聊天视图同一批已提交与活跃单元构造完整 transcript 阅读器。
    pub(crate) fn transcript_pager(&self, width: u16) -> TranscriptPager {
        TranscriptPager::new(self.transcript_lines(width, true))
    }

    /// 按宽度生成 transcript 行，供内联历史插入和窗口缩放回放共用。
    pub(crate) fn transcript_lines(&self, width: u16, include_active: bool) -> Vec<Line<'static>> {
        self.widget.transcript_lines(width, include_active)
    }

    /// 仅将已提交单元写入终端原生回滚区，不泄漏尚未完成的流式内容。
    pub(crate) fn insert_committed_history<B>(
        &self,
        terminal: &mut CustomTerminal<B>,
        width: u16,
    ) -> io::Result<()>
    where
        B: Backend<Error = io::Error> + Write,
    {
        insert_history_lines(terminal, self.transcript_lines(width, false))
    }

    /// 仅在宽度确实变化时返回可用于回滚重放的源数据行。
    pub(crate) fn resize_reflow_lines(&mut self, width: u16) -> Option<Vec<Line<'static>>> {
        let change = self.transcript_reflow.note_width(width);
        if change.changed {
            self.transcript_reflow.mark_reflowed_width(width);
            Some(self.transcript_lines(width, true))
        } else {
            None
        }
    }

    fn sync_runtime_session_seed(&mut self) {
        let title =
            (!self.state.session.title.trim().is_empty()).then(|| self.state.session.title.clone());
        self.runtime.bind_session_seed(
            self.state.session.session_id.clone(),
            self.state.session.scope.clone(),
            title,
        );
    }

    fn commit_session_switch(&mut self, snapshot: ChatSession) {
        let scope = self
            .runtime
            .scope()
            .map(ToOwned::to_owned)
            .or_else(|| self.state.session.scope.clone());
        let mut next_state = self.state.clone();
        reduce_tui_state(
            &mut next_state,
            TuiAction::ReplaceFromSnapshot { snapshot, scope, path: None },
        );

        let title =
            (!next_state.session.title.trim().is_empty()).then(|| next_state.session.title.clone());
        let mut next_runtime = self.runtime.clone();
        next_runtime.bind_session_seed(
            next_state.session.session_id.clone(),
            next_state.session.scope.clone(),
            title,
        );
        let draft = self.widget.draft().to_string();
        let next_widget = build_session_widget(&next_runtime, &next_state, Some(draft.as_str()));

        self.runtime = next_runtime;
        self.state = next_state;
        self.widget = next_widget;
        self.bridge = GatewayV3Bridge::default();
        self.transcript_reflow = TranscriptReflowState::default();
        self.interaction_refresh_requested = true;
    }

    fn commit_initial_session_state(&mut self, mut next_state: TuiState) {
        next_state.project = self.state.project.clone();
        next_state.session_options = self.state.session_options.clone();
        next_state.session_capabilities = self.state.session_capabilities.clone();
        next_state.model_catalog = self.state.model_catalog.clone();
        next_state.settings_warnings = self.state.settings_warnings.clone();
        next_state.integration_catalog = self.state.integration_catalog.clone();
        next_state.integration_warnings = self.state.integration_warnings.clone();
        next_state.orchestration = self.state.orchestration.clone();
        next_state.theme_name = self.state.theme_name.clone();
        next_state.status.provider_name = self.state.status.provider_name.clone();
        if next_state.status.model_name.is_none() {
            next_state.status.model_name = self.state.status.model_name.clone();
        }

        let title =
            (!next_state.session.title.trim().is_empty()).then(|| next_state.session.title.clone());
        self.runtime.bind_session_seed(
            next_state.session.session_id.clone(),
            next_state.session.scope.clone(),
            title,
        );
        let draft = self.widget.draft().to_string();
        self.widget = build_session_widget(&self.runtime, &next_state, Some(draft.as_str()));
        self.state = next_state;
        self.bridge = GatewayV3Bridge::default();
        self.transcript_reflow = TranscriptReflowState::default();
        self.interaction_refresh_requested = true;
    }

    /// 目标目录全部快照准备完成后，一次性替换活动 runtime、状态与界面。
    pub(crate) fn commit_directory_switch(&mut self, bootstrap: DirectorySwitchBootstrap) {
        let DirectorySwitchBootstrap { runtime, mut state } = bootstrap;
        state.session_options = self.state.session_options.clone();
        state.theme_name = self.state.theme_name.clone();
        state.status.provider_name = self.state.status.provider_name.clone();
        if state.status.model_name.is_none() {
            state.status.model_name = self.state.status.model_name.clone();
        }

        let draft = self.widget.draft().to_string();
        let widget = build_session_widget(&runtime, &state, Some(draft.as_str()));
        self.runtime = runtime;
        self.state = state;
        self.widget = widget;
        self.bridge = GatewayV3Bridge::default();
        self.transcript_reflow = TranscriptReflowState::default();
        self.interaction_refresh_requested = true;
    }
}

fn build_session_widget(
    runtime: &GatewayUiRuntime,
    state: &TuiState,
    draft: Option<&str>,
) -> ChatWidget {
    let display_data = GatewayV3Bridge::session_display_data(runtime, state);
    let mut widget = ChatWidget::new(ChatWidgetConfig::default());
    widget.set_command_palette_items(command_palette_items_with_integrations(
        &state.integration_catalog,
    ));
    widget.push_session_header(display_data);
    replay_session_history(&mut widget, state);
    if let Some(draft) = draft {
        widget.replace_draft(draft);
    }
    widget
}

fn setting_item(id: &str, label: &str, selected: bool) -> V3SelectionItem {
    V3SelectionItem {
        id: id.to_string(),
        label: label.to_string(),
        description: selected.then(|| "当前值".to_string()),
        search_value: Some(format!("{id} {label}")),
        ..V3SelectionItem::default()
    }
}

fn read_only_item(id: &str, label: &str, description: &str) -> V3SelectionItem {
    V3SelectionItem {
        id: id.to_string(),
        label: label.to_string(),
        description: Some(description.to_string()),
        search_value: Some(format!("{id} {label} {description}")),
        disabled: true,
        disabled_reason: Some("状态由 Gateway 管理".to_string()),
        ..V3SelectionItem::default()
    }
}

fn selectable_item(id: &str, label: &str, description: &str) -> V3SelectionItem {
    V3SelectionItem {
        id: id.to_string(),
        label: label.to_string(),
        description: Some(description.to_string()),
        search_value: Some(format!("{id} {label} {description}")),
        ..V3SelectionItem::default()
    }
}

fn effort_id(effort: ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::Minimal => "minimal",
        ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::High => "high",
        ReasoningEffort::XHigh => "xhigh",
    }
}

fn permission_mode_id(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::Default => "default",
        PermissionMode::Plan => "plan",
        PermissionMode::AcceptEdits => "accept-edits",
        PermissionMode::Bypass => "bypass",
    }
}

/// 作为 CLI 交互入口启动 Gateway、初始化会话并运行终端循环。
pub(crate) fn run_tui(config: &Config, setup: &CliSetup) -> Result<()> {
    let runtime = GatewayUiRuntime::for_workspace(config).map_err(|error| anyhow!(error))?;
    runtime.ensure_local_gateway_ready_blocking().map_err(|error| anyhow!(error))?;

    let mut state = initial_tui_state(&runtime);
    let loaded = load_session_capabilities_blocking(&runtime);
    let selected_model = if loaded.matrix.models.iter().any(|model| model == &setup.model_name) {
        Some(setup.model_name.clone())
    } else {
        loaded
            .default_model
            .clone()
            .or_else(|| loaded.matrix.models.first().cloned())
            .or_else(|| Some(setup.model_name.clone()))
    };
    state.session_capabilities = loaded.matrix;
    state.settings_warnings = loaded.warnings;
    state.session_options.model.clone_from(&selected_model);
    state.status.model_name = selected_model;
    let integrations = load_integrations_blocking(&runtime);
    state.integration_catalog = integrations.items;
    state.integration_warnings = integrations.warnings;
    state.orchestration = load_orchestration_blocking(&runtime);
    let app = V3GatewayApp::new_with_terminal_mode(runtime, state, TerminalMode::Alternate);
    let terminal_mode = app.terminal_mode();

    run_terminal(terminal_mode, || run_event_loop(app)).map_err(Into::into)
}

pub(crate) fn initial_tui_state(runtime: &GatewayUiRuntime) -> TuiState {
    let mut state = TuiState::default();
    state.session.scope = runtime.scope().map(ToOwned::to_owned);
    state.project.workspace_root = Some(runtime.directory().to_path_buf());
    state
}

/// 恢复显式指定的会话；普通启动始终创建新会话。
pub(crate) fn restore_or_create_session_state(
    runtime: &mut GatewayUiRuntime,
) -> Result<TuiState, String> {
    let explicit_session_id =
        normalize_optional_str_ref(runtime.session_id()).map(ToOwned::to_owned);
    let scope = runtime.scope().map(ToOwned::to_owned);
    let snapshot = match explicit_session_id {
        Some(session_id) => runtime.session_ui_load_blocking(session_id.as_str())?,
        None => {
            let title = default_session_title(runtime.directory());
            let created = runtime.session_create_blocking(Some(title.as_str()))?;
            let mut snapshot = empty_session_snapshot(created.id.as_str());
            snapshot.title = created.title;
            snapshot
        }
    };

    let mut state = TuiState::from_chat_session(&snapshot);
    state.session.scope = scope;
    state.project.workspace_root = Some(runtime.directory().to_path_buf());
    let title = (!state.session.title.trim().is_empty()).then(|| state.session.title.clone());
    runtime.bind_session_seed(state.session.session_id.clone(), state.session.scope.clone(), title);
    Ok(state)
}

/// 在临时 runtime 上解析并完整启动目标目录；调用方只提交成功结果。
pub(crate) fn bootstrap_directory_switch(
    mut runtime: GatewayUiRuntime,
    requested_path: &Path,
) -> Result<DirectorySwitchBootstrap, String> {
    let requested_path = if requested_path.is_absolute() {
        requested_path.to_path_buf()
    } else {
        runtime.directory().join(requested_path)
    };
    let project = runtime.resolve_project_directory_blocking(requested_path.as_path(), false)?;
    runtime.rebind_validated_directory(Path::new(project.directory.as_str()))?;

    let mut state = restore_or_create_session_state(&mut runtime)?;
    let capabilities = load_session_capabilities_blocking(&runtime);
    if !capabilities.warnings.is_empty() {
        return Err(format!(
            "target capability bootstrap failed: {}",
            capabilities.warnings.join("; ")
        ));
    }
    state.status.model_name =
        capabilities.default_model.clone().or_else(|| capabilities.matrix.models.first().cloned());
    state.session_options.model.clone_from(&state.status.model_name);
    state.session_capabilities = capabilities.matrix;
    state.settings_warnings = capabilities.warnings;

    let integrations = load_integrations_blocking(&runtime);
    if !integrations.warnings.is_empty() {
        return Err(format!(
            "target integration bootstrap failed: {}",
            integrations.warnings.join("; ")
        ));
    }
    state.integration_catalog = integrations.items;
    state.integration_warnings = integrations.warnings;

    let orchestration = load_orchestration_blocking(&runtime);
    if !orchestration.warnings.is_empty() {
        return Err(format!(
            "target orchestration bootstrap failed: {}",
            orchestration.warnings.join("; ")
        ));
    }
    state.orchestration = orchestration;
    Ok(DirectorySwitchBootstrap::new(runtime, state))
}

fn empty_session_snapshot(session_id: &str) -> ChatSession {
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

fn default_session_title(directory: &Path) -> String {
    directory
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(|name| format!("TUI {}", name.trim()))
        .unwrap_or_else(|| "TUI 会话".to_string())
}

/// 根据当前状态和提交快照构造 Gateway 请求，并在此收紧工具与权限选项。
pub(crate) fn gateway_stream_request_from_state(
    state: &TuiState,
    submission: &PromptSubmission,
) -> GatewayChatStreamRequest {
    let snapshot = state.to_chat_session();
    let acp_agent = submission.execution.acp_backend.clone();
    let full_access_enabled = submission.execution.full_access_enabled;
    let subagent_mode_enabled = submission.execution.subagent_mode_enabled;
    let mut allowed_tools = (!full_access_enabled
        && submission.execution.permission_mode == PermissionMode::Plan)
        .then(|| vec!["todowrite".to_string()]);
    if subagent_mode_enabled {
        // 子智能体模式只精确追加 AgentTool，不借此解锁其他受限工具。
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
                .or_else(|| {
                    state.project.workspace_root.as_ref().map(|path| path.display().to_string())
                })
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
    let mut messages: Vec<_> =
        snapshot.messages.iter().map(gateway_message_from_chat_message).collect();
    if !submission.attachments.is_empty()
        && let Some(message) = messages
            .iter_mut()
            .rev()
            .find(|message| message.get("role").and_then(serde_json::Value::as_str) == Some("user"))
    {
        message["content"] = gateway_structured_content(submission);
    }

    GatewayChatStreamRequest {
        session_id: normalize_optional_str_ref(submission.session_id.as_deref())
            .map(SessionId::from),
        messages,
        system,
        model: submission.model.clone(),
        reasoning_effort: Some(match submission.execution.effort {
            ReasoningEffort::Minimal => vw_gateway_client::vw_api_types::ReasoningEffortDto::Minimal,
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

fn gateway_structured_content(submission: &PromptSubmission) -> serde_json::Value {
    let mut blocks = Vec::with_capacity(submission.attachments.len().saturating_add(1));
    if !submission.text.trim().is_empty() {
        blocks.push(json!({"type": "text", "text": submission.text}));
    }
    blocks.extend(submission.attachments.iter().map(gateway_attachment_block));
    serde_json::Value::Array(blocks)
}

fn gateway_attachment_block(attachment: &PromptAttachment) -> serde_json::Value {
    match attachment.kind {
        PromptAttachmentKind::Image => json!({
            "type": "image",
            "mimeType": attachment.media_type.as_deref().unwrap_or("image/png"),
            "data": STANDARD.encode(&attachment.bytes),
        }),
        PromptAttachmentKind::PastedText => json!({
            "type": "text",
            "text": attachment.text.as_deref().unwrap_or_default(),
        }),
        PromptAttachmentKind::File | PromptAttachmentKind::Resource => json!({
            "type": "resource",
            "resource": {
                "uri": attachment.uri.as_deref().unwrap_or_default(),
                "text": attachment.text.as_deref().unwrap_or_default(),
            },
        }),
    }
}

const SUBAGENT_MODE_SYSTEM_PROMPT: &str = "## AgentTool-First Mode\n\n\
- Prefer AgentTool for meaningful independent workstreams when delegation improves speed, focus, or verification.\n\
- The parent agent may work directly when delegation would not help, and remains responsible for coordinating, verifying evidence, and synthesizing results.\n\
- If an AgentTool call fails, report the exact blocker and continue safely when the task can still be completed without that delegation.\n\
- Continue to follow all safety, permission, scope, and validation requirements.";

fn gateway_message_from_chat_message(message: &ChatMessage) -> serde_json::Value {
    json!({
        "role": gateway_role(message.role),
        "content": message.content,
    })
}

fn gateway_role(role: ChatRole) -> &'static str {
    match role {
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::System => "system",
        ChatRole::Tool => "tool",
    }
}

fn replay_session_history(widget: &mut ChatWidget, state: &TuiState) {
    let displayable =
        state.messages.iter().filter(|message| !matches!(message, UiMessage::Step(_))).count();
    let skip = displayable.saturating_sub(SESSION_HISTORY_DISPLAY_LIMIT);
    for message in
        state.messages.iter().filter(|message| !matches!(message, UiMessage::Step(_))).skip(skip)
    {
        widget.push_history(display_event_from_message(message));
    }
}

fn display_event_from_message(message: &UiMessage) -> V3DisplayEvent {
    match message {
        UiMessage::User(message) => {
            V3DisplayEvent::User { text: message.text.clone(), attachments: Vec::new() }
        }
        UiMessage::Assistant(message) => {
            V3DisplayEvent::Assistant { markdown: message.text.clone() }
        }
        UiMessage::ToolCall(message) => V3DisplayEvent::ToolCall {
            call_id: message
                .call_id
                .clone()
                .unwrap_or_else(|| message.base.id.as_str().to_string()),
            tool: message.tool_name.clone(),
            command: message
                .arguments
                .clone()
                .or_else(|| message.summary.clone())
                .unwrap_or_default(),
        },
        UiMessage::ToolResult(message) => V3DisplayEvent::ToolResult {
            call_id: message
                .call_id
                .clone()
                .unwrap_or_else(|| message.base.id.as_str().to_string()),
            tool: message.tool_name.clone(),
            command: message.command.clone().unwrap_or_default(),
            output: message.content.clone(),
            exit_code: i32::from(message.is_error),
            duration_ms: 0,
        },
        UiMessage::Thinking(message) => V3DisplayEvent::Thinking { text: message.content.clone() },
        UiMessage::Step(message) => V3DisplayEvent::Step {
            title: message
                .finish_reason
                .clone()
                .unwrap_or_else(|| format!("step {}", message.step_index)),
            status: match message.state {
                UiStepState::Pending => V3StepStatus::Pending,
                UiStepState::Running => V3StepStatus::InProgress,
                UiStepState::Complete => V3StepStatus::Completed,
                UiStepState::Cancelled | UiStepState::Failed => V3StepStatus::Failed,
            },
        },
        UiMessage::System(message) => V3DisplayEvent::System { text: message.text.clone() },
        UiMessage::Error(message) => V3DisplayEvent::Error { message: message.message.clone() },
        UiMessage::Attachment(message) => V3DisplayEvent::Attachment {
            attachment: V3Attachment {
                kind: match message.kind {
                    super::model::UiAttachmentKind::Image => V3AttachmentKind::Image,
                    super::model::UiAttachmentKind::PastedText => V3AttachmentKind::PastedText,
                    super::model::UiAttachmentKind::Resource
                    | super::model::UiAttachmentKind::Link => V3AttachmentKind::Resource,
                    super::model::UiAttachmentKind::File => V3AttachmentKind::File,
                },
                label: message.name.clone(),
                path: (!message.uri.trim().is_empty()).then(|| message.uri.clone()),
                media_type: message.media_type.clone(),
                text: message.preview.clone().or_else(|| message.alt_text.clone()),
                bytes: None,
            },
        },
        UiMessage::Progress(message) => V3DisplayEvent::Progress {
            label: message.label.clone(),
            detail: match (message.current, message.total) {
                (Some(current), Some(total)) => Some(format!("{current}/{total}")),
                (Some(current), None) => Some(current.to_string()),
                _ => None,
            },
        },
        UiMessage::CompactBoundary(message) => {
            V3DisplayEvent::CompactBoundary { summary: message.summary.clone() }
        }
        UiMessage::Unknown(message) => V3DisplayEvent::Unknown {
            type_name: message.external_kind.clone(),
            detail: message.summary.clone(),
        },
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TerminalBatchOutcome {
    pub(crate) processed: usize,
    pub(crate) quit: bool,
    pub(crate) editor_terminal_suspended: bool,
}

pub(crate) fn apply_ready_terminal_events(
    app: &mut V3GatewayApp,
    runner: &mut ActiveRunner,
    first_event: Event,
    mut read_ready: impl FnMut() -> io::Result<Option<Event>>,
) -> io::Result<TerminalBatchOutcome> {
    let mut outcome = TerminalBatchOutcome::default();
    let mut next_event = Some(first_event);

    while let Some(current_event) = next_event {
        outcome.processed += 1;
        if runner.has_transcript_pager() {
            match current_event {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    runner.handle_pager_key(key);
                }
                Event::Resize(width, _) => runner.refresh_pager(app, width),
                _ => {}
            }
        } else {
            let action = match current_event {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    app.widget_mut().handle_key_event(key)
                }
                Event::Paste(text) => app.widget_mut().handle_paste(text),
                Event::Mouse(mouse) => app.widget_mut().handle_mouse_event(mouse),
                Event::Resize(width, _) => {
                    let _ = app.resize_reflow_lines(width);
                    V3Action::None
                }
                _ => V3Action::None,
            };

            let opens_editor = matches!(action, V3Action::OpenExternalEditor);
            if opens_editor {
                crossterm::terminal::disable_raw_mode()?;
            }
            let control = runner.execute_action(app, action);
            if opens_editor {
                if runner.editor_in_flight() {
                    outcome.editor_terminal_suspended = true;
                } else {
                    crossterm::terminal::enable_raw_mode()?;
                }
            }
            if control == RunnerControl::Quit {
                outcome.quit = true;
                break;
            }
        }

        if outcome.processed == TERMINAL_EVENT_BUDGET {
            break;
        }
        next_event = read_ready()?;
    }

    Ok(outcome)
}

fn run_event_loop(mut app: V3GatewayApp) -> io::Result<()> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = V3Terminal::with_options(backend)?;
    let mut runner = ActiveRunner::new();
    runner.request_session_bootstrap(&mut app);
    let mut editor_terminal_suspended = false;

    loop {
        let background_work_remaining = runner.drain(&mut app);
        if editor_terminal_suspended && !runner.editor_in_flight() {
            crossterm::terminal::enable_raw_mode()?;
            terminal.clear()?;
            editor_terminal_suspended = false;
        }
        if runner.editor_in_flight() {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        let wait =
            if background_work_remaining { Duration::ZERO } else { Duration::from_millis(50) };
        if event::poll(wait)? {
            let first_event = event::read()?;
            let outcome = apply_ready_terminal_events(&mut app, &mut runner, first_event, || {
                if event::poll(Duration::ZERO)? { event::read().map(Some) } else { Ok(None) }
            })?;
            editor_terminal_suspended |= outcome.editor_terminal_suspended;
            if outcome.quit {
                return Ok(());
            }
        }

        let width = crossterm::terminal::size()?.0;
        runner.refresh_pager(&app, width);
        draw(&app, runner.transcript_pager.as_mut(), &mut terminal)?;
    }
}

fn spawn_stream(
    runtime: GatewayUiRuntime,
    request: GatewayChatStreamRequest,
    request_id: String,
) -> ActiveStream {
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stream_cancel = Arc::clone(&cancel);
    let worker_request_id = request_id.clone();
    std::thread::spawn(move || {
        let result =
            runtime.stream_chat_resumable_blocking(&request, worker_request_id.as_str(), |event| {
                !stream_cancel.load(Ordering::Acquire)
                    && sender.send(StreamMessage::ChatEvent(event)).is_ok()
            });
        let result = if stream_cancel.load(Ordering::Acquire) {
            Ok(GatewayStreamAttemptResult::CallerStopped)
        } else {
            result
        };
        let _ = sender.send(StreamMessage::Finished(result));
    });
    ActiveStream { kind: ActiveStreamKind::Chat, request_id, receiver, cancel }
}

fn spawn_file_search(
    runtime: GatewayUiRuntime,
    generation: u64,
    query: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = search_files_blocking(&runtime, query.as_str());
        let _ = sender.send(HostMessage::FileSearchFinished { generation, query, result });
    });
}

fn spawn_session_diff(
    runtime: GatewayUiRuntime,
    generation: u64,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let session_id = runtime.session_id().map(str::to_string);
        let result = load_session_diff_blocking(&runtime);
        let _ = sender.send(HostMessage::SessionDiffFinished { generation, session_id, result });
    });
}

fn spawn_doctor(runtime: GatewayUiRuntime, generation: u64, sender: mpsc::Sender<HostMessage>) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = load_gateway_diagnostics_blocking(&runtime);
        let _ = sender.send(HostMessage::DoctorFinished { generation, directory, result });
    });
}

fn spawn_cron_detail(
    runtime: GatewayUiRuntime,
    snapshot: Vec<CronJobSnapshot>,
    generation: u64,
    job_id: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result =
            load_owned_cron_detail_blocking(&runtime, &snapshot, job_id.as_str()).map(Box::new);
        let _ =
            sender.send(HostMessage::CronDetailFinished { generation, directory, job_id, result });
    });
}

fn spawn_task_detail(
    runtime: GatewayUiRuntime,
    snapshot: OrchestrationSnapshot,
    generation: u64,
    task_id: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result =
            load_owned_task_detail_blocking(&runtime, &snapshot, task_id.as_str()).map(Box::new);
        let _ =
            sender.send(HostMessage::TaskDetailFinished { generation, directory, task_id, result });
    });
}

fn spawn_workspace_files(
    runtime: GatewayUiRuntime,
    generation: u64,
    requested_path: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = load_workspace_files_blocking(&runtime, requested_path.as_str());
        let _ = sender.send(HostMessage::WorkspaceFilesFinished {
            generation,
            directory,
            requested_path,
            result,
        });
    });
}

fn spawn_workspace_file_read(
    runtime: GatewayUiRuntime,
    generation: u64,
    requested_path: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = read_workspace_file_blocking(&runtime, requested_path.as_str());
        let _ = sender.send(HostMessage::WorkspaceFileReadFinished {
            generation,
            directory,
            requested_path,
            result,
        });
    });
}

fn spawn_hook_catalog(
    runtime: GatewayUiRuntime,
    generation: u64,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = load_hook_catalog_blocking(&runtime);
        let _ = sender.send(HostMessage::HookCatalogFinished { generation, directory, result });
    });
}

fn spawn_hook_audit(runtime: GatewayUiRuntime, generation: u64, sender: mpsc::Sender<HostMessage>) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = load_hook_audit_blocking(&runtime);
        let _ = sender.send(HostMessage::HookAuditFinished { generation, directory, result });
    });
}

fn spawn_hook_mutation(
    runtime: GatewayUiRuntime,
    generation: u64,
    plugin_id: String,
    hook_hash: String,
    trusted: bool,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = mutate_hook_and_refresh_blocking(
            &runtime,
            plugin_id.as_str(),
            hook_hash.as_str(),
            trusted,
        );
        let _ = sender.send(HostMessage::HookMutationFinished {
            generation,
            directory,
            plugin_id,
            hook_hash,
            trusted,
            result,
        });
    });
}

fn spawn_memory_search(
    runtime: GatewayUiRuntime,
    generation: u64,
    query: Option<String>,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = query_memory_blocking(&runtime, query.as_deref());
        let _ =
            sender.send(HostMessage::MemorySearchFinished { generation, directory, query, result });
    });
}

fn spawn_memory_mutation(
    runtime: GatewayUiRuntime,
    generation: u64,
    mutation: MemoryMutation,
    query: Option<String>,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let action = mutation.action_name();
        let key = mutation.key().to_string();
        let result = mutate_memory_and_refresh_blocking(&runtime, &mutation, query.as_deref());
        let _ = sender.send(HostMessage::MemoryMutationFinished {
            generation,
            directory,
            query,
            action,
            key,
            result,
        });
    });
}

fn spawn_plan_run(
    runtime: GatewayUiRuntime,
    request: TaskPlanRunRequest,
    request_id: String,
) -> ActiveStream {
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stream_cancel = Arc::clone(&cancel);
    let worker_request_id = request_id.clone();
    std::thread::spawn(move || {
        let result = stream_plan_run_resumable_blocking(
            &runtime,
            &request,
            worker_request_id.as_str(),
            |event| {
                !stream_cancel.load(Ordering::Acquire)
                    && sender.send(StreamMessage::RuntimeEvent(event)).is_ok()
            },
        );
        let result = if stream_cancel.load(Ordering::Acquire) {
            Ok(GatewayStreamAttemptResult::CallerStopped)
        } else {
            result
        };
        let _ = sender.send(StreamMessage::Finished(result));
    });
    ActiveStream { kind: ActiveStreamKind::PlanRun, request_id, receiver, cancel }
}

fn spawn_workflow_run(
    runtime: GatewayUiRuntime,
    request: OwnedWorkflowRunRequest,
    request_id: String,
) -> ActiveStream {
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stream_cancel = Arc::clone(&cancel);
    std::thread::spawn(move || {
        let result = stream_workflow_run_blocking(&runtime, &request, |event| {
            !stream_cancel.load(Ordering::Acquire)
                && sender.send(StreamMessage::WorkflowEvent(event)).is_ok()
        });
        let result = if stream_cancel.load(Ordering::Acquire) {
            Ok(GatewayStreamAttemptResult::CallerStopped)
        } else {
            result
        };
        let _ = sender.send(StreamMessage::Finished(result));
    });
    ActiveStream { kind: ActiveStreamKind::Workflow, request_id, receiver, cancel }
}

fn spawn_stop(runtime: GatewayUiRuntime, request_id: String, sender: mpsc::Sender<HostMessage>) {
    std::thread::spawn(move || {
        let result = runtime.chat_stream_stop_blocking(request_id.as_str());
        let _ = sender.send(HostMessage::StopFinished { request_id, result });
    });
}

fn spawn_plan_run_stop(
    runtime: GatewayUiRuntime,
    request_id: String,
    task_id: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = stop_owned_plan_run_blocking(
            &runtime,
            Some(request_id.as_str()),
            Some(task_id.as_str()),
            request_id.as_str(),
            task_id.as_str(),
        );
        let _ = sender.send(HostMessage::PlanRunStopFinished { request_id, task_id, result });
    });
}

fn spawn_interaction_refresh(
    runtime: GatewayUiRuntime,
    session_id: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = runtime.interaction_snapshot_blocking(Some(session_id.as_str()));
        let _ = sender.send(HostMessage::InteractionsRefreshed(result));
    });
}

fn spawn_session_bootstrap(mut runtime: GatewayUiRuntime, sender: mpsc::Sender<HostMessage>) {
    std::thread::spawn(move || {
        let result = restore_or_create_session_state(&mut runtime).map(Box::new);
        let _ = sender.send(HostMessage::SessionBootstrapFinished(result));
    });
}

fn spawn_directory_switch(
    runtime: GatewayUiRuntime,
    requested_path: PathBuf,
    generation: u64,
    source_directory: PathBuf,
    recovery_draft: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = bootstrap_directory_switch(runtime, requested_path.as_path()).map(Box::new);
        let _ = sender.send(HostMessage::DirectorySwitchFinished {
            generation,
            source_directory,
            recovery_draft,
            result,
        });
    });
}

fn spawn_permission_reply(
    runtime: GatewayUiRuntime,
    request_id: String,
    reply: PendingPermissionReplyDto,
    message: Option<String>,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = runtime.permission_reply_blocking(request_id.as_str(), reply, message);
        let _ = sender.send(HostMessage::PermissionReplyFinished { request_id, result });
    });
}

fn spawn_question_reply(
    runtime: GatewayUiRuntime,
    request_id: String,
    answers: Option<Vec<Vec<String>>>,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = match answers {
            Some(answers) => runtime.question_reply_blocking(request_id.as_str(), answers),
            None => runtime.question_reject_blocking(request_id.as_str()),
        };
        let _ = sender.send(HostMessage::QuestionReplyFinished { request_id, result });
    });
}

fn spawn_workflow_resume(
    runtime: GatewayUiRuntime,
    pause_request_id: String,
    request: WorkflowResumeRequest,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = resume_workflow_run_blocking(&runtime, &request).map(Box::new);
        let _ = sender.send(HostMessage::WorkflowResumeFinished { pause_request_id, result });
    });
}

fn next_stream_request_id() -> String {
    static NEXT_SEQUENCE: AtomicU64 = AtomicU64::new(1);
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default();
    let sequence = NEXT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("tui-{now_ms}-{sequence}")
}

fn next_plan_run_request_id() -> String {
    format!("plan-run-{}", next_stream_request_id())
}

fn next_workflow_run_request_id() -> String {
    format!("workflow-run-{}", next_stream_request_id())
}

fn draw(
    app: &V3GatewayApp,
    pager: Option<&mut TranscriptPager>,
    terminal: &mut V3Terminal,
) -> io::Result<()> {
    let (width, height) = crossterm::terminal::size()?;
    let y = match app.terminal_mode() {
        TerminalMode::Inline => terminal.viewport_area.y.min(height.saturating_sub(1)),
        TerminalMode::Alternate => 0,
    };
    terminal.set_viewport_area(Rect::new(0, y, width, height.saturating_sub(y)));
    terminal.draw(|frame| {
        let area = frame.area();
        if let Some(pager) = pager {
            pager.render(area, frame.buffer_mut());
        } else {
            app.widget().render(area, frame.buffer_mut());
            frame.set_cursor_style(app.widget().cursor_style(area));
            if let Some(position) = app.widget().cursor_pos(area) {
                frame.set_cursor_position(position);
            }
        }
    })
}
