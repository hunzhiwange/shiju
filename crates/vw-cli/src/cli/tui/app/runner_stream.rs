//! 活动流事件归并。

use super::{
    ActiveRunner,
    ActiveStreamKind,
    GatewayStreamAttemptResult,
    Ordering,
    STREAM_EVENT_BUDGET,
    StreamMessage,
    TryRecvError,
    UiRuntimeEvent,
    UiRuntimeTerminalEvent,
    UiTurnTerminal,
    V3GatewayApp,
    push_runner_error,
    spawn_session_title_read,
    terminal_from_error_message,
};

impl ActiveRunner {
    pub(super) fn drain_active_stream(&mut self, app: &mut V3GatewayApp) -> bool {
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
                    if !stream.cancel.load(Ordering::Acquire) {
                        app.accept_gateway_event(event);
                    }
                }
                Ok(StreamMessage::ChatPublished { preview, recovered }) => {
                    processed += 1;
                    if stream.cancel.load(Ordering::Acquire)
                        || app.runtime().session_id() != Some(preview.id.as_str())
                    {
                        continue;
                    }
                    if let Some(snapshot) = recovered {
                        app.restore_recovered_chat(&snapshot);
                    }
                    if app.apply_chat_publication(preview) {
                        self.title_request_id = Some(stream.request_id.clone());
                        spawn_session_title_read(
                            app.runtime().clone(),
                            stream.request_id.clone(),
                            self.host_sender.clone(),
                        );
                    }
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
        let stream_error = match result {
            Ok(
                GatewayStreamAttemptResult::Terminated | GatewayStreamAttemptResult::CallerStopped,
            ) => None,
            Ok(GatewayStreamAttemptResult::Incomplete) => {
                Some("gateway stream ended before a terminal event".to_string())
            }
            Err(error) => Some(format!("gateway stream failed: {error}")),
        };
        if let Some(error) = stream_error {
            if kind == ActiveStreamKind::Chat {
                app.accept_runtime_event(UiRuntimeEvent::Terminal(
                    terminal_from_error_message(error),
                ));
            } else {
                push_runner_error(app, error);
            }
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
        if kind == ActiveStreamKind::Chat {
            if let Some((text, attachments)) = self.chat_submission.take()
                && app.pending_first_response
                && !matches!(app.state.status.turn_terminal, UiTurnTerminal::Done { .. })
                && app.widget().draft().is_empty()
                && app.widget().attachments().is_empty()
            {
                app.widget_mut().restore_submission(&text, attachments);
            }
        } else if let Err(error) = app.persist_session_snapshot() {
            push_runner_error(app, format!("session save failed: {error}"));
        }
        self.active_stream = None;
        self.start_next_queued_prompt(app);
        false
    }
}
