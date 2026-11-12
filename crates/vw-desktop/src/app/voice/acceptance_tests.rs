//! Cross-feature cancellation uses real HTTP sockets and the production message router.
use super::{playback, preview::Preview};
use crate::app::{
    App,
    message::{voice, voice_draft::Phase},
};
use std::{
    io::Read,
    net::TcpListener,
    sync::mpsc,
    time::{Duration, Instant},
};
use vw_config_types::voice::VoiceName;
use vw_gateway_client::GatewayEndpoint;

#[test]
fn voice_task_19_voice_acceptance_cancel_has_no_external_write() {
    let mut app = App::new().0;
    app.active_session_id = Some("acceptance-cancel".into());
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("原始草稿");
    // Capture a failed OS boundary: cancellation must discard the external destination,
    // rather than fall back into the current chat or preserve text for later copying.
    let endpoint = crate::app::gateway_client_endpoint();
    let scope = crate::app::message::voice_draft::Scope {
        endpoint: endpoint.base_url(),
        tenant: serde_json::to_string(endpoint.tenant_scope()).unwrap(),
        project: app.project_path.clone(),
        composer: crate::app::message::voice_draft::Composer::Session("acceptance-cancel".into()),
    };
    let generation = app.voice.draft.start(scope, "原始草稿".into()).unwrap();
    app.voice.draft.phase = Phase::Transcribing;
    app.voice.insertion.target = Some(Err("目标已关闭".into()));
    let _ = voice::update(&mut app, voice::Event::Cancel);
    let _ = voice::update(
        &mut app,
        voice::Event::Transcribed(generation, Ok(("迟到结果".into(), vec![1, 2]))),
    );
    let _ = voice::update(&mut app, voice::Event::Completed(generation, Ok("late result".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "原始草稿");
    assert!(!app.current_session_runtime_ref().unwrap().is_requesting);
    assert!(app.voice.insertion.target.is_none());
    assert!(app.voice.insertion.pending.is_none());
    assert_eq!(app.voice.draft.phase, Phase::Idle);
    assert!(app.files.is_empty());
}

#[test]
fn voice_task_19_voice_acceptance_end_closes_all_streams() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let (ready, waiting) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut headers = vec![];
        let mut byte = [0];
        while !headers.ends_with(b"\r\n\r\n") {
            socket.read_exact(&mut byte).unwrap();
            headers.push(byte[0]);
            assert!(headers.len() < 8192);
        }
        ready.send(()).unwrap();
        let mut buffer = [0; 4096];
        loop {
            if socket.read(&mut buffer).unwrap() == 0 {
                break;
            }
        }
    });
    let mut app = App::new().0;
    let (output, _) = playback::fixture(24000, 1);
    app.voice.settings.preview =
        Some(Preview::start_output(endpoint, VoiceName::Marin, || Ok(output)).unwrap());
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    let began = Instant::now();
    // The same shutdown entry point is called when the production window closes.
    voice::cancel(&mut app);
    let stopped = app.voice.settings.preview.is_none();
    // Reap even on the red path, before reporting the failed lifecycle assertion.
    app.voice.settings.preview.take();
    server.join().unwrap();
    assert!(stopped, "ending voice must cancel the pending preview HTTP stream");
    assert!(began.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn voice_task_19_gateway_http_ws_roundtrip_releases_connection() {
    let captions = super::acceptance_fixture::roundtrip().await.unwrap();
    assert_eq!(captions[0].1, "中文动态字幕 🌱");
    assert_eq!(captions[1].1, "English caption");
}

#[cfg(target_os = "macos")]
#[test]
fn voice_task_19_full_app_cancel_releases_local_preview_and_preserves_draft() {
    use crate::app::Message;
    let (mut app, _) = App::new();
    app.active_session_id = Some("acceptance-local".into());
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("保留本机草稿");
    let (started, ready) = mpsc::channel();
    let (stopped, done) = mpsc::channel();
    app.voice.settings.preview = Some(
        Preview::start_local_worker(None, move |_, cancel| {
            started.send(()).unwrap();
            while !cancel.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
            stopped.send(()).unwrap();
            Ok(())
        })
        .unwrap(),
    );
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let _ = app.update(Message::Voice(voice::Event::ChatStart));
    assert!(app.voice.local_conversation.is_none(), "preview must exclude local chat");
    assert!(app.voice.conversation.is_none(), "preview must exclude cloud chat");
    let _ = app.update(Message::Voice(voice::Event::Start));
    assert_eq!(app.voice.draft.phase, Phase::Idle, "preview must exclude dictation");
    let _ = app.update(Message::Voice(voice::Event::Cancel));
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(app.voice.settings.preview.is_none());
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "保留本机草稿");
    assert!(app.current_session_runtime_ref().unwrap().active_agent_request.is_none());
}
