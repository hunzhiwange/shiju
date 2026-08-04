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
        AcpEventClient {
            expected_session_id,
            event_tx,
            filesystem: FileSystemHandlers::new(FileSystemHandlersOptions {
                cwd: cwd.to_path_buf(),
                permission_mode: self.permission_mode,
                full_access: self.full_access,
                non_interactive_permissions: self.non_interactive_permissions,
                on_operation: on_operation.clone(),
                confirm_write: None,
            }),
            terminal_manager: TerminalManager::new(TerminalManagerOptions {
                cwd: cwd.to_path_buf(),
                permission_mode: self.permission_mode,
                non_interactive_permissions: self.non_interactive_permissions,
                on_operation,
                confirm_execute: None,
                kill_grace_ms: None,
            }),
            permission_mode: self.permission_mode,
            non_interactive_permissions: self.non_interactive_permissions,
            on_session_update: self.on_session_update.clone(),
            permission_stats: self.permission_stats.clone(),
            cancelling_session_ids: self.cancelling_session_ids.clone(),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for AcpEventClient {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        if self.cancelling_session_ids.lock().contains(args.session_id.0.as_ref()) {
            self.permission_stats.lock().cancelled += 1;
            return Ok(cancelled_permission_response());
        }
        self.permission_stats.lock().requested += 1;
        let response = resolve_permission_request(
            &args,
            self.permission_mode,
            self.non_interactive_permissions,
        )
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
        self.filesystem
            .write_text_file(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> acp::Result<acp::ReadTextFileResponse> {
        self.filesystem
            .read_text_file(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn create_terminal(
        &self,
        args: acp::CreateTerminalRequest,
    ) -> acp::Result<acp::CreateTerminalResponse> {
        self.terminal_manager
            .create_terminal(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn terminal_output(
        &self,
        args: acp::TerminalOutputRequest,
    ) -> acp::Result<acp::TerminalOutputResponse> {
        self.terminal_manager
            .terminal_output(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn release_terminal(
        &self,
        args: acp::ReleaseTerminalRequest,
    ) -> acp::Result<acp::ReleaseTerminalResponse> {
        self.terminal_manager
            .release_terminal(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn wait_for_terminal_exit(
        &self,
        args: acp::WaitForTerminalExitRequest,
    ) -> acp::Result<acp::WaitForTerminalExitResponse> {
        self.terminal_manager
            .wait_for_terminal_exit(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn kill_terminal(
        &self,
        args: acp::KillTerminalRequest,
    ) -> acp::Result<acp::KillTerminalResponse> {
        self.terminal_manager
            .kill_terminal(&args)
            .await
            .map_err(|err| map_client_error(err, &self.permission_stats))
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
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
