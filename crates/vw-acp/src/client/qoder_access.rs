//! Apply Qoder's advertised ACP permission control without changing model routing.
use super::*;
use crate::access_policy::qoder::{
    FULL_MODE, RESTRICTED_MODE, advertises_mode, current_mode, restricted_mode,
};

impl AcpClient {
    pub(super) async fn guard_qoder_mode(
        &self,
        session_id: &str,
        mode_id: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "qoder" {
            return Ok(());
        }
        if !restricted_mode(mode_id)
            && !(self.full_access && self.permission_mode == PermissionMode::ApproveAll)
        {
            return Err(AcpError::Prompt(
                "Qoder elevated permissions require the current host full access grant".into(),
            ));
        }
        if self.has_active_prompt() {
            self.cancel(session_id).await?;
            return Err(AcpError::Prompt("Qoder mode changes cannot revoke an active turn; cancellation requested, wait for completion before retrying".into()));
        }
        Ok(())
    }

    pub(super) async fn sync_qoder_access(
        &self,
        conn: &acp::ClientSideConnection,
        session_id: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "qoder" {
            return Ok(());
        }
        let capabilities = self
            .actor_state
            .lock()
            .session_capabilities
            .clone()
            .filter(|info| info.session_id == session_id);
        let options = capabilities.as_ref().and_then(|info| info.config_options.as_deref());
        let config_mode = current_mode(options);
        let legacy_mode = capabilities
            .as_ref()
            .and_then(|info| info.modes.as_ref())
            .map(|modes| modes.current_mode_id.0.as_ref());
        let unrestricted = self.full_access && self.permission_mode == PermissionMode::ApproveAll;
        // Closing access must preserve planning; opening it cannot silently exit planning.
        if config_mode.or(legacy_mode) == Some("plan") {
            if unrestricted {
                return self.block_qoder_access(session_id, "Qoder full access cannot replace plan mode; explicitly leave planning before retrying");
            }
            return Ok(());
        }
        let can_restrict = advertises_mode(options, RESTRICTED_MODE);
        let can_enable = can_restrict && advertises_mode(options, FULL_MODE);
        let desired = if unrestricted && can_enable { FULL_MODE } else { RESTRICTED_MODE };
        if !can_restrict {
            if [config_mode, legacy_mode].into_iter().flatten().any(|mode| !restricted_mode(mode)) {
                return self.block_qoder_access(session_id, "Qoder session has elevated permissions but no advertised default config control; restore default mode before prompting");
            }
            // Older or custom entries retain normal ACP client approvals.
            return Ok(());
        }
        let response = self.confirmed_config_control(conn, session_id, "mode", desired).await?;
        let previous_model = options
            .and_then(|options| options.iter().find(|option| option.id.0.as_ref() == "model"));
        let current_model =
            response.config_options.iter().find(|option| option.id.0.as_ref() == "model");
        if previous_model != current_model {
            return self.block_qoder_access(
                session_id,
                "Qoder permission control changed model configuration; refusing to prompt",
            );
        }
        Ok(())
    }

    fn block_qoder_access(&self, session_id: &str, reason: &str) -> Result<(), AcpError> {
        self.actor_state
            .lock()
            .pending_controls
            .insert((session_id.into(), "mode".into()), RESTRICTED_MODE.into());
        Err(AcpError::Prompt(reason.into()))
    }
}
