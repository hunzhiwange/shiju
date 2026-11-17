//! DimCode starts read-only. Enable only host-mediated workspace writes, never native bypass.
use super::*;

impl AcpClient {
    pub(super) async fn guard_dimcode_permission(
        &self,
        session: &str,
        value: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "dimcode" {
            return Ok(());
        }
        if value != "read-only"
            && !(value == "workspace-write"
                && self.full_access
                && self.permission_mode == PermissionMode::ApproveAll)
        {
            return Err(AcpError::Prompt("DimCode requires host full access for workspace writes; native bypass is not verified".into()));
        }
        if self.has_active_prompt() {
            self.cancel(session).await?;
            return Err(AcpError::Prompt(
                "DimCode permission changes require completion of the cancelled turn".into(),
            ));
        }
        Ok(())
    }

    pub(super) async fn sync_dimcode_access(
        &self,
        conn: &acp::ClientSideConnection,
        session: &str,
    ) -> Result<(), AcpError> {
        if self.agent_name != "dimcode" {
            return Ok(());
        }
        let info = self
            .actor_state
            .lock()
            .session_capabilities
            .clone()
            .filter(|info| info.session_id == session);
        let options =
            info.as_ref().and_then(|info| info.config_options.as_deref()).unwrap_or_default();
        let desired = if self.full_access && self.permission_mode == PermissionMode::ApproveAll {
            "workspace-write"
        } else {
            "read-only"
        };
        let advertised = options.iter().any(|option| option.id.0.as_ref() == "permission" && matches!(&option.kind,
            acp::SessionConfigKind::Select(select) if matches!(&select.options,
                acp::SessionConfigSelectOptions::Ungrouped(values) if values.iter().any(|v| v.value.0.as_ref() == desired))));
        if !advertised {
            return Err(AcpError::Prompt(
                "DimCode has no verified permission control; refusing to prompt".into(),
            ));
        }
        let response = self.confirmed_config_control(conn, session, "permission", desired).await?;
        let old_model = options.iter().find(|option| option.id.0.as_ref() == "model");
        let new_model =
            response.config_options.iter().find(|option| option.id.0.as_ref() == "model");
        if old_model != new_model {
            self.actor_state
                .lock()
                .pending_controls
                .insert((session.into(), "permission".into()), "read-only".into());
            return Err(AcpError::Prompt(
                "DimCode permission control changed model selection; refusing to prompt".into(),
            ));
        }
        Ok(())
    }
}
