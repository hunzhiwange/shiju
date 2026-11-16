//! 运行审批与交互同步。

use super::{
    GatewayV3Bridge,
    InteractionSnapshot,
    PermissionQueue,
    PermissionRequest,
    Result,
    TaskPlanRunRequest,
    TuiAction,
    UiMessage,
    UiMessageBase,
    UiMessageId,
    UiMessageOrigin,
    UiOverlay,
    UiPermissionOverlay,
    UiTodoOverlay,
    UiTurnTerminal,
    UiUserMessage,
    V3DisplayEvent,
    V3GatewayApp,
    push_runner_error,
    reduce_tui_state,
};

impl V3GatewayApp {
    pub(super) fn plan_run_request(&self, query: &str) -> Result<TaskPlanRunRequest, String> {
        cli_gateway_session::plan_run::plan_run_request(
            query,
            self.runtime.directory(),
            self.state.session_options.model.clone(),
            self.state.session_options.delegate_agent.clone(),
            self.state.session_options.acp_backend.clone(),
            self.state.session.session_id.clone(),
        )
    }

    pub(super) fn begin_plan_run(&mut self, request_id: &str, query: &str) {
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
        self.widget
            .set_session_header(GatewayV3Bridge::session_display_data(&self.runtime, &self.state));
    }

    pub(super) fn begin_workflow_run(&mut self, request_id: &str, query: &str) {
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
        self.widget
            .set_session_header(GatewayV3Bridge::session_display_data(&self.runtime, &self.state));
    }

    pub(super) fn begin_workflow_resume(&mut self) {
        self.state.status.turn_terminal = UiTurnTerminal::Streaming;
        self.state.status.last_error = None;
    }

    pub(super) fn replace_workflow_pause_question(
        &mut self,
        question: Option<cli_interaction_model::UiQuestionOverlay>,
    ) {
        reduce_tui_state(&mut self.state, TuiAction::WorkflowPauseQuestionReplaced(question));
        self.show_next_interaction();
    }

    pub(super) fn request_interaction_refresh(&mut self) {
        self.interaction_refresh_requested = true;
    }

    pub(super) fn take_interaction_refresh_requested(&mut self) -> bool {
        std::mem::take(&mut self.interaction_refresh_requested)
    }

    pub(super) fn apply_interaction_snapshot(&mut self, snapshot: InteractionSnapshot) {
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
                    .map(cli_interaction_model::UiQuestionOverlay::from_request)
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

    pub(super) fn complete_permission(&mut self, request_id: &str) {
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

    pub(super) fn complete_question(&mut self, request_id: &str) {
        self.state.tasks.pending_questions.retain(|question| question.request_id != request_id);
        reduce_tui_state(&mut self.state, TuiAction::TaskSyncErrorSet(None));
        self.show_next_interaction();
    }

    pub(super) fn record_interaction_error(&mut self, message: impl Into<String>) {
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
}
