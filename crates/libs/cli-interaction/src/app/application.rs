//! 应用组装与消息接收。

use super::{
    PromptAttachmentKind,
    ChatWidget,
    GatewayChatStreamEvent,
    GatewayChatStreamRequest,
    GatewayUiRuntime,
    GatewayV3Bridge,
    PromptSubmission,
    Result,
    TerminalMode,
    TranscriptReflowState,
    TuiController,
    TuiControllerCommand,
    TuiState,
    UiAttachment,
    UiAttachmentKind,
    UiMessage,
    UiMessageBase,
    UiMessageId,
    UiMessageOrigin,
    UiRuntimeEvent,
    V3Action,
    V3DisplayEvent,
    V3Event,
    V3GatewayApp,
    build_session_widget,
    display_event_from_message,
    new_draft_id,
};

impl V3GatewayApp {
    /// 以内联终端模式创建应用，并用会话配置补齐缺失的模型名。
    #[cfg(test)]
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
        let pending_first_response = state.session.session_id.is_none();
        Self {
            runtime,
            state,
            controller: TuiController,
            widget,
            bridge: GatewayV3Bridge::default(),
            terminal_mode,
            transcript_reflow: TranscriptReflowState::default(),
            interaction_refresh_requested: false,
            pending_first_response,
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

    /// 将界面操作路由为控制器命令；提交成功时先追加本地用户历史。
    pub(crate) fn handle_action(
        &mut self,
        action: V3Action,
    ) -> Result<TuiControllerCommand, String> {
        let visible_attachments = match &action {
            V3Action::Submit { attachments, .. } => attachments.clone(),
            _ => Vec::new(),
        };
        let mut command =
            self.bridge.controller_command(&mut self.state, &mut self.controller, action)?;
        if let TuiControllerCommand::SubmitPrompt(submission) = &mut command {
            if self.state.session.session_id.is_none() {
                let id = new_draft_id();
                self.bind_draft(id.clone());
                submission.session_id = Some(id.clone());
                self.state.prompt.active_submission = Some(submission.clone());
                if let Some(UiMessage::User(user)) = self.state.messages.last_mut() {
                    user.base.session_id = Some(id);
                }
            }
            self.widget.push_history(vw_tui::V3DisplayEvent::User {
                text: submission.text.clone(),
                attachments: visible_attachments,
            });
            self.widget.set_session_header(GatewayV3Bridge::session_display_data(
                &self.runtime,
                &self.state,
            ));
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

    pub(super) fn record_submission_attachments(&mut self, submission: &PromptSubmission) {
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
    #[cfg(test)]
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
}
