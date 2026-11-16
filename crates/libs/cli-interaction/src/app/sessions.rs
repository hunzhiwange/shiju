//! 会话恢复与切换操作。

use super::{
    normalize_optional_str_ref,
    ChatSession,
    ChatSessionMeta,
    DirectorySwitchBootstrap,
    GatewayV3Bridge,
    Result,
    RunnerControl,
    SESSION_RESUME_SURFACE,
    SystemTime,
    TranscriptReflowState,
    TuiAction,
    TuiState,
    UNIX_EPOCH,
    UiConfirmIntent,
    UiConfirmOverlay,
    UiOverlay,
    V3DisplayEvent,
    V3GatewayApp,
    V3SelectionItem,
    build_session_widget,
    initial_tui_state,
    reduce_tui_state,
    session_resume_items,
};

impl V3GatewayApp {
    /// 将当前视图状态转换为会话快照并同步保存。
    pub(crate) fn persist_session_snapshot(&mut self) -> Result<(), String> {
        if self.pending_first_response || self.state.session.session_id.is_none() {
            return Ok(());
        }
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
        let snapshot = self.runtime.session_ui_resume_blocking(session_id)?;
        self.commit_session_switch(snapshot);
        Ok(())
    }

    /// 切换到空白草稿，保留当前目录和会话设置。
    pub(crate) fn create_new_session(&mut self) -> Result<(), String> {
        self.commit_initial_session_state(initial_tui_state(&self.runtime));
        Ok(())
    }

    pub(super) fn prime_session_command(&mut self, draft: &str, hint: &str) -> RunnerControl {
        self.widget.replace_draft(draft);
        self.widget.push_history(V3DisplayEvent::System { text: hint.to_string() });
        RunnerControl::Continue
    }

    pub(super) fn open_session_message_picker(&mut self, surface: &str, title: &str) -> Result<(), String> {
        let items = session_message_items(&self.state);
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

    pub(super) fn open_reset_confirmation(&mut self, message_id: &str) -> Result<(), String> {
        let message_id = self.require_persisted_message_id(message_id)?.to_string();
        self.push_session_confirmation(UiConfirmOverlay {
            request_id: format!("session-confirm:reset:{message_id}"),
            title: "确认重置会话".to_string(),
            body: format!("将会话历史截断到消息 {message_id}。此操作不可在 TUI 内撤销。"),
            confirm_label: "确认".to_string(),
            cancel_label: "取消".to_string(),
            destructive: true,
            intent: UiConfirmIntent::ResetSession { message_id },
        });
        Ok(())
    }

    pub(super) fn open_compact_confirmation(&mut self, message_id: &str) -> Result<(), String> {
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

    pub(super) fn open_archive_confirmation(&mut self) -> Result<(), String> {
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

    pub(super) fn push_session_confirmation(&mut self, confirmation: UiConfirmOverlay) {
        let request = confirmation.to_v3_request();
        reduce_tui_state(
            &mut self.state,
            TuiAction::OverlayPushed(UiOverlay::Confirm(confirmation)),
        );
        self.widget.show_approval(request);
    }

    pub(super) fn dismiss_confirmation(&mut self, request_id: &str) -> Result<(), String> {
        let Some(UiOverlay::Confirm(confirmation)) = self.state.overlays.active() else {
            return Err("no session confirmation is active".to_string());
        };
        if confirmation.request_id != request_id {
            return Err("session confirmation does not match the active intent".to_string());
        }
        reduce_tui_state(&mut self.state, TuiAction::OverlayPopped);
        Ok(())
    }

    pub(super) fn reset_session(&mut self, message_id: &str) -> Result<(), String> {
        let message_id = self.require_persisted_message_id(message_id)?.to_string();
        self.runtime.session_reset_blocking(message_id.as_str())?;
        self.refresh_current_session_snapshot()?;
        self.widget.push_history(V3DisplayEvent::System {
            text: format!("会话已回退到消息 {message_id}"),
        });
        Ok(())
    }

    pub(super) fn compact_session(&mut self, message_id: &str) -> Result<(), String> {
        let message_id = self.require_persisted_message_id(message_id)?.to_string();
        let compacted = self.runtime.session_compact_blocking(message_id.as_str())?;
        self.refresh_current_session_snapshot()?;
        let outcome = if compacted { "已压缩" } else { "无需压缩" };
        self.widget.push_history(V3DisplayEvent::System {
            text: format!("会话上下文{outcome}（起点 {message_id}）"),
        });
        Ok(())
    }

    pub(super) fn archive_session(&mut self) -> Result<(), String> {
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

    pub(super) fn bind_draft(&mut self, session_id: String) {
        self.state.session.session_id = Some(session_id);
        self.sync_runtime_session_seed();
        self.widget
            .set_session_header(GatewayV3Bridge::session_display_data(&self.runtime, &self.state));
    }

    pub(super) fn restore_recovered_chat(&mut self, snapshot: &ChatSession) {
        let draft = self.widget.draft().to_string();
        let attachments = self.widget.attachments().to_vec();
        let mut queued = Vec::new();
        while let Some(prompt) = self.widget.pop_queued_input() {
            queued.push(prompt);
        }
        let mut state = TuiState::from_chat_session(snapshot);
        state.session.scope.clone_from(&self.state.session.scope);
        self.commit_initial_session_state(state);
        self.widget.restore_submission(&draft, attachments);
        for (text, attachments) in queued {
            self.widget.queue_input(text, attachments);
        }
    }

    pub(super) fn apply_chat_publication(&mut self, preview: ChatSessionMeta) -> bool {
        self.pending_first_response = false;
        self.state.session.first_response_preview = preview.last_content;
        let updated_ms = self.state.session.updated_ms.max(preview.updated_ms);
        reduce_tui_state(&mut self.state, TuiAction::SessionUpdatedMsSet(updated_ms));
        if self.state.session.title.trim().is_empty() {
            self.set_server_title(preview.title);
        }
        self.state.session.title.trim().is_empty()
    }

    pub(super) fn set_server_title(&mut self, title: String) {
        reduce_tui_state(&mut self.state, TuiAction::SessionTitleSet(title));
        self.sync_runtime_session_seed();
        self.widget
            .set_session_header(GatewayV3Bridge::session_display_data(&self.runtime, &self.state));
    }

    pub(super) fn sync_runtime_session_seed(&mut self) {
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
        self.pending_first_response = false;
        self.state = next_state;
        self.widget = next_widget;
        self.bridge = GatewayV3Bridge::default();
        self.transcript_reflow = TranscriptReflowState::default();
        self.interaction_refresh_requested = true;
    }

    pub(super) fn commit_initial_session_state(&mut self, mut next_state: TuiState) {
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
        self.pending_first_response = next_state.session.session_id.is_none();
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
        self.pending_first_response = state.session.session_id.is_none();
        self.state = state;
        self.widget = widget;
        self.bridge = GatewayV3Bridge::default();
        self.transcript_reflow = TranscriptReflowState::default();
        self.interaction_refresh_requested = true;
    }
}

/// 按当前快照的消息身份匹配正文，不使用渲染行位置推断对应关系。
fn session_message_items(state: &TuiState) -> Vec<V3SelectionItem> {
    use super::{UiMessage, UiMessageId};
    let messages = state.messages.iter()
        .filter(|message| message.base().session_id.as_deref() == state.session.session_id.as_deref())
        .map(|message| (message.id(), message))
        .collect::<std::collections::HashMap<_, _>>();
    state.session.persisted_messages.iter().enumerate().filter_map(|(index, metadata)| {
        let id = metadata.raw_message_id.as_deref()?;
        let (role, body) = match messages.get(&UiMessageId::gateway(id)) {
            Some(UiMessage::User(message)) => ("用户", message.text.as_str()),
            Some(UiMessage::Assistant(message)) => ("助手", message.text.as_str()),
            Some(UiMessage::ToolCall(message)) => ("工具调用", message.arguments.as_deref().or(message.summary.as_deref()).unwrap_or(&message.tool_name)),
            Some(UiMessage::ToolResult(message)) => ("工具结果", message.content.as_str()),
            Some(UiMessage::Thinking(message)) => ("思考", message.content.as_str()),
            Some(UiMessage::System(message)) => ("系统", message.text.as_str()),
            Some(UiMessage::Error(message)) => ("错误", message.message.as_str()),
            Some(UiMessage::Attachment(message)) => ("附件", message.preview.as_deref().unwrap_or(&message.name)),
            Some(UiMessage::Progress(message)) => ("进度", message.label.as_str()),
            Some(UiMessage::CompactBoundary(message)) => ("压缩边界", message.summary.as_str()),
            Some(UiMessage::Unknown(message)) => ("未知", message.summary.as_str()),
            Some(UiMessage::Step(_)) | None => ("正文缺失", "当前 canonical snapshot 中没有可显示正文"),
        };
        let mut chars = body.chars().map(|ch| if ch.is_control() || ch.is_whitespace() { ' ' } else { ch });
        let mut summary = chars.by_ref().take(100).collect::<String>();
        if chars.next().is_some() { summary.push('…'); }
        if summary.trim().is_empty() { summary = "（空正文）".to_string(); }
        Some(V3SelectionItem {
            id: id.to_string(),
            label: format!("#{} · {role} · {summary}", index + 1),
            description: Some(format!("消息 ID：{id} · 输入正文或 ID 搜索")),
            search_value: Some(format!("{id} {role} {body}")),
            ..V3SelectionItem::default()
        })
    }).collect()
}

#[cfg(test)]
#[path = "../workflow_req_04_tests.rs"]
mod workflow_req_04_tests;
