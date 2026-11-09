//! ACP 事件客户端实现。
//!
//! 该模块实现 `agent_client_protocol::Client`，处理代理回调到客户端的权限、
//! 文件系统、终端和会话通知请求，并把可消费的文本增量转发给 actor 循环。

use super::*;

impl AcpClient {
    pub(super) fn build_event_client(
        &self,
        cwd: &Path,
        expected_session_id: Arc<Mutex<Option<String>>>,
        event_tx: mpsc::UnboundedSender<InternalEvent>,
    ) -> AcpEventClient {
        let on_operation = self.on_client_operation.clone();
        let permission_mode = match self.permission_mode {
            PermissionMode::ApproveAll if self.desktop_workspace.is_some() && !self.full_access => {
                PermissionMode::ApproveReads
            }
            mode => mode,
        };
        let actor_state = Arc::downgrade(&self.actor_state);
        let on_update = self.on_session_update.clone();
        let on_session_update: SessionUpdateCallback = Arc::new(move |notification| {
            if let Some(state) = actor_state.upgrade() {
                let mut state = state.lock();
                if let Some(info) = state.session_capabilities.as_mut()
                    && info.session_id == notification.session_id.to_string()
                {
                    match &notification.update {
                        acp::SessionUpdate::ConfigOptionUpdate(update) => {
                            info.config_options = Some(update.config_options.clone())
                        }
                        acp::SessionUpdate::CurrentModeUpdate(update) => {
                            if let Some(modes) = info.modes.as_mut() {
                                modes.current_mode_id = update.current_mode_id.clone();
                            }
                        }
                        _ => {}
                    }
                }
            }
            if let Some(callback) = &on_update {
                callback(notification);
            }
        });
        AcpEventClient {
            claude_permissions: self.agent_name == "claude",
            kimi_permissions: self.agent_name == "kimi",
            codebuddy_permissions: self.agent_name == "codebuddy",
            transient_permissions: matches!(
                self.agent_name.as_str(),
                "opencode"
                    | "copilot"
                    | "kimi"
                    | "auggie"
                    | "codebuddy"
                    | "goose"
                    | "snow"
                    | "cursor"
                    | "droid"
                    | "kilocode"
                    | "openclaw"
                    | "pi"
                    | "trae"
            ),
            reject_untyped_workflow_permissions: matches!(
                self.agent_name.as_str(),
                "gemini" | "qwen"
            ),
            expected_session_id,
            desktop_workspace: self.desktop_workspace.clone(),
            active_prompt: self.active_prompt.clone(),
            event_tx,
            filesystem: FileSystemHandlers::new(FileSystemHandlersOptions {
                cwd: cwd.to_path_buf(),
                permission_mode,
                full_access: self.full_access,
                non_interactive_permissions: self.non_interactive_permissions,
                on_operation: on_operation.clone(),
                confirm_write: None,
            })
            .with_skill_read_roots(
                if matches!(
                    self.agent_name.as_str(),
                    "claude" | "claude code" | "qwen" | "gemini" | "codebuddy" | "kiro"
                ) {
                    self.skill_mounts
                        .as_ref()
                        .into_iter()
                        .flat_map(|selection| &selection.skills)
                        .filter_map(|skill| Path::new(&skill.path).canonicalize().ok())
                        .collect()
                } else {
                    Vec::new()
                },
            ),
            terminal_manager: TerminalManager::new(TerminalManagerOptions {
                cwd: cwd.to_path_buf(),
                permission_mode,
                non_interactive_permissions: self.non_interactive_permissions,
                on_operation,
                confirm_execute: None,
                kill_grace_ms: None,
            })
            .with_workspace_auto_execute(self.desktop_workspace.is_none()),
            permission_mode,
            non_interactive_permissions: self.non_interactive_permissions,
            on_session_update: Some(on_session_update),
            permission_stats: self.permission_stats.clone(),
            cancelling_session_ids: self.cancelling_session_ids.clone(),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for AcpEventClient {
    async fn request_permission(
        &self,
        mut args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        if self.cancelling_session_ids.lock().contains(args.session_id.0.as_ref()) {
            self.permission_stats.lock().cancelled += 1;
            return Ok(cancelled_permission_response());
        }
        if self.desktop_workspace.is_some() && !self.owns_active_prompt(&args.session_id) {
            self.permission_stats.lock().cancelled += 1;
            return Ok(cancelled_permission_response());
        }
        self.permission_stats.lock().requested += 1;
        // Gemini maps plan/question tools to Other without a structured payload.
        // Do not guess from user-controlled titles or answer these as tool approvals.
        if self.reject_untyped_workflow_permissions
            && matches!(
                args.tool_call.fields.kind,
                None | Some(acp::ToolKind::Other | acp::ToolKind::SwitchMode)
            )
        {
            return Err(acp::Error::invalid_params().data("Agent workflow confirmation requires explicit user interaction; untyped tools cannot be automatically approved"));
        }
        // ExitPlanMode is a workflow decision, not an ordinary tool approval.
        // This client has no structured plan-confirmation UI. Fail explicitly rather
        // than auto-selecting the offered bypassPermissions option.
        if self.claude_permissions && args.tool_call.fields.kind == Some(acp::ToolKind::SwitchMode)
        {
            return Err(acp::Error::invalid_params().data("Claude plan confirmation requires explicit user interaction; automatic tool approval cannot answer it"));
        }

        if self.kimi_permissions && crate::access_policy::kimi::has_workflow_answers(&args) {
            return Err(acp::Error::invalid_params().data("Kimi question or plan confirmation requires explicit user interaction; tool approval cannot answer it"));
        }

        if self.codebuddy_permissions && crate::access_policy::codebuddy::is_workflow_request(&args)
        {
            return Err(acp::Error::invalid_params().data("CodeBuddy question or plan confirmation requires explicit user interaction; tool approval cannot answer it"));
        }

        // Keep explicit read permissions, but a title cannot turn an untyped
        // desktop tool into a read. A generic approval cannot constrain the
        // agent's subsequent local write, even when it describes a valid path.
        if self.desktop_workspace.is_some() && args.tool_call.fields.kind.is_none() {
            args.tool_call.fields.kind = Some(acp::ToolKind::Other);
        }
        let resolve = if self.transient_permissions || self.desktop_workspace.is_some() {
            crate::access_policy::opencode::resolve_permission_request
        } else {
            resolve_permission_request
        };
        let response = resolve(&args, self.permission_mode, self.non_interactive_permissions)
            .map_err(acp::Error::into_internal_error)?;
        match classify_permission_decision(&args, &response) {
            PermissionDecision::Approved => self.permission_stats.lock().approved += 1,
            PermissionDecision::Denied => self.permission_stats.lock().denied += 1,
            PermissionDecision::Cancelled => self.permission_stats.lock().cancelled += 1,
        }
        Ok(response)
    }

    async fn write_text_file(
        &self,
        args: acp::WriteTextFileRequest,
    ) -> acp::Result<acp::WriteTextFileResponse> {
        if self.desktop_workspace.is_some() && !self.owns_active_prompt(&args.session_id) {
            return Err(acp::Error::invalid_params()
                .data("Write request has no matching active host session"));
        }
        self.filesystem
            .write_text_file_with_policy(&args, self.desktop_workspace.as_deref())
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> acp::Result<acp::ReadTextFileResponse> {
        self.ensure_desktop_session(&args.session_id)?;
        if self.desktop_workspace.is_some()
            && self.permission_mode == PermissionMode::ApproveAll
            && !self.owns_active_prompt(&args.session_id)
        {
            return Err(acp::Error::invalid_params()
                .data("Full-access read requires an active host session"));
        }
        self.filesystem
            .read_text_file(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn create_terminal(
        &self,
        args: acp::CreateTerminalRequest,
    ) -> acp::Result<acp::CreateTerminalResponse> {
        if self.desktop_workspace.is_some() && !self.owns_active_prompt(&args.session_id) {
            return Err(
                acp::Error::invalid_params().data("Terminal request has no matching host session")
            );
        }
        self.terminal_manager
            .create_terminal(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn terminal_output(
        &self,
        args: acp::TerminalOutputRequest,
    ) -> acp::Result<acp::TerminalOutputResponse> {
        self.ensure_desktop_session(&args.session_id)?;
        self.terminal_manager
            .terminal_output(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn release_terminal(
        &self,
        args: acp::ReleaseTerminalRequest,
    ) -> acp::Result<acp::ReleaseTerminalResponse> {
        self.ensure_desktop_session(&args.session_id)?;
        self.terminal_manager
            .release_terminal(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn wait_for_terminal_exit(
        &self,
        args: acp::WaitForTerminalExitRequest,
    ) -> acp::Result<acp::WaitForTerminalExitResponse> {
        self.ensure_desktop_session(&args.session_id)?;
        self.terminal_manager
            .wait_for_terminal_exit(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn kill_terminal(
        &self,
        args: acp::KillTerminalRequest,
    ) -> acp::Result<acp::KillTerminalResponse> {
        self.ensure_desktop_session(&args.session_id)?;
        self.terminal_manager
            .kill_terminal(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        if self.desktop_workspace.is_some()
            && self
                .expected_session_id
                .lock()
                .as_deref()
                .is_some_and(|id| id != args.session_id.0.as_ref())
        {
            return Ok(());
        }
        // A delayed control notification from another session must not retarget
        // the active prompt or reach consumers as current session state.
        if matches!(
            &args.update,
            acp::SessionUpdate::ConfigOptionUpdate(_) | acp::SessionUpdate::CurrentModeUpdate(_)
        ) && self
            .expected_session_id
            .lock()
            .as_deref()
            .is_some_and(|id| id != args.session_id.0.as_ref())
        {
            return Ok(());
        }
        if let Some(callback) = &self.on_session_update {
            callback(args.clone());
        }
        let session_id = args.session_id.to_string();
        let expected_session_id = self.expected_session_id.lock().as_ref().cloned();
        if let Some(expected_session_id) = expected_session_id
            && session_id != expected_session_id
        {
            *self.expected_session_id.lock() = Some(session_id.clone());
            let _ = self.event_tx.send(InternalEvent::SessionChanged {
                expected: expected_session_id,
                actual: session_id.clone(),
            });
        }

        if let acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk { content, .. }) =
            &args.update
        {
            let delta = match content {
                acp::ContentBlock::Text(text_content) => text_content.text.clone(),
                acp::ContentBlock::ResourceLink(resource_link) => resource_link.uri.clone(),
                _ => String::new(),
            };
            if !delta.is_empty() {
                let _ = self.event_tx.send(InternalEvent::Delta(delta));
            }
        } else if let acp::SessionUpdate::AgentThoughtChunk(acp::ContentChunk { content, .. }) =
            args.update
        {
            let delta = match content {
                acp::ContentBlock::Text(text_content) => text_content.text,
                _ => String::new(),
            };
            if !delta.is_empty() {
                let _ = self.event_tx.send(InternalEvent::Delta(delta));
            }
        }

        Ok(())
    }

    async fn ext_method(&self, _args: acp::ExtRequest) -> acp::Result<acp::ExtResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn ext_notification(&self, _args: acp::ExtNotification) -> acp::Result<()> {
        Err(acp::Error::method_not_found())
    }
}

impl AcpEventClient {
    fn ensure_desktop_session(&self, session_id: &acp::SessionId) -> acp::Result<()> {
        if self.desktop_workspace.is_some()
            && self.expected_session_id.lock().as_deref() != Some(session_id.0.as_ref())
        {
            return Err(acp::Error::invalid_params().data("Request has no matching host session"));
        }
        Ok(())
    }

    fn owns_active_prompt(&self, session_id: &acp::SessionId) -> bool {
        let session_id = session_id.0.as_ref();
        self.expected_session_id.lock().as_deref() == Some(session_id)
            && self.active_prompt.lock().as_ref().is_some_and(|prompt| {
                prompt.session_id == session_id && !*prompt.cancel_tx.borrow()
            })
            && !self.cancelling_session_ids.lock().contains(session_id)
    }
}
