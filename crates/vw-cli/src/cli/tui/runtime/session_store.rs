//! Session persistence operations required by the active TUI host.

use serde::Deserialize;
use vw_gateway_client::GatewaySessionCreateBody;
use vw_shared::session::ui_types::{ChatSession, ChatSessionMeta};

use super::gateway::{
    GatewayUiRuntime, annotate_gateway_transport_error, block_on_gateway,
    normalize_optional_str_ref,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct UiSessionCreateInfo {
    pub(crate) id: String,
    pub(crate) title: String,
}

impl GatewayUiRuntime {
    async fn session_create(
        &self,
        title: Option<&str>,
    ) -> Result<UiSessionCreateInfo, String> {
        let directory = self
            .directory_value()
            .unwrap_or_else(|| self.directory().to_string_lossy().to_string());
        let body = Some(GatewaySessionCreateBody {
            parent_id: None,
            title: normalize_optional_str_ref(title).map(ToOwned::to_owned),
        });
        self.client()
            .session_create(directory.as_str(), &body)
            .await
            .map_err(|error| annotate_gateway_transport_error(error, self.endpoint()))
    }

    async fn session_ui_load_any(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ChatSession>, String> {
        let session_id = self.resolve_session_id(session_id)?;
        self.client().session_ui_get_any(session_id).await
    }

    async fn session_ui_save(&self, session: &ChatSession) -> Result<(), String> {
        let session_id = normalize_optional_str_ref(Some(session.id.as_str()))
            .ok_or_else(|| "session_ui save requires a non-empty session id".to_string())?;
        let directory = self.directory_value();
        self.client().session_ui_save(session_id, directory.as_deref(), session).await.map(|_| ())
    }

    async fn session_ui_previews(&self) -> Result<Vec<ChatSessionMeta>, String> {
        let directory = self.directory_value();
        self.client().session_ui_previews(directory.as_deref()).await
    }

    async fn session_scope_get(&self) -> Result<Option<String>, String> {
        self.client().session_scope_get(None, None).await
    }

    pub(crate) fn session_create_blocking(
        &self,
        title: Option<&str>,
    ) -> Result<UiSessionCreateInfo, String> {
        block_on_gateway(self.session_create(title))
    }

    pub(crate) fn session_ui_load_any_blocking(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ChatSession>, String> {
        block_on_gateway(self.session_ui_load_any(session_id))
    }

    pub(crate) fn session_ui_save_blocking(&self, session: &ChatSession) -> Result<(), String> {
        block_on_gateway(self.session_ui_save(session))
    }

    pub(crate) fn session_ui_previews_blocking(&self) -> Result<Vec<ChatSessionMeta>, String> {
        block_on_gateway(self.session_ui_previews())
    }

    pub(crate) fn session_scope_get_blocking(&self) -> Result<Option<String>, String> {
        block_on_gateway(self.session_scope_get())
    }
}
