//! 运行器提交与审批操作。

#[cfg(all(not(test), not(target_arch = "wasm32")))]
use super::{
    ActiveRunner,
    CronMutation,
    MemoryMutation,
    PathBuf,
    PendingPermissionReplyDto,
    PermissionDecision,
    Result,
    RunnerControl,
    TuiOverlayCommand,
    UiConfirmIntent,
    V3DisplayEvent,
    V3GatewayApp,
    WorkflowRunOwnership,
    new_draft_id,
    next_plan_run_request_id,
    next_workflow_run_request_id,
    push_runner_error,
    spawn_directory_switch,
    spawn_permission_reply,
    spawn_plan_run,
    spawn_question_reply,
    spawn_workflow_resume,
    spawn_workflow_run,
    workflow_run_request,
};
#[cfg(any(test, target_arch = "wasm32"))]
use super::{
    ActiveRunner,
    CronMutation,
    MemoryMutation,
    PathBuf,
    PendingPermissionReplyDto,
    PermissionDecision,
    Result,
    RunnerControl,
    TuiOverlayCommand,
    UiConfirmIntent,
    V3DisplayEvent,
    V3GatewayApp,
    WorkflowRunOwnership,
    new_draft_id,
    next_plan_run_request_id,
    next_workflow_run_request_id,
    plan_run_request,
    push_runner_error,
    spawn_directory_switch,
    spawn_permission_reply,
    spawn_plan_run,
    spawn_question_reply,
    spawn_workflow_resume,
    spawn_workflow_run,
    workflow_run_request,
};

impl ActiveRunner {
    pub(super) fn start_plan_run(
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

    pub(super) fn start_workflow_run(
        &mut self,
        app: &mut V3GatewayApp,
        uuid: String,
        query: String,
    ) -> RunnerControl {
        if self.active_stream.is_some() {
            push_runner_error(app, "a gateway stream is already active");
            return RunnerControl::Continue;
        }
        let session_id = app.state().session.session_id.clone().unwrap_or_else(new_draft_id);
        let request = match workflow_run_request(
            &app.state().orchestration,
            Some(&session_id),
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
        if app.state().session.session_id.is_none() {
            app.bind_draft(session_id.clone());
        }
        app.begin_workflow_run(request_id.as_str(), query.as_str());
        self.workflow_run = Some(WorkflowRunOwnership::new_for_session(
            request_id.clone(),
            request.uuid.clone(),
            session_id,
        ));
        self.active_stream = Some(spawn_workflow_run(app.runtime().clone(), request, request_id));
        RunnerControl::Continue
    }

    pub(super) fn execute_session_operation(
        &mut self,
        app: &mut V3GatewayApp,
        operation: impl FnOnce(&mut V3GatewayApp) -> Result<(), String>,
    ) -> RunnerControl {
        let previous_session_id = app.state().session.session_id.clone();
        let had_workflow_question = app.state().tasks.workflow_pause_request_id.is_some();
        let result = if self.hook_mutation.is_some() {
            Err("cannot switch directories while a Hook mutation is active".to_string())
        } else if self.session_bootstrap_in_flight {
            Err("session history is still loading; retry in a moment".to_string())
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

    pub(super) fn start_directory_switch(&mut self, app: &mut V3GatewayApp, path: String) -> RunnerControl {
        if path.trim().is_empty() {
            return app
                .prime_session_command("/cd ", "请输入 Gateway 已知的工作目录，然后按 Enter。");
        }
        let result = if self.directory_switch_source.is_some() {
            Err("a directory switch is already in progress".to_string())
        } else if self.session_bootstrap_in_flight {
            Err("session history is still loading; retry in a moment".to_string())
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

    pub(super) fn execute_overlay_command(&mut self, app: &mut V3GatewayApp, command: TuiOverlayCommand) {
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
}
