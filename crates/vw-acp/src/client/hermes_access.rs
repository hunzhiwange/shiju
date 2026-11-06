//! Synchronize Hermes edit approval modes without changing models or terminal policy.
use super::*;
use crate::access_policy::hermes::{FULL_MODE, RESTRICTED_MODE};

impl AcpClient {
    pub(super) async fn guard_hermes_mode(
        &self,
        session_id: &str,
        mode_id: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "hermes" {
            return Ok(());
        }
        if mode_id != RESTRICTED_MODE
            && !(self.full_access && self.permission_mode == PermissionMode::ApproveAll)
        {
            return Err(AcpError::Prompt(
                "Hermes automatic edit approvals require the current host full access grant".into(),
            ));
        }
        if self.has_active_prompt() {
            self.cancel(session_id).await?;
            return Err(AcpError::Prompt("Hermes mode changes cannot revoke an active turn; cancellation requested, wait for completion before retrying".into()));
        }
        Ok(())
    }

    pub(super) async fn sync_hermes_access(
        &self,
        conn: &acp::ClientSideConnection,
        session_id: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "hermes" {
            return Ok(());
        }
        let capabilities = self
            .actor_state
            .lock()
            .session_capabilities
            .clone()
            .filter(|info| info.session_id == session_id);
        let modes = capabilities.as_ref().and_then(|info| info.modes.as_ref());
        let advertises = |value: &str| {
            modes.is_some_and(|modes| {
                modes.available_modes.iter().any(|mode| mode.id.0.as_ref() == value)
            })
        };
        let can_restrict = advertises(RESTRICTED_MODE);
        let full_access = self.full_access && self.permission_mode == PermissionMode::ApproveAll;
        if !can_restrict {
            if !full_access
                && modes.is_some_and(|modes| modes.current_mode_id.0.as_ref() != RESTRICTED_MODE)
            {
                self.actor_state
                    .lock()
                    .pending_controls
                    .insert((session_id.into(), "session/set_mode".into()), RESTRICTED_MODE.into());
                return Err(AcpError::Prompt("Hermes elevated edit approvals cannot be revoked without an advertised default mode".into()));
            }
            // Older/custom agents keep normal client approvals without invented options.
            return Ok(());
        }
        let desired =
            if full_access && advertises(FULL_MODE) { FULL_MODE } else { RESTRICTED_MODE };
        // 0.19.0 acknowledges set_mode without CurrentModeUpdate. Reapply before
        // each prompt and after new/load/resume; never fake a current-mode readback.
        self.confirmed_mode_control(conn, session_id, desired).await
    }
}
