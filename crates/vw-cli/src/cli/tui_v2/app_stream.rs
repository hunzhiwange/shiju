//! TUI gateway stream 副作用。

use vw_gateway_client::vw_api_types::task::TaskPlanRunRequest;

use crate::cli::tui_v2::runtime::plan_run::plan_run_request_for_submission;

use super::{
    Arc, AtomicBool, BUSY_HOST_POLL_RATE, CANCEL_REQUESTED_STATUS, Ordering, PromptSubmission,
    Result, SessionProcessorComparableResult, TuiAction, TuiApp, TuiAppCommandOutcome,
    TuiControllerCommand, TuiDeltaRedrawSnapshot, TuiTerminal, UiRuntimeEvent,
    UiRuntimeTerminalEvent, UiSystemMessageLevel, anyhow, apply_runtime_event,
    comparable_terminal_from_runtime_terminal, comparable_usage_from_runtime_terminal,
    compare_shadow_results, gateway_stream_request_from_state, legacy_processor,
    legacy_shadow_request_from_state, mpsc, normalize_optional_str_ref, reduce_tui_state,
    run_legacy_shadow_compare_blocking, runtime_event_fallback_overlay,
    shadow_compare_success_message, should_redraw_after_runtime_delta, thread,
};

impl TuiApp {
    pub(super) fn submit_prompt(
        &mut self,
        terminal: &mut TuiTerminal,
        submission: PromptSubmission,
    ) -> Result<TuiAppCommandOutcome> {
        match plan_run_request_for_submission(&submission) {
            Ok(Some(request)) => {
                if let Some(session_id) = submission.session_id.as_deref() {
                    match self.runtime.session_is_plan_run_child_blocking(session_id) {
                        Ok(true) => {}
                        Ok(false) => {
                            return self.submit_plan_run(terminal, submission, request);
                        }
                        Err(error) => {
                            self.push_overlay_error("工作流会话检查失败", error);
                            return Ok(TuiAppCommandOutcome::Continue);
                        }
                    }
                } else {
                    return self.submit_plan_run(terminal, submission, request);
                }
            }
            Ok(None) => {}
            Err(error) => {
                self.push_overlay_error("工作流启动失败", error);
                return Ok(TuiAppCommandOutcome::Continue);
            }
        }

        let shadow_request = self
            .run_mode
            .shadow_compare_enabled()
            .then(|| legacy_shadow_request_from_state(&self.state, &submission))
            .flatten();

        reduce_tui_state(&mut self.state, TuiAction::PromptSubmissionStarted(submission.clone()));

        let feedback = self.draw(terminal)?;
        self.state.apply_transcript_measurements(&feedback.transcript_measurements);
        self.controller.sync_layout(&mut self.state, &feedback);

        let request = gateway_stream_request_from_state(&self.state, &submission);
        let runtime = self.runtime.clone();
        let (sender, receiver) = mpsc::channel();
        let cancel_requested = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel_requested);
        let mut gateway_shadow_output = String::new();
        let mut gateway_shadow_step_finishes = 0usize;
        let mut gateway_shadow_terminal = None::<UiRuntimeTerminalEvent>;
        let mut gateway_stream_started = false;

        let worker = thread::spawn(move || {
            let mut saw_terminal = false;
            let terminal_event = runtime.stream_chat_blocking(&request, |event| {
                if matches!(event, UiRuntimeEvent::Terminal(_)) {
                    saw_terminal = true;
                }

                if sender.send(event).is_err() {
                    return false;
                }

                !worker_cancel.load(Ordering::Relaxed)
            });

            if !saw_terminal {
                let _ = sender.send(UiRuntimeEvent::Terminal(terminal_event));
            }
        });

        let mut quit_after_submission = false;
        let mut saw_terminal = false;
        let mut worker_disconnected = false;

        while !saw_terminal && !worker_disconnected {
            loop {
                match receiver.try_recv() {
                    Ok(event) => {
                        match &event {
                            UiRuntimeEvent::Delta(delta) => {
                                gateway_stream_started = true;
                                gateway_shadow_output.push_str(delta);
                            }
                            UiRuntimeEvent::StepFinish { .. } => {
                                gateway_stream_started = true;
                                gateway_shadow_step_finishes =
                                    gateway_shadow_step_finishes.saturating_add(1);
                            }
                            UiRuntimeEvent::Terminal(terminal_event) => {
                                gateway_shadow_terminal = Some(terminal_event.clone());
                            }
                            UiRuntimeEvent::TaskStateChanged { .. }
                            | UiRuntimeEvent::PlanRunTaskStarted { .. }
                            | UiRuntimeEvent::ContextLoaded { .. }
                            | UiRuntimeEvent::Message(_)
                            | UiRuntimeEvent::SessionMetadataChanged { .. }
                            | UiRuntimeEvent::UsageUpdated { .. }
                            | UiRuntimeEvent::StepStart { .. }
                            | UiRuntimeEvent::PostToolRound
                            | UiRuntimeEvent::Unknown { .. } => {
                                gateway_stream_started = true;
                            }
                        }
                        saw_terminal = self.apply_runtime_event_and_draw(terminal, event)?;
                        if saw_terminal {
                            break;
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        worker_disconnected = true;
                        break;
                    }
                }
            }

            if saw_terminal || worker_disconnected {
                break;
            }

            let event = self.controller.next_event_with_timeout(BUSY_HOST_POLL_RATE)?;
            let command = self.controller.handle_event(&mut self.state, event);
            match command {
                TuiControllerCommand::Continue
                | TuiControllerCommand::SubmitPrompt(_)
                | TuiControllerCommand::ExecuteSlashCommand(_)
                | TuiControllerCommand::MessageAction(_) => {}
                TuiControllerCommand::PromptExternalEditor(intent) => {
                    let _ = self.handle_command(
                        terminal,
                        TuiControllerCommand::PromptExternalEditor(intent),
                    )?;
                }
                TuiControllerCommand::CopyPromptSelection(text) => {
                    let _ = self.handle_command(
                        terminal,
                        TuiControllerCommand::CopyPromptSelection(text),
                    )?;
                }
                TuiControllerCommand::TranscriptEffect(effect) => {
                    let _ = self
                        .handle_command(terminal, TuiControllerCommand::TranscriptEffect(effect))?;
                }
                TuiControllerCommand::CancelActiveSubmission => {
                    self.request_submission_cancel(cancel_requested.as_ref());
                }
                TuiControllerCommand::Quit => {
                    quit_after_submission = true;
                    self.request_submission_cancel(cancel_requested.as_ref());
                }
                TuiControllerCommand::Overlay(command) => {
                    if matches!(self.handle_overlay_command(command)?, TuiAppCommandOutcome::Quit) {
                        quit_after_submission = true;
                        self.request_submission_cancel(cancel_requested.as_ref());
                    }
                }
            }

            let feedback = self.draw(terminal)?;
            self.state.apply_transcript_measurements(&feedback.transcript_measurements);
            self.controller.sync_layout(&mut self.state, &feedback);
        }

        worker.join().map_err(|_| anyhow!("tui_v2 stream worker panicked"))?;

        if worker_disconnected && !saw_terminal {
            return Err(anyhow!("tui_v2 stream worker exited before sending a terminal event"));
        }
        let acp_session_started = gateway_stream_started
            || gateway_shadow_terminal
                .as_ref()
                .is_some_and(|terminal| !matches!(terminal, UiRuntimeTerminalEvent::Error(_)));
        self.state
            .session_options
            .finish_acp_session_start(&submission.execution, acp_session_started);

        let gateway_shadow_result = gateway_shadow_terminal.as_ref().map(|terminal_event| {
            SessionProcessorComparableResult {
                output: gateway_shadow_output,
                usage: comparable_usage_from_runtime_terminal(terminal_event),
                step_finishes: gateway_shadow_step_finishes,
                terminal: comparable_terminal_from_runtime_terminal(terminal_event),
            }
        });

        self.refresh_task_state_blocking();
        self.refresh_project_context_blocking();
        self.run_shadow_compare_blocking(shadow_request, gateway_shadow_result);
        self.persist_session_snapshot_blocking();

        if quit_after_submission {
            Ok(TuiAppCommandOutcome::Quit)
        } else {
            Ok(TuiAppCommandOutcome::Continue)
        }
    }

    fn submit_plan_run(
        &mut self,
        terminal: &mut TuiTerminal,
        submission: PromptSubmission,
        request: TaskPlanRunRequest,
    ) -> Result<TuiAppCommandOutcome> {
        reduce_tui_state(&mut self.state, TuiAction::PromptSubmissionStarted(submission));

        let feedback = self.draw(terminal)?;
        self.state.apply_transcript_measurements(&feedback.transcript_measurements);
        self.controller.sync_layout(&mut self.state, &feedback);

        let runtime = self.runtime.clone();
        let (sender, receiver) = mpsc::channel();
        let cancel_requested = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel_requested);
        let worker = thread::spawn(move || {
            let mut saw_terminal = false;
            let terminal_event = runtime.stream_task_plan_run_blocking(&request, |event| {
                if matches!(event, UiRuntimeEvent::Terminal(_)) {
                    saw_terminal = true;
                }
                if sender.send(event).is_err() {
                    return false;
                }
                !worker_cancel.load(Ordering::Relaxed)
            });
            if !saw_terminal {
                let _ = sender.send(UiRuntimeEvent::Terminal(terminal_event));
            }
        });

        let mut quit_after_submission = false;
        let mut saw_terminal = false;
        let mut worker_disconnected = false;

        while !saw_terminal && !worker_disconnected {
            loop {
                match receiver.try_recv() {
                    Ok(event) => {
                        saw_terminal = self.apply_runtime_event_and_draw(terminal, event)?;
                        if saw_terminal {
                            break;
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        worker_disconnected = true;
                        break;
                    }
                }
            }

            if saw_terminal || worker_disconnected {
                break;
            }

            let event = self.controller.next_event_with_timeout(BUSY_HOST_POLL_RATE)?;
            let command = self.controller.handle_event(&mut self.state, event);
            match command {
                TuiControllerCommand::Continue
                | TuiControllerCommand::SubmitPrompt(_)
                | TuiControllerCommand::ExecuteSlashCommand(_)
                | TuiControllerCommand::MessageAction(_) => {}
                TuiControllerCommand::PromptExternalEditor(intent) => {
                    let _ = self.handle_command(
                        terminal,
                        TuiControllerCommand::PromptExternalEditor(intent),
                    )?;
                }
                TuiControllerCommand::CopyPromptSelection(text) => {
                    let _ = self.handle_command(
                        terminal,
                        TuiControllerCommand::CopyPromptSelection(text),
                    )?;
                }
                TuiControllerCommand::TranscriptEffect(effect) => {
                    let _ = self
                        .handle_command(terminal, TuiControllerCommand::TranscriptEffect(effect))?;
                }
                TuiControllerCommand::CancelActiveSubmission => {
                    self.request_plan_run_cancel(cancel_requested.as_ref());
                }
                TuiControllerCommand::Quit => {
                    quit_after_submission = true;
                    self.request_plan_run_cancel(cancel_requested.as_ref());
                }
                TuiControllerCommand::Overlay(command) => {
                    if matches!(self.handle_overlay_command(command)?, TuiAppCommandOutcome::Quit) {
                        quit_after_submission = true;
                        self.request_plan_run_cancel(cancel_requested.as_ref());
                    }
                }
            }

            let feedback = self.draw(terminal)?;
            self.state.apply_transcript_measurements(&feedback.transcript_measurements);
            self.controller.sync_layout(&mut self.state, &feedback);
        }

        worker.join().map_err(|_| anyhow!("tui_v2 plan-run worker panicked"))?;
        if worker_disconnected && !saw_terminal {
            return Err(anyhow!("tui_v2 plan-run worker exited before sending a terminal event"));
        }

        self.refresh_task_state_blocking();
        self.refresh_project_context_blocking();
        self.persist_session_snapshot_blocking();

        if quit_after_submission {
            Ok(TuiAppCommandOutcome::Quit)
        } else {
            Ok(TuiAppCommandOutcome::Continue)
        }
    }

    fn request_plan_run_cancel(&mut self, cancel_requested: &AtomicBool) {
        if cancel_requested.load(Ordering::Relaxed) {
            return;
        }
        if let Some(task_id) = self.state.runtime.active_plan_run_task_id.clone()
            && let Err(error) = self.runtime.task_plan_run_stop_blocking(task_id.as_str())
        {
            self.push_overlay_error("工作流停止失败", error);
        }
        self.request_submission_cancel(cancel_requested);
    }

    fn apply_runtime_event_and_draw(
        &mut self,
        terminal: &mut TuiTerminal,
        event: UiRuntimeEvent,
    ) -> Result<bool> {
        let should_refresh_task_state = matches!(
            &event,
            UiRuntimeEvent::TaskStateChanged { session_id }
                if self.runtime_event_targets_current_session(session_id.as_deref())
        );
        let should_refresh_session_metadata = matches!(
            &event,
            UiRuntimeEvent::SessionMetadataChanged { session_id, .. }
                if self.runtime_event_targets_current_session(session_id.as_deref())
        );
        let delta_snapshot = matches!(event, UiRuntimeEvent::Delta(_))
            .then(|| TuiDeltaRedrawSnapshot::capture(&self.state));
        let saw_terminal = matches!(event, UiRuntimeEvent::Terminal(_));
        let fallback_overlay = runtime_event_fallback_overlay(&event);

        apply_runtime_event(&mut self.state, event);
        if should_refresh_session_metadata {
            self.sync_session_metadata_blocking();
        }
        if should_refresh_task_state {
            self.refresh_task_state_blocking();
        }
        self.reload_active_task_overlay();
        if let Some(overlay) = fallback_overlay {
            self.push_error_overlay(overlay);
        }

        if let Some(delta_snapshot) = delta_snapshot.as_ref()
            && !should_redraw_after_runtime_delta(delta_snapshot, &self.state)
        {
            return Ok(saw_terminal);
        }

        let feedback = self.draw(terminal)?;
        self.state.apply_transcript_measurements(&feedback.transcript_measurements);
        self.controller.sync_layout(&mut self.state, &feedback);
        Ok(saw_terminal)
    }

    fn runtime_event_targets_current_session(&self, session_id: Option<&str>) -> bool {
        match normalize_optional_str_ref(session_id) {
            Some(session_id) => self.current_session_id() == Some(session_id),
            None => true,
        }
    }

    fn request_submission_cancel(&mut self, cancel_requested: &AtomicBool) {
        if cancel_requested.swap(true, Ordering::Relaxed) {
            return;
        }

        reduce_tui_state(
            &mut self.state,
            TuiAction::StatusErrorSet(Some(CANCEL_REQUESTED_STATUS.to_string())),
        );
    }

    fn run_shadow_compare_blocking(
        &mut self,
        request: Option<legacy_processor::Request>,
        gateway_result: Option<SessionProcessorComparableResult>,
    ) {
        if !self.run_mode.shadow_compare_enabled() {
            return;
        }

        let Some(request) = request else {
            self.push_app_system_message(
                "Shadow compare skipped because no stable session binding was available for the current turn.",
                UiSystemMessageLevel::Warning,
            );
            return;
        };

        let Some(gateway_result) = gateway_result else {
            self.push_app_system_message(
                "Shadow compare skipped because the gateway turn did not produce a comparable terminal result.",
                UiSystemMessageLevel::Warning,
            );
            return;
        };

        match run_legacy_shadow_compare_blocking(request) {
            Ok(legacy_result) => match compare_shadow_results(&gateway_result, &legacy_result) {
                Ok(()) => {
                    self.push_app_system_message(
                        shadow_compare_success_message(&gateway_result),
                        UiSystemMessageLevel::Success,
                    );
                }
                Err(diff) => {
                    self.push_app_system_message(
                        format!("Shadow compare diverged from legacy: {diff}."),
                        UiSystemMessageLevel::Warning,
                    );
                }
            },
            Err(err) => {
                self.push_app_system_message(
                    format!("Shadow compare failed before legacy replay completed: {err}."),
                    UiSystemMessageLevel::Warning,
                );
            }
        }
    }
}
