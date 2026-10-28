//! ACP 协议级会话调用。
//!
//! 本模块集中处理 initialize、session/new、session/load、session/resume 以及
//! 会话策略解析。actor 只负责何时调用这些操作，以及如何维护运行时生命周期。

use super::*;

impl AcpClient {
    pub(super) async fn initialize_connection(
        &self,
        conn: &acp::ClientSideConnection,
    ) -> Result<(), AcpError> {
        let capabilities = acp::ClientCapabilities::new()
            .fs(acp::FileSystemCapabilities::new().read_text_file(true).write_text_file(true))
            .terminal(true);
        let response = conn.initialize(
            acp::InitializeRequest::new(acp::ProtocolVersion::V1)
                .client_info(
                    acp::Implementation::new(self.client_name.clone(), self.client_version.clone())
                        .title("VibeWindow ACP Client"),
                )
                .client_capabilities(capabilities),
        );
        let response = if self.is_gemini_acp_command() {
            timeout(resolve_gemini_acp_startup_timeout(), response)
                .await
                .map_err(|_| {
                    AcpError::GeminiStartupTimeout(build_gemini_acp_startup_timeout_message(
                        &self.config.command,
                    ))
                })?
                .map_err(|err| AcpError::Initialize(err.to_string()))?
        } else {
            response.await.map_err(|err| AcpError::Initialize(err.to_string()))?
        };
        self.authenticate_if_required(conn, &response.auth_methods).await?;
        Ok(())
    }

    pub(super) async fn resolve_session(
        &self,
        conn: &acp::ClientSideConnection,
        cwd: &Path,
        session_strategy: &SessionStrategy,
        expected_session_id: &Arc<Mutex<Option<String>>>,
    ) -> Result<String, AcpError> {
        let session_id = match session_strategy {
            // actor 已验证连接健康且 cwd 未变。活跃会话的历史仍在对端，
            // 重复 resume/load 不仅多余，还可能失败或重放旧输出。
            SessionStrategy::ResumeOrLoad(session_id)
            | SessionStrategy::ResumeLoadOrNew(session_id)
            | SessionStrategy::LoadOrNew(session_id)
                if self.has_reusable_session(session_id)
                    && expected_session_id.lock().as_deref() == Some(session_id.as_str()) =>
            {
                session_id.clone()
            }
            SessionStrategy::New => self.new_session_id(conn, cwd).await?,
            SessionStrategy::Load(session_id) => {
                self.load_session_id(conn, cwd, session_id.clone()).await?
            }
            SessionStrategy::Resume(session_id) => {
                self.resume_session_id(conn, cwd, session_id.clone()).await?
            }
            SessionStrategy::ResumeOrLoad(session_id) => {
                self.resolve_existing_session(conn, cwd, session_id.clone(), expected_session_id)
                    .await?;
                session_id.clone()
            }
            SessionStrategy::ResumeLoadOrNew(session_id) => {
                match self.resume_session_id(conn, cwd, session_id.clone()).await {
                    Ok(session_id) => session_id,
                    Err(_) => match self.load_session_id(conn, cwd, session_id.clone()).await {
                        Ok(session_id) => session_id,
                        Err(_) => self.new_session_id(conn, cwd).await?,
                    },
                }
            }
            SessionStrategy::LoadOrNew(session_id) => {
                match self.load_session_id(conn, cwd, session_id.clone()).await {
                    Ok(session_id) => session_id,
                    Err(_) => self.new_session_id(conn, cwd).await?,
                }
            }
        };

        *expected_session_id.lock() = Some(session_id.clone());
        Ok(session_id)
    }

    pub(super) async fn resolve_existing_session(
        &self,
        conn: &acp::ClientSideConnection,
        cwd: &Path,
        session_id: String,
        expected_session_id: &Arc<Mutex<Option<String>>>,
    ) -> Result<(), AcpError> {
        if self.has_reusable_session(&session_id)
            && expected_session_id.lock().as_deref() == Some(session_id.as_str())
        {
            return Ok(());
        }
        match self.resume_session_id(conn, cwd, session_id.clone()).await {
            Ok(resolved) => {
                *expected_session_id.lock() = Some(resolved);
                Ok(())
            }
            Err(resume_error) => {
                let resolved = self.load_session_id(conn, cwd, session_id.clone()).await.map_err(
                    |load_error| {
                        AcpError::LoadSession(format!(
                            "Could not restore ACP session {session_id}: \
                             session/resume failed: {resume_error}; \
                             session/load failed: {load_error}. \
                             No replacement session was created; retry or explicitly start a new session."
                        ))
                    },
                )?;
                *expected_session_id.lock() = Some(resolved);
                Ok(())
            }
        }
    }

    pub(super) async fn new_session_id(
        &self,
        conn: &acp::ClientSideConnection,
        cwd: &Path,
    ) -> Result<String, AcpError> {
        Ok(self.new_session_info(conn, cwd).await?.session_id)
    }

    pub(super) async fn new_session_info(
        &self,
        conn: &acp::ClientSideConnection,
        cwd: &Path,
    ) -> Result<SessionInfo, AcpError> {
        let mut request = acp::NewSessionRequest::new(cwd).mcp_servers(self.mcp_servers.clone());
        if let Some(meta) = build_session_options_meta(self.session_options.as_ref()) {
            request = request.meta(meta);
        }
        let session = if self.is_claude_acp_command() {
            timeout(resolve_claude_acp_session_create_timeout(), conn.new_session(request))
                .await
                .map_err(|_| {
                    AcpError::ClaudeSessionCreateTimeout(
                        build_claude_acp_session_create_timeout_message(),
                    )
                })?
                .map_err(|err| AcpError::NewSession(err.to_string()))?
        } else {
            conn.new_session(request).await.map_err(|err| AcpError::NewSession(err.to_string()))?
        };
        Ok(SessionInfo {
            session_id: session.session_id.0.to_string(),
            models: session.models,
            config_options: session.config_options,
        })
    }

    pub(super) async fn load_session_id(
        &self,
        conn: &acp::ClientSideConnection,
        cwd: &Path,
        session_id: String,
    ) -> Result<String, AcpError> {
        Ok(self.load_session_info(conn, cwd, session_id).await?.session_id)
    }

    pub(super) async fn load_session_info(
        &self,
        conn: &acp::ClientSideConnection,
        cwd: &Path,
        session_id: String,
    ) -> Result<SessionInfo, AcpError> {
        let response = conn
            .load_session(
                acp::LoadSessionRequest::new(session_id.clone(), cwd)
                    .mcp_servers(self.mcp_servers.clone()),
            )
            .await
            .map_err(|err| AcpError::LoadSession(err.to_string()))?;
        Ok(SessionInfo {
            session_id,
            models: response.models,
            config_options: response.config_options,
        })
    }

    pub(super) async fn resume_session_id(
        &self,
        conn: &acp::ClientSideConnection,
        cwd: &Path,
        session_id: String,
    ) -> Result<String, AcpError> {
        Ok(self.resume_session_info(conn, cwd, session_id).await?.session_id)
    }

    pub(super) async fn resume_session_info(
        &self,
        conn: &acp::ClientSideConnection,
        cwd: &Path,
        session_id: String,
    ) -> Result<SessionInfo, AcpError> {
        let response = conn
            .resume_session(
                acp::ResumeSessionRequest::new(session_id.clone(), cwd)
                    .mcp_servers(self.mcp_servers.clone()),
            )
            .await
            .map_err(|err| AcpError::ResumeSession(err.to_string()))?;
        Ok(SessionInfo {
            session_id,
            models: response.models,
            config_options: response.config_options,
        })
    }

    pub(crate) fn is_gemini_acp_command(&self) -> bool {
        basename_token(&self.config.command) == "gemini"
            && self.config.args.iter().any(|arg| arg == "--acp" || arg == "--experimental-acp")
    }

    pub(crate) fn is_claude_acp_command(&self) -> bool {
        let command_token = basename_token(&self.config.command);
        command_token == "claude-agent-acp"
            || self.config.args.iter().any(|arg| arg.contains("claude-agent-acp"))
    }
}
