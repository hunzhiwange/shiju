//! Reconcile desktop drafts with the gateway's first-response publication.

use super::ChatMessage;
use crate::app::{App, Message, models};
use iced::Task;
use std::time::Duration;
use vw_gateway_client::GatewayClient;
use vw_shared::session::info::Info;
use vw_shared::session::ui_types::ChatSessionMeta;

#[derive(Debug, Clone)]
pub struct DraftPublication {
    pub(super) info: Info,
    pub(super) preview: Option<ChatSessionMeta>,
}

pub(super) fn reload_after_response(
    session_id: String,
    directory: String,
    id: u64,
    usage: models::TokenUsage,
    assistant_message_id: Option<String>,
    parent_message_id: Option<String>,
) -> Task<Message> {
    let endpoint = crate::app::config::gateway_client_endpoint();
    Task::perform(
        async move {
            let result = async {
                let client = GatewayClient::new(endpoint)?;
                read_publication(&client, &session_id, &directory).await
            }
            .await;
            match result {
                Ok(publication) => publication.map(Box::new),
                Err(error) => {
                    tracing::warn!(%session_id, %error, "failed to reconcile desktop draft");
                    None
                }
            }
        },
        move |publication| {
            Message::Chat(ChatMessage::DraftResponseReloaded {
                id,
                usage: usage.clone(),
                assistant_message_id: assistant_message_id.clone(),
                parent_message_id: parent_message_id.clone(),
                publication,
            })
        },
    )
}

pub(super) async fn read_publication(
    client: &GatewayClient,
    session_id: &str,
    directory: &str,
) -> Result<Option<DraftPublication>, String> {
    let info = client.session_get::<Info>(session_id, Some(directory)).await?;
    if info.id != session_id || info.is_first_response_pending() || info.time.archived.is_some() {
        return Ok(None);
    }
    let preview = client.session_preview_meta_get(session_id, Some(directory)).await.ok().flatten();
    Ok(Some(DraftPublication { info, preview }))
}

#[derive(Debug, Clone)]
pub struct RecoveredDraft {
    publication: DraftPublication,
    snapshot: models::ChatSession,
    assistant_id: String,
    user_id: String,
}

pub(super) fn recover_expired_replay(
    id: u64,
    error: String,
    session_id: String,
    directory: String,
    query: String,
) -> Task<Message> {
    let endpoint = crate::app::config::gateway_client_endpoint();
    Task::perform(
        async move {
            let client = GatewayClient::new(endpoint).ok()?;
            let publication = read_publication(&client, &session_id, &directory).await.ok()??;
            let snapshot = client
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
                || user.role != models::ChatRole::User
                || user.content != query
                || assistant.role != models::ChatRole::Assistant
                || assistant.content.trim().is_empty()
            {
                return None;
            }
            let user_id = user_id.clone();
            let assistant_id = assistant_id.clone();
            Some(Box::new(RecoveredDraft { publication, snapshot, assistant_id, user_id }))
        },
        move |recovered| {
            Message::Chat(ChatMessage::DraftReplayChecked { id, error: error.clone(), recovered })
        },
    )
}

pub(super) fn complete_recovered(
    app: &mut App,
    id: u64,
    recovered: RecoveredDraft,
) -> Task<Message> {
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    let RecoveredDraft { publication, snapshot, assistant_id, user_id } = recovered;
    app.get_session_runtime_mut(&session_id).pending_stream_delta.clear();
    if app.active_session_id.as_deref() == Some(session_id.as_str()) {
        app.chat = snapshot.messages.clone();
        app.chat_message_ids = snapshot.message_ids.clone();
        app.invalidate_chat_ui_state();
    }
    app.store_session_chat_snapshot(
        session_id,
        crate::app::session::shared_chat_messages(snapshot.messages),
        snapshot.message_ids,
    );
    super::stream_events::complete_agent_stream(
        app,
        id,
        models::TokenUsage::default(),
        Some(assistant_id),
        Some(user_id),
        Some(publication),
    )
}

pub(super) fn publish(
    app: &mut App,
    session_id: &str,
    publication: DraftPublication,
) -> Task<Message> {
    let DraftPublication { info, preview } = publication;
    if info.id != session_id || info.is_first_response_pending() || info.time.archived.is_some() {
        return Task::none();
    }
    let Some(draft) = app.get_session_runtime_mut(session_id).draft.take() else {
        return Task::none();
    };

    let list = app.project_sessions.entry(draft.project_path.clone()).or_default();
    if !list.iter().any(|entry| entry.id == session_id) {
        list.insert(0, info.clone());
    }
    if app.project_path.as_deref() == Some(draft.project_path.as_str())
        && !app.sessions.iter().any(|entry| entry.id == session_id)
    {
        app.sessions.insert(0, info.clone());
    }
    app.session_previews
        .extend(crate::app::session::build_session_previews(preview.into_iter().collect()));
    apply_published_title(app, &info);

    if !info.title.trim().is_empty() {
        return Task::none();
    }
    let endpoint = crate::app::config::gateway_client_endpoint();
    Task::perform(
        async move {
            let client = GatewayClient::new(endpoint).ok()?;
            for delay in [1, 2, 4, 8, 16, 16, 16] {
                crate::app::message::sleep(Duration::from_secs(delay)).await;
                if let Ok(updated) =
                    client.session_get::<Info>(&info.id, Some(&draft.directory)).await
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
        },
        |result| {
            result.map_or(Message::None, |info| {
                Message::Chat(ChatMessage::PublishedSessionTitleLoaded(info))
            })
        },
    )
}

pub(super) fn apply_published_title(app: &mut App, info: &Info) {
    if info.title.trim().is_empty()
        || info.is_first_response_pending()
        || info.time.archived.is_some()
    {
        return;
    }
    let mut changed = false;
    for session in app.sessions.iter_mut().chain(app.project_sessions.values_mut().flatten()) {
        if session.id == info.id
            && session.project_id == info.project_id
            && session.title.trim().is_empty()
        {
            session.title.clone_from(&info.title);
            session.time.updated = session.time.updated.max(info.time.updated);
            changed = true;
        }
    }
    if changed {
        app.refresh_task_pet_session_title(&info.id, &info.title);
    }
}
