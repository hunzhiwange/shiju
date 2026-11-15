//! Synchronize Goose ACP server approvals without editing global CLI/provider config.
use super::*;
use crate::access_policy::goose::{
    RESTRICTED_MODE, advertises_mode, current_mode, restricted_mode,
};

impl AcpClient {
    pub(super) async fn guard_goose_mode(
        &self,
        session_id: &str,
        mode_id: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "goose" {
            return Ok(());
        }
        if !restricted_mode(mode_id) {
            return Err(AcpError::Prompt(
                "Goose native automatic modes can bypass tool deny rules; use approve mode with client approvals".into(),
            ));
        }
        if self.has_active_prompt() {
            self.cancel(session_id).await?;
            return Err(AcpError::Prompt("Goose mode changes cannot revoke an active turn; cancellation requested, wait for completion before retrying".into()));
        }
        Ok(())
    }

    pub(super) async fn sync_goose_access(
        &self,
        conn: &acp::ClientSideConnection,
        session_id: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "goose" {
            return Ok(());
        }
        let capabilities = self
            .actor_state
            .lock()
            .session_capabilities
            .clone()
            .filter(|info| info.session_id == session_id);
        let options = capabilities.as_ref().and_then(|info| info.config_options.as_deref());
        let modes = capabilities.as_ref().and_then(|info| info.modes.as_ref());
        let config_mode = current_mode(options);
        let legacy_mode = modes.map(|modes| modes.current_mode_id.0.as_ref());
        // Chat suppresses tools; a permission toggle must not silently enable them.
        if config_mode.or(legacy_mode) == Some("chat") {
            return Ok(());
        }
        let has_config_mode = config_mode.is_some();
        let advertises = |value: &str| {
            if has_config_mode {
                advertises_mode(options, value)
            } else {
                modes.is_some_and(|modes| {
                    modes.available_modes.iter().any(|mode| mode.id.0.as_ref() == value)
                })
            }
        };
        if !advertises(RESTRICTED_MODE) {
            if [config_mode, legacy_mode].into_iter().flatten().any(|mode| !restricted_mode(mode)) {
                return self.block_goose_access(session_id, "Goose elevated approvals cannot be revoked without an advertised approve control; restore approve mode before prompting");
            }
            // Older/custom entries retain client approvals without invented options.
            return Ok(());
        }
        // Auto ignores NeverAllow user rules. Keep native denies and let the
        // current host permission mode resolve individual ACP approval requests.
        let desired = RESTRICTED_MODE;
        if config_mode.or(legacy_mode) == Some(desired)
            && legacy_mode.is_none_or(|mode| mode == desired)
        {
            return Ok(());
        }
        if !has_config_mode {
            return self.confirmed_mode_control(conn, session_id, desired).await;
        }
        let response = self.confirmed_config_control(conn, session_id, "mode", desired).await?;
        if !crate::access_policy::goose::same_selected_settings(
            options.unwrap_or_default(),
            &response.config_options,
        ) {
            return self.block_goose_access(session_id, "Goose permission control changed provider, model or thinking configuration; refusing to prompt");
        }
        Ok(())
    }

    fn block_goose_access(&self, session_id: &str, reason: &str) -> Result<(), AcpError> {
        self.actor_state
            .lock()
            .pending_controls
            .insert((session_id.into(), "mode".into()), RESTRICTED_MODE.into());
        Err(AcpError::Prompt(reason.into()))
    }
}
