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
        let confirm_execute = self.permission_confirmation.clone().map(|confirm| {
            let policy = self.non_interactive_permissions;
            let handler: crate::terminal::TerminalConfirmExecuteFn = Arc::new(move |command| {
                let confirm = confirm.clone();
                Box::pin(async move {
                    match policy {
                        None => Ok(confirm("Terminal".into(), json!({"command": command})).await),
                        Some(NonInteractivePermissionPolicy::Deny) => Ok(false),
                        Some(NonInteractivePermissionPolicy::Fail) => {
                            Err(Box::new(crate::errors::PermissionPromptUnavailableError::new())
                                as crate::errors::ErrorSource)
                        }
                    }
                })
            });
            handler
        });
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
        let managed_workspace_io = self.managed_workspace_io();
        let mut gemini_mcp_servers = HashSet::new();
        let mut qwen_mcp_servers = HashSet::new();
        let managed_gemini = self.agent_name == "gemini"
            && self.config.args.iter().any(|arg| arg == "--shiju-model-source")
            && crate::verify_reviewed_agent_executable("gemini", self.config.command.trim()).is_ok();
        if !self.mcp_servers.is_empty()
            && (managed_gemini
                || (self.agent_name == "qwen"
                    && self.config.args.iter().any(|arg| arg == "--shiju-model-source")
                    && crate::verify_reviewed_agent_executable("qwen", self.config.command.trim())
                        .is_ok()))
        {
            let mounted = if self.agent_name == "gemini" {
                &mut gemini_mcp_servers
            } else {
                &mut qwen_mcp_servers
            };
            mounted.extend(self.mcp_servers.iter().filter_map(|server| match server {
                acp::McpServer::Stdio(server) => Some(server.name.clone()),
                acp::McpServer::Http(server) => Some(server.name.clone()),
                acp::McpServer::Sse(server) => Some(server.name.clone()),
                _ => None,
            }));
        }
        AcpEventClient {
            managed_workspace_io,
            sandboxed_native_io: self.sandboxed_native_io(),
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
            managed_gemini,
            gemini_mcp_servers,
            qwen_mcp_servers,
            expected_session_id,
            desktop_workspace: if managed_workspace_io {
                Some(cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf()))
            } else {
                self.desktop_workspace.clone()
            },
            active_prompt: self.active_prompt.clone(),
            event_tx,
            filesystem: FileSystemHandlers::new(FileSystemHandlersOptions {
                cwd: cwd.to_path_buf(),
                permission_mode,
                full_access: self.full_access && !managed_workspace_io,
                non_interactive_permissions: self.non_interactive_permissions,
                on_operation: on_operation.clone(),
                confirm_write: None,
            })
            .with_skill_read_roots(
                if matches!(
                    self.agent_name.as_str(),
                    "claude" | "claude code" | "qwen" | "gemini" | "codebuddy" | "kiro"
                ) || (matches!(self.agent_name.as_str(), "amp-acp" | "deepagents" | "dimcode" | "grok" | "kimi" | "sigit")
                    && self.skill_mount_lease.is_some()
                    && self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE"))
                {
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
                confirm_execute,
                kill_grace_ms: None,
            })
            .with_workspace_auto_execute(self.desktop_workspace.is_none()),
            permission_mode,
            non_interactive_permissions: self.non_interactive_permissions,
            permission_confirmation: self.permission_confirmation.clone(),
            managed_mcp_approval: self.managed_mcp_approval.clone(),
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
        if self.permission_confirmation.is_some()
            && self.expected_session_id.lock().as_deref() != Some(args.session_id.0.as_ref())
        {
            self.permission_stats.lock().cancelled += 1;
            return Ok(cancelled_permission_response());
        }
        self.permission_stats.lock().requested += 1;
        // Only the reviewed Gemini bundle emits this marker for an MCP invocation.
        let gemini_mcp = args.meta.as_ref().and_then(|meta| meta.get("shiju.gemini/mcp"));
        if let Some(mcp) = gemini_mcp
            && (args.tool_call.fields.kind != Some(acp::ToolKind::Other)
                || !self.owns_active_prompt(&args.session_id)
                || !mcp["server"]
                    .as_str()
                    .is_some_and(|name| self.gemini_mcp_servers.contains(name))
                || !mcp["tool"].as_str().is_some_and(|name| !name.trim().is_empty()))
        {
            return Err(acp::Error::invalid_params()
                .data("Gemini MCP approval requires a mounted server in the active session"));
        }
        let gemini_skill = args.meta.as_ref().and_then(|meta| meta.get("shiju.gemini/skill"));
        if let Some(skill) = gemini_skill {
            let Some(name) = skill.as_str().filter(|name| !name.trim().is_empty()) else {
                return Err(acp::Error::invalid_params().data("Gemini skill approval requires a skill name"));
            };
            if !self.managed_gemini
                || gemini_mcp.is_some()
                || args.tool_call.fields.kind != Some(acp::ToolKind::Other)
                || !self.owns_active_prompt(&args.session_id)
            {
                return Err(acp::Error::invalid_params()
                    .data("Gemini skill approval requires a verified active session"));
            }
            args.tool_call.fields.title = Some(format!("Activate Skill: {name}"));
            args.tool_call.fields.raw_input = Some(serde_json::json!({"name": name}));
        }
        // The reviewed Qwen scheduler supplies the registered tool name here.
        // The title is only an argument preview and carries no tool identity.
        let qwen_mcp = args
            .tool_call
            .meta
            .as_ref()
            .and_then(|meta| meta.get("toolName"))
            .and_then(Value::as_str)
            .filter(|name| {
                args.tool_call.fields.kind == Some(acp::ToolKind::Other)
                    && name
                        .strip_prefix("mcp__")
                        .and_then(|name| name.split_once("__"))
                        .is_some_and(|(server, tool)| {
                            self.qwen_mcp_servers.contains(server) && !tool.trim().is_empty()
                        })
                    && self.permission_mode == PermissionMode::ApproveReads
                    && self.permission_confirmation.is_some()
                    && args.tool_call.fields.raw_input.as_ref().is_some_and(Value::is_object)
                    && self.owns_active_prompt(&args.session_id)
            })
            .map(str::to_owned);
        // Untyped workflow requests must not be treated as tool approvals.
        if self.reject_untyped_workflow_permissions
            && gemini_mcp.is_none()
            && gemini_skill.is_none()
            && qwen_mcp.is_none()
            && matches!(
                args.tool_call.fields.kind,
                None | Some(acp::ToolKind::Other | acp::ToolKind::SwitchMode)
            )
        {
            return Err(acp::Error::invalid_params().data("Agent workflow confirmation requires explicit user interaction; untyped tools cannot be automatically approved"));
        }
        if let Some(name) = &qwen_mcp {
            args.tool_call.fields.title = Some(name.clone());
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
        let managed_mcp = self.managed_mcp_approval.as_ref().zip(
            args.tool_call.fields.title.as_deref().filter(|name| name.starts_with("mcp__")),
        );
        let resolve = if gemini_skill.is_some()
            || gemini_mcp.is_some()
            || qwen_mcp.is_some()
            || managed_mcp.is_some()
            || self.transient_permissions
            || self.desktop_workspace.is_some()
        {
            resolve_transient_permission_request
        } else {
            resolve_permission_request
        };
        // A native skill confirmation exposes instructions and resources to the model.
        // Preserve explicit confirmation even when other tools can be auto-approved.
        let mode = if gemini_skill.is_some() && self.permission_mode == PermissionMode::ApproveAll {
            PermissionMode::ApproveReads
        } else {
            self.permission_mode
        };
        let response = if let Some(confirm) = &self.permission_confirmation
            && self.non_interactive_permissions.is_none()
        {
            match resolve(&args, mode, Some(NonInteractivePermissionPolicy::Fail)) {
                Ok(response) => response,
                Err(_) => {
                    let has_allow_once = args
                        .options
                        .iter()
                        .any(|option| option.kind == acp::PermissionOptionKind::AllowOnce);
                    let approved = has_allow_once
                        && confirm(
                            args.tool_call
                                .fields
                                .title
                                .clone()
                                .unwrap_or_else(|| "ACP tool".into()),
                            args.tool_call.fields.raw_input.clone().unwrap_or(Value::Null),
                        )
                        .await;
                    if self.cancelling_session_ids.lock().contains(args.session_id.0.as_ref()) {
                        cancelled_permission_response()
                    } else {
                        crate::permissions::resolve_host_confirmation(&args, approved)
                    }
                }
            }
        } else {
            resolve(&args, mode, self.non_interactive_permissions)
                .map_err(acp::Error::into_internal_error)?
        };
        if let Some((grant, title)) = managed_mcp
            && matches!(classify_permission_decision(&args, &response), PermissionDecision::Approved)
            && (!self.owns_active_prompt(&args.session_id)
                || !args.tool_call.fields.raw_input.as_ref().is_some_and(|input| grant(title, input)))
        {
            self.permission_stats.lock().cancelled += 1;
            return Ok(cancelled_permission_response());
        }
        if qwen_mcp.is_some()
            && matches!(
                classify_permission_decision(&args, &response),
                PermissionDecision::Approved
            )
            && !self.owns_active_prompt(&args.session_id)
        {
            self.permission_stats.lock().cancelled += 1;
            return Ok(cancelled_permission_response());
        }
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
        if self.sandboxed_native_io {
            return Err(acp::Error::method_not_found());
        }
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
        if self.sandboxed_native_io || self.managed_workspace_io {
            return Err(acp::Error::method_not_found());
        }
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
        if self.sandboxed_native_io || self.managed_workspace_io {
            return Err(acp::Error::method_not_found());
        }
        if (self.desktop_workspace.is_some() && !self.owns_active_prompt(&args.session_id))
            || (self.permission_confirmation.is_some()
                && self.expected_session_id.lock().as_deref() != Some(args.session_id.0.as_ref()))
        {
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

fn resolve_transient_permission_request(
    request: &acp::RequestPermissionRequest,
    mode: crate::PermissionMode,
    non_interactive_policy: Option<crate::NonInteractivePermissionPolicy>,
) -> Result<acp::RequestPermissionResponse, crate::errors::PermissionPromptUnavailableError> {
    crate::permissions::resolve_permission_request(
        &crate::access_policy::opencode::transient_request(request),
        mode,
        non_interactive_policy,
    )
}
