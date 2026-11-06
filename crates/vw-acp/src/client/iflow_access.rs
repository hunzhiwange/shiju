//! Follow the current host grant instead of iFlow's default YOLO setting.
use super::*;
use crate::access_policy::iflow::{FULL_MODE, RESTRICTED_MODE, restricted_mode};

impl AcpClient {
    pub(super) async fn guard_iflow_mode(
        &self,
        session_id: &str,
        mode_id: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "iflow" {
            return Ok(());
        }
        if !restricted_mode(mode_id)
            && !(self.full_access && self.permission_mode == PermissionMode::ApproveAll)
        {
            return Err(AcpError::Prompt(
                "iFlow automatic approvals require the current host full access grant".into(),
            ));
        }
        if self.has_active_prompt() {
            self.cancel(session_id).await?;
            return Err(AcpError::Prompt("iFlow mode changes cannot revoke an active turn; cancellation requested, wait for completion before retrying".into()));
        }
        Ok(())
    }

    pub(super) async fn sync_iflow_access(
        &self,
        conn: &acp::ClientSideConnection,
        session_id: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "iflow" {
            return Ok(());
        }
        let capabilities = self.session_capabilities(session_id);
        // Planning changes the task contract. Keep an explicitly selected plan
        // instead of silently replacing it with a tool-execution mode.
        let acknowledged = self.actor_state.lock().iflow_acknowledged_mode.clone();
        let current_mode = acknowledged
            .as_ref()
            .filter(|(id, _)| id == session_id)
            .map(|(_, mode)| mode.as_str())
            .or_else(|| {
                capabilities
                    .as_ref()
                    .and_then(|info| info.modes.as_ref())
                    .map(|modes| modes.current_mode_id.0.as_ref())
            });
        if current_mode == Some("plan") {
            return Ok(());
        }
        let desired = if self.full_access && self.permission_mode == PermissionMode::ApproveAll {
            FULL_MODE
        } else {
            RESTRICTED_MODE
        };
        // Reapply after new/load/resume and before every prompt: the legacy
        // response is an acknowledgement, not a CurrentModeUpdate readback.
        self.confirmed_mode_control(conn, session_id, desired).await
    }
}
