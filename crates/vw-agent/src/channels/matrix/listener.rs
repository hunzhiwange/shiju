use super::MatrixChannel;
use crate::app::agent::channels::traits::ChannelMessage;
use crate::app::agent::gateway::api::handlers::task_pool_matrix::record_inbound_task_pool_matrix_event;
use matrix_sdk::{
    Client as MatrixSdkClient, Room,
    config::SyncSettings,
    event_handler::RawEvent,
    media::{MediaFormat, MediaRequestParameters},
    ruma::{
        OwnedRoomId, OwnedUserId,
        events::{
            AnySyncTimelineEvent,
            room::message::{MessageType, OriginalSyncRoomMessageEvent},
        },
    },
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashSet, VecDeque};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{Mutex, mpsc};
use vw_api_types::agent_scheduler::{
    AGENT_SCHEDULER_MATRIX_DISPATCH_EVENT_TYPE, AgentSchedulerMatrixDispatchEventDto,
};
use vw_api_types::task::{TASK_POOL_MATRIX_EVENT_TYPE, TaskPoolMatrixEventDto};

#[derive(Debug, Deserialize)]
struct RawTaskPoolMatrixTimelineEvent {
    #[serde(rename = "type")]
    event_type: String,
    sender: String,
    #[serde(default)]
    event_id: Option<String>,
    #[serde(default)]
    content: Value,
}

#[derive(Debug, Deserialize)]
struct RawAgentSchedulerMatrixDispatchTimelineEvent {
    #[serde(rename = "type")]
    event_type: String,
    sender: String,
    #[serde(default)]
    event_id: Option<String>,
    #[serde(default)]
    content: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MatrixAgentSchedulerDispatchTimelineEvent {
    pub(crate) matrix_event_id: String,
    pub(crate) room_id: String,
    pub(crate) dispatch: AgentSchedulerMatrixDispatchEventDto,
}

impl MatrixChannel {
    /// 检查事件是否是对机器人已发送消息的回复
    async fn is_reply_to_cached_bot_event(
        event: &OriginalSyncRoomMessageEvent,
        bot_event_cache: &Mutex<(VecDeque<String>, HashSet<String>)>,
    ) -> bool {
        let Some(target_event_id) = Self::reply_target_event_id(event) else {
            return false;
        };

        let guard = bot_event_cache.lock().await;
        let (_, known_bot_events) = &*guard;
        known_bot_events.contains(&target_event_id)
    }

    pub(crate) async fn listen_agent_scheduler_dispatch_events(
        &self,
        tx: mpsc::Sender<MatrixAgentSchedulerDispatchTimelineEvent>,
    ) -> anyhow::Result<()> {
        if self.otk_conflict_detected.load(Ordering::Relaxed) {
            anyhow::bail!("{}", self.otk_conflict_recovery_message());
        }

        let target_room_id = self.target_room_id().await?;
        self.ensure_room_supported(&target_room_id).await?;
        let target_room: OwnedRoomId = target_room_id.parse()?;
        let client = self.matrix_client().await?;
        self.log_e2ee_diagnostics(&client).await;
        let _ = client.sync_once(SyncSettings::new()).await;

        tracing::info!(
            "Matrix agent scheduler worker listening on room {} (configured as {})...",
            target_room_id,
            self.room_id
        );

        let dispatch_target_room_for_handler = target_room.clone();
        let dispatch_allowed_users_for_handler = self.allowed_users.clone();
        let tx_handler = tx.clone();

        client.add_event_handler(move |_event: AnySyncTimelineEvent, raw: RawEvent, room: Room| {
            let target_room = dispatch_target_room_for_handler.clone();
            let allowed_users = dispatch_allowed_users_for_handler.clone();
            let tx = tx_handler.clone();

            async move {
                let raw_value = match serde_json::from_str::<Value>(raw.get()) {
                    Ok(value) => value,
                    Err(error) => {
                        tracing::warn!("Ignoring unreadable Matrix scheduler dispatch raw event: {error}");
                        return;
                    }
                };

                if let Some(event) = MatrixChannel::agent_scheduler_dispatch_event_from_raw_timeline(
                    target_room.as_str(),
                    room.room_id().as_str(),
                    &raw_value,
                    &allowed_users,
                ) {
                    let _ = tx.send(event).await;
                }
            }
        });

        let sync_tx = tx.clone();
        self.sync_loop(&client, move || sync_tx.is_closed()).await?;

        if self.otk_conflict_detected.load(Ordering::Relaxed) {
            anyhow::bail!("{}", self.otk_conflict_recovery_message());
        }

        Ok(())
    }

    pub(super) async fn listen_impl(&self, tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        if self.otk_conflict_detected.load(Ordering::Relaxed) {
            anyhow::bail!("{}", self.otk_conflict_recovery_message());
        }

        let target_room_id = self.target_room_id().await?;
        self.ensure_room_supported(&target_room_id).await?;
        let target_room: OwnedRoomId = target_room_id.parse()?;

        let my_user_id: OwnedUserId = match self.get_my_user_id().await {
            Ok(user_id) => user_id.parse()?,
            Err(error) => {
                if let Some(hinted) = self.session_owner_hint.as_ref() {
                    let safe_error = Self::sanitize_error_for_log(&error);
                    tracing::warn!(
                        "Matrix whoami failed while resolving listener user_id; using configured user_id hint: {safe_error}"
                    );
                    hinted.parse()?
                } else {
                    return Err(error);
                }
            }
        };
        let client = self.matrix_client().await?;
        self.log_e2ee_diagnostics(&client).await;
        let _ = client.sync_once(SyncSettings::new()).await;

        tracing::info!(
            "Matrix channel listening on room {} (configured as {})...",
            target_room_id,
            self.room_id
        );

        let recent_event_cache = Arc::new(Mutex::new((VecDeque::new(), HashSet::new())));
        let recent_bot_event_cache = Arc::new(Mutex::new((VecDeque::new(), HashSet::new())));

        let tx_handler = tx.clone();
        let target_room_for_handler = target_room.clone();
        let my_user_id_for_handler = my_user_id.clone();
        let allowed_users_for_handler = self.allowed_users.clone();
        let dedupe_for_handler = Arc::clone(&recent_event_cache);
        let bot_dedupe_for_handler = Arc::clone(&recent_bot_event_cache);
        let mention_only_for_handler = self.mention_only;
        let transcription_for_handler = self.transcription.clone();

        let task_pool_target_room_for_handler = target_room.clone();
        let task_pool_allowed_users_for_handler = self.allowed_users.clone();
        let task_pool_dedupe_for_handler = Arc::new(Mutex::new((VecDeque::new(), HashSet::new())));

        client.add_event_handler(move |_event: AnySyncTimelineEvent, raw: RawEvent, room: Room| {
            let target_room = task_pool_target_room_for_handler.clone();
            let allowed_users = task_pool_allowed_users_for_handler.clone();
            let dedupe = Arc::clone(&task_pool_dedupe_for_handler);

            async move {
                let raw_value = match serde_json::from_str::<Value>(raw.get()) {
                    Ok(value) => value,
                    Err(error) => {
                        tracing::warn!("Ignoring unreadable Matrix task-pool raw event: {error}");
                        return;
                    }
                };

                if let Some(event) = MatrixChannel::task_pool_matrix_event_from_raw_timeline(
                    target_room.as_str(),
                    room.room_id().as_str(),
                    &raw_value,
                    &allowed_users,
                    &dedupe,
                )
                .await
                {
                    record_inbound_task_pool_matrix_event(event);
                }
            }
        });

        client.add_event_handler(move |event: OriginalSyncRoomMessageEvent, room: Room| {
            let tx = tx_handler.clone();
            let target_room = target_room_for_handler.clone();
            let my_user_id = my_user_id_for_handler.clone();
            let allowed_users = allowed_users_for_handler.clone();
            let dedupe = Arc::clone(&dedupe_for_handler);
            let bot_dedupe = Arc::clone(&bot_dedupe_for_handler);
            let transcription = transcription_for_handler.clone();

            async move {
                if room.room_id().as_str() != target_room.as_str() {
                    return;
                }

                let event_id = event.event_id.to_string();

                if event.sender == my_user_id {
                    let mut guard = bot_dedupe.lock().await;
                    let (recent_order, recent_lookup) = &mut *guard;
                    MatrixChannel::cache_event_id(&event_id, recent_order, recent_lookup);
                    return;
                }

                let sender = event.sender.to_string();
                if !MatrixChannel::is_sender_allowed(&allowed_users, &sender) {
                    return;
                }

                let body = match &event.content.msgtype {
                    MessageType::Text(content) => content.body.clone(),
                    MessageType::Notice(content) => content.body.clone(),
                    MessageType::Audio(content) => {
                        if let Some(ref tc) = transcription {
                            if tc.enabled {
                                let media_request = MediaRequestParameters {
                                    source: content.source.clone(),
                                    format: MediaFormat::File,
                                };
                                match room.client().media().get_media_content(&media_request, true).await {
                                    Ok(audio_data) => {
                                        match crate::app::agent::channels::transcription::transcribe_audio(
                                            audio_data,
                                            &content.body,
                                            tc,
                                        )
                                        .await
                                        {
                                            Ok(text) => text,
                                            Err(e) => {
                                                tracing::warn!("Matrix voice transcription failed: {e}");
                                                format!("[Audio transcription failed: {}]", content.body)
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let safe_error = MatrixChannel::sanitize_error_for_log(&e);
                                        tracing::warn!("Failed to download Matrix audio: {safe_error}");
                                        format!("[Audio download failed: {}]", content.body)
                                    }
                                }
                            } else {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    _ => return,
                };

                if !MatrixChannel::has_non_empty_body(&body) {
                    return;
                }

                if mention_only_for_handler {
                    let is_direct_room = room.is_direct().await.unwrap_or_else(|error| {
                        let safe_error = MatrixChannel::sanitize_error_for_log(&error);
                        tracing::warn!(
                            "Matrix is_direct() failed while evaluating mention_only gate: {safe_error}"
                        );
                        false
                    });

                    let mut is_mentioned = false;
                    let mut is_reply_to_bot = false;

                    if !is_direct_room {
                        is_mentioned =
                            MatrixChannel::event_mentions_user(&event, &body, my_user_id.as_str());
                        is_reply_to_bot =
                            MatrixChannel::is_reply_to_cached_bot_event(&event, &bot_dedupe).await;
                    }

                    if !MatrixChannel::should_process_message(
                        mention_only_for_handler,
                        is_direct_room,
                        is_mentioned,
                        is_reply_to_bot,
                    ) {
                        return;
                    }
                }

                {
                    let mut guard = dedupe.lock().await;
                    let (recent_order, recent_lookup) = &mut *guard;
                    if MatrixChannel::cache_event_id(&event_id, recent_order, recent_lookup) {
                        return;
                    }
                }

                let msg = ChannelMessage {
                    id: event_id,
                    sender: sender.clone(),
                    reply_target: sender,
                    content: body,
                    channel: "matrix".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    thread_ts: None,
                };

                let _ = tx.send(msg).await;
            }
        });

        let sync_tx = tx.clone();
        self.sync_loop(&client, move || sync_tx.is_closed()).await?;

        if self.otk_conflict_detected.load(Ordering::Relaxed) {
            anyhow::bail!("{}", self.otk_conflict_recovery_message());
        }

        Ok(())
    }

    async fn sync_loop<F>(&self, client: &MatrixSdkClient, is_closed: F) -> anyhow::Result<()>
    where
        F: Fn() -> bool,
    {
        let mut sync_settings = SyncSettings::new().timeout(std::time::Duration::from_secs(30));
        let otk_conflict_detected = Arc::clone(&self.otk_conflict_detected);

        loop {
            if is_closed() {
                break;
            }

            match client.sync_once(sync_settings.clone()).await {
                Ok(response) => {
                    sync_settings = sync_settings.token(response.next_batch);
                }
                Err(error) => {
                    let raw_error = error.to_string();
                    if MatrixChannel::is_otk_conflict_message(&raw_error) {
                        Self::mark_otk_conflict(&otk_conflict_detected);
                        break;
                    }

                    let safe_error = MatrixChannel::sanitize_error_for_log(&error);
                    tracing::warn!("Matrix sync error: {safe_error}, retrying...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }

        Ok(())
    }

    fn mark_otk_conflict(otk_conflict_detected: &Arc<AtomicBool>) {
        let first_detection = !otk_conflict_detected.swap(true, Ordering::SeqCst);
        if first_detection {
            tracing::error!(
                "Matrix detected one-time key upload conflict; stopping listener to avoid retry loop."
            );
        }
    }

    fn agent_scheduler_dispatch_event_from_raw_timeline(
        target_room_id: &str,
        event_room_id: &str,
        raw_event: &Value,
        allowed_users: &[String],
    ) -> Option<MatrixAgentSchedulerDispatchTimelineEvent> {
        if event_room_id != target_room_id {
            return None;
        }

        let raw = match serde_json::from_value::<RawAgentSchedulerMatrixDispatchTimelineEvent>(
            raw_event.clone(),
        ) {
            Ok(raw) => raw,
            Err(error) => {
                tracing::warn!(
                    "Ignoring malformed Matrix scheduler dispatch timeline event: {error}"
                );
                return None;
            }
        };

        if raw.event_type != AGENT_SCHEDULER_MATRIX_DISPATCH_EVENT_TYPE {
            return None;
        }

        let Some(matrix_event_id) = raw.event_id.filter(|event_id| !event_id.trim().is_empty())
        else {
            tracing::warn!("Ignoring Matrix scheduler dispatch event without Matrix event_id");
            return None;
        };

        if !Self::is_sender_allowed(allowed_users, &raw.sender) {
            return None;
        }

        let dispatch = match serde_json::from_value::<AgentSchedulerMatrixDispatchEventDto>(
            raw.content,
        ) {
            Ok(dispatch) => dispatch,
            Err(error) => {
                tracing::warn!(
                    "Ignoring Matrix scheduler dispatch event with unreadable payload from {}: {error}",
                    raw.sender
                );
                return None;
            }
        };

        if let Err(error) = dispatch.validate_for_matrix_room_event() {
            tracing::warn!(
                "Ignoring Matrix scheduler dispatch event with invalid payload from {}: {error}",
                raw.sender
            );
            return None;
        }

        Some(MatrixAgentSchedulerDispatchTimelineEvent {
            matrix_event_id,
            room_id: event_room_id.to_string(),
            dispatch,
        })
    }

    pub(super) async fn task_pool_matrix_event_from_raw_timeline(
        target_room_id: &str,
        event_room_id: &str,
        raw_event: &Value,
        allowed_users: &[String],
        dedupe: &Mutex<(VecDeque<String>, HashSet<String>)>,
    ) -> Option<TaskPoolMatrixEventDto> {
        if event_room_id != target_room_id {
            return None;
        }

        let raw = match serde_json::from_value::<RawTaskPoolMatrixTimelineEvent>(raw_event.clone())
        {
            Ok(raw) => raw,
            Err(error) => {
                tracing::warn!("Ignoring malformed Matrix task-pool timeline event: {error}");
                return None;
            }
        };

        if raw.event_type != TASK_POOL_MATRIX_EVENT_TYPE {
            return None;
        }

        let Some(matrix_event_id) = raw.event_id.filter(|event_id| !event_id.trim().is_empty())
        else {
            tracing::warn!("Ignoring Matrix task-pool event without Matrix event_id");
            return None;
        };

        if !Self::is_sender_allowed(allowed_users, &raw.sender) {
            return None;
        }

        let event = match serde_json::from_value::<TaskPoolMatrixEventDto>(raw.content) {
            Ok(event) => event,
            Err(error) => {
                tracing::warn!(
                    "Ignoring Matrix task-pool event with unreadable payload from {}: {error}",
                    raw.sender
                );
                return None;
            }
        };

        if let Err(error) = event.validate_for_matrix_room_event() {
            tracing::warn!(
                "Ignoring Matrix task-pool event with invalid payload from {}: {error}",
                raw.sender
            );
            return None;
        }

        let mut guard = dedupe.lock().await;
        let (recent_order, recent_lookup) = &mut *guard;
        let dedupe_key = format!("matrix:{matrix_event_id}");
        if Self::cache_event_id(&dedupe_key, recent_order, recent_lookup) {
            return None;
        }

        Some(event)
    }
}

#[cfg(test)]
#[path = "listener_tests.rs"]
mod listener_tests;
