use super::transcription::*;
use std::{sync::Arc, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
use vw_api_types::voice::VoiceErrorCode;
use vw_config_types::voice::{RecognitionLanguage, VoiceConfig};

struct Fixture {
    provider: TranscriptionProvider,
    request: tokio::sync::oneshot::Receiver<Vec<u8>>,
    calls: Arc<std::sync::atomic::AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
async fn fixture(status: u16, body: &str, delay: Duration) -> Fixture {
    delayed_fixture(status, body, delay, false).await
}
async fn delayed_fixture(status: u16, body: &str, delay: Duration, delay_body: bool) -> Fixture {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v1/audio/transcriptions", listener.local_addr().unwrap());
    let (tx, request) = tokio::sync::oneshot::channel();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = calls.clone();
    let body = body.to_owned();
    let server = tokio::spawn(async move {
        let mut tx = Some(tx);
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut bytes = Vec::new();
            loop {
                let mut buf = [0; 4096];
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&buf[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                    let len: usize = head
                        .lines()
                        .find_map(|s| s.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if bytes.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            if let Some(tx) = tx.take() {
                let _ = tx.send(bytes);
            }
            let location = if status == 302 { "Location: /redirected\r\n" } else { "" };
            let head = format!(
                "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{location}\r\n",
                body.len()
            );
            if delay_body {
                let _ = socket.write_all(head.as_bytes()).await;
            }
            tokio::time::sleep(delay).await;
            if !delay_body {
                let _ = socket.write_all(head.as_bytes()).await;
            }
            let _ = socket.write_all(body.as_bytes()).await;
        }
    });
    Fixture { provider: TranscriptionProvider::fixture(&endpoint), request, calls, server }
}
fn wav() -> Vec<u8> {
    let samples = vec![1u8; 480];
    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&(36u32 + samples.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
    wav.extend_from_slice(&24000u32.to_le_bytes());
    wav.extend_from_slice(&48000u32.to_le_bytes());
    wav.extend_from_slice(b"\x02\0\x10\0data");
    wav.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    wav.extend_from_slice(&samples);
    wav
}
#[tokio::test]
async fn voice_task_05_transcription_forwards_language_dictionary() {
    let mut f = fixture(
        200,
        r#"{"text":"ShiJu 好。","languages":[{"code":"zh"}],"usage":{}}"#,
        Duration::ZERO,
    )
    .await;
    let config = VoiceConfig {
        language: RecognitionLanguage::Zh,
        dictionary: vec!["ShiJu".into(), "识矩".into()],
        prompt: "技术讨论".into(),
        ..Default::default()
    };
    let result =
        f.provider.transcribe(wav(), "audio/wav", &config, &CancellationToken::new()).await;
    assert!(result.is_ok(), "completed recording must produce final text: {result:?}");
    let result = result.unwrap();
    assert_eq!(result.text, "ShiJu 好。");
    assert_eq!(result.detected_languages, ["zh"]);
    let request = (&mut f.request).await.unwrap();
    let text = String::from_utf8_lossy(&request);
    assert!(text.starts_with("POST /v1/audio/transcriptions HTTP/1.1"));
    for (field, value) in [
        ("model", "gpt-transcribe"),
        ("languages[]", "zh"),
        ("keywords[]", "ShiJu"),
        ("keywords[]", "识矩"),
        ("prompt", "技术讨论"),
        ("response_format", "json"),
        ("stream", "false"),
    ] {
        assert!(text.contains(&format!("name=\"{field}\"\r\n\r\n{value}\r\n")), "missing {field}");
    }
    assert!(text.contains("filename=\"dictation.wav\"\r\nContent-Type: audio/wav"));
    assert!(request.windows(wav().len()).any(|part| part == wav()));
    assert!(text.to_ascii_lowercase().contains("authorization: bearer fixture-secret"));
    assert!(!text.contains("name=\"language\""));
}
#[tokio::test]
async fn voice_task_05_transcription_error_redacts_payload() {
    for (status, expected) in [
        (401, VoiceErrorCode::ProviderAuth),
        (429, VoiceErrorCode::RateLimited),
        (500, VoiceErrorCode::ProviderUnavailable),
    ] {
        let f = fixture(
            status,
            r#"{"error":{"message":"fixture-secret private-audio private-dictionary"}}"#,
            Duration::ZERO,
        )
        .await;
        let error = f
            .provider
            .transcribe(wav(), "audio/wav", &VoiceConfig::default(), &CancellationToken::new())
            .await
            .unwrap_err();
        assert_eq!(error, TranscriptionError::Service(expected));
        let displayed = format!("{error} {error:?}");
        for secret in ["fixture-secret", "private-audio", "private-dictionary"] {
            assert!(!displayed.contains(secret));
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(
            f.calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "billable request must not retry"
        );
    }
}

#[tokio::test]
async fn voice_task_05_auto_language_omits_hints_and_empty_text_explains() {
    let mut f = fixture(200, r#"{"text":"plain final text"}"#, Duration::ZERO).await;
    let result = f
        .provider
        .transcribe(wav(), "audio/wav", &VoiceConfig::default(), &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(result.text, "plain final text");
    assert!(result.detected_languages.is_empty());
    let request = (&mut f.request).await.unwrap();
    let request = String::from_utf8_lossy(&request);
    for field in ["language", "languages[]", "keywords[]", "prompt"] {
        assert!(!request.contains(&format!("name=\"{field}\"")));
    }
    let f = fixture(200, r#"{"text":"  \n","languages":[]}"#, Duration::ZERO).await;
    let error = f
        .provider
        .transcribe(wav(), "audio/wav", &VoiceConfig::default(), &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error, TranscriptionError::EmptyTranscript);
    assert!(error.to_string().contains("重新录音"));
}

#[tokio::test]
async fn voice_task_05_invalid_media_and_hints_never_upload() {
    let f = fixture(200, r#"{"text":"unexpected"}"#, Duration::ZERO).await;
    let mut wrong_rate = wav();
    wrong_rate[24..28].copy_from_slice(&16000u32.to_le_bytes());
    let mut truncated = wav();
    truncated.pop();
    let mut too_long = wav();
    too_long.resize(44 + 180 * 24000 * 2 + 2, 0);
    let len = too_long.len();
    too_long[4..8].copy_from_slice(&((len - 8) as u32).to_le_bytes());
    too_long[40..44].copy_from_slice(&((len - 44) as u32).to_le_bytes());
    for (audio, media, expected) in [
        (Vec::new(), "audio/wav", VoiceErrorCode::InvalidRequest),
        (wav(), "audio/mpeg", VoiceErrorCode::UnsupportedMedia),
        (wrong_rate, "audio/wav", VoiceErrorCode::UnsupportedMedia),
        (truncated, "audio/wav", VoiceErrorCode::UnsupportedMedia),
        (too_long, "audio/wav", VoiceErrorCode::PayloadTooLarge),
        (vec![0; 24_000_001], "audio/wav", VoiceErrorCode::PayloadTooLarge),
    ] {
        assert_eq!(
            f.provider
                .transcribe(audio, media, &VoiceConfig::default(), &CancellationToken::new())
                .await,
            Err(TranscriptionError::Service(expected))
        );
    }
    for config in [
        VoiceConfig { dictionary: vec!["x".into(); 101], ..Default::default() },
        VoiceConfig { dictionary: vec!["secret\nvalue".into()], ..Default::default() },
        VoiceConfig { dictionary: vec!["字".repeat(81)], ..Default::default() },
        VoiceConfig { prompt: "字".repeat(2001), ..Default::default() },
    ] {
        assert_eq!(
            f.provider.transcribe(wav(), "audio/wav", &config, &CancellationToken::new()).await,
            Err(TranscriptionError::Service(VoiceErrorCode::InvalidRequest))
        );
    }
    assert_eq!(f.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[tokio::test]
async fn voice_task_05_cancel_and_timeout_drop_request_without_retry() {
    let mut f = fixture(200, r#"{"text":"late private text"}"#, Duration::from_secs(10)).await;
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        f.provider.transcribe(wav(), "audio/wav", &VoiceConfig::default(), &cancel).await,
        Err(TranscriptionError::Service(VoiceErrorCode::Cancelled))
    );
    assert_eq!(f.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    let cancel = CancellationToken::new();
    let config = VoiceConfig::default();
    let call = f.provider.transcribe(wav(), "audio/wav", &config, &cancel);
    tokio::pin!(call);
    tokio::select! {
        biased;
        result = &mut call => panic!("must wait for provider, got {result:?}"),
        result = &mut f.request => { result.unwrap(); },
    }
    cancel.cancel();
    assert_eq!(call.await, Err(TranscriptionError::Service(VoiceErrorCode::Cancelled)));
    let f = fixture(200, r#"{"text":"late"}"#, Duration::from_secs(10)).await;
    assert_eq!(
        f.provider
            .transcribe(wav(), "audio/wav", &VoiceConfig::default(), &CancellationToken::new())
            .await,
        Err(TranscriptionError::Service(VoiceErrorCode::Timeout))
    );
    assert_eq!(f.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn voice_task_05_malformed_oversized_and_redirect_responses_fail_closed() {
    for body in [
        r#"{"usage":1}"#.to_owned(),
        "private invalid JSON".into(),
        format!(r#"{{"text":"{}"}}"#, "x".repeat(65537)),
        format!(r#"{{"text":"{}"}}"#, "x".repeat(256 * 1024)),
    ] {
        let f = fixture(200, &body, Duration::ZERO).await;
        assert_eq!(
            f.provider
                .transcribe(wav(), "audio/wav", &VoiceConfig::default(), &CancellationToken::new())
                .await,
            Err(TranscriptionError::Service(VoiceErrorCode::ProtocolError))
        );
    }
    let f = fixture(302, "private redirect body", Duration::ZERO).await;
    assert_eq!(
        f.provider
            .transcribe(wav(), "audio/wav", &VoiceConfig::default(), &CancellationToken::new())
            .await,
        Err(TranscriptionError::Service(VoiceErrorCode::ProviderUnavailable))
    );
    assert_eq!(f.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn voice_task_05_upstream_requires_tls_and_rejects_credential_urls() {
    for base in [
        "http://example.com/v1",
        "https://user:secret@example.com/v1",
        "https://example.com/v1?token=secret",
        "https://example.com/v1#secret",
        "file:///tmp/voice",
    ] {
        assert!(matches!(
            TranscriptionProvider::connect(
                base,
                "fixture-secret",
                reqwest::Client::builder().no_proxy(),
                Duration::from_secs(2)
            ),
            Err(TranscriptionError::Service(VoiceErrorCode::InvalidRequest))
        ));
    }
    assert!(matches!(
        TranscriptionProvider::connect(
            "https://api.openai.com/v1",
            "secret\nheader",
            reqwest::Client::builder().no_proxy(),
            Duration::from_secs(2)
        ),
        Err(TranscriptionError::Service(VoiceErrorCode::ProviderAuth))
    ));
}

#[tokio::test]
async fn voice_task_05_timeout_covers_response_body() {
    let f = delayed_fixture(200, r#"{"text":"late"}"#, Duration::from_secs(10), true).await;
    assert_eq!(
        f.provider
            .transcribe(wav(), "audio/wav", &VoiceConfig::default(), &CancellationToken::new())
            .await,
        Err(TranscriptionError::Service(VoiceErrorCode::Timeout))
    );
    assert_eq!(f.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
}
