//! One-use, authenticated desktop voice channels. No credentials in URLs or media logs.
use super::voice::{failure, fallback_id, realtime_identity};
use super::voice_handoff::{Binding, Handoffs, Update};
use crate::provider::voice::realtime::RealtimeSession;
use axum::{
    Extension, Json, Router,
    extract::{
        Path, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use vw_api_types::{id::TenantScope, voice::*};
use vw_config_types::voice::VoiceConfig;

const CONNECTION: &str = "x-shiju-voice-connection-id";
const QUEUE: usize = 8;
struct QueuedFrame {
    frame: VoiceFrame,
    _frame: OwnedSemaphorePermit,
    _samples: Option<OwnedSemaphorePermit>,
}
fn bounded_frame(
    frame: VoiceFrame,
    frames: &Arc<Semaphore>,
    samples: &Arc<Semaphore>,
) -> Result<QueuedFrame, VoiceErrorCode> {
    let frame_slot =
        frames.clone().try_acquire_owned().map_err(|_| VoiceErrorCode::Backpressure)?;
    let sample_slot = match &frame.event {
        VoiceEvent::AudioDelta { samples_per_channel, .. } => Some(
            samples
                .clone()
                .try_acquire_many_owned(*samples_per_channel as u32)
                .map_err(|_| VoiceErrorCode::Backpressure)?,
        ),
        _ => None,
    };
    Ok(QueuedFrame { frame, _frame: frame_slot, _samples: sample_slot })
}
struct Reservation {
    tenant: TenantScope,
    principal: [u8; 32],
    connection: VoiceId,
    start: VoiceSessionStart,
    config: VoiceConfig,
    created: Instant,
    _permit: OwnedSemaphorePermit,
}
struct Channels {
    pending: Mutex<HashMap<String, Reservation>>,
    capacity: Arc<Semaphore>,
}
impl Default for Channels {
    fn default() -> Self {
        Self { pending: Mutex::new(HashMap::new()), capacity: Arc::new(Semaphore::new(4)) }
    }
}
impl Channels {
    fn take(
        &self,
        id: &VoiceId,
        tenant: &TenantScope,
        principal: [u8; 32],
        conn: &VoiceId,
    ) -> Result<Reservation, VoiceErrorCode> {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        pending.retain(|_, r| r.created.elapsed() < Duration::from_secs(30));
        let r = pending.get(id.as_ref()).ok_or(VoiceErrorCode::Forbidden)?;
        if &r.tenant != tenant || r.principal != principal || &r.connection != conn {
            return Err(VoiceErrorCode::Forbidden);
        }
        pending.remove(id.as_ref()).ok_or(VoiceErrorCode::Forbidden)
    }
}
pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/voice/sessions", post(start))
        .route("/voice/sessions/{id}/stream", get(upgrade))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_METADATA_BYTES))
        .layer(Extension(Arc::new(Channels::default())))
}
fn connection(headers: &HeaderMap) -> Result<VoiceId, VoiceErrorCode> {
    if headers.get_all(CONNECTION).iter().count() != 1 {
        return Err(VoiceErrorCode::InvalidRequest);
    }
    headers
        .get(CONNECTION)
        .and_then(|h| h.to_str().ok())
        .ok_or(VoiceErrorCode::InvalidRequest)?
        .to_owned()
        .try_into()
        .map_err(|_| VoiceErrorCode::InvalidRequest)
}
fn configured_voice(config: &VoiceConfig) -> VoiceName {
    use vw_config_types::voice::VoiceName as Config;
    match config.voice {
        Config::Alloy => VoiceName::Alloy,
        Config::Ash => VoiceName::Ash,
        Config::Ballad => VoiceName::Ballad,
        Config::Coral => VoiceName::Coral,
        Config::Echo => VoiceName::Echo,
        Config::Sage => VoiceName::Sage,
        Config::Shimmer => VoiceName::Shimmer,
        Config::Verse => VoiceName::Verse,
        Config::Marin => VoiceName::Marin,
        Config::Cedar => VoiceName::Cedar,
    }
}
async fn start(
    Extension(tenant): Extension<TenantScope>,
    Extension(channels): Extension<Arc<Channels>>,
    headers: HeaderMap,
    Json(start): Json<VoiceSessionStart>,
) -> Response {
    let id = start.request_id.clone();
    let result = async {
        let (principal, config) = realtime_identity(&tenant, &headers).await?;
        let connection = connection(&headers)?;
        if start.generation != VoiceCounter::FIRST {
            return Err(VoiceErrorCode::InvalidRequest);
        }
        if !config.realtime_enabled {
            return Err(VoiceErrorCode::UnsupportedCapability);
        }
        if config.provider_id.as_deref() != Some(start.provider_id.as_ref()) {
            return Err(VoiceErrorCode::Forbidden);
        }
        let scope = tenant.clone();
        let chat = start.chat_session_id.as_ref().to_owned();
        let session = crate::global_database::run_isolated(move || {
            crate::session::ui_store::load_agent_session_any_for_tenant_fallible(&scope, &chat)
        })
        .await
        .map_err(|_| VoiceErrorCode::ProviderUnavailable)?
        .map_err(|_| VoiceErrorCode::ProviderUnavailable)?;
        session.ok_or(VoiceErrorCode::Forbidden)?;
        let mut pending = channels.pending.lock().unwrap_or_else(|e| e.into_inner());
        pending.retain(|_, r| r.created.elapsed() < Duration::from_secs(30));
        let permit = channels
            .capacity
            .clone()
            .try_acquire_owned()
            .map_err(|_| VoiceErrorCode::Backpressure)?;
        let voice_session_id: VoiceId = uuid::Uuid::new_v4().to_string().try_into().expect("UUID");
        let response = VoiceSessionStarted {
            request_id: start.request_id.clone(),
            chat_session_id: start.chat_session_id.clone(),
            generation: VoiceCounter::FIRST,
            voice_session_id: voice_session_id.clone(),
            stream_path: format!("/api/voice/sessions/{}/stream", voice_session_id.as_ref()),
            format: AudioFormat::default(),
            voice: configured_voice(&config),
            expires_in_ms: MAX_SESSION_MS,
        };
        pending.insert(
            voice_session_id.as_ref().to_owned(),
            Reservation {
                tenant,
                principal,
                connection,
                start,
                config,
                created: Instant::now(),
                _permit: permit,
            },
        );
        Ok::<_, VoiceErrorCode>(response)
    }
    .await;
    match result {
        Ok(response) => {
            ([(axum::http::header::CACHE_CONTROL, "no-store")], Json(response)).into_response()
        }
        Err(code) => failure(code, id),
    }
}
async fn upgrade(
    Extension(tenant): Extension<TenantScope>,
    Extension(channels): Extension<Arc<Channels>>,
    Path(id): Path<String>,
    axum::extract::RawQuery(query): axum::extract::RawQuery,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let result = async {
        let (principal, current) = realtime_identity(&tenant, &headers).await?;
        if query.is_some() {
            return Err(VoiceErrorCode::InvalidRequest);
        }
        let conn = connection(&headers)?;
        let id: VoiceId = id.try_into().map_err(|_| VoiceErrorCode::InvalidRequest)?;
        let reservation = channels.take(&id, &tenant, principal, &conn)?;
        // Re-read policy and session ownership before consuming the one-use upgrade.
        if !current.realtime_enabled || current != reservation.config {
            return Err(VoiceErrorCode::Forbidden);
        }
        let scope = tenant.clone();
        let chat = reservation.start.chat_session_id.as_ref().to_owned();
        let session = crate::global_database::run_isolated(move || {
            crate::session::ui_store::load_agent_session_any_for_tenant_fallible(&scope, &chat)
        })
        .await
        .map_err(|_| VoiceErrorCode::ProviderUnavailable)?
        .map_err(|_| VoiceErrorCode::ProviderUnavailable)?;
        let session = session.ok_or(VoiceErrorCode::Forbidden)?;
        Ok::<_, VoiceErrorCode>((id, reservation, session.directory))
    }
    .await;
    match result {
        Err(code) => failure(code, fallback_id()),
        Ok((id, reservation, directory)) => ws
            .write_buffer_size(0)
            .max_write_buffer_size(MAX_FRAME_BYTES * 2)
            .max_message_size(MAX_FRAME_BYTES)
            .max_frame_size(MAX_FRAME_BYTES)
            .on_upgrade(move |socket| {
                crate::session::ui_store::with_tenant_scope(tenant, async move {
                    let _permit = reservation._permit;
                    // Establish upstream concurrently with reading peer closure; an abandoned
                    // upgrade must not leave a credential resolution/connect future running.
                    let mut socket = socket;
                    let connect = tokio::time::timeout(
                        Duration::from_secs(20),
                        RealtimeSession::from_config(
                            &reservation.config,
                            id.clone(),
                            reservation.start.turn_mode,
                        ),
                    );
                    tokio::pin!(connect);
                    let upstream = tokio::select! { biased;
                        _ = socket.recv() => return,
                        result = &mut connect => result.unwrap_or(Err(VoiceErrorCode::Timeout)),
                    };
                    match upstream {
                        Ok(upstream) => {
                            let voice = configured_voice(&reservation.config);
                            let binding = Binding {
                                tenant: reservation.tenant,
                                chat_session: reservation.start.chat_session_id,
                                directory,
                            };
                            bridge(socket, upstream, id, voice, Some(binding)).await;
                        }
                        Err(code) => {
                            let mut error = VoiceError::new(code, reservation.start.request_id);
                            error.voice_session_id = Some(id.clone());
                            error.generation = Some(VoiceCounter::FIRST);
                            let frame = VoiceFrame {
                                voice_session_id: id,
                                generation: VoiceCounter::FIRST,
                                seq: VoiceCounter::FIRST,
                                event: VoiceEvent::Error(error),
                            };
                            if let Ok(text) = serde_json::to_string(&frame) {
                                let _ = tokio::time::timeout(
                                    Duration::from_secs(2),
                                    socket.send(Message::Text(text.into())),
                                )
                                .await;
                            }
                            let _ =
                                tokio::time::timeout(Duration::from_secs(1), socket.close()).await;
                        }
                    }
                })
            })
            .into_response(),
    }
}

// The reader checks wire ordering before separating priority controls from media.
// Priority controls may overtake media; the generation check discards retired audio.
async fn read_peer(
    mut reader: futures_util::stream::SplitStream<WebSocket>,
    id: VoiceId,
    media: mpsc::Sender<VoiceFrame>,
    control: mpsc::Sender<VoiceFrame>,
    closed: CancellationToken,
) {
    let mut seq = 0;
    let mut window = Instant::now();
    let mut count = 0;
    let mut sample_budget = 2400.0f64;
    let mut sample_clock = Instant::now();
    while let Some(Ok(message)) = reader.next().await {
        let bytes = match message {
            Message::Text(text) => text.as_bytes().to_vec(),
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => continue,
            _ => break,
        };
        let Ok(frame) = VoiceFrame::parse(&bytes) else {
            break;
        };
        if frame.voice_session_id != id || frame.seq.get() != seq + 1 {
            break;
        }
        seq = frame.seq.get();
        if window.elapsed() >= Duration::from_secs(1) {
            window = Instant::now();
            count = 0;
        }
        count += 1;
        if count > 100 {
            break;
        }
        let target = match &frame.event {
            VoiceEvent::AudioAppend(audio) => {
                let now = Instant::now();
                sample_budget = (sample_budget
                    + now.duration_since(sample_clock).as_secs_f64() * 24_000.0)
                    .min(2400.0);
                sample_clock = now;
                if audio.samples_per_channel as f64 > sample_budget {
                    break;
                }
                sample_budget -= audio.samples_per_channel as f64;
                &media
            }
            VoiceEvent::SessionStop {}
            | VoiceEvent::ResponseInterrupt { .. }
            | VoiceEvent::InputMute { .. }
            | VoiceEvent::PlaybackProgress { .. } => &control,
            VoiceEvent::InputCommit {} => &media,
            _ => break,
        };
        let stop = matches!(frame.event, VoiceEvent::SessionStop {});
        if target.try_send(frame).is_err() {
            break;
        }
        if stop {
            break;
        }
    }
    closed.cancel();
}
async fn bridge(
    socket: WebSocket,
    mut upstream: RealtimeSession,
    id: VoiceId,
    voice: VoiceName,
    binding: Option<Binding>,
) {
    let mut handoffs = binding.map(Handoffs::new);
    let mut generation = upstream.generation();
    let mut result_to_service: Option<Update> = None;
    let (mut writer, reader) = socket.split();
    let closed = CancellationToken::new();
    let (media_tx, mut media) = mpsc::channel(QUEUE);
    let (control_tx, mut control) = mpsc::channel(16);
    let (output, mut outgoing) = mpsc::channel::<QueuedFrame>(QUEUE);
    let frame_budget = Arc::new(Semaphore::new(50));
    let sample_budget = Arc::new(Semaphore::new(24_000));
    let mut tasks = tokio::task::JoinSet::new();
    let read = tasks.spawn(read_peer(reader, id.clone(), media_tx, control_tx, closed.clone()));
    let writer_closed = closed.clone();
    tasks.spawn(async move {
        loop {
            let frame = tokio::select! { biased;
                _ = writer_closed.cancelled() => break,
                frame = outgoing.recv() => match frame { Some(frame) => frame, None => break },
            };
            let Ok(text) = serde_json::to_string(&frame.frame) else {
                break;
            };
            let sent = tokio::select! { biased;
                _ = writer_closed.cancelled() => break,
                result = tokio::time::timeout(Duration::from_secs(2), writer.send(Message::Text(text.into()))) => result,
            };
            if !matches!(sent, Ok(Ok(()))) {
                break;
            }
        }
        writer_closed.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(1), writer.close()).await;
    });
    let ready = VoiceFrame {
        voice_session_id: id,
        generation: VoiceCounter::FIRST,
        seq: VoiceCounter::FIRST,
        event: VoiceEvent::SessionReady { format: AudioFormat::default(), voice },
    };
    // All outbound frames are resequenced here, including the ready acknowledgement.
    let mut seq = VoiceCounter::FIRST;
    let ready = bounded_frame(ready, &frame_budget, &sample_budget).expect("empty queue");
    let _ = output.try_send(ready);
    let deadline = tokio::time::sleep(Duration::from_millis(MAX_SESSION_MS));
    tokio::pin!(deadline);
    let mut pending = std::collections::VecDeque::<QueuedFrame>::new();
    let mut input_floor = 0;
    loop {
        let mut frames = tokio::select! { biased;
            _ = closed.cancelled() => break,
            _ = &mut deadline => break,
            frame = control.recv() => {
                let Some(frame) = frame else { break; };
                if matches!(frame.event, VoiceEvent::SessionStop {}) { break; }
                if frame.generation < upstream.generation() { continue; }
                // Muting does not advance the adapter generation. Invalidate media
                // captured before either mute transition so unmute cannot replay it.
                if matches!(frame.event, VoiceEvent::InputMute { .. }) { input_floor = frame.seq.get(); }
                match tokio::select! { biased; _ = closed.cancelled() => break, result = upstream.input(frame.generation, frame.event) => result } {
                    Ok(frame) => {
                        if frame.is_some() { pending.clear(); }
                        frame.into_iter().collect()
                    },
                    Err(_) => break,
                }
            }
            update = async {
                match &mut handoffs {
                    Some(h) => h.updates.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                let Some(update) = update else { break; };
                if update.call.generation != upstream.generation() { continue; }
                let event = VoiceEvent::HandoffStatus { call_id: update.call.call_id.clone(), status: update.status, text: update.text.clone() };
                let frame = VoiceFrame { voice_session_id: upstream.voice_session_id().clone(), generation: upstream.generation(), seq: VoiceCounter::FIRST, event };
                if matches!(update.status, HandoffStatus::Completed | HandoffStatus::Failed | HandoffStatus::Cancelled) {
                    if let Some(h) = &mut handoffs { h.finish(&update.call); }
                    result_to_service = Some(update);
                }
                vec![frame]
            }
            _ = std::future::ready(()), if result_to_service.is_some() && !upstream.responding() => {
                let Some(result) = result_to_service.take() else { break; };
                let sent = tokio::select! { biased;
                    _ = closed.cancelled() => break,
                    sent = upstream.handoff_result(&result.call, result.status, &result.text) => sent,
                };
                if sent.is_err() { break; }
                Vec::new()
            }
            permit = output.reserve(), if !pending.is_empty() => {
                let Ok(permit) = permit else { break; };
                let Some(mut frame) = pending.pop_front() else { break; };
                let Ok(next) = seq.checked_next() else { break; };
                seq = next;
                frame.frame.seq = seq;
                permit.send(frame);
                continue;
            }
            frame = media.recv() => {
                let Some(frame) = frame else { break; };
                if frame.generation < upstream.generation() || frame.seq.get() < input_floor { continue; }
                match tokio::select! { biased; _ = closed.cancelled() => break, result = upstream.input(frame.generation, frame.event) => result } {
                    Ok(frame) => frame.into_iter().collect(), Err(_) => break,
                }
            }
            frames = upstream.next(), if pending.is_empty() => match frames { Ok(frames) => frames, Err(_) => break },
        };
        if generation != upstream.generation() {
            generation = upstream.generation();
            result_to_service = None;
            if let Some(cancelled) = handoffs.as_mut().and_then(Handoffs::generation_changed) {
                frames.push(VoiceFrame {
                    voice_session_id: upstream.voice_session_id().clone(),
                    generation,
                    seq: VoiceCounter::FIRST,
                    event: VoiceEvent::HandoffStatus {
                        call_id: cancelled.call.call_id,
                        status: cancelled.status,
                        text: cancelled.text,
                    },
                });
            }
        }
        let mut invalid = false;
        let calls = upstream.take_handoffs();
        // Reject an ambiguous multi-tool response before admitting its first side effect.
        if calls.len() > 1 {
            break;
        }
        for call in calls {
            match &mut handoffs {
                Some(h) if result_to_service.is_none() => {
                    if h.submit(call, generation).is_err() {
                        invalid = true;
                        break;
                    }
                }
                _ => {
                    invalid = true;
                    break;
                }
            }
        }
        if invalid {
            break;
        }
        // The same frame/sample permits cover pending batches, queued output and
        // the writer's in-flight send: at most 50 frames and one second of PCM.
        let mut overflow = false;
        for frame in frames {
            match bounded_frame(frame, &frame_budget, &sample_budget) {
                Ok(frame) => pending.push_back(frame),
                Err(_) => {
                    overflow = true;
                    break;
                }
            }
        }
        if overflow {
            break;
        }
    }
    drop(handoffs);
    closed.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(2), upstream.stop()).await;
    read.abort();
    while tasks.join_next().await.is_some() {}
}
#[cfg(test)]
#[path = "voice_realtime_tests.rs"]
mod tests;
