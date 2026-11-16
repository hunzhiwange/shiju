//! 运行器命令分发。

use super::{
    ActiveRunner,
    ActiveStreamKind,
    CommandRoute,
    Ordering,
    RunnerControl,
    SESSION_COMPACT_SURFACE,
    SESSION_RESET_SURFACE,
    TuiControllerCommand,
    UiRuntimeEvent,
    UiRuntimeTerminalEvent,
    V3Action,
    V3Attachment,
    V3DisplayEvent,
    V3GatewayApp,
    help_text,
    load_owned_workflow_detail_blocking,
    next_stream_request_id,
    push_runner_error,
    spawn_plan_run_stop,
    spawn_stop,
    spawn_stream,
    terminal_from_error_message,
};

impl ActiveRunner {
    pub(super) fn execute_command(
        &mut self,
        app: &mut V3GatewayApp,
        command: TuiControllerCommand,
        recovery: Option<(String, Vec<V3Attachment>)>,
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
                            app.accept_runtime_event(UiRuntimeEvent::Terminal(
                                terminal_from_error_message(
                                    "cancelled by user".to_string(),
                                ),
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
                CommandRoute::CopyAnswer { code_block } => {
                    let result = if self.active_stream.is_some()
                        || self.session_bootstrap_in_flight
                        || self.directory_switch_source.is_some()
                    {
                        Err("回答生成或会话切换尚未完成，请稍后复制".to_string())
                    } else {
                        super::semantic_copy::answer_text(app.state(), code_block)
                    };
                    match result.and_then(|text| app.widget_mut().copy_text(&text)) {
                        Ok(()) => app.widget_mut().push_history(V3DisplayEvent::System {
                            text: "已复制到剪贴板".to_string(),
                        }),
                        Err(error) => push_runner_error(app, error),
                    }
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
            TuiControllerCommand::SubmitPrompt(ref submission) => {
                if self.active_stream.is_some() {
                    push_runner_error(
                        app,
                        "a gateway stream is already active; queue the follow-up instead",
                    );
                    return RunnerControl::Continue;
                }
                let Some(mut request) = app.request_for_command(&command) else {
                    push_runner_error(app, "TUI submission did not produce a request");
                    return RunnerControl::Continue;
                };
                app.record_submission_attachments(submission);
                if app.pending_first_response
                    && let Some(current_prompt) = request.messages.pop()
                {
                    request.messages = vec![current_prompt];
                }
                let request_id = next_stream_request_id();
                self.chat_submission = recovery;
                self.active_stream = Some(spawn_stream(
                    app.runtime().clone(),
                    request,
                    request_id,
                    app.pending_first_response.then(|| submission.text.clone()),
                ));
                RunnerControl::Continue
            }
        }
    }
}
