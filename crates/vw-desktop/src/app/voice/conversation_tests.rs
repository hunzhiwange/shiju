use super::*;
use crate::app::voice::capture::{CaptureBackend, CaptureError, CaptureSink};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
struct Device(Arc<AtomicBool>);
struct Stream(Arc<AtomicBool>);
impl Drop for Stream {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
impl CaptureBackend for Device {
    type Stream = Stream;
    fn open(self, _: Option<String>, _: CaptureSink) -> Result<Stream, CaptureError> {
        Ok(Stream(self.0))
    }
}
#[test]
fn voice_task_11_conversation_end_releases_capture() {
    let released = Arc::new(AtomicBool::new(false));
    let lease = Arc::new(AtomicBool::new(false));
    let capture = Capture::start_backend(
        Device(released.clone()),
        None,
        Duration::from_secs(1),
        lease.clone(),
    )
    .unwrap();
    let mut media = Media::<()> { capture: Some(capture), playback: None };
    media.end();
    assert!(released.load(Ordering::Acquire), "ending must release the physical capture stream");
    assert!(!lease.load(Ordering::Acquire));
    assert!(media.capture.is_none());
}
#[test]
fn voice_task_11_conversation_interrupt_stops_old_audio() {
    let (mut playback, mut output) = crate::app::voice::playback::fixture(24000, 1);
    playback.push_pcm(&[100, 10, 100, 10]).unwrap();
    let mut media = Media { capture: None, playback: Some(playback) };
    media.interrupt();
    let mut audio = [1.0f32; 2];
    output.render(&mut audio);
    assert_eq!(audio, [0.0; 2], "interruption must silence queued output synchronously");
    assert!(media.playback.is_none());
}
#[test]
fn voice_task_11_captions_replace_deltas_and_ignore_repeated_final() {
    let mut state = Snapshot::default();
    let item = VoiceId::try_from("item-1".to_owned()).unwrap();
    state.caption(TranscriptRole::User, item.clone(), 0, "你".into(), false);
    state.caption(TranscriptRole::User, item.clone(), 0, "好".into(), false);
    state.caption(TranscriptRole::User, item.clone(), 0, "你好。".into(), true);
    state.caption(TranscriptRole::User, item.clone(), 0, "你好。".into(), true);
    state.caption(TranscriptRole::User, item.clone(), 0, "旧增量".into(), false);
    state.caption(TranscriptRole::Assistant, item, 0, "你好".into(), true);
    assert_eq!(state.captions.len(), 2);
    assert_eq!(state.captions[0].text, "你好。");
    assert_eq!(state.captions[1].role, TranscriptRole::Assistant);
}
#[test]
fn voice_task_11_playback_progress_tracks_consumed_samples() {
    let (mut playback, mut output) = crate::app::voice::playback::fixture(48000, 1);
    playback.push_pcm(&[1; 96]).unwrap();
    assert!(playback.pending());
    output.render(&mut [0.0f32; 48]);
    assert_eq!(playback.played_ms(), 1);
    assert!(playback.pending());
    output.render(&mut [0.0f32; 48]);
    assert_eq!(playback.played_ms(), 2);
    assert!(!playback.pending());
}
#[test]
fn voice_task_11_live_capture_has_bounded_frames_and_clear_discards_pending() {
    use std::sync::Mutex;
    struct Input(Arc<Mutex<Option<CaptureSink>>>, Arc<AtomicBool>);
    impl CaptureBackend for Input {
        type Stream = Stream;
        fn open(self, _: Option<String>, sink: CaptureSink) -> Result<Stream, CaptureError> {
            *self.0.lock().unwrap() = Some(sink);
            Ok(Stream(self.1))
        }
    }
    let sink = Arc::new(Mutex::new(None));
    let released = Arc::new(AtomicBool::new(false));
    let live = LiveCapture::start_backend(
        Input(sink.clone(), released.clone()),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    let (capture, frames) = live.into_parts();
    let sender = sink.lock().unwrap().as_ref().unwrap().clone();
    sender.send([100; 480], 480);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(frame) = frames.next_frame() {
            assert_eq!(frame, vec![100; 480]);
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    sender.send([101; 480], 480);
    frames.clear();
    sender.send([201; 480], 480);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(frame) = frames.next_frame() {
            assert_eq!(
                frame,
                vec![201; 480],
                "clear must invalidate frames still queued in capture worker"
            );
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    let mut media = Media::<()> { capture: Some(capture), playback: None };
    media.end();
    sender.send([200; 480], 480);
    assert!(frames.next_frame().is_none());
    assert!(released.load(Ordering::Acquire));
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn voice_task_11_end_cancels_pending_gateway_connection() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let (ready, waiting) = tokio::sync::oneshot::channel();
    let gateway = tokio::spawn(async move {
        let (mut config, _) = listener.accept().await.unwrap();
        let mut headers = vec![];
        let mut byte = [0];
        while !headers.ends_with(b"\r\n\r\n") {
            config.read_exact(&mut byte).await.unwrap();
            headers.push(byte[0]);
            assert!(headers.len() < 8192);
        }
        let body = serde_json::to_string(&vw_config_types::voice::VoiceConfig {
            realtime_enabled: true,
            provider_id: Some("openai".into()),
            ..Default::default()
        })
        .unwrap();
        config.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        drop(config);
        let (mut pending, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        assert!(pending.read(&mut request).await.unwrap() > 0);
        ready.send(()).unwrap();
        // Drain any remaining request bytes, then observe socket destruction.
        loop {
            if pending.read(&mut request).await.unwrap() == 0 {
                break;
            }
        }
    });
    let mut conversation = Conversation::start(endpoint, "chat".into()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), waiting).await.unwrap().unwrap();
    conversation.end();
    assert!(!conversation.snapshot().microphone);
    assert_eq!(conversation.snapshot().phase, Phase::Ending);
    tokio::time::timeout(Duration::from_secs(5), gateway).await.unwrap().unwrap();
}
