//! Public Realtime WebSocket adapter. One socket, no reconnect or command replay.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        Message,
        protocol::{Role, WebSocketConfig},
    },
};
use vw_api_types::voice::*;
use vw_config_types::voice::VoiceConfig;

type Result<T> = std::result::Result<T, VoiceErrorCode>;
type Socket = WebSocketStream<reqwest::Upgraded>;

pub struct RealtimeSession {
    socket: Socket,
    handoffs: super::realtime_handoff::HandoffItems,
    provider_id: VoiceId,
    voice_session_id: VoiceId,
    generation: VoiceCounter,
    seq: VoiceCounter,
    upstream_session: String,
    mode: TurnMode,
    muted: bool,
    closed: bool,
    response: Option<String>,
    playback_response: Option<String>,
    retired: HashSet<String>,
    seen_responses: HashSet<String>,
    pending_truncation: HashMap<(String, u32), u64>,
    pending_clear: usize,
    control_deadline: Option<tokio::time::Instant>,
    items: HashMap<String, Item>,
    transcripts: HashMap<(String, u32), (String, bool)>,
    input_samples: usize,
    deadline: tokio::time::Instant,
}
struct Item {
    generation: VoiceCounter,
    response: String,
    audio: HashMap<u32, (u64, u64)>, // delivered samples, played milliseconds
}
impl RealtimeSession {
    pub(crate) async fn connect(
        base: &str,
        key: &str,
        builder: reqwest::ClientBuilder,
        config: &VoiceConfig,
        provider_id: VoiceId,
        voice_session_id: VoiceId,
        mode: TurnMode,
    ) -> Result<Self> {
        config.validate().map_err(|_| VoiceErrorCode::InvalidRequest)?;
        if !config.realtime_enabled {
            return Err(VoiceErrorCode::UnsupportedCapability);
        }
        let mut url: reqwest::Url = format!("{}/realtime", base.trim_end_matches('/'))
            .parse()
            .map_err(|_| VoiceErrorCode::InvalidRequest)?;
        if !(url.scheme() == "https"
            || (url.scheme() == "http"
                && matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "localhost"))))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(VoiceErrorCode::InvalidRequest);
        }
        url.query_pairs_mut().append_pair("model", "gpt-realtime-2.1");
        let mut auth = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", key.trim()))
            .map_err(|_| VoiceErrorCode::ProviderAuth)?;
        auth.set_sensitive(true);
        let client = builder
            .http1_only()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .map_err(|_| VoiceErrorCode::ProviderUnavailable)?;
        let work = async {
            let nonce = tokio_tungstenite::tungstenite::handshake::client::generate_key();
            let response = client
                .get(url)
                .header("authorization", auth)
                .header("connection", "Upgrade")
                .header("upgrade", "websocket")
                .header("sec-websocket-version", "13")
                .header("sec-websocket-key", &nonce)
                .send()
                .await
                .map_err(|_| VoiceErrorCode::ProviderUnavailable)?;
            if response.status().as_u16() != 101 {
                return Err(match response.status().as_u16() {
                    401 | 403 => VoiceErrorCode::ProviderAuth,
                    429 => VoiceErrorCode::RateLimited,
                    _ => VoiceErrorCode::ProviderUnavailable,
                });
            }
            let headers = response.headers();
            if headers.get("sec-websocket-accept").and_then(|s| s.to_str().ok())
                != Some(
                    tokio_tungstenite::tungstenite::handshake::derive_accept_key(nonce.as_bytes())
                        .as_str(),
                )
                || !headers
                    .get("upgrade")
                    .is_some_and(|v| v.as_bytes().eq_ignore_ascii_case(b"websocket"))
                || !headers
                    .get("connection")
                    .and_then(|s| s.to_str().ok())
                    .is_some_and(|s| s.split(',').any(|s| s.trim().eq_ignore_ascii_case("upgrade")))
                || headers.contains_key("sec-websocket-extensions")
                || headers.contains_key("sec-websocket-protocol")
            {
                return Err(VoiceErrorCode::ProtocolError);
            }
            let stream = response.upgrade().await.map_err(|_| VoiceErrorCode::ProtocolError)?;
            let socket = WebSocketStream::from_raw_socket(
                stream,
                Role::Client,
                Some(
                    WebSocketConfig::default()
                        .max_message_size(Some(256 * 1024))
                        .max_frame_size(Some(256 * 1024))
                        .max_write_buffer_size(512 * 1024),
                ),
            )
            .await;
            let mut session = Self {
                socket,
                handoffs: Default::default(),
                provider_id,
                voice_session_id,
                generation: VoiceCounter::FIRST,
                seq: VoiceCounter::FIRST,
                upstream_session: String::new(),
                mode,
                muted: false,
                closed: false,
                response: None,
                playback_response: None,
                retired: HashSet::new(),
                seen_responses: HashSet::new(),
                pending_truncation: HashMap::new(),
                pending_clear: 0,
                control_deadline: None,
                items: HashMap::new(),
                transcripts: HashMap::new(),
                input_samples: 0,
                deadline: tokio::time::Instant::now() + Duration::from_millis(MAX_SESSION_MS),
            };
            let created = session.receive().await?;
            if created["type"] != "session.created" {
                return Err(VoiceErrorCode::ProtocolError);
            }
            session.upstream_session = id(&created["session"], "id")?;
            let mut transcription = json!({"model":"gpt-transcribe","keywords":config.dictionary,"prompt":config.prompt});
            if let Some(language) = config.language.service_code() {
                transcription["languages"] = json!([language]);
            }
            let turn = match mode {
                TurnMode::Manual => Value::Null,
                TurnMode::ServerVad => {
                    json!({"type":"server_vad","create_response":true,"interrupt_response":true})
                }
            };
            session.send(json!({"type":"session.update","session":{"type":"realtime","model":"gpt-realtime-2.1","output_modalities":["audio"],"instructions":"用简短中文回答。需要执行工作时调用 shiju_handoff；遵守审批结果。","tools":super::realtime_handoff::tools(),"tool_choice":"auto","audio":{"input":{"format":{"type":"audio/pcm","rate":24000},"transcription":transcription,"turn_detection":turn},"output":{"format":{"type":"audio/pcm","rate":24000},"voice":config.voice}}}})).await?;
            let updated = session.receive().await?;
            let s = &updated["session"];
            if updated["type"] != "session.updated"
                || s["id"] != session.upstream_session
                || s["type"] != "realtime"
                || s["model"] != "gpt-realtime-2.1"
                || s["output_modalities"] != json!(["audio"])
                || s["tools"] != super::realtime_handoff::tools()
                || s["tool_choice"] != "auto"
                || s["audio"]["input"]["transcription"]["model"] != "gpt-transcribe"
                || (mode == TurnMode::Manual && !s["audio"]["input"]["turn_detection"].is_null())
                || (mode == TurnMode::ServerVad
                    && (s["audio"]["input"]["turn_detection"]["type"] != "server_vad"
                        || s["audio"]["input"]["turn_detection"]["create_response"] != true
                        || s["audio"]["input"]["turn_detection"]["interrupt_response"] != true))
                || s["audio"]["input"]["format"] != json!({"type":"audio/pcm","rate":24000})
                || s["audio"]["output"]["format"] != json!({"type":"audio/pcm","rate":24000})
                || s["audio"]["output"]["voice"]
                    != serde_json::to_value(config.voice)
                        .map_err(|_| VoiceErrorCode::InvalidRequest)?
            {
                return Err(VoiceErrorCode::UnsupportedCapability);
            }
            Ok(session)
        };
        tokio::time::timeout(Duration::from_secs(15), work)
            .await
            .map_err(|_| VoiceErrorCode::Timeout)?
    }
    async fn send(&mut self, value: Value) -> Result<()> {
        if self.closed {
            return Err(VoiceErrorCode::Cancelled);
        }
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            self.socket.send(Message::Text(value.to_string().into())),
        )
        .await;
        if !matches!(result, Ok(Ok(()))) {
            self.closed = true;
            return Err(VoiceErrorCode::ProviderUnavailable);
        }
        Ok(())
    }
    async fn receive(&mut self) -> Result<Value> {
        if self.closed {
            return Err(VoiceErrorCode::Cancelled);
        }
        loop {
            let deadline = self.control_deadline.map_or(self.deadline, |d| d.min(self.deadline));
            let value = tokio::time::timeout_at(deadline, self.socket.next()).await;
            match value {
                Ok(Some(Ok(Message::Text(text)))) => {
                    return serde_json::from_str(&text).map_err(|_| VoiceErrorCode::ProtocolError);
                }
                Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => continue,
                Err(_) => {
                    self.closed = true;
                    return Err(VoiceErrorCode::Timeout);
                }
                _ => {
                    self.closed = true;
                    return Err(VoiceErrorCode::ProviderUnavailable);
                }
            }
        }
    }
    pub async fn stop(&mut self) {
        self.closed = true;
        let _ = tokio::time::timeout(Duration::from_secs(1), self.socket.close(None)).await;
    }
    pub fn provider_id(&self) -> &VoiceId {
        &self.provider_id
    }
    pub fn voice_session_id(&self) -> &VoiceId {
        &self.voice_session_id
    }
    pub fn generation(&self) -> VoiceCounter {
        self.generation
    }
    /// A returned frame acknowledges an interruption and invalidates old client input/playback.
    pub async fn input(
        &mut self,
        generation: VoiceCounter,
        event: VoiceEvent,
    ) -> Result<Option<VoiceFrame>> {
        if self.closed || generation != self.generation {
            return Err(VoiceErrorCode::Cancelled);
        }
        let result = self.input_inner(event).await;
        if result.is_err() {
            self.stop().await;
        }
        result
    }
    async fn input_inner(&mut self, event: VoiceEvent) -> Result<Option<VoiceFrame>> {
        event.validate().map_err(|_| VoiceErrorCode::InvalidRequest)?;
        match event {
            VoiceEvent::AudioAppend(a) if !self.muted => {
                self.input_samples = self
                    .input_samples
                    .checked_add(a.samples_per_channel)
                    .filter(|s| *s <= 24000 * 180)
                    .ok_or(VoiceErrorCode::PayloadTooLarge)?;
                self.send(json!({"type":"input_audio_buffer.append","audio":a.audio})).await?;
            }
            VoiceEvent::AudioAppend(_) => {}
            VoiceEvent::InputCommit {}
                if self.mode == TurnMode::Manual && self.input_samples >= 2400 =>
            {
                self.input_samples = 0;
                self.send(json!({"type":"input_audio_buffer.commit"})).await?;
                self.send(json!({"type":"response.create"})).await?;
            }
            VoiceEvent::InputMute { muted } => {
                self.muted = muted;
                self.input_samples = 0;
                if self.pending_clear >= 16 {
                    return Err(VoiceErrorCode::Backpressure);
                }
                self.pending_clear += 1;
                self.control_deadline
                    .get_or_insert(tokio::time::Instant::now() + Duration::from_secs(10));
                self.send(json!({"type":"input_audio_buffer.clear"})).await?;
            }
            VoiceEvent::ResponseInterrupt { response_id, item_id, content_index, audio_end_ms } => {
                if self.playback_response.as_deref() != Some(response_id.as_ref()) {
                    return Err(VoiceErrorCode::Conflict);
                }
                if item_id.is_none()
                    && self
                        .items
                        .values()
                        .any(|i| i.response == response_id.as_ref() && !i.audio.is_empty())
                {
                    return Err(VoiceErrorCode::InvalidRequest);
                }
                if let (Some(item), Some(index), Some(ms)) = (item_id, content_index, audio_end_ms)
                {
                    let state = self
                        .items
                        .get(item.as_ref())
                        .filter(|i| i.response == response_id.as_ref())
                        .and_then(|i| i.audio.get(&index))
                        .ok_or(VoiceErrorCode::InvalidRequest)?;
                    if ms > state.1 {
                        return Err(VoiceErrorCode::InvalidRequest);
                    }
                    if self.response.is_some() {
                        self.send(json!({"type":"response.cancel","response_id":response_id}))
                            .await?;
                    }
                    self.truncate(item.as_ref(), index, ms).await?;
                } else if self.response.is_some() {
                    self.send(json!({"type":"response.cancel","response_id":response_id})).await?;
                }
                self.retire_response()?;
                let event = VoiceEvent::GenerationChanged {
                    reason: GenerationReason::Interrupt,
                    response_id: Some(response_id),
                    item_id: None,
                    content_index: None,
                };
                return Ok(Some(self.frame(event)?));
            }
            VoiceEvent::PlaybackProgress { response_id, item_id, content_index, audio_end_ms } => {
                let state = self
                    .items
                    .get_mut(item_id.as_ref())
                    .filter(|i| i.response == response_id.as_ref())
                    .and_then(|i| i.audio.get_mut(&content_index))
                    .ok_or(VoiceErrorCode::InvalidRequest)?;
                if audio_end_ms < state.1 || audio_end_ms > state.0 / 24 {
                    return Err(VoiceErrorCode::InvalidRequest);
                }
                state.1 = audio_end_ms;
            }
            VoiceEvent::SessionStop {} => self.stop().await,
            _ => return Err(VoiceErrorCode::InvalidRequest),
        }
        Ok(None)
    }
    async fn truncate(&mut self, item: &str, index: u32, ms: u64) -> Result<()> {
        if self.pending_truncation.len() >= 16
            || self.pending_truncation.insert((item.into(), index), ms).is_some()
        {
            return Err(VoiceErrorCode::Conflict);
        }
        self.control_deadline.get_or_insert(tokio::time::Instant::now() + Duration::from_secs(10));
        self.send(json!({"type":"conversation.item.truncate","item_id":item,"content_index":index,"audio_end_ms":ms})).await
    }
    fn retire_response(&mut self) -> Result<()> {
        if self.retired.len() >= 256 {
            return Err(VoiceErrorCode::Backpressure);
        }
        self.response = None;
        self.handoffs.retire();
        if let Some(response) = self.playback_response.take() {
            self.retired.insert(response);
        }
        self.generation =
            self.generation.checked_next().map_err(|_| VoiceErrorCode::ProtocolError)?;
        Ok(())
    }
    fn frame(&mut self, event: VoiceEvent) -> Result<VoiceFrame> {
        let seq = self.seq;
        self.seq = self.seq.checked_next().map_err(|_| VoiceErrorCode::ProtocolError)?;
        Ok(VoiceFrame {
            voice_session_id: self.voice_session_id.clone(),
            generation: self.generation,
            seq,
            event,
        })
    }
    pub(crate) fn take_handoffs(&mut self) -> Vec<super::realtime_handoff::HandoffCall> {
        self.handoffs.take()
    }
    pub(crate) fn responding(&self) -> bool {
        self.response.is_some()
    }
    /// A result is sent once on this socket, never replayed after reconnection.
    pub(crate) async fn handoff_result(
        &mut self,
        call: &super::realtime_handoff::HandoffCall,
        status: HandoffStatus,
        text: &str,
    ) -> Result<()> {
        if call.generation != self.generation || self.closed {
            return Err(VoiceErrorCode::Cancelled);
        }
        if self.response.is_some() {
            return Err(VoiceErrorCode::Conflict);
        }
        if text.len() > 16 * 1024 {
            return Err(VoiceErrorCode::PayloadTooLarge);
        }
        let output = json!({"status":status,"text":text}).to_string();
        self.send(json!({"type":"conversation.item.create","item":{"type":"function_call_output","call_id":call.call_id,"output":output}})).await?;
        self.send(json!({"type":"response.create","response":{"tools":[],"tool_choice":"none"}}))
            .await
    }
    pub async fn next(&mut self) -> Result<Vec<VoiceFrame>> {
        let result = self.next_inner().await;
        if result.is_err() {
            self.stop().await;
        }
        result
    }
}
pub(super) fn id(value: &Value, field: &str) -> Result<String> {
    let s = value[field].as_str().ok_or(VoiceErrorCode::ProtocolError)?.to_owned();
    voice_id(s.clone())?;
    Ok(s)
}
pub(super) fn voice_id(value: String) -> Result<VoiceId> {
    value.try_into().map_err(|_| VoiceErrorCode::ProtocolError)
}

#[path = "realtime_events.rs"]
mod events;
