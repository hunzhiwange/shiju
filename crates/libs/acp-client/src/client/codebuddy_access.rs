//! Apply CodeBuddy's advertised ACP permission control without changing model routing.
use super::*;
use crate::access_policy::codebuddy::{
    FULL_MODE, RESTRICTED_MODE, advertises_mode, current_mode, restricted_mode,
};

impl AcpClient {
    fn managed_codebuddy_linux(&self) -> bool {
        cfg!(all(
            target_os = "linux",
            target_env = "gnu",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )) && self.config.env.contains_key("VW_ACP_LAUNCH_REFERENCE")
    }

    fn check_codebuddy_host_permissions(&self) -> Result<(), AcpError> {
        if self.managed_codebuddy_linux()
            && (self.full_access || self.permission_mode != PermissionMode::ApproveReads)
        {
            return Err(AcpError::Prompt(
                "Managed CodeBuddy on Linux requires restricted permissions; native tools cannot enforce the requested host policy".into(),
            ));
        }
        Ok(())
    }

    pub(super) async fn guard_codebuddy_mode(
        &self,
        session_id: &str,
        mode_id: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "codebuddy" {
            return Ok(());
        }
        if self.managed_codebuddy_linux() && !matches!(mode_id, RESTRICTED_MODE | "plan") {
            return Err(AcpError::Prompt(
                "Managed CodeBuddy on Linux only supports default and plan permission modes".into(),
            ));
        }
        if !restricted_mode(mode_id)
            && !(self.full_access && self.permission_mode == PermissionMode::ApproveAll)
        {
            return Err(AcpError::Prompt(
                "CodeBuddy elevated permissions require the current host full access grant".into(),
            ));
        }
        if self.has_active_prompt() {
            self.cancel(session_id).await?;
            return Err(AcpError::Prompt("CodeBuddy mode changes cannot revoke an active turn; cancellation requested, wait for completion before retrying".into()));
        }
        Ok(())
    }

    pub(super) async fn sync_codebuddy_access(
        &self,
        conn: &acp::ClientSideConnection,
        session_id: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "codebuddy" {
            return Ok(());
        }
        self.check_codebuddy_host_permissions()?;
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
        let managed_linux = self.managed_codebuddy_linux();
        let can_restrict = advertises_mode(options, RESTRICTED_MODE);
        if managed_linux && (!can_restrict || config_mode.is_none()) {
            return self.block_codebuddy_access(session_id, "Managed CodeBuddy did not report a verifiable default mode control; refusing to prompt");
        }
        let unrestricted = self.full_access && self.permission_mode == PermissionMode::ApproveAll;
        // Closing access must preserve planning; opening it cannot silently exit planning.
        if config_mode.or(legacy_mode) == Some("plan") {
            if unrestricted {
                return self.block_codebuddy_access(session_id, "CodeBuddy full access cannot replace plan mode; explicitly leave planning before retrying");
            }
            if !managed_linux || legacy_mode.is_none_or(restricted_mode) {
                return Ok(());
            }
            return self.block_codebuddy_access(
                session_id,
                "CodeBuddy reported conflicting plan and elevated modes; refusing to prompt",
            );
        }
        let can_enable = can_restrict && advertises_mode(options, FULL_MODE);
        let desired = if managed_linux {
            RESTRICTED_MODE
        } else if unrestricted && can_enable {
            FULL_MODE
        } else {
            config_mode.filter(|mode| restricted_mode(mode)).unwrap_or(RESTRICTED_MODE)
        };
        if !can_restrict {
            if [config_mode, legacy_mode].into_iter().flatten().any(|mode| !restricted_mode(mode)) {
                return self.block_codebuddy_access(session_id, "CodeBuddy session has elevated permissions but no advertised default config control; restore default mode before prompting");
            }
            // Older or custom entries retain normal ACP client approvals.
            return Ok(());
        }
        let response = self.confirmed_config_control(conn, session_id, "mode", desired).await?;
        let previous_settings: Vec<_> = options
            .unwrap_or_default()
            .iter()
            .filter(|option| option.id.0.as_ref() != "mode")
            .collect();
        let current_settings: Vec<_> = response
            .config_options
            .iter()
            .filter(|option| option.id.0.as_ref() != "mode")
            .collect();
        if previous_settings != current_settings {
            return self.block_codebuddy_access(
                session_id,
                "CodeBuddy permission control changed model or thinking configuration; refusing to prompt",
            );
        }
        Ok(())
    }

    fn block_codebuddy_access(&self, session_id: &str, reason: &str) -> Result<(), AcpError> {
        self.actor_state
            .lock()
            .pending_controls
            .insert((session_id.into(), "mode".into()), RESTRICTED_MODE.into());
        Err(AcpError::Prompt(reason.into()))
    }
}

#[cfg(all(
    test,
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[path = "codebuddy_access_tests.rs"]
mod tests;
