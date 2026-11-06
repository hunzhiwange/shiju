//! Keep Vibe profile selection explicit while applying the current host grant.
use super::*;

impl AcpClient {
    pub(super) async fn guard_vibe_mode(
        &self,
        session_id: &str,
        mode_id: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "vibe" {
            return Ok(());
        }
        if mode_id == crate::access_policy::vibe::FULL_MODE
            && !(self.full_access && self.permission_mode == PermissionMode::ApproveAll)
        {
            return Err(AcpError::Prompt(
                "Vibe auto-approve requires the current host full access grant".into(),
            ));
        }
        if self.has_active_prompt() {
            self.cancel(session_id).await?;
            return Err(AcpError::Prompt(
                "Vibe agent changes require the active turn to finish; cancellation requested, retry after completion".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn ensure_vibe_access(&self, session_id: &str) -> Result<(), AcpError> {
        if self.agent_name != "vibe"
            || (self.full_access && self.permission_mode == PermissionMode::ApproveAll)
        {
            return Ok(());
        }
        let state = self.actor_state.lock();
        if state.session_capabilities.as_ref().is_some_and(|info| {
            info.session_id == session_id
                && crate::access_policy::vibe::has_auto_approve(
                    info.modes.as_ref(),
                    info.config_options.as_deref(),
                )
        }) {
            // Replacing an agent can also change its instructions and tools. Require
            // explicit selection instead of silently replacing a restored profile.
            return Err(AcpError::Prompt(
                "Vibe auto-approve no longer has host authorization; explicitly select an approval agent such as ask before continuing".into(),
            ));
        }
        Ok(())
    }
}
