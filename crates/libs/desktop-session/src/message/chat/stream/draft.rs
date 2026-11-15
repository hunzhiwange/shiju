use session_info::Info;
use session_ui_types::{ChatSessionMeta, ChatSession};
use vw_gateway_client::GatewayClient;
#[derive(Debug, Clone)]
pub struct DraftPublication {
    pub info: Info,
    pub preview: Option<ChatSessionMeta>,
}

pub async fn read_publication(
    client: &GatewayClient,
    session_id: &str,
    directory: &str,
) -> Result<Option<DraftPublication>, String> {
    let info = gateway_session::GatewaySessionClient::new(client.transport().clone()).session_get::<Info>(session_id, Some(directory)).await?;
    if info.id != session_id || info.is_first_response_pending() || info.time.archived.is_some() {
        return Ok(None);
    }
    let preview = gateway_session::GatewaySessionClient::new(client.transport().clone()).session_preview_meta_get(session_id, Some(directory)).await.ok().flatten();
    Ok(Some(DraftPublication { info, preview }))
}
#[derive(Debug, Clone)]
pub struct RecoveredDraft {
    pub publication: DraftPublication,
    pub snapshot: ChatSession,
    pub assistant_id: String,
    pub user_id: String,
}

pub async fn recover(client: &GatewayClient, session_id: &str, directory: &str, query: &str) -> Option<Box<RecoveredDraft>> {
            let publication = read_publication(&client, &session_id, &directory).await.ok()??;
            let snapshot = gateway_session::GatewaySessionClient::new(client.transport().clone())
                .session_ui_get_without_raw(&session_id, Some(&directory))
                .await
                .ok()??
                .session;
            let [.., user, assistant] = snapshot.messages.as_slice() else {
                return None;
            };
            let [.., Some(user_id), Some(assistant_id)] = snapshot.message_ids.as_slice() else {
                return None;
            };
            if snapshot.id != session_id
                || snapshot.messages.len() != snapshot.message_ids.len()
                || user.role != session_ui_types::ChatRole::User
                || user.content != query
                || assistant.role != session_ui_types::ChatRole::Assistant
                || assistant.content.trim().is_empty()
            {
                return None;
            }
            let user_id = user_id.clone();
            let assistant_id = assistant_id.clone();
            Some(Box::new(RecoveredDraft { publication, snapshot, assistant_id, user_id }))
}
