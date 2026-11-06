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
