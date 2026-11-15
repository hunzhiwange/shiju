//! 按需读取服务端会话悬浮详情。
use vw_api_chat_identity::SessionAgentIdentity;
#[derive(Debug, Clone, Default)]
pub struct SessionHoverDetails {
    pub primary_workspace: Option<bool>,
    pub agent: Option<SessionAgentIdentity>,
}

pub async fn load_session_hover_details(client: Option<vw_gateway_client::GatewayClient>, session_id: String, project_id: String, directory: String) -> (String, SessionHoverDetails) {
    let mut details = SessionHoverDetails::default();
    if let Some(client) = client {
        let scope = vw_api_transport::ProjectScope::new(
            client.endpoint().tenant_scope().clone(),
            project_id.into(),
        );
        let session_client = gateway_session::GatewaySessionClient::new(client.transport().clone());
        let (project, summary) = tokio::join!(
            async {
                let scope = scope.map_err(|error| error.to_string())?;
                gateway_project::GatewayProjectClient::new(client.transport().clone()).project_get(&scope).await
            },
            session_client.session_ui_summary(&session_id, Some(&directory)),
        );
        if let Ok(response) = project
            && response.project.git.is_repo
        {
            details.primary_workspace = Some(
                std::path::Path::new(&response.project.directory)
                    == std::path::Path::new(&directory),
            );
        }
        if let Ok(summary) = summary {
            details.agent = summary.agent_binding;
        }
    }
    (session_id, details)
}
