//! Reconcile desktop drafts with the gateway's first-response publication.

use super::ChatMessage;
use crate::app::{App, Message, models};
use iced::Task;
use std::time::Duration;
use vw_gateway_client::GatewayClient;
use session_info::Info;

pub use desktop_session::message::chat::stream::draft::{DraftPublication, RecoveredDraft};

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
                desktop_session::message::chat::stream::draft::read_publication(&client, &session_id, &directory).await
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
            desktop_session::message::chat::stream::draft::recover(&client, &session_id, &directory, &query).await
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
    let local_len = if app.active_session_id.as_deref() == Some(session_id.as_str()) {
        app.chat.len()
    } else {
        app.cached_chat_messages(&session_id).map_or(0, |messages| messages.len())
    };
    if snapshot.messages.len() < local_len {
        return super::stream_events::handle_agent_stream_error(
            app,
            id,
            "恢复快照缺少本地历史，已保留现有消息，请重新加载会话".into(),
        );
    }
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
                    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_get::<Info>(&info.id, Some(&draft.directory)).await
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

#[cfg(test)]
pub(in crate::app::message::chat) fn publication_fixture(info: Info, preview: session_ui_types::ChatSessionMeta) -> DraftPublication {
 DraftPublication { info, preview: Some(preview) }
}
