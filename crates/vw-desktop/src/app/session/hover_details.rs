//! Read server-owned session metadata only when its sidebar details are requested.
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::vw_api_types::chat::SessionAgentIdentity;

#[derive(Debug, Clone, Default)]
pub struct SessionHoverDetails {
    pub(crate) primary_workspace: Option<bool>,
    pub(crate) agent: Option<SessionAgentIdentity>,
}

impl App {
    pub(crate) fn load_session_hover_details(
        &mut self,
        session_id: String,
        project_id: String,
        directory: String,
    ) -> Task<Message> {
        if !self.session_ui_projection.hover_details_loading.insert(session_id.clone()) {
            return Task::none();
        }
        Task::perform(
            async move {
                let mut details = SessionHoverDetails::default();
                if let Ok(client) = crate::app::gateway_client() {
                    let scope = vw_gateway_client::vw_api_types::id::ProjectScope::new(
                        client.endpoint().tenant_scope().clone(),
                        project_id.into(),
                    );
                    let (project, summary) = tokio::join!(
                        async {
                            let scope = scope.map_err(|error| error.to_string())?;
                            client.project_get(&scope).await
                        },
                        client.session_ui_summary(&session_id, Some(&directory)),
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
            },
            |(id, details)| Message::SessionDetailsLoaded(id, details),
        )
    }
}
