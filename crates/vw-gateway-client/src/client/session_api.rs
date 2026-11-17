//! 会话 API 兼容转发。

use serde::de::DeserializeOwned;


use super::GatewayClient;
use crate::session::{
    GatewaySessionCreateBody, GatewaySessionDiffQuery, GatewaySessionForkBody,
    GatewaySessionMessageDeltaResponse, GatewaySessionMessageListQuery, GatewaySessionMessagePage,
    GatewaySessionMessagePageQuery, GatewaySessionPatchBody, GatewaySessionResetBody,
    GatewaySessionSummarizeBody, GatewaySessionTitleGenerateBody,
    GatewaySessionTitleGenerateResponse, GatewaySessionTodoPutBody, GatewaySessionUiSummary,
};
use session_ui_types::{
    ChatSession, ChatSessionMeta, SessionMessageDelta, SessionMessageRecord,
};
use todo_types::Todo;

pub use gateway_session::SessionUiLoadResult;

impl GatewayClient {
    pub async fn acp_model_source_capabilities(
        &self,
        agent: &str,
    ) -> Result<vw_api_acp_model::acp_session::AcpSessionOptions, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .acp_model_source_capabilities(agent).await
    }

    pub async fn acp_session_model_source(
        &self,
        session_id: &str,
        directory: Option<&str>,
        update: &vw_api_acp_model::acp_model_source::AcpModelSourceUpdate,
    ) -> Result<vw_api_acp_model::acp_session::AcpSessionOptions, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .acp_session_model_source(session_id, directory, update).await
    }

    pub async fn acp_session_options(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<vw_api_acp_model::acp_session::AcpSessionOptions, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .acp_session_options(session_id, directory).await
    }

    pub async fn session_set_full_access(
        &self,
        session_id: &str,
        directory: Option<&str>,
        full_access: bool,
    ) -> Result<bool, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_set_full_access(session_id, directory, full_access).await
    }

    pub async fn acp_session_select(
        &self,
        session_id: &str,
        directory: Option<&str>,
        selection: &vw_api_acp_model::acp_session::AcpSessionSelection,
    ) -> Result<vw_api_acp_model::acp_session::AcpSessionOptions, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .acp_session_select(session_id, directory, selection).await
    }

    pub async fn chat_stream_stop(&self, request_id: &str) -> Result<bool, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .chat_stream_stop(request_id).await
    }

    pub async fn session_ui_get_with_raw(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<SessionUiLoadResult>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_ui_get_with_raw(session_id, directory).await
    }

    pub async fn session_ui_get_without_raw(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<SessionUiLoadResult>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_ui_get_without_raw(session_id, directory).await
    }

    pub async fn session_ui_get_any_with_raw(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionUiLoadResult>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_ui_get_any_with_raw(session_id).await
    }

    pub async fn session_list<T: DeserializeOwned>(
        &self,
        directory: Option<&str>,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_list(directory).await
    }

    pub async fn session_list_for_project<T: DeserializeOwned>(
        &self,
        project_id: &vw_api_transport::ProjectId,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_list_for_project(project_id).await
    }

    pub async fn session_status<T: DeserializeOwned>(
        &self,
        directory: Option<&str>,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_status(directory).await
    }

    pub async fn session_get<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_get(session_id, directory).await
    }

    pub async fn session_children<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_children(session_id, directory).await
    }

    pub async fn session_messages<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_messages(session_id, directory).await
    }

    pub async fn session_ui_summary(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<GatewaySessionUiSummary, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_ui_summary(session_id, directory).await
    }

    pub async fn session_message_page(
        &self,
        session_id: &str,
        query: &GatewaySessionMessagePageQuery,
    ) -> Result<GatewaySessionMessagePage<SessionMessageRecord>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_message_page(session_id, query).await
    }

    pub async fn session_message_delta(
        &self,
        session_id: &str,
        directory: Option<&str>,
        delta: &SessionMessageDelta,
    ) -> Result<GatewaySessionMessageDeltaResponse, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_message_delta(session_id, directory, delta).await
    }

    pub async fn session_messages_query<T: DeserializeOwned>(
        &self,
        session_id: &str,
        query: &GatewaySessionMessageListQuery,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_messages_query(session_id, query).await
    }

    pub async fn session_message_get<T: DeserializeOwned>(
        &self,
        session_id: &str,
        message_id: &str,
        directory: Option<&str>,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_message_get(session_id, message_id, directory).await
    }

    pub async fn session_message_part_update<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
        directory: Option<&str>,
        body: &B,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_message_part_update(session_id, message_id, part_id, directory, body).await
    }

    pub async fn session_message_part_delete(
        &self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
        directory: Option<&str>,
    ) -> Result<bool, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_message_part_delete(session_id, message_id, part_id, directory).await
    }

    pub async fn session_create<T: DeserializeOwned>(
        &self,
        directory: &str,
        body: &Option<GatewaySessionCreateBody>,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_create(directory, body).await
    }

    pub async fn session_update<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &GatewaySessionPatchBody,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_update(session_id, directory, body).await
    }

    pub async fn session_delete(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<(), String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_delete(session_id, directory).await
    }

    pub async fn session_delete_archived(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<vw_api_session::session::ArchivedChatDeleteResponse, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_delete_archived(session_id, directory).await
    }

    pub async fn session_fork<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &Option<GatewaySessionForkBody>,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_fork(session_id, directory, body).await
    }

    pub async fn session_reset<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &GatewaySessionResetBody,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_reset(session_id, directory, body).await
    }

    pub async fn session_summarize(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &GatewaySessionSummarizeBody,
    ) -> Result<bool, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_summarize(session_id, directory, body).await
    }

    pub async fn session_todo_update(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &GatewaySessionTodoPutBody,
    ) -> Result<bool, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_todo_update(session_id, directory, body).await
    }

    pub async fn session_todo_get(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Vec<Todo>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_todo_get(session_id, directory).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn session_todo_get_blocking(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Vec<Todo>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_todo_get_blocking(session_id, directory)
    }

    pub async fn session_diff<T: DeserializeOwned>(
        &self,
        session_id: &str,
        query: &GatewaySessionDiffQuery,
    ) -> Result<T, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_diff(session_id, query).await
    }

    pub async fn session_ui_get(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<ChatSession>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_ui_get(session_id, directory).await
    }

    pub async fn session_ui_get_any(
        &self,
        session_id: &str,
    ) -> Result<Option<ChatSession>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_ui_get_any(session_id).await
    }

    pub async fn session_ui_save(
        &self,
        session_id: &str,
        directory: Option<&str>,
        session: &ChatSession,
    ) -> Result<bool, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_ui_save(session_id, directory, session).await
    }

    pub async fn session_ui_previews(
        &self,
        directory: Option<&str>,
    ) -> Result<Vec<ChatSessionMeta>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_ui_previews(directory).await
    }

    pub async fn session_preview_meta_get(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<ChatSessionMeta>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_preview_meta_get(session_id, directory).await
    }

    pub async fn session_path_get(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<String>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_path_get(session_id, directory).await
    }

    pub async fn session_step_llm_raw_get(
        &self,
        session_id: &str,
        turn_id: &str,
        step_index: u32,
        directory: Option<&str>,
    ) -> Result<Option<serde_json::Value>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_step_llm_raw_get(session_id, turn_id, step_index, directory).await
    }

    pub async fn session_archived_get(
        &self,
        directory: Option<&str>,
    ) -> Result<Vec<String>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_archived_get(directory).await
    }

    pub async fn session_archived_put(
        &self,
        directory: Option<&str>,
        ids: &[String],
    ) -> Result<bool, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_archived_put(directory, ids).await
    }

    pub async fn session_scope_get(
        &self,
        _directory: Option<&str>,
        _project_id: Option<&str>,
    ) -> Result<Option<String>, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_scope_get(_directory, _project_id).await
    }

    pub async fn session_scope_put(&self, scope: Option<&str>) -> Result<bool, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_scope_put(scope).await
    }

    pub async fn session_title_generate(
        &self,
        session_id: &str,
        body: &GatewaySessionTitleGenerateBody,
    ) -> Result<GatewaySessionTitleGenerateResponse, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_title_generate(session_id, body).await
    }

    pub async fn session_title_generate_in_directory(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &GatewaySessionTitleGenerateBody,
    ) -> Result<GatewaySessionTitleGenerateResponse, String> {
        gateway_session::GatewaySessionClient::new(self.transport.clone())
            .session_title_generate_in_directory(session_id, directory, body).await
    }
}
