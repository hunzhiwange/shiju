//! 协议控制的期望值、确认与 prompt 屏障；不推测供应商权限语义。
use super::*;

#[cfg(not(test))]
const CONTROL_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(test)]
const CONTROL_TIMEOUT: Duration = Duration::from_millis(100);

impl AcpClient {
    pub(super) fn ensure_session_controls_ready(&self, session_id: &str) -> Result<(), AcpError> {
        if self.cancelling_session_ids.lock().contains(session_id) {
            return Err(AcpError::Prompt(
                "ACP session cancellation is in progress; retry later".into(),
            ));
        }
        if self.actor_state.lock().pending_controls.keys().any(|(id, _)| id == session_id) {
            return Err(AcpError::Prompt(
                "ACP session control is not confirmed; retry the failed control before prompting"
                    .into(),
            ));
        }
        Ok(())
    }

    pub(super) async fn confirmed_config_control(
        &self,
        conn: &acp::ClientSideConnection,
        session_id: &str,
        option_name: &str,
        value_id: &str,
    ) -> Result<acp::SetSessionConfigOptionResponse, AcpError> {
        if self.agent_name == "hermes" && option_name == "edit_approval_policy" {
            let mode = if value_id == "ask" { "default" } else { "dont_ask" };
            self.guard_hermes_mode(session_id, mode).await?;
        }
        if option_name == "mode" {
            self.guard_native_mode(session_id, value_id).await?;
        }
        let key = (session_id.to_string(), option_name.to_string());
        self.actor_state.lock().pending_controls.insert(key.clone(), value_id.to_string());
        let response = timeout(
            CONTROL_TIMEOUT,
            conn.set_session_config_option(acp::SetSessionConfigOptionRequest::new(
                session_id.to_string(),
                option_name.to_string(),
                acp::SessionConfigValueId::new(value_id.to_string()),
            )),
        )
        .await
        .map_err(|_| {
            AcpError::SetSessionConfigOption(
                "session/set_config_option timed out; retry before prompting".into(),
            )
        })?
        .map_err(|err| {
            wrap_session_control_error(
                "session/set_config_option",
                Some(format!(r#"for "{}"="{}""#, option_name, value_id)),
                err,
                AcpError::SetSessionConfigOption,
            )
        })?;
        let confirmed = response.config_options.iter().any(|option| {
            option.id.0.as_ref() == option_name && matches!(&option.kind,
                acp::SessionConfigKind::Select(select) if select.current_value.0.as_ref() == value_id)
        });
        if !confirmed {
            return Err(AcpError::SetSessionConfigOption("session/set_config_option did not confirm the requested value; retry before prompting".into()));
        }
        let mut state = self.actor_state.lock();
        if state.pending_controls.get(&key).is_some_and(|desired| desired == value_id) {
            state.pending_controls.remove(&key);
            if let Some(info) = state.session_capabilities.as_mut()
                && info.session_id == session_id
            {
                info.config_options = Some(response.config_options.clone());
            }
        }
        Ok(response)
    }

    pub(super) async fn confirmed_mode_control(
        &self,
        conn: &acp::ClientSideConnection,
        session_id: &str,
        mode_id: &str,
    ) -> Result<(), AcpError> {
        self.guard_native_mode(session_id, mode_id).await?;
        let key = (session_id.to_string(), "session/set_mode".to_string());
        self.actor_state.lock().pending_controls.insert(key.clone(), mode_id.to_string());
        let capabilities = self
            .actor_state
            .lock()
            .session_capabilities
            .clone()
            .filter(|info| info.session_id == session_id);
        // A published config surface is authoritative; rejection must not fall back to modes.
        if let Some(options) = capabilities.as_ref().and_then(|info| info.config_options.as_ref())
            && let Some(option) = options
                .iter()
                .find(|option| option.category == Some(acp::SessionConfigOptionCategory::Mode))
        {
            crate::session_mode_preference::resolve_config_value(
                Some(options),
                &option.id.to_string(),
                mode_id,
            )
            .map_err(AcpError::SetSessionConfigOption)?;
            self.confirmed_config_control(conn, session_id, &option.id.to_string(), mode_id)
                .await?;
            self.actor_state.lock().pending_controls.remove(&key);
            return Ok(());
        }
        if !capabilities.as_ref().and_then(|info| info.modes.as_ref()).is_some_and(|modes| {
            modes.available_modes.iter().any(|mode| mode.id.0.as_ref() == mode_id)
        }) {
            return Err(AcpError::Prompt("Agent did not advertise the requested session mode; retry with a supported control".into()));
        }
        timeout(
            CONTROL_TIMEOUT,
            conn.set_session_mode(acp::SetSessionModeRequest::new(
                session_id.to_string(),
                mode_id.to_string(),
            )),
        )
        .await
        .map_err(|_| AcpError::Prompt("session/set_mode timed out; retry before prompting".into()))?
        .map_err(|err| AcpError::Prompt(err.to_string()))?;
        self.actor_state.lock().pending_controls.remove(&key);
        // The empty response acknowledges the request only. CurrentModeUpdate is the
        // readback authority; never assign current_mode_id from the desired value.
        Ok(())
    }
}

impl AcpClient {
    async fn guard_native_mode(&self, session_id: &str, mode_id: &str) -> Result<(), AcpError> {
        self.guard_hermes_mode(session_id, mode_id).await?;
        self.guard_qoder_mode(session_id, mode_id).await?;
        self.guard_vibe_mode(session_id, mode_id).await?;
        if self.agent_name == "gemini" {
            if !crate::access_policy::gemini::restricted_mode(mode_id) {
                return Err(AcpError::Prompt("Gemini native approval modes lack confirmed revocation and change tools or prompts; use client approvals".into()));
            }
            if self.has_active_prompt() {
                self.cancel(session_id).await?;
                return Err(AcpError::Prompt("Gemini mode changes cannot revoke an active turn; cancellation requested, wait for completion before retrying".into()));
            }
        }
        if self.agent_name == "claude" {
            self.actor_state.lock().claude_confirmed_session = None;
            if mode_id == crate::access_policy::claude::FULL_MODE
                && !(self.full_access && self.permission_mode == PermissionMode::ApproveAll)
            {
                return Err(AcpError::Prompt(
                    "Claude bypass requires the current host grant; host deny cannot be overridden"
                        .into(),
                ));
            }
            if self.has_active_prompt() {
                self.cancel(session_id).await?;
                return Err(AcpError::Prompt("Claude mode changes cannot revoke an active turn; cancellation requested, wait for completion before retrying".into()));
            }
        }
        if self.agent_name == "qwen" {
            self.actor_state.lock().qwen_confirmed_session = None;
            if mode_id == crate::access_policy::qwen::FULL_MODE
                && !self.actor_state.lock().qwen_runtime_verified
            {
                return Err(AcpError::Prompt(
                    "Qwen native access requires a verified runtime and confirmed config control"
                        .into(),
                ));
            }
            if mode_id == crate::access_policy::qwen::FULL_MODE
                && !(self.full_access && self.permission_mode == PermissionMode::ApproveAll)
            {
                return Err(AcpError::Prompt(
                    "Qwen bypass requires the current host grant; host deny cannot be overridden"
                        .into(),
                ));
            }
            if self.has_active_prompt() {
                self.cancel(session_id).await?;
                return Err(AcpError::Prompt("Qwen mode changes cannot revoke an active turn; cancellation requested, wait for completion before retrying".into()));
            }
        }
        self.guard_codex_mode(session_id, mode_id).await
    }

    pub(super) async fn sync_native_access(
        &self,
        conn: &acp::ClientSideConnection,
        session_id: &str,
    ) -> Result<(), AcpError> {
        self.sync_codex_access(conn, session_id).await?;
        self.sync_claude_access(conn, session_id).await?;
        self.sync_qwen_access(conn, session_id).await?;
        self.sync_qoder_access(conn, session_id).await?;
        self.sync_hermes_access(conn, session_id).await?;
        self.ensure_vibe_access(session_id)?;
        if self.agent_name == "gemini" {
            let state = self.actor_state.lock();
            let restricted = state.session_capabilities.as_ref().is_some_and(|info| {
                info.session_id == session_id
                    && crate::access_policy::gemini::restricted_session(
                        info.modes.as_ref(),
                        info.config_options.as_deref(),
                    )
            });
            if !restricted {
                return Err(AcpError::Prompt("Cannot confirm restricted Gemini permissions; native revocation is unsupported, restore default mode before prompting".into()));
            }
        }
        Ok(())
    }

    async fn guard_codex_mode(&self, session_id: &str, mode_id: &str) -> Result<(), AcpError> {
        if self.agent_name != "codex" {
            return Ok(());
        }
        self.actor_state.lock().codex_confirmed_session = None;
        if mode_id == crate::access_policy::codex::FULL_MODE
            && !(self.full_access && self.permission_mode == PermissionMode::ApproveAll)
        {
            return Err(AcpError::Prompt(
                "Codex full access requires the current host grant; host deny cannot be overridden"
                    .into(),
            ));
        }
        if self.has_active_prompt() {
            self.cancel(session_id).await?;
            return Err(AcpError::Prompt("Codex mode changes cannot revoke an active turn; cancellation requested, wait for completion before retrying".into()));
        }
        Ok(())
    }

    pub(super) async fn sync_codex_access(
        &self,
        conn: &acp::ClientSideConnection,
        session_id: &str,
    ) -> Result<(), AcpError> {
        use crate::access_policy::codex::{FULL_MODE, RESTRICTED_MODE, advertises_mode};
        let capabilities = {
            let mut state = self.actor_state.lock();
            if !state.codex_runtime_verified {
                let elevated = state.session_capabilities.as_ref().is_some_and(|info| {
                    info.session_id == session_id
                        && (crate::access_policy::codex::current_mode(
                            info.config_options.as_deref(),
                        ) == Some(FULL_MODE)
                            || info
                                .modes
                                .as_ref()
                                .is_some_and(|modes| modes.current_mode_id.0.as_ref() == FULL_MODE))
                });
                if self.agent_name == "codex"
                    && elevated
                    && !(self.full_access && self.permission_mode == PermissionMode::ApproveAll)
                {
                    state.codex_confirmed_session = None;
                    state
                        .pending_controls
                        .insert((session_id.into(), "mode".into()), RESTRICTED_MODE.into());
                    return Err(AcpError::Prompt("Cannot revoke full access from an unverified Codex runtime; use a reviewed runtime before prompting".into()));
                }
                return Ok(());
            }
            state.codex_confirmed_session = None;
            state.session_capabilities.clone().filter(|info| info.session_id == session_id)
        };
        let desired = if self.full_access && self.permission_mode == PermissionMode::ApproveAll {
            FULL_MODE
        } else {
            RESTRICTED_MODE
        };
        if !advertises_mode(
            capabilities.as_ref().and_then(|info| info.config_options.as_deref()),
            desired,
        ) {
            self.actor_state
                .lock()
                .pending_controls
                .insert((session_id.into(), "mode".into()), desired.into());
            return Err(AcpError::Prompt("Verified Codex runtime did not advertise its required permission control; refusing to prompt".into()));
        }
        self.confirmed_config_control(conn, session_id, "mode", desired).await?;
        if desired == FULL_MODE {
            self.actor_state.lock().codex_confirmed_session = Some(session_id.into());
        }
        Ok(())
    }
    pub(super) async fn sync_claude_access(
        &self,
        conn: &acp::ClientSideConnection,
        session_id: &str,
    ) -> Result<(), AcpError> {
        use crate::access_policy::claude::{FULL_MODE, RESTRICTED_MODE, advertises_mode};
        let capabilities = {
            let mut state = self.actor_state.lock();
            if !state.claude_runtime_verified {
                let elevated = state.session_capabilities.as_ref().is_some_and(|info| {
                    info.session_id == session_id
                        && (crate::access_policy::claude::current_mode(
                            info.config_options.as_deref(),
                        ) == Some(FULL_MODE)
                            || info
                                .modes
                                .as_ref()
                                .is_some_and(|modes| modes.current_mode_id.0.as_ref() == FULL_MODE))
                });
                if self.agent_name == "claude"
                    && elevated
                    && !(self.full_access && self.permission_mode == PermissionMode::ApproveAll)
                {
                    state.claude_confirmed_session = None;
                    state
                        .pending_controls
                        .insert((session_id.into(), "mode".into()), RESTRICTED_MODE.into());
                    return Err(AcpError::Prompt("Cannot revoke full access from an unverified Claude runtime; use a reviewed runtime before prompting".into()));
                }
                return Ok(());
            }
            state.claude_confirmed_session = None;
            state.session_capabilities.clone().filter(|info| info.session_id == session_id)
        };
        // Planning changes tools and prompts. Permission intent must not exit it.
        if crate::access_policy::claude::current_mode(
            capabilities.as_ref().and_then(|info| info.config_options.as_deref()),
        ) == Some("plan")
        {
            if self.full_access && self.permission_mode == PermissionMode::ApproveAll {
                self.actor_state
                    .lock()
                    .pending_controls
                    .insert((session_id.into(), "mode".into()), FULL_MODE.into());
                return Err(AcpError::Prompt("Claude full access cannot replace plan mode; explicitly leave planning before retrying".into()));
            }
            return Ok(());
        }
        let desired = if self.full_access && self.permission_mode == PermissionMode::ApproveAll {
            FULL_MODE
        } else {
            RESTRICTED_MODE
        };
        if !advertises_mode(
            capabilities.as_ref().and_then(|info| info.config_options.as_deref()),
            desired,
        ) {
            self.actor_state
                .lock()
                .pending_controls
                .insert((session_id.into(), "mode".into()), desired.into());
            return Err(AcpError::Prompt("Verified Claude runtime did not advertise its required permission control; refusing to prompt".into()));
        }
        self.confirmed_config_control(conn, session_id, "mode", desired).await?;
        if desired == FULL_MODE {
            self.actor_state.lock().claude_confirmed_session = Some(session_id.into());
        }
        Ok(())
    }
    pub(super) async fn sync_qwen_access(
        &self,
        conn: &acp::ClientSideConnection,
        session_id: &str,
    ) -> Result<(), AcpError> {
        use crate::access_policy::qwen::{FULL_MODE, RESTRICTED_MODE, advertises_mode};
        let capabilities = {
            let mut state = self.actor_state.lock();
            if !state.qwen_runtime_verified {
                let elevated = state.session_capabilities.as_ref().is_some_and(|info| {
                    info.session_id == session_id
                        && (crate::access_policy::qwen::current_mode(
                            info.config_options.as_deref(),
                        ) == Some(FULL_MODE)
                            || info
                                .modes
                                .as_ref()
                                .is_some_and(|modes| modes.current_mode_id.0.as_ref() == FULL_MODE))
                });
                if self.agent_name == "qwen" && elevated {
                    state.qwen_confirmed_session = None;
                    state
                        .pending_controls
                        .insert((session_id.into(), "mode".into()), RESTRICTED_MODE.into());
                    return Err(AcpError::Prompt("Cannot revoke full access from an unverified Qwen runtime; use a reviewed runtime before prompting".into()));
                }
                return Ok(());
            }
            state.qwen_confirmed_session = None;
            state.session_capabilities.clone().filter(|info| info.session_id == session_id)
        };
        // Planning changes tools and prompts. Permission intent must not exit it.
        if crate::access_policy::qwen::current_mode(
            capabilities.as_ref().and_then(|info| info.config_options.as_deref()),
        ) == Some("plan")
        {
            if self.full_access && self.permission_mode == PermissionMode::ApproveAll {
                self.actor_state
                    .lock()
                    .pending_controls
                    .insert((session_id.into(), "mode".into()), FULL_MODE.into());
                return Err(AcpError::Prompt("Qwen full access cannot replace plan mode; explicitly leave planning before retrying".into()));
            }
            return Ok(());
        }
        let desired = if self.full_access && self.permission_mode == PermissionMode::ApproveAll {
            FULL_MODE
        } else {
            RESTRICTED_MODE
        };
        if !advertises_mode(
            capabilities.as_ref().and_then(|info| info.config_options.as_deref()),
            desired,
        ) {
            self.actor_state
                .lock()
                .pending_controls
                .insert((session_id.into(), "mode".into()), desired.into());
            return Err(AcpError::Prompt("Verified Qwen runtime did not advertise its required permission control; refusing to prompt".into()));
        }
        let response = self.confirmed_config_control(conn, session_id, "mode", desired).await?;
        let previous_model = capabilities
            .as_ref()
            .and_then(|info| info.config_options.as_ref())
            .and_then(|options| options.iter().find(|option| option.id.0.as_ref() == "model"));
        let current_model =
            response.config_options.iter().find(|option| option.id.0.as_ref() == "model");
        if previous_model != current_model {
            self.actor_state
                .lock()
                .pending_controls
                .insert((session_id.into(), "mode".into()), desired.into());
            return Err(AcpError::Prompt(
                "Qwen permission control changed model configuration; refusing to prompt".into(),
            ));
        }
        if desired == FULL_MODE {
            self.actor_state.lock().qwen_confirmed_session = Some(session_id.into());
        }
        Ok(())
    }
}
