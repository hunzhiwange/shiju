use super::{TelegramChannel, voice::VoiceMetadata};
use axum::{
    Router,
    body::Body,
    extract::{OriginalUri, State},
    http::{Method, StatusCode, header::CONTENT_TYPE},
    response::Response,
    routing::any,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Once},
};
use tokio::sync::{Mutex, oneshot};

#[derive(Clone, Debug)]
struct ResponseSpec {
    status: StatusCode,
    body: Vec<u8>,
    content_type: &'static str,
}

struct TestServerState {
    requests: Mutex<Vec<String>>,
    responses: Mutex<VecDeque<ResponseSpec>>,
}

struct TestServer {
    base_url: String,
    state: Arc<TestServerState>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl TestServer {
    async fn spawn(responses: Vec<ResponseSpec>) -> Self {
        let state = Arc::new(TestServerState {
            requests: Mutex::new(Vec::new()),
            responses: Mutex::new(VecDeque::from(responses)),
        });
        let app = Router::new().fallback(any(record_request)).with_state(state.clone());
        let listener =
            tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("bind test server");
        let addr = listener.local_addr().expect("server addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("serve test server");
        });

        Self { base_url: format!("http://{addr}"), state, shutdown: Some(shutdown_tx) }
    }

    async fn requests(&self) -> Vec<String> {
        self.state.requests.lock().await.clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

async fn record_request(
    State(state): State<Arc<TestServerState>>,
    method: Method,
    uri: OriginalUri,
) -> Response<Body> {
    state.requests.lock().await.push(format!(
        "{method} {}",
        uri.path_and_query().map(ToString::to_string).unwrap_or_default()
    ));
    let response = state.responses.lock().await.pop_front().expect("test response");

    Response::builder()
        .status(response.status)
        .header(CONTENT_TYPE, response.content_type)
        .body(Body::from(response.body))
        .expect("response should build")
}

fn json_response(body: &str) -> ResponseSpec {
    ResponseSpec {
        status: StatusCode::OK,
        body: body.as_bytes().to_vec(),
        content_type: "application/json",
    }
}

fn bytes_response(status: StatusCode, body: &[u8]) -> ResponseSpec {
    ResponseSpec { status, body: body.to_vec(), content_type: "application/octet-stream" }
}

fn transcription_config(api_url: String) -> crate::app::agent::config::TranscriptionConfig {
    enable_test_tracing();
    crate::app::agent::config::TranscriptionConfig {
        enabled: true,
        api_url,
        max_duration_secs: 60,
        ..Default::default()
    }
}

fn set_transcription_api_key() {
    unsafe { std::env::set_var("GROQ_API_KEY", "test-key") };
}

struct TestSubscriber;

impl tracing::Subscriber for TestSubscriber {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}

    fn event(&self, _: &tracing::Event<'_>) {}

    fn enter(&self, _: &tracing::span::Id) {}

    fn exit(&self, _: &tracing::span::Id) {}
}

fn enable_test_tracing() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = tracing::subscriber::set_global_default(TestSubscriber);
    });
}

#[test]
fn voice_metadata_parser_reads_required_file_id() {
    let value = serde_json::json!({
        "voice": {
            "file_id": "voice-file",
            "file_unique_id": "unique",
            "mime_type": "audio/ogg",
            "duration": 9
        }
    });

    let metadata = TelegramChannel::parse_voice_metadata(&value).unwrap();

    assert_eq!(metadata.file_id, "voice-file");
    assert_eq!(metadata.mime_type_hint.as_deref(), Some("audio/ogg"));
    assert_eq!(metadata.duration_secs, 9);
    assert!(metadata.voice_note);
}

#[test]
fn voice_metadata_struct_preserves_optional_fields() {
    let metadata = VoiceMetadata {
        file_id: "id".to_string(),
        duration_secs: 0,
        file_name_hint: Some("clip.ogg".to_string()),
        mime_type_hint: None,
        voice_note: true,
    };

    assert_eq!(metadata.file_name_hint.as_deref(), Some("clip.ogg"));
}

#[test]
fn voice_metadata_prefers_voice_over_audio_and_filters_blank_hints() {
    let value = serde_json::json!({
        "voice": {
            "file_id": "voice-id",
            "file_name": "   ",
            "mime_type": "",
            "duration": 3
        },
        "audio": {
            "file_id": "audio-id",
            "file_name": "song.mp3",
            "mime_type": "audio/mpeg",
            "duration": 9
        }
    });

    let metadata = TelegramChannel::parse_voice_metadata(&value).unwrap();

    assert_eq!(metadata.file_id, "voice-id");
    assert_eq!(metadata.file_name_hint, None);
    assert_eq!(metadata.mime_type_hint, None);
    assert!(metadata.voice_note);
}

#[test]
fn voice_metadata_reads_audio_file_name_and_defaults_duration() {
    let value = serde_json::json!({
        "audio": {
            "file_id": "audio-id",
            "file_name": "track.mp3",
            "mime_type": "audio/mpeg"
        }
    });

    let metadata = TelegramChannel::parse_voice_metadata(&value).unwrap();

    assert_eq!(metadata.file_id, "audio-id");
    assert_eq!(metadata.duration_secs, 0);
    assert_eq!(metadata.file_name_hint.as_deref(), Some("track.mp3"));
    assert_eq!(metadata.mime_type_hint.as_deref(), Some("audio/mpeg"));
    assert!(!metadata.voice_note);
}

#[test]
fn voice_metadata_requires_string_file_id() {
    assert!(TelegramChannel::parse_voice_metadata(&serde_json::json!({"voice": {}})).is_none());
    assert!(
        TelegramChannel::parse_voice_metadata(&serde_json::json!({"voice": {"file_id": 123}}))
            .is_none()
    );
    assert!(TelegramChannel::parse_voice_metadata(&serde_json::json!({"text": "hello"})).is_none());
}

#[test]
fn extension_from_audio_mime_type_accepts_supported_aliases() {
    let cases = [
        (" audio/flac ", Some("flac")),
        ("AUDIO/X-FLAC", Some("flac")),
        ("audio/mpeg", Some("mp3")),
        ("audio/mp4", Some("mp4")),
        ("audio/x-m4a", Some("m4a")),
        ("application/ogg", Some("ogg")),
        ("audio/opus", Some("opus")),
        ("audio/x-wav", Some("wav")),
        ("audio/wave", Some("wav")),
        ("audio/webm", Some("webm")),
        ("audio/aac", None),
    ];

    for (mime, expected) in cases {
        assert_eq!(TelegramChannel::extension_from_audio_mime_type(mime), expected);
    }
}

#[test]
fn has_file_extension_requires_non_empty_extension() {
    assert!(TelegramChannel::has_file_extension("clip.ogg"));
    assert!(TelegramChannel::has_file_extension("/tmp/archive.tar.gz"));
    assert!(!TelegramChannel::has_file_extension("clip"));
    assert!(!TelegramChannel::has_file_extension("clip."));
}

#[test]
fn infer_voice_filename_prefers_path_basename_with_extension() {
    let metadata = VoiceMetadata {
        file_id: "id".to_string(),
        duration_secs: 0,
        file_name_hint: Some("hint.mp3".to_string()),
        mime_type_hint: Some("audio/ogg".to_string()),
        voice_note: true,
    };

    assert_eq!(
        TelegramChannel::infer_voice_filename("voice/path/server-name.opus", &metadata),
        "server-name.opus"
    );
}

#[test]
fn infer_voice_filename_uses_hint_extension_before_mime() {
    let metadata = VoiceMetadata {
        file_id: "id".to_string(),
        duration_secs: 0,
        file_name_hint: Some("hint.m4a".to_string()),
        mime_type_hint: Some("audio/ogg".to_string()),
        voice_note: false,
    };

    assert_eq!(TelegramChannel::infer_voice_filename("voice/no_extension", &metadata), "hint.m4a");
}

#[test]
fn infer_voice_filename_uses_stem_hint_and_defaults_for_missing_path() {
    let voice = VoiceMetadata {
        file_id: "id".to_string(),
        duration_secs: 0,
        file_name_hint: Some("spoken-note".to_string()),
        mime_type_hint: None,
        voice_note: true,
    };
    assert_eq!(TelegramChannel::infer_voice_filename("", &voice), "spoken-note.ogg");

    let audio = VoiceMetadata { voice_note: false, ..voice };
    assert_eq!(TelegramChannel::infer_voice_filename("downloads/audio.", &audio), "audio.mp3");
}

#[test]
fn infer_voice_filename_uses_default_stem_when_path_and_hints_are_blank() {
    let voice = VoiceMetadata {
        file_id: "id".to_string(),
        duration_secs: 0,
        file_name_hint: None,
        mime_type_hint: Some("audio/webm".to_string()),
        voice_note: true,
    };
    assert_eq!(TelegramChannel::infer_voice_filename("/", &voice), "voice.webm");

    let audio = VoiceMetadata { mime_type_hint: None, voice_note: false, ..voice };
    assert_eq!(TelegramChannel::infer_voice_filename("", &audio), "audio.mp3");
}

#[tokio::test]
async fn try_parse_voice_message_returns_none_when_transcription_disabled() {
    enable_test_tracing();
    let channel = TelegramChannel::new("token".into(), vec!["*".into()], false);

    let update = serde_json::json!({
        "message": {
            "message_id": 1,
            "voice": {"file_id": "voice-file", "duration": 1},
            "from": {"id": 11, "username": "alice"},
            "chat": {"id": 100, "type": "private"}
        }
    });

    assert!(channel.try_parse_voice_message(&update).await.is_none());
    assert!(channel.voice_transcriptions.lock().is_empty());

    assert!(
        channel
            .try_parse_voice_message(&serde_json::json!({
                "message": {
                    "message_id": 2,
                    "text": "not voice",
                    "from": {"id": 11, "username": "alice"},
                    "chat": {"id": 100, "type": "private"}
                }
            }))
            .await
            .is_none()
    );
    assert!(channel.try_parse_voice_message(&serde_json::json!({})).await.is_none());
}

#[tokio::test]
async fn try_parse_voice_message_returns_none_for_missing_update_message_or_metadata() {
    let mut config = crate::app::agent::config::TranscriptionConfig::default();
    config.enabled = true;
    let channel =
        TelegramChannel::new("token".into(), vec!["*".into()], false).with_transcription(config);

    assert!(channel.try_parse_voice_message(&serde_json::json!({})).await.is_none());
    assert!(
        channel
            .try_parse_voice_message(&serde_json::json!({
                "message": {
                    "message_id": 1,
                    "text": "not voice",
                    "from": {"id": 1},
                    "chat": {"id": 10}
                }
            }))
            .await
            .is_none()
    );
}

#[tokio::test]
async fn try_parse_voice_message_rejects_group_mention_only_before_download() {
    let mut config = crate::app::agent::config::TranscriptionConfig::default();
    config.enabled = true;
    config.max_duration_secs = 60;
    let channel =
        TelegramChannel::new("token".into(), vec!["alice".into()], true).with_transcription(config);

    let update = serde_json::json!({
        "message": {
            "message_id": 1,
            "voice": {"file_id": "voice-file", "duration": 1},
            "from": {"id": 11, "username": "alice"},
            "chat": {"id": -100, "type": "group"}
        }
    });

    assert!(channel.try_parse_voice_message(&update).await.is_none());
    assert!(channel.voice_transcriptions.lock().is_empty());
}

#[tokio::test]
async fn try_parse_voice_message_skips_over_duration_before_network() {
    let mut config = crate::app::agent::config::TranscriptionConfig::default();
    config.enabled = true;
    config.max_duration_secs = 5;
    let channel =
        TelegramChannel::new("token".into(), vec!["*".into()], false).with_transcription(config);

    let update = serde_json::json!({
        "message": {
            "message_id": 1,
            "voice": {"file_id": "voice-file", "duration": 6},
            "from": {"id": 11, "username": "alice"},
            "chat": {"id": 100, "type": "private"}
        }
    });

    assert!(channel.try_parse_voice_message(&update).await.is_none());
    assert!(channel.voice_transcriptions.lock().is_empty());
}

#[tokio::test]
async fn try_parse_voice_message_returns_none_when_chat_id_missing() {
    let mut config = crate::app::agent::config::TranscriptionConfig::default();
    config.enabled = true;
    config.max_duration_secs = 60;
    let channel =
        TelegramChannel::new("token".into(), vec!["*".into()], false).with_transcription(config);

    let update = serde_json::json!({
        "message": {
            "message_id": 1,
            "voice": {"file_id": "voice-file", "duration": 1},
            "from": {"id": 11, "username": "alice"},
            "chat": {"type": "private"}
        }
    });

    assert!(channel.try_parse_voice_message(&update).await.is_none());
    assert!(channel.voice_transcriptions.lock().is_empty());
}

#[tokio::test]
async fn try_parse_voice_message_rejects_unauthorized_sender_before_download() {
    let mut config = crate::app::agent::config::TranscriptionConfig::default();
    config.enabled = true;
    config.max_duration_secs = 60;
    let channel = TelegramChannel::new("token".into(), vec!["alice".into()], false)
        .with_transcription(config);

    let update = serde_json::json!({
        "message": {
            "message_id": 1,
            "voice": {"file_id": "voice-file", "duration": 1},
            "from": {"id": 11, "username": "bob"},
            "chat": {"id": 100, "type": "private"}
        }
    });

    assert!(channel.try_parse_voice_message(&update).await.is_none());
    assert!(channel.voice_transcriptions.lock().is_empty());
}

#[tokio::test]
async fn try_parse_voice_message_transcribes_private_voice_and_clears_full_cache() {
    set_transcription_api_key();
    let server = TestServer::spawn(vec![
        json_response(r#"{"ok":true,"result":{"file_path":"voice/server-clip"}}"#),
        bytes_response(StatusCode::OK, b"audio-bytes"),
        json_response(r#"{"text":"hello from voice"}"#),
    ])
    .await;
    let channel = TelegramChannel::new("123:ABC".into(), vec!["alice".into()], false)
        .with_api_base(server.base_url.clone())
        .with_transcription(transcription_config(format!("{}/transcribe", server.base_url)));
    {
        let mut cache = channel.voice_transcriptions.lock();
        for index in 0..100 {
            cache.insert(format!("old:{index}"), "old".to_string());
        }
    }

    let update = serde_json::json!({
        "message": {
            "message_id": 77,
            "voice": {
                "file_id": "voice-file",
                "duration": 5,
                "mime_type": "audio/ogg"
            },
            "from": {"id": 42, "username": "alice"},
            "chat": {"id": 100, "type": "private"}
        }
    });

    let message = channel.try_parse_voice_message(&update).await.expect("voice message");

    assert_eq!(message.id, "telegram_100_77");
    assert_eq!(message.sender, "alice");
    assert_eq!(message.reply_target, "100");
    assert_eq!(message.content, "[Voice] hello from voice");
    assert_eq!(message.channel, "telegram");
    assert_eq!(message.thread_ts, None);
    let cache = channel.voice_transcriptions.lock();
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get("100:77").map(String::as_str), Some("hello from voice"));
    drop(cache);
    assert_eq!(
        server.requests().await,
        vec![
            "GET /bot123:ABC/getFile?file_id=voice-file".to_string(),
            "GET /file/bot123:ABC/voice/server-clip".to_string(),
            "POST /transcribe".to_string(),
        ]
    );
}

#[tokio::test]
async fn try_parse_voice_message_transcribes_audio_with_thread_and_reply_quote() {
    set_transcription_api_key();
    let server = TestServer::spawn(vec![
        json_response(r#"{"ok":true,"result":{"file_path":"telegram/audio."}}"#),
        bytes_response(StatusCode::OK, b"audio-bytes"),
        json_response(r#"{"text":"thread words"}"#),
    ])
    .await;
    let mut config = transcription_config(format!("{}/transcribe", server.base_url));
    config.language = Some("en".to_string());
    let channel = TelegramChannel::new("123:ABC".into(), vec!["42".into()], true)
        .with_group_reply_allowed_senders(vec!["42".into()])
        .with_api_base(server.base_url.clone())
        .with_transcription(config);

    let update = serde_json::json!({
        "message": {
            "message_id": 78,
            "message_thread_id": 9,
            "audio": {
                "file_id": "audio-file",
                "duration": 5,
                "mime_type": "audio/mpeg",
                "file_name": "clip"
            },
            "from": {"id": 42},
            "chat": {"id": 100, "type": "supergroup"},
            "reply_to_message": {
                "message_id": 76,
                "from": {"username": "bob"},
                "text": "quoted\ntext"
            }
        }
    });

    let message = channel.try_parse_voice_message(&update).await.expect("audio message");

    assert_eq!(message.id, "telegram_100_78");
    assert_eq!(message.sender, "42");
    assert_eq!(message.reply_target, "100:9");
    assert_eq!(message.thread_ts.as_deref(), Some("9"));
    assert_eq!(message.content, "> @bob:\n> quoted\n> text\n\n[Voice] thread words");
    assert_eq!(
        server.requests().await,
        vec![
            "GET /bot123:ABC/getFile?file_id=audio-file".to_string(),
            "GET /file/bot123:ABC/telegram/audio.".to_string(),
            "POST /transcribe".to_string(),
        ]
    );
}

#[tokio::test]
async fn try_parse_voice_message_returns_none_when_get_file_path_fails() {
    let server = TestServer::spawn(vec![json_response(r#"{"ok":true,"result":{}}"#)]).await;
    let channel = TelegramChannel::new("123:ABC".into(), vec!["*".into()], false)
        .with_api_base(server.base_url.clone())
        .with_transcription(transcription_config(format!("{}/transcribe", server.base_url)));

    let update = serde_json::json!({
        "message": {
            "message_id": 1,
            "voice": {"file_id": "voice-file", "duration": 1},
            "from": {"id": 11, "username": "alice"},
            "chat": {"id": 100, "type": "private"}
        }
    });

    assert!(channel.try_parse_voice_message(&update).await.is_none());
    assert!(channel.voice_transcriptions.lock().is_empty());
    assert_eq!(
        server.requests().await,
        vec!["GET /bot123:ABC/getFile?file_id=voice-file".to_string()]
    );
}

#[tokio::test]
async fn try_parse_voice_message_returns_none_when_download_fails() {
    let server = TestServer::spawn(vec![
        json_response(r#"{"ok":true,"result":{"file_path":"voice/file.ogg"}}"#),
        bytes_response(StatusCode::NOT_FOUND, b"missing"),
    ])
    .await;
    let channel = TelegramChannel::new("123:ABC".into(), vec!["*".into()], false)
        .with_api_base(server.base_url.clone())
        .with_transcription(transcription_config(format!("{}/transcribe", server.base_url)));

    let update = serde_json::json!({
        "message": {
            "message_id": 1,
            "voice": {"file_id": "voice-file", "duration": 1},
            "from": {"id": 11, "username": "alice"},
            "chat": {"id": 100, "type": "private"}
        }
    });

    assert!(channel.try_parse_voice_message(&update).await.is_none());
    assert!(channel.voice_transcriptions.lock().is_empty());
    assert_eq!(
        server.requests().await,
        vec![
            "GET /bot123:ABC/getFile?file_id=voice-file".to_string(),
            "GET /file/bot123:ABC/voice/file.ogg".to_string(),
        ]
    );
}

#[tokio::test]
async fn try_parse_voice_message_returns_none_when_transcription_fails() {
    let server = TestServer::spawn(vec![
        json_response(r#"{"ok":true,"result":{"file_path":"voice/file.aac"}}"#),
        bytes_response(StatusCode::OK, b"audio-bytes"),
    ])
    .await;
    let channel = TelegramChannel::new("123:ABC".into(), vec!["*".into()], false)
        .with_api_base(server.base_url.clone())
        .with_transcription(transcription_config(format!("{}/transcribe", server.base_url)));

    let update = serde_json::json!({
        "message": {
            "message_id": 1,
            "voice": {"file_id": "voice-file", "duration": 1},
            "from": {"id": 11, "username": "alice"},
            "chat": {"id": 100, "type": "private"}
        }
    });

    assert!(channel.try_parse_voice_message(&update).await.is_none());
    assert!(channel.voice_transcriptions.lock().is_empty());
    assert_eq!(
        server.requests().await,
        vec![
            "GET /bot123:ABC/getFile?file_id=voice-file".to_string(),
            "GET /file/bot123:ABC/voice/file.aac".to_string(),
        ]
    );
}

#[tokio::test]
async fn try_parse_voice_message_returns_none_for_empty_transcription() {
    set_transcription_api_key();
    let server = TestServer::spawn(vec![
        json_response(r#"{"ok":true,"result":{"file_path":"voice/file.ogg"}}"#),
        bytes_response(StatusCode::OK, b"audio-bytes"),
        json_response(r#"{"text":"   "}"#),
    ])
    .await;
    let channel = TelegramChannel::new("123:ABC".into(), vec!["*".into()], false)
        .with_api_base(server.base_url.clone())
        .with_transcription(transcription_config(format!("{}/transcribe", server.base_url)));

    let update = serde_json::json!({
        "message": {
            "message_id": 1,
            "voice": {"file_id": "voice-file", "duration": 1},
            "from": {"id": 11, "username": "alice"},
            "chat": {"id": 100, "type": "private"}
        }
    });

    assert!(channel.try_parse_voice_message(&update).await.is_none());
    assert!(channel.voice_transcriptions.lock().is_empty());
    assert_eq!(
        server.requests().await,
        vec![
            "GET /bot123:ABC/getFile?file_id=voice-file".to_string(),
            "GET /file/bot123:ABC/voice/file.ogg".to_string(),
            "POST /transcribe".to_string(),
        ]
    );
}
