//! TUI command 与 overlay 副作用。

use super::{
    PromptEditorIntent, Result, TodoSessionAccessAction, TranscriptEffectIntent, TuiAction, TuiApp,
    TuiAppCommandOutcome, TuiControllerCommand, TuiOverlayCommand, TuiSlashCommandOutcome,
    TuiTerminal, UiConfirmOverlay, UiErrorOverlay, UiMessage, UiMessageBase, UiMessageId,
    UiOverlay, UiQuestionOverlay, UiSystemMessage, UiSystemMessageLevel, UiTaskOverlay,
    UiTaskStepItem, UiTodoOverlay, build_exit_confirm_body, build_mcp_overlay,
    build_memory_overlay, build_project_info, collect_git_workspace_status, disable_raw_mode,
    enable_raw_mode, execute_slash_command, is_session_unavailable_error,
    question_overlay_submission_answers, reduce_tui_state, todo_overlay_items_as_shared_todos,
    todo_session_unavailable_overlay,
};
use crate::cli::tui_v2::commands::directory::DirectoryCommand;
use crate::cli::tui_v2::commands::inspect::execute_inspect_command;
use crate::cli::tui_v2::commands::integrations::IntegrationCommand;
use crate::cli::tui_v2::commands::registry::CommandSource;
use crate::cli::tui_v2::commands::schedules::ScheduleCommand;
use crate::cli::tui_v2::commands::session::{
    SessionCommand, execute_session_command, session_confirmation_overlay,
};
use crate::cli::tui_v2::commands::settings::{
    SettingsCommand, SettingsRuntime, apply_settings_command, commit_settings,
    validate_settings_values,
};
use crate::cli::tui_v2::completion::sync_skill_completion_catalog;
use crate::cli::tui_v2::message_action_controller::open_message_actions;
use crate::cli::tui_v2::model::agent_view::AgentViewState;
use crate::cli::tui_v2::model::background_task::BackgroundTaskPanel;
use crate::cli::tui_v2::model::integration::{IntegrationKind, IntegrationPanel};
use crate::cli::tui_v2::model::permission::{PermissionDecision, PermissionQueue};
use crate::cli::tui_v2::model::schedule_panel::SchedulePanel;
use crate::cli::tui_v2::model::settings_panel::{SettingsPanel, SettingsValues};
use crate::cli::tui_v2::model::workflow_panel::WorkflowPanel;
use crate::cli::tui_v2::model::{
    MessageActionIntent, MessageActionKind, MessageActionState, UiConfirmIntent,
    UiPermissionOverlay, UiReportOverlay,
};
use crate::cli::tui_v2::paste::apply_prompt_editor_result;
use crate::cli::tui_v2::runtime::agents::AgentsRuntime;
use crate::cli::tui_v2::runtime::integrations::{
    IntegrationRuntime, integration_command_specs, refresh_integrations, switch_skill_provider,
    toggle_selected_integration,
};
use crate::cli::tui_v2::runtime::permission::{
    FullAccessPendingDrain, approve_current_directory_for_full_access, reply_current_permission,
};
use crate::cli::tui_v2::runtime::schedules::ScheduleRuntime;
use crate::cli::tui_v2::runtime::session_store::GatewaySessionCommandRuntime;
use crate::cli::tui_v2::runtime::tasks::refresh_background_tasks;
use crate::cli::tui_v2::runtime::workflows::{WorkflowRuntime, workflow_command_specs};

fn gateway_message_id(message_id: &UiMessageId) -> Result<String, String> {
    message_id.as_str().strip_prefix("gateway:").map(str::to_string).ok_or_else(|| {
        format!("message {} 还没有稳定的 gateway ID，不能执行会话副作用", message_id.as_str())
    })
}

impl TuiApp {
    pub(super) fn handle_command(
        &mut self,
        terminal: &mut TuiTerminal,
        command: TuiControllerCommand,
    ) -> Result<TuiAppCommandOutcome> {
        match command {
            TuiControllerCommand::Continue | TuiControllerCommand::CancelActiveSubmission => {
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiControllerCommand::Quit => Ok(TuiAppCommandOutcome::Quit),
            TuiControllerCommand::Overlay(command) => self.handle_overlay_command(command),
            TuiControllerCommand::ExecuteSlashCommand(invocation) => {
                Ok(match execute_slash_command(&mut self.state, &invocation) {
                    TuiSlashCommandOutcome::Quit => TuiAppCommandOutcome::Quit,
                    TuiSlashCommandOutcome::Continue => TuiAppCommandOutcome::Continue,
                    TuiSlashCommandOutcome::Resume { session_id } => {
                        self.restore_session_from_command(session_id.as_deref());
                        TuiAppCommandOutcome::Continue
                    }
                    TuiSlashCommandOutcome::Inspect { command, argument } => {
                        if let Err(error) =
                            execute_inspect_command(&mut self.state, command, argument.as_deref())
                        {
                            self.push_overlay_error("检查命令失败", error);
                        }
                        TuiAppCommandOutcome::Continue
                    }
                    TuiSlashCommandOutcome::Session(command) => {
                        self.execute_or_confirm_session_command(command, None);
                        TuiAppCommandOutcome::Continue
                    }
                    TuiSlashCommandOutcome::Settings(command) => {
                        self.execute_settings_command(command);
                        TuiAppCommandOutcome::Continue
                    }
                    TuiSlashCommandOutcome::Directory(command) => {
                        self.execute_directory_command(command);
                        TuiAppCommandOutcome::Continue
                    }
                    TuiSlashCommandOutcome::Integration(command) => {
                        self.execute_integration_command(command);
                        TuiAppCommandOutcome::Continue
                    }
                    TuiSlashCommandOutcome::OpenBackgroundTasks => {
                        let mut panel = BackgroundTaskPanel::default();
                        refresh_background_tasks(&mut panel, &mut self.runtime);
                        reduce_tui_state(
                            &mut self.state,
                            TuiAction::OverlayPushed(UiOverlay::BackgroundTasks(panel)),
                        );
                        TuiAppCommandOutcome::Continue
                    }
                    TuiSlashCommandOutcome::OpenAgents => {
                        let mut view = AgentViewState::default();
                        match self.runtime.list_agents() {
                            Ok(members) => {
                                view.viewed_agent_id =
                                    members.first().map(|member| member.id.clone());
                                view.members = members;
                            }
                            Err(error) => view.error = Some(error),
                        }
                        reduce_tui_state(
                            &mut self.state,
                            TuiAction::OverlayPushed(UiOverlay::Agents(view)),
                        );
                        TuiAppCommandOutcome::Continue
                    }
                    TuiSlashCommandOutcome::OpenWorkflows => {
                        let mut panel = WorkflowPanel::default();
                        match self.runtime.list_workflows() {
                            Ok(items) => {
                                let specs = workflow_command_specs(&items);
                                self.state.command_registry = self
                                    .state
                                    .command_registry
                                    .replace_source(CommandSource::Workflow, Ok(specs));
                                panel.items = items;
                            }
                            Err(error) => panel.error = Some(error),
                        }
                        reduce_tui_state(
                            &mut self.state,
                            TuiAction::OverlayPushed(UiOverlay::Workflows(panel)),
                        );
                        TuiAppCommandOutcome::Continue
                    }
                    TuiSlashCommandOutcome::RunWorkflow { uuid, query } => {
                        match self.runtime.run_workflow(uuid.as_str(), query.as_str()) {
                            Ok(summary) => reduce_tui_state(
                                &mut self.state,
                                TuiAction::OverlayPushed(UiOverlay::Report(UiReportOverlay {
                                    title: format!("Workflow {uuid}"),
                                    lines: summary.lines().map(str::to_string).collect(),
                                    capability_notice: Some(
                                        "Workflow 运行与主聊天 submission 相互隔离".to_string(),
                                    ),
                                    scroll_offset: 0,
                                })),
                            ),
                            Err(error) => self.push_overlay_error("Workflow 运行失败", error),
                        }
                        TuiAppCommandOutcome::Continue
                    }
                    TuiSlashCommandOutcome::Schedule(command) => {
                        self.execute_schedule_command(command);
                        TuiAppCommandOutcome::Continue
                    }
                    TuiSlashCommandOutcome::OpenRewindSelector => {
                        if !open_message_actions(&mut self.state, MessageActionKind::Rewind) {
                            self.push_overlay_error(
                                "回退不可用",
                                "当前没有可选择的持久化消息。".to_string(),
                            );
                        }
                        TuiAppCommandOutcome::Continue
                    }
                })
            }
            TuiControllerCommand::TranscriptEffect(effect) => {
                self.handle_transcript_effect(terminal, effect);
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiControllerCommand::MessageAction(intent) => {
                self.handle_message_action_intent(intent);
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiControllerCommand::PromptExternalEditor(intent) => {
                self.handle_prompt_external_editor(terminal, intent);
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiControllerCommand::CopyPromptSelection(text) => {
                Self::copy_prompt_selection_to_writer(
                    &mut self.state,
                    terminal.backend_mut(),
                    text.as_str(),
                );
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiControllerCommand::SubmitPrompt(submission) => {
                self.submit_prompt(terminal, submission)
            }
        }
    }

    pub(crate) fn copy_prompt_selection_to_writer(
        state: &mut super::TuiState,
        writer: &mut impl std::io::Write,
        text: &str,
    ) {
        match crate::cli::tui_v2::clipboard::write_osc52(writer, text) {
            Ok(()) => {
                let message_index = state.messages.len();
                reduce_tui_state(
                    state,
                    TuiAction::MessagePushed(UiMessage::System(UiSystemMessage {
                        base: UiMessageBase::new(UiMessageId::local(format!(
                            "ui-app-system-{message_index}"
                        ))),
                        text: "已复制输入选区。".to_string(),
                        level: UiSystemMessageLevel::Success,
                    })),
                );
            }
            Err(error) => reduce_tui_state(
                state,
                TuiAction::OverlayPushed(UiOverlay::Error(UiErrorOverlay {
                    title: "复制失败".to_string(),
                    message: format!("无法写入终端剪贴板；输入草稿和选区保持不变：{error}"),
                    recoverable: true,
                })),
            ),
        }
    }

    fn execute_or_confirm_session_command(
        &mut self,
        command: SessionCommand,
        confirmation: Option<UiConfirmIntent>,
    ) {
        if confirmation.is_none()
            && let Some(overlay) = session_confirmation_overlay(&command)
        {
            reduce_tui_state(
                &mut self.state,
                TuiAction::OverlayPushed(UiOverlay::Confirm(overlay)),
            );
            return;
        }

        let label = format!("{command:?}");
        let result = {
            let mut runtime = GatewaySessionCommandRuntime::new(&mut self.runtime);
            execute_session_command(&mut self.state, &command, confirmation.as_ref(), &mut runtime)
        };
        match result {
            Ok(()) => {
                self.sync_session_metadata_blocking();
                self.refresh_task_state_blocking();
                self.refresh_project_context_blocking();
                self.push_app_system_message(
                    format!("会话命令已完成: {label}"),
                    UiSystemMessageLevel::Success,
                );
            }
            Err(error) => self.push_overlay_error("会话命令失败", error),
        }
    }

    fn handle_message_action_intent(&mut self, intent: MessageActionIntent) {
        let draft = self.state.message_actions.preserved_prompt_draft.clone();
        self.state.message_actions = MessageActionState::default();
        self.state.prompt.set_value(draft);
        let command = match intent {
            MessageActionIntent::Fork { message_id } => gateway_message_id(&message_id)
                .map(|message_id| SessionCommand::Fork { message_id: Some(message_id) }),
            MessageActionIntent::Summarize { message_id } => gateway_message_id(&message_id)
                .map(|message_id| SessionCommand::Compact { message_id }),
            MessageActionIntent::Rewind { message_id } => gateway_message_id(&message_id)
                .map(|message_id| SessionCommand::Reset { message_id }),
            MessageActionIntent::Copy { .. }
            | MessageActionIntent::Edit { .. }
            | MessageActionIntent::Restore { .. } => return,
        };
        match command {
            Ok(command) => self.execute_or_confirm_session_command(command, None),
            Err(error) => self.push_overlay_error("会话消息操作不可用", error),
        }
    }

    fn handle_prompt_external_editor(
        &mut self,
        terminal: &mut TuiTerminal,
        intent: PromptEditorIntent,
    ) {
        let editor = std::env::var_os("VISUAL").or_else(|| std::env::var_os("EDITOR"));
        let Some(editor) = editor else {
            self.push_overlay_error(
                "外部编辑器不可用",
                "请设置 VISUAL 或 EDITOR；输入草稿保持不变".to_string(),
            );
            return;
        };
        let _ = disable_raw_mode();
        let status = std::process::Command::new(editor).arg(&intent.path).status();
        let _ = enable_raw_mode();
        let _ = terminal.clear();

        let edited = match status {
            Ok(status) if status.success() => std::fs::read_to_string(&intent.path),
            Ok(status) => Err(std::io::Error::other(format!("editor exited with status {status}"))),
            Err(error) => Err(error),
        };
        let result = apply_prompt_editor_result(&mut self.state.prompt, &intent, edited);
        let _ = std::fs::remove_file(&intent.path);
        if let Err(error) = result {
            self.push_overlay_error("外部编辑器失败", format!("输入草稿保持不变：{error}"));
        }
    }

    fn handle_transcript_effect(
        &mut self,
        terminal: &mut TuiTerminal,
        effect: TranscriptEffectIntent,
    ) {
        match effect {
            TranscriptEffectIntent::Dumped { path } => {
                self.push_app_system_message(
                    format!("Transcript 已导出到 {}", path.display()),
                    UiSystemMessageLevel::Success,
                );
            }
            TranscriptEffectIntent::OpenExternalEditor { path } => {
                let editor = std::env::var_os("VISUAL").or_else(|| std::env::var_os("EDITOR"));
                let Some(editor) = editor else {
                    self.push_overlay_error(
                        "外部编辑器不可用",
                        format!("请设置 VISUAL 或 EDITOR；transcript 已保存到 {}", path.display()),
                    );
                    return;
                };
                let _ = disable_raw_mode();
                let result = std::process::Command::new(editor).arg(&path).status();
                let _ = enable_raw_mode();
                let _ = terminal.clear();
                if let Err(error) = result {
                    self.push_overlay_error(
                        "外部编辑器启动失败",
                        format!("{}: {error}", path.display()),
                    );
                }
            }
        }
    }

    pub(super) fn handle_overlay_command(
        &mut self,
        command: TuiOverlayCommand,
    ) -> Result<TuiAppCommandOutcome> {
        match command {
            TuiOverlayCommand::OpenSearchOverlay => {
                self.open_search_overlay();
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::OpenPendingQuestions => {
                if !self.state.prompt.is_busy() {
                    self.refresh_task_state_blocking();
                }
                self.open_pending_question_overlay();
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::OpenTodoPanel => {
                if !self.state.prompt.is_busy() {
                    self.refresh_task_state_blocking();
                }
                self.open_todo_overlay();
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::OpenTaskPanel => {
                if !self.state.prompt.is_busy() {
                    self.refresh_task_state_blocking();
                }
                self.open_task_overlay();
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::OpenMcpPanel => {
                self.open_mcp_overlay();
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::OpenMemoryPanel => {
                self.open_memory_overlay();
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::ConfirmExit => {
                self.open_exit_confirm_overlay();
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::ConfirmAccepted(overlay) => {
                reduce_tui_state(&mut self.state, TuiAction::OverlayPopped);
                match overlay.intent {
                    UiConfirmIntent::ExitTui => return Ok(TuiAppCommandOutcome::Quit),
                    intent @ UiConfirmIntent::ClearSession => {
                        self.execute_or_confirm_session_command(
                            SessionCommand::Clear,
                            Some(intent),
                        );
                    }
                    UiConfirmIntent::ResetSession { message_id } => {
                        let intent =
                            UiConfirmIntent::ResetSession { message_id: message_id.clone() };
                        self.execute_or_confirm_session_command(
                            SessionCommand::Reset { message_id },
                            Some(intent),
                        );
                    }
                    UiConfirmIntent::CompactSession { message_id } => {
                        let intent =
                            UiConfirmIntent::CompactSession { message_id: message_id.clone() };
                        self.execute_or_confirm_session_command(
                            SessionCommand::Compact { message_id },
                            Some(intent),
                        );
                    }
                    UiConfirmIntent::IntegrationDelete { kind, id } => {
                        self.delete_integration(kind, id.as_str());
                    }
                    UiConfirmIntent::ScheduleDelete { id } => {
                        self.delete_schedule(id.as_str());
                    }
                    UiConfirmIntent::EnableFullAccess { values } => {
                        self.enable_full_access_after_confirmation(*values);
                    }
                    UiConfirmIntent::Generic => {}
                }
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::QuestionSubmitted(overlay) => {
                let answers = question_overlay_submission_answers(&overlay);
                if answers.iter().all(Vec::is_empty) {
                    self.push_overlay_error(
                        overlay.empty_submission_title(),
                        overlay.empty_submission_message(),
                    );
                    return Ok(TuiAppCommandOutcome::Continue);
                }

                match self.runtime.question_reply_blocking(overlay.request_id.as_str(), answers) {
                    Ok(()) => {
                        reduce_tui_state(&mut self.state, TuiAction::OverlayPopped);
                        self.refresh_task_state_blocking();
                        self.push_app_system_message(
                            format!(
                                "{} {} 已提交回答",
                                overlay.request_label(),
                                overlay.request_id
                            ),
                            UiSystemMessageLevel::Success,
                        );
                    }
                    Err(err) => {
                        self.push_overlay_error(
                            overlay.reply_error_title(),
                            format!("问题回复失败: {err}"),
                        );
                    }
                }

                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::QuestionRejected(overlay) => {
                match self.runtime.question_reject_blocking(overlay.request_id.as_str()) {
                    Ok(()) => {
                        reduce_tui_state(&mut self.state, TuiAction::OverlayPopped);
                        self.refresh_task_state_blocking();
                        self.push_app_system_message(
                            format!("{} {} 已拒绝", overlay.request_label(), overlay.request_id),
                            UiSystemMessageLevel::Warning,
                        );
                    }
                    Err(err) => {
                        self.push_overlay_error(
                            overlay.reject_error_title(),
                            format!("问题拒绝失败: {err}"),
                        );
                    }
                }

                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::TodoRefresh(_) => {
                if self.current_session_id().is_none() {
                    self.push_error_overlay(todo_session_unavailable_overlay(
                        TodoSessionAccessAction::Refresh,
                    ));
                    return Ok(TuiAppCommandOutcome::Continue);
                }

                if !self.state.prompt.is_busy() {
                    self.refresh_task_state_blocking();
                }
                self.reload_active_todo_overlay();
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::TodoSave(overlay) => {
                if self.current_session_id().is_none() {
                    self.push_error_overlay(todo_session_unavailable_overlay(
                        TodoSessionAccessAction::Save,
                    ));
                    return Ok(TuiAppCommandOutcome::Continue);
                }

                let todos = todo_overlay_items_as_shared_todos(&overlay);
                match self
                    .runtime
                    .session_todo_update_blocking(overlay.session_id.as_deref(), &todos)
                {
                    Ok(()) => {
                        self.refresh_task_state_blocking();
                        self.reload_active_todo_overlay();
                        self.push_app_system_message(
                            "待办面板已保存",
                            UiSystemMessageLevel::Success,
                        );
                    }
                    Err(err) => {
                        self.push_overlay_error("待办保存失败", format!("待办更新失败: {err}"));
                    }
                }

                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::SettingsSave(mut panel) => {
                if let Some(confirmation) = full_access_confirmation(
                    &panel,
                    self.state.session_options.full_access_enabled(),
                ) {
                    reduce_tui_state(&mut self.state, TuiAction::OverlayPopped);
                    reduce_tui_state(
                        &mut self.state,
                        TuiAction::OverlayPushed(UiOverlay::Confirm(confirmation)),
                    );
                    return Ok(TuiAppCommandOutcome::Continue);
                }
                let capabilities = self.state.session_capabilities.clone();
                if let Err(error) =
                    validate_settings_values(&panel.pending, &panel.original, &capabilities, false)
                        .and_then(|()| commit_settings(&mut panel, &mut self.runtime))
                {
                    self.push_overlay_error("设置保存失败", error);
                } else {
                    reduce_tui_state(&mut self.state, TuiAction::OverlayPopped);
                    self.apply_settings_values(&panel.original);
                    self.push_app_system_message(
                        "当前会话设置已更新。".to_string(),
                        UiSystemMessageLevel::Success,
                    );
                }
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::PermissionDecision(decision) => {
                self.reply_permission(decision);
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::IntegrationRefresh(mut panel) => {
                refresh_integrations(&mut panel, &mut self.runtime);
                self.update_integration_commands(&panel);
                if let Some(UiOverlay::Integration(current)) = self.state.overlays.stack.last_mut()
                {
                    *current = panel;
                }
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::IntegrationSkillProviderSelected(mut panel, provider) => {
                match switch_skill_provider(&mut panel, &mut self.runtime, provider) {
                    Ok(()) => {
                        self.update_integration_commands(&panel);
                        if let Some(UiOverlay::Integration(current)) =
                            self.state.overlays.stack.last_mut()
                        {
                            *current = panel;
                        }
                        self.push_app_system_message(
                            format!("技能平台已切换到 {}。", provider.label()),
                            UiSystemMessageLevel::Success,
                        );
                    }
                    Err(error) => self.push_overlay_error("技能平台切换失败", error),
                }
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::IntegrationToggle(mut panel) => {
                match toggle_selected_integration(&panel, &mut self.runtime) {
                    Ok(true) => {
                        refresh_integrations(&mut panel, &mut self.runtime);
                        self.update_integration_commands(&panel);
                        if let Some(UiOverlay::Integration(current)) =
                            self.state.overlays.stack.last_mut()
                        {
                            *current = panel;
                        }
                    }
                    Ok(false) => {}
                    Err(error) => self.push_overlay_error("集成启停失败", error),
                }
                Ok(TuiAppCommandOutcome::Continue)
            }
            TuiOverlayCommand::IntegrationDeleteRequested(panel) => {
                let Some(item) = panel.items.get(panel.selected) else {
                    return Ok(TuiAppCommandOutcome::Continue);
                };
                reduce_tui_state(
                    &mut self.state,
                    TuiAction::OverlayPushed(UiOverlay::Confirm(UiConfirmOverlay {
                        title: "删除集成".to_string(),
                        body: format!("确认删除 {} 吗？此操作不可撤销。", item.title),
                        confirm_label: "删除".to_string(),
                        cancel_label: "取消".to_string(),
                        destructive: true,
                        intent: UiConfirmIntent::IntegrationDelete {
                            kind: panel.kind,
                            id: item.id.clone(),
                        },
                    })),
                );
                Ok(TuiAppCommandOutcome::Continue)
            }
        }
    }

    fn current_settings_values(&self) -> SettingsValues {
        SettingsValues {
            model: self
                .state
                .session_options
                .model
                .clone()
                .or_else(|| self.state.status.model_name.clone()),
            delegate_agent: self.state.session_options.delegate_agent.clone(),
            acp_backend: self.state.session_options.acp_backend.clone(),
            full_access_enabled: self.state.session_options.full_access_enabled(),
            subagent_mode_enabled: self.state.session_options.subagent_mode_enabled(),
            workflow_mode: if self.state.session_options.workflow_mode_enabled() {
                crate::cli::tui_v2::model::session_mode::WorkflowMode::Enabled
            } else {
                crate::cli::tui_v2::model::session_mode::WorkflowMode::Disabled
            },
            goal_mode: crate::cli::tui_v2::model::session_mode::GoalMode::Enabled,
            effort: self.state.session_options.effort,
            fast: self.state.session_options.fast,
            theme: if self.state.theme_name.trim().is_empty() {
                "Dark".to_string()
            } else {
                self.state.theme_name.clone()
            },
            permission_mode: self.state.session_options.permission_mode,
            vim_mode: self.state.prompt.vim_mode,
        }
    }

    fn execute_settings_command(&mut self, command: SettingsCommand) {
        match command {
            SettingsCommand::Open => {
                let panel = SettingsPanel::for_capabilities(
                    self.current_settings_values(),
                    &self.state.session_capabilities,
                );
                reduce_tui_state(
                    &mut self.state,
                    TuiAction::OverlayPushed(UiOverlay::Settings(panel)),
                );
            }
            SettingsCommand::OpenModelPicker => {
                let mut panel = SettingsPanel::for_capabilities(
                    self.current_settings_values(),
                    &self.state.session_capabilities,
                );
                if panel.open_model_picker() {
                    reduce_tui_state(
                        &mut self.state,
                        TuiAction::OverlayPushed(UiOverlay::Settings(panel)),
                    );
                } else {
                    self.push_overlay_error("模型选择失败", "当前没有发现可选模型");
                }
            }
            SettingsCommand::OpenAcpBackendPicker => {
                let mut panel = SettingsPanel::for_capabilities(
                    self.current_settings_values(),
                    &self.state.session_capabilities,
                );
                if panel.open_acp_backend_picker() {
                    reduce_tui_state(
                        &mut self.state,
                        TuiAction::OverlayPushed(UiOverlay::Settings(panel)),
                    );
                } else {
                    self.push_overlay_error("ACP 后端选择失败", "当前没有发现已启用 ACP 后端");
                }
            }
            SettingsCommand::Permissions => self.open_permission_overlay(),
            command => {
                let mut panel = SettingsPanel::for_capabilities(
                    self.current_settings_values(),
                    &self.state.session_capabilities,
                );
                let capabilities = self.state.session_capabilities.clone();
                match apply_settings_command(
                    &mut panel,
                    &command,
                    &capabilities,
                    false,
                    &mut self.runtime,
                ) {
                    Ok(()) => {
                        self.apply_settings_values(&panel.original);
                        self.push_app_system_message(
                            "当前会话设置已更新。".to_string(),
                            UiSystemMessageLevel::Success,
                        );
                    }
                    Err(error) => self.push_overlay_error("设置命令失败", error),
                }
            }
        }
    }

    fn apply_settings_values(&mut self, values: &SettingsValues) {
        apply_settings_values_to_state(&mut self.state, values);
        if self.renderer.set_theme_name(values.theme.as_str()).is_ok() {
            self.state.theme_name.clone_from(&values.theme);
        }
    }

    fn enable_full_access_after_confirmation(&mut self, values: SettingsValues) {
        let current = self.current_settings_values();
        let capabilities = self.state.session_capabilities.clone();
        if let Err(error) = validate_settings_values(&values, &current, &capabilities, false) {
            self.push_overlay_error("完全访问启用失败", error);
            return;
        }
        let mut panel = SettingsPanel::for_capabilities(current, &capabilities);
        *panel.pending = values;
        let drain = match Self::commit_full_access_and_drain(&mut panel, &mut self.runtime) {
            Ok(drain) => drain,
            Err(error) => {
                self.push_overlay_error("完全访问启用失败", error);
                return;
            }
        };
        self.apply_settings_values(&panel.original);
        let warning = drain
            .warning
            .as_deref()
            .map(|warning| format!(" {warning}；完全访问仍保持启用。"))
            .unwrap_or_default();
        self.push_app_system_message(
            format!(
                "当前会话已启用完全访问；批准了 {} 个当前目录待处理请求，{} 个失败。{warning}",
                drain.approved, drain.failed
            ),
            UiSystemMessageLevel::Warning,
        );
    }

    pub(crate) fn commit_full_access_and_drain<R>(
        panel: &mut SettingsPanel,
        runtime: &mut R,
    ) -> Result<FullAccessPendingDrain, String>
    where
        R: SettingsRuntime + crate::cli::tui_v2::runtime::permission::FullAccessPermissionRuntime,
    {
        commit_settings(panel, runtime)?;
        Ok(approve_current_directory_for_full_access(runtime))
    }

    fn open_permission_overlay(&mut self) {
        match self.runtime.permission_list_for_directory_blocking() {
            Ok(requests) => {
                let mut queue = PermissionQueue::default();
                queue.replace(requests);
                reduce_tui_state(
                    &mut self.state,
                    TuiAction::OverlayPushed(UiOverlay::Permissions(UiPermissionOverlay {
                        queue,
                        notice: None,
                    })),
                );
            }
            Err(error) => self.push_overlay_error("权限请求加载失败", error),
        }
    }

    fn reply_permission(&mut self, decision: PermissionDecision) {
        let Some(UiOverlay::Permissions(mut overlay)) = self.state.overlays.active().cloned()
        else {
            return;
        };
        match reply_current_permission(&mut overlay.queue, decision, &mut self.runtime) {
            Ok(()) => {
                reduce_tui_state(&mut self.state, TuiAction::OverlayPopped);
                if overlay.queue.len() > 0 {
                    reduce_tui_state(
                        &mut self.state,
                        TuiAction::OverlayPushed(UiOverlay::Permissions(overlay)),
                    );
                } else {
                    self.push_app_system_message(
                        "权限请求已回复。".to_string(),
                        UiSystemMessageLevel::Success,
                    );
                }
            }
            Err(error) => {
                if let Some(UiOverlay::Permissions(current)) = self.state.overlays.stack.last_mut()
                {
                    current.notice = Some(error);
                }
            }
        }
    }

    fn execute_directory_command(&mut self, command: DirectoryCommand) {
        match command {
            DirectoryCommand::Project => {
                match self
                    .runtime
                    .resolve_project_directory_blocking(self.runtime.directory(), false)
                {
                    Ok(project) => reduce_tui_state(
                        &mut self.state,
                        TuiAction::OverlayPushed(UiOverlay::Report(UiReportOverlay {
                            title: "当前项目".to_string(),
                            lines: vec![
                                format!("ID: {}", project.id.as_ref()),
                                format!("名称: {}", project.name),
                                format!("目录: {}", project.directory),
                                format!("显示路径: {}", project.display_path),
                                format!("状态: {:?}", project.status),
                                format!(
                                    "Git: 仓库={} 未提交={} 分支={}",
                                    project.git.is_repo,
                                    project.git.has_uncommitted_changes,
                                    project.current_branch.as_deref().unwrap_or("-")
                                ),
                            ],
                            capability_notice: None,
                            scroll_offset: 0,
                        })),
                    ),
                    Err(error) => self.push_overlay_error("项目解析失败", error),
                }
            }
            DirectoryCommand::Change(path) => self.change_directory(path),
        }
    }

    fn change_directory(&mut self, path: std::path::PathBuf) {
        let target = if path.is_absolute() { path } else { self.runtime.directory().join(path) };
        let canonical = match target.canonicalize() {
            Ok(path) if path.is_dir() => path,
            Ok(path) => {
                self.push_overlay_error(
                    "目录切换失败",
                    format!("目标不是目录: {}", path.display()),
                );
                return;
            }
            Err(error) => {
                self.push_overlay_error(
                    "目录切换失败",
                    format!("无法解析 {}: {error}", target.display()),
                );
                return;
            }
        };
        let project = match self.runtime.resolve_project_directory_blocking(&canonical, true) {
            Ok(project) => project,
            Err(error) => {
                self.push_overlay_error("目录切换失败", format!("Gateway 项目解析失败: {error}"));
                return;
            }
        };
        if let Some(session_id) = self.state.session.session_id.clone() {
            let mut snapshot = self.state.to_chat_session();
            if snapshot.id.trim().is_empty() {
                snapshot.id = session_id;
            }
            if let Err(error) = self.runtime.session_ui_save_blocking(&snapshot) {
                self.push_overlay_error(
                    "目录切换失败",
                    format!("保存当前会话失败，目录保持不变: {error}"),
                );
                return;
            }
        }

        let previous_directory = self.runtime.directory().to_path_buf();
        if let Err(error) = self.runtime.rebind_validated_directory(&canonical) {
            self.push_overlay_error("目录切换失败", error);
            return;
        }
        let title = format!("TUI {}", project.name);
        let created = match self.runtime.session_create_blocking(Some(title.as_str())) {
            Ok(created) => created,
            Err(error) => {
                let _ = self.runtime.rebind_validated_directory(&previous_directory);
                self.push_overlay_error(
                    "目录切换失败",
                    format!("新项目会话创建失败，已恢复原目录: {error}"),
                );
                return;
            }
        };

        let draft = self.state.prompt.value.clone();
        let registry = self.state.command_registry.clone();
        let model_catalog = self.state.model_catalog.clone();
        let capabilities = self.state.session_capabilities.clone();
        let options = self.state.session_options.clone();
        let theme_name = self.state.theme_name.clone();
        let mut snapshot = super::empty_session_snapshot(created.id.as_str());
        snapshot.title = created.title;
        reduce_tui_state(
            &mut self.state,
            TuiAction::ReplaceFromSnapshot {
                snapshot,
                scope: Some(project.id.as_ref().to_string()),
                path: None,
            },
        );
        self.state.command_registry = registry;
        self.state.model_catalog = model_catalog;
        self.state.session_capabilities = capabilities;
        self.state.session_options = options;
        self.state.theme_name = theme_name;
        self.state.prompt.set_value(draft);
        self.sync_runtime_session_seed_from_state();
        self.refresh_project_context_blocking();
        self.refresh_task_state_blocking();
        self.push_app_system_message(
            format!("已切换项目目录: {}", canonical.display()),
            UiSystemMessageLevel::Success,
        );
    }

    fn execute_integration_command(&mut self, command: IntegrationCommand) {
        if command.kind() == IntegrationKind::Memory {
            self.open_memory_overlay();
            return;
        }
        let mut panel = IntegrationPanel::new(command.kind());
        refresh_integrations(&mut panel, &mut self.runtime);
        self.update_integration_commands(&panel);
        if command.refresh() {
            self.push_app_system_message(
                format!("{:?} 已刷新。", command.kind()),
                UiSystemMessageLevel::Success,
            );
        }
        reduce_tui_state(&mut self.state, TuiAction::OverlayPushed(UiOverlay::Integration(panel)));
    }

    pub(super) fn update_integration_commands(&mut self, panel: &IntegrationPanel) {
        sync_skill_completion_catalog(&mut self.state.completion_catalog, panel);
        if matches!(
            panel.state,
            crate::cli::tui_v2::model::integration::IntegrationLoadState::Error(_)
        ) {
            return;
        }
        let source = match panel.kind {
            IntegrationKind::Skill => Some(CommandSource::Skill),
            IntegrationKind::Mcp => Some(CommandSource::Mcp),
            IntegrationKind::Plugin => Some(CommandSource::Plugin),
            IntegrationKind::Hook | IntegrationKind::Memory | IntegrationKind::Doctor => None,
        };
        let Some(source) = source else {
            return;
        };
        let specs = integration_command_specs(panel.kind, &panel.items);
        self.state.command_registry = self.state.command_registry.replace_source(source, Ok(specs));
    }

    fn delete_integration(&mut self, kind: IntegrationKind, id: &str) {
        match self.runtime.delete(kind, id) {
            Ok(()) => {
                let Some(UiOverlay::Integration(mut panel)) = self.state.overlays.active().cloned()
                else {
                    return;
                };
                refresh_integrations(&mut panel, &mut self.runtime);
                self.update_integration_commands(&panel);
                if let Some(UiOverlay::Integration(current)) = self.state.overlays.stack.last_mut()
                {
                    *current = panel;
                }
                self.push_app_system_message(
                    format!("集成 {id} 已删除。"),
                    UiSystemMessageLevel::Success,
                );
            }
            Err(error) => self.push_overlay_error("集成删除失败", error),
        }
    }

    fn execute_schedule_command(&mut self, command: ScheduleCommand) {
        match command {
            ScheduleCommand::Open => self.open_schedule_panel(),
            ScheduleCommand::Enable(id) => self.set_schedule_enabled(id.as_str(), true),
            ScheduleCommand::Disable(id) => self.set_schedule_enabled(id.as_str(), false),
            ScheduleCommand::Delete(id) => reduce_tui_state(
                &mut self.state,
                TuiAction::OverlayPushed(UiOverlay::Confirm(UiConfirmOverlay {
                    title: "删除定时任务".to_string(),
                    body: format!("确认删除 Cron 任务 {id} 吗？此操作不可撤销。"),
                    confirm_label: "删除".to_string(),
                    cancel_label: "取消".to_string(),
                    destructive: true,
                    intent: UiConfirmIntent::ScheduleDelete { id },
                })),
            ),
        }
    }

    fn open_schedule_panel(&mut self) {
        let mut panel = SchedulePanel::default();
        match self.runtime.list_schedules() {
            Ok(items) => panel.items = items,
            Err(error) => panel.error = Some(error),
        }
        reduce_tui_state(&mut self.state, TuiAction::OverlayPushed(UiOverlay::Schedules(panel)));
    }

    fn set_schedule_enabled(&mut self, id: &str, enabled: bool) {
        match self.runtime.set_schedule_enabled(id, enabled) {
            Ok(()) => {
                self.push_app_system_message(
                    format!("Cron 任务 {id} 已{}。", if enabled { "启用" } else { "停用" }),
                    UiSystemMessageLevel::Success,
                );
                self.open_schedule_panel();
            }
            Err(error) => self.push_overlay_error("Cron 更新失败", error),
        }
    }

    fn delete_schedule(&mut self, id: &str) {
        match self.runtime.delete_schedule(id) {
            Ok(()) => {
                self.push_app_system_message(
                    format!("Cron 任务 {id} 已删除。"),
                    UiSystemMessageLevel::Success,
                );
                self.open_schedule_panel();
            }
            Err(error) => self.push_overlay_error("Cron 删除失败", error),
        }
    }

    pub(super) fn refresh_task_state_blocking(&mut self) {
        let question_session_id = self
            .state
            .session
            .session_id
            .clone()
            .or_else(|| self.runtime.session_id().map(ToOwned::to_owned));
        let todo_session_id = self
            .state
            .session
            .session_id
            .clone()
            .or_else(|| self.runtime.session_id().map(ToOwned::to_owned));
        let mut errors = Vec::new();

        if let Some(session_id) = question_session_id.as_deref() {
            match self.runtime.question_list_for_session_blocking(Some(session_id)) {
                Ok(requests) => {
                    let overlays =
                        requests.iter().map(UiQuestionOverlay::from_request).collect::<Vec<_>>();
                    reduce_tui_state(&mut self.state, TuiAction::QuestionsReplaced(overlays));
                }
                Err(err) => {
                    reduce_tui_state(&mut self.state, TuiAction::QuestionsReplaced(Vec::new()));
                    errors.push(format!("问题同步失败: {err}"));
                }
            }
        } else {
            reduce_tui_state(&mut self.state, TuiAction::QuestionsReplaced(Vec::new()));
        }

        if let Some(session_id) = todo_session_id.as_deref() {
            match self.runtime.session_todo_get_blocking(Some(session_id)) {
                Ok(todos) => {
                    let overlay = UiTodoOverlay::from_todos(Some(session_id), &todos);
                    reduce_tui_state(
                        &mut self.state,
                        TuiAction::TodoOverlayReplaced(Some(overlay)),
                    );
                }
                Err(err) => {
                    reduce_tui_state(&mut self.state, TuiAction::TodoOverlayReplaced(None));
                    errors.push(format!("待办同步失败: {err}"));
                }
            }
        } else {
            reduce_tui_state(&mut self.state, TuiAction::TodoOverlayReplaced(None));
        }

        reduce_tui_state(
            &mut self.state,
            TuiAction::TaskSyncErrorSet((!errors.is_empty()).then(|| errors.join("; "))),
        );
        self.reload_active_todo_overlay();
        self.reload_active_task_overlay();
    }

    pub(super) fn refresh_project_context_blocking(&mut self) {
        let workspace_root = self.runtime.directory().to_path_buf();
        let project_info = build_project_info(&workspace_root);
        let git_status = collect_git_workspace_status(&workspace_root);

        reduce_tui_state(&mut self.state, TuiAction::ProjectWorkspaceRootSet(Some(workspace_root)));
        self.refresh_completion_file_index_blocking();
        reduce_tui_state(&mut self.state, TuiAction::ProjectInfoSet(project_info));
        reduce_tui_state(&mut self.state, TuiAction::ProjectGitStatusSet(git_status));
    }

    fn open_search_overlay(&mut self) {
        reduce_tui_state(&mut self.state, TuiAction::SearchQuerySet(String::new()));
    }

    fn open_pending_question_overlay(&mut self) {
        if let Some(error) = self.state.tasks.sync_error.clone() {
            self.push_overlay_error(
                "问题面板不可用",
                format!("由于任务同步失败，当前无法读取待处理问题列表。\n{error}"),
            );
            return;
        }

        let Some(question) = self.state.tasks.pending_questions.first().cloned() else {
            self.push_overlay_error("问题面板", "当前会话没有待处理问题。");
            return;
        };

        reduce_tui_state(&mut self.state, TuiAction::OverlayPushed(UiOverlay::Question(question)));
    }

    fn open_todo_overlay(&mut self) {
        if self.current_session_id().is_none() {
            self.push_error_overlay(todo_session_unavailable_overlay(
                TodoSessionAccessAction::OpenPanel,
            ));
            return;
        }

        if let Some(error) = self.state.tasks.sync_error.clone() {
            let overlay = if is_session_unavailable_error(error.as_str()) {
                todo_session_unavailable_overlay(TodoSessionAccessAction::OpenPanel)
            } else {
                UiErrorOverlay {
                    title: "待办面板不可用".to_string(),
                    message: format!("由于任务同步失败，当前无法打开待办面板。\n{error}"),
                    recoverable: true,
                }
            };
            self.push_error_overlay(overlay);
            return;
        }

        let Some(todo_overlay) = self.state.tasks.todo_overlay.clone() else {
            self.push_overlay_error("待办面板", "当前会话没有可用的待办列表。");
            return;
        };

        reduce_tui_state(&mut self.state, TuiAction::OverlayPushed(UiOverlay::Todo(todo_overlay)));
    }

    fn open_task_overlay(&mut self) {
        let task_overlay = self.build_task_overlay();
        reduce_tui_state(&mut self.state, TuiAction::OverlayPushed(UiOverlay::Task(task_overlay)));
    }

    fn open_mcp_overlay(&mut self) {
        let overlay = build_mcp_overlay(self.state.project.workspace_root.as_deref());
        reduce_tui_state(&mut self.state, TuiAction::OverlayPushed(UiOverlay::Mcp(overlay)));
    }

    fn open_memory_overlay(&mut self) {
        let overlay = build_memory_overlay(self.state.project.workspace_root.as_deref());
        reduce_tui_state(&mut self.state, TuiAction::OverlayPushed(UiOverlay::Memory(overlay)));
    }

    fn open_exit_confirm_overlay(&mut self) {
        // 若已有退出确认弹层（同 kind），不重复叠加
        if self.state.overlays.active().is_some_and(|o| {
            matches!(
                o,
                UiOverlay::Confirm(c) if c.intent == UiConfirmIntent::ExitTui
            )
        }) {
            return;
        }

        let body = build_exit_confirm_body(&self.state);
        reduce_tui_state(
            &mut self.state,
            TuiAction::OverlayPushed(UiOverlay::Confirm(UiConfirmOverlay {
                title: "退出 TUI".to_string(),
                body,
                confirm_label: "退出".to_string(),
                cancel_label: "继续留在这里".to_string(),
                destructive: false,
                intent: UiConfirmIntent::ExitTui,
            })),
        );
    }

    fn build_task_overlay(&self) -> UiTaskOverlay {
        let steps = self
            .state
            .messages
            .iter()
            .filter_map(|message| match message {
                UiMessage::Step(step) => Some(UiTaskStepItem {
                    message_id: step.base.id.clone(),
                    step_index: step.step_index,
                    state: step.state.clone(),
                    started_ms: step.started_ms,
                    finished_ms: step.finished_ms,
                    model: step.model.clone(),
                    finish_reason: step.finish_reason.clone(),
                    usage: step.usage.clone(),
                }),
                _ => None,
            })
            .collect::<Vec<_>>();

        UiTaskOverlay {
            session_id: self.state.session.session_id.clone(),
            turn_terminal: self.state.status.turn_terminal.clone(),
            pending_questions: self.state.tasks.pending_questions.len(),
            todo_count: self
                .state
                .tasks
                .todo_overlay
                .as_ref()
                .map(|overlay| overlay.items.len())
                .unwrap_or_default(),
            sync_error: self.state.tasks.sync_error.clone(),
            selected_index: steps.len().saturating_sub(1),
            steps,
        }
    }

    fn reload_active_todo_overlay(&mut self) {
        if !matches!(self.state.overlays.active(), Some(UiOverlay::Todo(_))) {
            return;
        }

        let has_session = self.current_session_id().is_some();

        if let Some(error) = self.state.tasks.sync_error.clone() {
            let overlay = if !has_session || is_session_unavailable_error(error.as_str()) {
                todo_session_unavailable_overlay(TodoSessionAccessAction::OpenPanel)
            } else {
                UiErrorOverlay {
                    title: "待办面板不可用".to_string(),
                    message: format!("由于任务同步失败，当前无法打开待办面板。\n{error}"),
                    recoverable: true,
                }
            };

            if let Some(active_overlay) = self.state.overlays.stack.last_mut() {
                *active_overlay = UiOverlay::Error(overlay);
            }
            return;
        }

        let Some(todo_overlay) = self.state.tasks.todo_overlay.clone() else {
            let overlay = if has_session {
                UiErrorOverlay {
                    title: "待办面板".to_string(),
                    message: "当前会话没有可用的待办列表。".to_string(),
                    recoverable: true,
                }
            } else {
                todo_session_unavailable_overlay(TodoSessionAccessAction::OpenPanel)
            };

            if let Some(active_overlay) = self.state.overlays.stack.last_mut() {
                *active_overlay = UiOverlay::Error(overlay);
            }
            return;
        };

        if let Some(UiOverlay::Todo(active_overlay)) = self.state.overlays.stack.last_mut() {
            *active_overlay = todo_overlay;
        }
    }

    pub(super) fn reload_active_task_overlay(&mut self) {
        let next_overlay = self.build_task_overlay();

        if let Some(UiOverlay::Task(active_overlay)) = self.state.overlays.stack.last_mut() {
            let selected_index =
                active_overlay.selected_index.min(next_overlay.steps.len().saturating_sub(1));
            *active_overlay = next_overlay;
            active_overlay.selected_index = selected_index;
        }
    }

    pub(super) fn current_session_id(&self) -> Option<&str> {
        self.state.session.session_id.as_deref().or_else(|| self.runtime.session_id())
    }

    pub(super) fn push_error_overlay(&mut self, overlay: UiErrorOverlay) {
        if matches!(self.state.overlays.active(), Some(UiOverlay::Error(active)) if active == &overlay)
        {
            return;
        }

        reduce_tui_state(&mut self.state, TuiAction::OverlayPushed(UiOverlay::Error(overlay)));
    }

    pub(super) fn push_overlay_error(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.push_error_overlay(UiErrorOverlay {
            title: title.into(),
            message: message.into(),
            recoverable: true,
        });
    }

    pub(super) fn push_app_system_message(
        &mut self,
        text: impl Into<String>,
        level: UiSystemMessageLevel,
    ) {
        let message_index = self.state.messages.len();
        reduce_tui_state(
            &mut self.state,
            TuiAction::MessagePushed(UiMessage::System(UiSystemMessage {
                base: UiMessageBase::new(UiMessageId::local(format!(
                    "ui-app-system-{}",
                    message_index
                ))),
                text: text.into(),
                level,
            })),
        );
    }
}

pub(crate) fn apply_settings_values_to_state(state: &mut super::TuiState, values: &SettingsValues) {
    state.session_options.model = values.model.clone();
    state.session_options.effort = values.effort;
    state.session_options.fast = values.fast;
    state.session_options.permission_mode = values.permission_mode;
    state.session_options.delegate_agent.clone_from(&values.delegate_agent);
    state.session_options.select_acp_backend(values.acp_backend.clone());
    state.session_options.set_full_access(values.full_access_enabled);
    state.session_options.set_subagent_mode(values.subagent_mode_enabled);
    state.session_options.set_workflow_mode(
        values.workflow_mode == crate::cli::tui_v2::model::session_mode::WorkflowMode::Enabled,
    );
    state.session_options.set_goal_mode(true);
    state.prompt.vim_mode = values.vim_mode;
    if let Some(model) = values.model.clone() {
        reduce_tui_state(
            state,
            TuiAction::StatusProviderSet(super::provider_name_from_model(model.as_str())),
        );
        reduce_tui_state(state, TuiAction::StatusModelSet(Some(model)));
    }
}

pub(crate) fn full_access_confirmation(
    panel: &SettingsPanel,
    full_access_enabled: bool,
) -> Option<UiConfirmOverlay> {
    (panel.pending.full_access_enabled && !full_access_enabled).then(|| UiConfirmOverlay {
        title: "启用完全访问".to_string(),
        body: "完全访问会允许当前 TUI 会话绕过普通工具审批，并自动批准当前目录已有的待处理请求。确认继续？"
            .to_string(),
        confirm_label: "启用".to_string(),
        cancel_label: "取消".to_string(),
        destructive: true,
        intent: UiConfirmIntent::EnableFullAccess { values: panel.pending.clone() },
    })
}
