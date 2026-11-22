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
        let native_io = !self.sandboxed_native_io();
        let mut capabilities = acp::ClientCapabilities::new()
            .fs(acp::FileSystemCapabilities::new()
                .read_text_file(native_io && !self.managed_workspace_io())
                .write_text_file(native_io))
            .terminal(native_io && !self.managed_workspace_io());
        if self.agent_name == "nova" {
            capabilities = capabilities
                .meta(Map::from_iter([("compass.owner-approval.v1".into(), json!(true))]));
        }
        if self.agent_name == "autohand" {
            // The host owns workspace preparation and verification. Native ACP
            // permission callbacks remain active; this does not select yolo mode.
            capabilities = capabilities
                .meta(Map::from_iter([("autohandWorkspaceSetup".into(), json!("agent-managed"))]));
        }
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
        if self.agent_name == "nova" {
            let caps = serde_json::to_value(&response.agent_capabilities)
                .map_err(|e| AcpError::Initialize(e.to_string()))?;
            let initialization =
                serde_json::to_value(&response).map_err(|e| AcpError::Initialize(e.to_string()))?;
            if caps["_meta"]["compass.owner-approval.v1"] != true
                || initialization["_meta"]["ownerApprovalRequired"] != true
            {
                return Err(AcpError::Initialize(
                    "Nova lacks mandatory owner approval capability".into(),
                ));
            }
        }
        if self.agent_name.eq_ignore_ascii_case("codex")
            && self.skill_mounts.as_ref().is_some_and(|selection| !selection.skills.is_empty())
        {
            let capabilities = serde_json::to_value(&response.agent_capabilities)
                .map_err(|error| AcpError::Initialize(error.to_string()))?;
            if capabilities["_meta"][crate::SESSION_SKILL_MOUNTS_META_KEY]["version"] != 1
                || capabilities["_meta"][crate::SESSION_SKILL_MOUNTS_META_KEY]["processScoped"]
                    != true
            {
                return Err(AcpError::Initialize(
                    "Codex adapter has no confirmed native skill mount capability".into(),
                ));
            }
        }
        self.actor_state.lock().agent_version = response.agent_info.as_ref().map(|info| {
            info.version
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'))
                .take(80)
                .collect()
        });
        self.actor_state.lock().codex_runtime_verified = self.agent_name == "codex"
            && cfg!(all(target_os = "macos", target_arch = "aarch64"))
            && response.agent_info.as_ref().is_some_and(|info| {
                info.name == "@agentclientprotocol/codex-acp" && info.version == "2.5.0"
            })
            && ["CODEX_PATH", "NODE_OPTIONS", "NODE_PATH"].into_iter().all(|key| {
                self.config.env.get(key).map_or_else(
                    || std::env::var_os(key).is_none_or(|value| value.is_empty()),
                    |value| value.is_empty(),
                )
            })
            && crate::verify_reviewed_agent_executable("codex", &self.config.command).is_ok();
        self.actor_state.lock().codex_confirmed_session = None;
        self.actor_state.lock().claude_runtime_verified = self.agent_name == "claude"
            && cfg!(all(target_os = "macos", target_arch = "aarch64"))
            && response.agent_info.as_ref().is_some_and(|info| {
                info.name == "@agentclientprotocol/claude-agent-acp" && info.version == "0.26.0"
            })
            && ["CLAUDE_CODE_EXECUTABLE", "NODE_OPTIONS", "NODE_PATH"].into_iter().all(|key| {
                self.config.env.get(key).map_or_else(
                    || std::env::var_os(key).is_none_or(|value| value.is_empty()),
                    |value| value.is_empty(),
                )
            })
            && !self.config.env.contains_key("CLAUDE_AGENT_ACP_IS_SINGLE_FILE_BUN")
            && std::env::var_os("CLAUDE_AGENT_ACP_IS_SINGLE_FILE_BUN").is_none()
            && crate::access_policy::claude::verified_entry(
                &self.config.command,
                &self.config.args,
            );
        self.actor_state.lock().claude_confirmed_session = None;
        self.actor_state.lock().qwen_runtime_verified = self.agent_name == "qwen"
            && cfg!(all(target_os = "macos", target_arch = "aarch64"))
            && response
                .agent_info
                .as_ref()
                .is_some_and(|info| info.name == "qwen-code" && info.version == "0.26.7")
            && [
                "QWEN_CODE_RELAUNCH_ARGS",
                "QWEN_CODE_LAUNCHER_PATH",
                "QWEN_CODE_STARTUP_VERSION",
                "NODE_OPTIONS",
                "NODE_PATH",
            ]
            .into_iter()
            .all(|key| {
                self.config.env.get(key).map_or_else(
                    || std::env::var_os(key).is_none_or(|value| value.is_empty()),
                    |value| value.is_empty(),
                )
            })
            && crate::access_policy::qwen::verified_entry(&self.config.command, &self.config.args);
        self.actor_state.lock().qwen_confirmed_session = None;
        // Managed Claude authenticates exclusively through its request-owned relay.
        if managed_claude_settings(&self.config).is_none() {
            self.authenticate_if_required(conn, &response.auth_methods).await?;
        }
        Ok(())
    }

    pub(super) async fn resolve_session(
        &self,
        conn: &acp::ClientSideConnection,
        cwd: &Path,
        session_strategy: &SessionStrategy,
        expected_session_id: &Arc<Mutex<Option<String>>>,
    ) -> Result<String, AcpError> {
        match session_strategy {
            SessionStrategy::New => {}
            SessionStrategy::Load(id)
            | SessionStrategy::Resume(id)
            | SessionStrategy::ResumeOrLoad(id)
            | SessionStrategy::ResumeLoadOrNew(id)
            | SessionStrategy::LoadOrNew(id) => self.ensure_session_controls_ready(id)?,
        }
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
            // A mount selection (including explicit revocation) must restore the
            // original history. A failed restore is never permission to create
            // a replacement session with a different native skill collection.
            SessionStrategy::ResumeLoadOrNew(session_id) if self.skill_mounts.is_some() => {
                self.resolve_existing_session(conn, cwd, session_id.clone(), expected_session_id)
                    .await?;
                session_id.clone()
            }
            SessionStrategy::LoadOrNew(session_id) if self.skill_mounts.is_some() => {
                self.load_session_id(conn, cwd, session_id.clone()).await?
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
        if self.cancelling_session_ids.lock().contains(&session_id) {
            return Err(AcpError::Prompt(
                "ACP session cancellation is in progress; retry later".into(),
            ));
        }
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

    pub(super) fn validate_linux_iflow_mcp(&self) -> Result<(), AcpError> {
        if !self.managed_linux_iflow() || self.mcp_servers.is_empty() {
            return Ok(());
        }
        let projected = (|| {
            let [acp::McpServer::Http(server)] = self.mcp_servers.as_slice() else {
                return None;
            };
            let origin = self.config.env.get("SHIJU_IFLOW_MCP_RELAY")?;
            let token = self.config.env.get("SHIJU_IFLOW_MCP_TOKEN")?;
            let address = origin.strip_prefix("http://")?.parse::<std::net::SocketAddr>().ok()?;
            let source = server.meta.as_ref()?.get("shiju.iflow.mcp-v4")?.as_str()?;
            let [auth] = server.headers.as_slice() else { return None };
            (self.managed_mcp_approval.is_some()
                && self.permission_confirmation.is_some()
                && self.non_interactive_permissions.is_none()
                && self.permission_mode != PermissionMode::DenyAll
                && !self.full_access
                && (address.ip() == std::net::Ipv4Addr::LOCALHOST
                    || address.ip() == std::net::Ipv6Addr::LOCALHOST)
                && !matches!(address.port(), 0 | 80)
                && *origin == format!("http://{address}")
                && matches!(source, "project" | "mounted")
                && !server.name.is_empty()
                && server.name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                && server.url == format!("{origin}/shiju-vtcode-mcp/{}", server.name)
                && !token.is_empty()
                && auth.name.eq_ignore_ascii_case("authorization")
                && auth.value == format!("Bearer {token}"))
                .then_some(())
        })();
        projected.ok_or_else(|| AcpError::NewSession(
            "iFlow Linux requires an active reviewed host MCP projection".into(),
        ))
    }

    fn has_reviewed_linux_http_projection(&self, vtcode: bool) -> bool {
        if !cfg!(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))
            || self.managed_mcp_approval.is_none()
            || self.permission_confirmation.is_none()
            || self.non_interactive_permissions.is_some()
            || self.permission_mode == PermissionMode::DenyAll
            || self.full_access
        {
            return false;
        }
        let (origin, model_token) = if vtcode {
            ("http://127.0.0.1:1", None)
        } else {
            let Some(origin) = self.config.env.get("ACP_GLM_BASE_URL")
                .and_then(|url| url.strip_suffix("/v1")) else {
                return false;
            };
            let Some(address) = origin.strip_prefix("http://")
                .and_then(|value| value.parse::<std::net::SocketAddrV4>().ok()) else {
                return false;
            };
            let Some(model_token) = self.config.env.get("Z_AI_API_KEY").filter(|token| !token.is_empty()) else {
                return false;
            };
            if *address.ip() != std::net::Ipv4Addr::LOCALHOST
                || matches!(address.port(), 0 | 80)
                || origin != format!("http://{address}")
            {
                return false;
            }
            (origin, Some(model_token.as_str()))
        };
        let mut names = std::collections::HashSet::new();
        self.mcp_servers.iter().all(|server| {
            let acp::McpServer::Http(server) = server else { return false };
            let [header] = server.headers.as_slice() else { return false };
            let Some(token) = header.value.strip_prefix("Bearer ") else { return false };
            server.name.len() <= 64
                && server.name.bytes().any(|byte| byte.is_ascii_alphanumeric())
                && server.name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                && names.insert(server.name.clone())
                && server.url == format!("{origin}/shiju-vtcode-mcp/{}", server.name)
                && header.name.eq_ignore_ascii_case("authorization")
                && token.len() == 64
                && token.bytes().all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                && model_token != Some(token)
        })
    }

    fn validate_mcp_contract(&self) -> Result<(), AcpError> {
        let backend = acp_agent_registry::agent_registry::desktop_permission_agent(&self.agent_name);
        self.validate_linux_iflow_mcp()?;
        let vtcode_linux = self.vtcode_linux_sandbox.is_some() && self.managed_vtcode();
        let glm_linux = cfg!(target_os = "linux")
            && backend == Some("glm-acp-agent")
            && self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE");
        if !self.mcp_servers.is_empty()
            && ((backend.is_some_and(acp_agent_registry::linux_native_only) && !vtcode_linux)
                || (cfg!(target_os = "linux") && matches!(backend, Some("amp-acp" | "pi")))
                || ((vtcode_linux || glm_linux) && !self.has_reviewed_linux_http_projection(vtcode_linux)))
        {
            return Err(AcpError::NewSession(
                "Session MCP has no verified Linux launch boundary for this agent".into(),
            ));
        }
        if !self.mcp_servers.is_empty() && (vtcode_linux || glm_linux) {
            if vtcode_linux {
                acp_agent_registry::verify_vtcode_mcp_executable(&self.config.command)
            } else {
                crate::verify_reviewed_agent_executable("glm-acp-agent", &self.config.command)
            }
            .map_err(AcpError::NewSession)?;
        }
        if self.agent_name == "glm-acp-agent"
            && self.mcp_servers.iter().any(|server| {
                let meta = match server {
                    acp::McpServer::Stdio(server) => server.meta.as_ref(),
                    acp::McpServer::Http(server) => server.meta.as_ref(),
                    acp::McpServer::Sse(server) => server.meta.as_ref(),
                    _ => None,
                };
                meta.is_some_and(|meta| meta.contains_key("shiju.glm.toolPolicy"))
            })
        {
            if self.permission_mode == PermissionMode::DenyAll {
                return Err(AcpError::NewSession(
                    "GLM session MCP is denied by the host permission policy".into(),
                ));
            }
            crate::verify_reviewed_agent_executable(&self.agent_name, &self.config.command)
                .map_err(AcpError::NewSession)?;
        }
        if self.agent_name == "sigit" && !self.mcp_servers.is_empty() {
            if !cfg!(target_os = "macos")
                || self.config.args != ["--acp"]
                || !self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
                || self.mcp_servers.iter().any(|server| {
                    !matches!(server, acp::McpServer::Stdio(stdio) if stdio.command.is_absolute() && stdio.env.is_empty())
                })
            {
                return Err(AcpError::Initialize(
                    "siGit does not support MCP without a managed macOS session-model connection and absolute stdio servers without environment overrides".into(),
                ));
            }
            crate::verify_reviewed_agent_executable(&self.agent_name, &self.config.command)
                .map_err(AcpError::Initialize)?;
        }
        if self.agent_name == "nova" && !self.mcp_servers.is_empty() {
            if self.permission_mode == PermissionMode::DenyAll
                || self.config.args != ["acp"]
                || !self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
                || self.mcp_servers.iter().any(|server| {
                    !matches!(server, acp::McpServer::Stdio(stdio) if stdio.command.is_absolute() && stdio.env.is_empty())
                })
            {
                return Err(AcpError::NewSession(
                    "Nova MCP requires an approved managed session-model connection and absolute stdio servers without environment overrides".into(),
                ));
            }
            crate::verify_reviewed_agent_executable(&self.agent_name, &self.config.command)
                .map_err(AcpError::NewSession)?;
        }
        if self.agent_name == "dirac" && !self.mcp_servers.is_empty() {
            let [flag, config_flag, directory] = self.config.args.as_slice() else {
                return Err(AcpError::Prompt(
                    "Dirac does not consume ACP MCP servers without the reviewed session-model runtime".into(),
                ));
            };
            if flag != "--acp"
                || config_flag != "--config"
                || !Path::new(directory).is_absolute()
                || self.config.env.get("DIRAC_DIR") != Some(directory)
                || self.config.env.get("DIRAC_DATA_DIR") != Some(directory)
                || !self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
                || self.permission_mode == PermissionMode::DenyAll
            {
                return Err(AcpError::NewSession(
                    "Dirac MCP requires an approved managed session-model runtime".into(),
                ));
            }
            if self.mcp_servers.iter().any(|server| {
                !matches!(server, acp::McpServer::Stdio(stdio) if stdio.command.is_absolute() && stdio.env.is_empty())
            }) {
                return Err(AcpError::NewSession(
                    "Dirac MCP requires absolute stdio servers without environment overrides".into(),
                ));
            }
            crate::verify_reviewed_agent_executable("dirac", &self.config.command)
                .map_err(AcpError::NewSession)?;
        }
        if self.agent_name == "dimcode"
            && self.mcp_servers.iter().any(|server| matches!(server, acp::McpServer::Stdio(_)))
        {
            return Err(AcpError::Prompt(
                "DimCode does not support ACP stdio MCP; select an HTTP or SSE server".into(),
            ));
        }
        if self.agent_name == "dimcode"
            && (!self.mcp_servers.is_empty() || self.config.env.contains_key("SHIJU_ACP_DIMCODE_SKILL_ROOT"))
        {
            if self.config.env.contains_key("SHIJU_ACP_DIMCODE_SKILL_ROOT")
                && (self.skill_mount_lease.is_none()
                    || self.skill_mounts.as_ref().is_none_or(|selection| selection.skills.is_empty()))
            {
                return Err(AcpError::NewSession("DimCode skill mount requires a live host selection".into()));
            }
            if self.permission_mode == PermissionMode::DenyAll
                || self.config.args != ["acp"]
                || !self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
                || self.mcp_servers.iter().any(|server| {
                    !matches!(server, acp::McpServer::Http(_) | acp::McpServer::Sse(_))
                })
            {
                return Err(AcpError::NewSession(
                    "DimCode MCP requires an approved managed session-model connection over HTTP or SSE".into(),
                ));
            }
            crate::verify_reviewed_agent_executable(&self.agent_name, &self.config.command)
                .map_err(AcpError::NewSession)?;
        }
        Ok(())
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
        self.validate_mcp_contract()?;
        let mut request = acp::NewSessionRequest::new(self.protocol_workspace(cwd)).mcp_servers(self.mcp_servers.clone());
        if let Some(meta) = self.mounted_session_meta() {
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
        if self.agent_name == "nova"
            && serde_json::to_value(&session).map_err(|e| AcpError::NewSession(e.to_string()))?["_meta"]
                ["ownerApprovalRequired"]
                != true
        {
            return Err(AcpError::NewSession("Nova session did not confirm owner approval".into()));
        }
        self.actor_state.lock().iflow_acknowledged_mode = None;
        self.actor_state.lock().session_capabilities = Some(crate::SessionCapabilities {
            session_id: session.session_id.to_string(),
            modes: session.modes.clone(),
            models: session.models.clone(),
            config_options: session.config_options.clone(),
        });
        self.sync_native_access(conn, &session.session_id.to_string()).await?;
        Ok(SessionInfo {
            session_id: session.session_id.0.to_string(),
            models: session.models,
            config_options: self
                .actor_state
                .lock()
                .session_capabilities
                .as_ref()
                .and_then(|info| info.config_options.clone()),
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
        self.validate_mcp_contract()?;
        let mut request = acp::LoadSessionRequest::new(session_id.clone(), self.protocol_workspace(cwd))
            .mcp_servers(self.mcp_servers.clone());
        if (managed_claude_settings(&self.config).is_some()
            || self.skill_mounts.is_some()
            || matches!(self.agent_name.as_str(), "kiro" | "nova"))
            && let Some(meta) = self.mounted_session_meta()
        {
            request = request.meta(meta);
        }
        let response = conn
            .load_session(request)
            .await
            .map_err(|err| AcpError::LoadSession(err.to_string()))?;
        if self.agent_name == "nova"
            && serde_json::to_value(&response).map_err(|e| AcpError::LoadSession(e.to_string()))?["_meta"]["conversationHistoryLength"].as_u64().unwrap_or(0) == 0
        {
            return Err(AcpError::LoadSession("Nova did not restore a recorded conversation; refusing its empty-session fallback".into()));
        }
        self.actor_state.lock().iflow_acknowledged_mode = None;
        self.actor_state.lock().session_capabilities = Some(crate::SessionCapabilities {
            session_id: session_id.clone(),
            modes: response.modes.clone(),
            models: response.models.clone(),
            config_options: response.config_options.clone(),
        });
        self.sync_native_access(conn, &session_id).await?;
        Ok(SessionInfo {
            session_id,
            models: response.models,
            config_options: self
                .actor_state
                .lock()
                .session_capabilities
                .as_ref()
                .and_then(|info| info.config_options.clone()),
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
        // Restore Kiro and Nova through load so the current client approval
        // contract accompanies restoration.
        if matches!(self.agent_name.as_str(), "kiro" | "nova") {
            return self.load_session_info(conn, cwd, session_id).await;
        }
        self.validate_mcp_contract()?;
        let mut request = acp::ResumeSessionRequest::new(session_id.clone(), self.protocol_workspace(cwd))
            .mcp_servers(self.mcp_servers.clone());
        if (managed_claude_settings(&self.config).is_some() || self.skill_mounts.is_some())
            && let Some(meta) = self.mounted_session_meta()
        {
            request = request.meta(meta);
        }
        let response = conn
            .resume_session(request)
            .await
            .map_err(|err| AcpError::ResumeSession(err.to_string()))?;
        self.actor_state.lock().iflow_acknowledged_mode = None;
        self.actor_state.lock().session_capabilities = Some(crate::SessionCapabilities {
            session_id: session_id.clone(),
            modes: response.modes.clone(),
            models: response.models.clone(),
            config_options: response.config_options.clone(),
        });
        self.sync_native_access(conn, &session_id).await?;
        Ok(SessionInfo {
            session_id,
            models: response.models,
            config_options: self
                .actor_state
                .lock()
                .session_capabilities
                .as_ref()
                .and_then(|info| info.config_options.clone()),
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
