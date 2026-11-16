use session_info::Info;
use session_ui_types::{ChatSession, ChatSessionMeta};
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
    let info = gateway_session::GatewaySessionClient::new(client.transport().clone())
        .session_get::<Info>(session_id, Some(directory))
        .await?;
    if info.id != session_id || info.is_first_response_pending() || info.time.archived.is_some() {
        return Ok(None);
    }
    let preview = gateway_session::GatewaySessionClient::new(client.transport().clone())
        .session_preview_meta_get(session_id, Some(directory))
        .await
        .ok()
        .flatten();
    Ok(Some(DraftPublication { info, preview }))
}
#[derive(Debug, Clone)]
pub struct RecoveredDraft {
    pub publication: DraftPublication,
    pub snapshot: ChatSession,
    pub assistant_id: String,
    pub user_id: String,
}

pub async fn recover(
    client: &GatewayClient,
    session_id: &str,
    directory: &str,
    query: &str,
) -> Option<Box<RecoveredDraft>> {
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

/// 发布标题只更新同项目的未命名副本，返回是否需要刷新跨域宠物标题。
pub fn apply_published_title<'a>(
    sessions: impl Iterator<Item = &'a mut Info>,
    info: &Info,
) -> bool {
    if info.title.trim().is_empty()
        || info.is_first_response_pending()
        || info.time.archived.is_some()
    {
        return false;
    }
    let mut changed = false;
    for session in sessions {
        if session.id == info.id
            && session.project_id == info.project_id
            && session.title.trim().is_empty()
        {
            session.title.clone_from(&info.title);
            session.time.updated = session.time.updated.max(info.time.updated);
            changed = true;
        }
    }
    changed
}

pub fn take_publication<R>(
    runtime: &mut crate::session::agent::SessionRuntimeState<R>,
    session_id: &str,
    info: &Info,
) -> Option<crate::session::agent::SessionDraft> {
    if info.id != session_id || info.is_first_response_pending() || info.time.archived.is_some() {
        return None;
    }
    runtime.draft.take()
}

pub fn insert_publication(sessions: &mut Vec<Info>, info: &Info) {
    if !sessions.iter().any(|entry| entry.id == info.id) {
        sessions.insert(0, info.clone());
    }
}

pub async fn poll_published_title<F, Fut>(
    client: &GatewayClient,
    info: Info,
    directory: String,
    mut delay_fn: F,
) -> Option<Info>
where
    F: FnMut(std::time::Duration) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    for delay in [1, 2, 4, 8, 16, 16, 16] {
        delay_fn(std::time::Duration::from_secs(delay)).await;
        if let Ok(updated) = gateway_session::GatewaySessionClient::new(client.transport().clone())
            .session_get::<Info>(&info.id, Some(&directory))
            .await
            && updated.id == info.id
            && updated.project_id == info.project_id
            && !updated.is_first_response_pending()
            && updated.time.archived.is_none()
            && !updated.title.trim().is_empty()
        {
            return Some(updated);
        }
    }
    None
}
