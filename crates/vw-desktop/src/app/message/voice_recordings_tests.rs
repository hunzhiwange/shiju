use super::*;
fn app() -> App {
    let (mut app, _) = App::new();
    app.active_session_id = Some("voice-archive".into());
    app.show_system_settings = true;
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Voice;
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("原草稿");
    app
}
#[test]
fn voice_task_14_recordings_retry_requires_explicit_apply_and_rejects_late_reply() {
    let mut app = app();
    let scope = super::super::voice::scope(&app).unwrap();
    app.voice.recordings.scope = Some(scope);
    app.voice.recordings.generation = 7;
    app.voice.recordings.busy = true;
    let _ = update(&mut app, Event::Retried(7, Ok("识别结果".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "原草稿");
    assert_eq!(app.voice.recordings.result.as_deref(), Some("识别结果"));
    let _ = update(&mut app, Event::Apply);
    assert!(app.current_session_runtime_ref().unwrap().input_editor.text().contains("识别结果"));
    assert!(!app.current_session_runtime_ref().unwrap().is_requesting);
    let _ = update(&mut app, Event::Retried(7, Ok("迟到".into())));
    assert!(app.voice.recordings.result.is_none());
}
#[test]
fn voice_task_14_recordings_session_switch_discards_retry_result() {
    let mut app = app();
    let scope = super::super::voice::scope(&app).unwrap();
    app.voice.recordings.scope = Some(scope);
    app.voice.recordings.generation = 7;
    app.voice.recordings.busy = true;
    app.active_session_id = Some("other".into());
    let _ = update(&mut app, Event::Retried(7, Ok("旧结果".into())));
    assert!(app.voice.recordings.result.is_none());
    assert!(!app.voice.recordings.busy);
}
#[tokio::test]
async fn voice_task_14_recordings_retry_reads_stored_wav_over_loopback() {
    use std::io::{BufRead, Read, Write};
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("archive")).unwrap();
    let audio = crate::app::voice::capture::Recording { generation: 1, pcm: vec![1234; 480] }
        .wav()
        .unwrap();
    store.save(true, &audio).unwrap();
    let id = store.list().unwrap()[0].id.clone();
    let saved = store.read(&id).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        for index in 0..2 {
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(s) => break s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(std::time::Instant::now() < deadline, "missing retry request");
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut reader = std::io::BufReader::new(&mut stream);
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse::<usize>().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            drop(reader);
            let response = if index == 0 {
                assert!(request.starts_with("GET /api/voice/config "));
                serde_json::to_vec(&vw_config_types::voice::VoiceConfig {
                    dictation_enabled: true,
                    provider_id: Some("openai".into()),
                    ..Default::default()
                })
                .unwrap()
            } else {
                assert!(request.starts_with("POST /api/voice/transcriptions "));
                assert!(body.windows(audio.len()).any(|part| part == audio));
                let begin = body.windows(14).position(|p| p == b"{\"request_id\":").unwrap();
                let end = begin + body[begin..].iter().position(|b| *b == b'}').unwrap() + 1;
                let mut metadata: serde_json::Value =
                    serde_json::from_slice(&body[begin..end]).unwrap();
                assert_eq!(metadata["generation"], 9);
                metadata.as_object_mut().unwrap().remove("provider_id");
                metadata["text"] = serde_json::json!("本机录音重新识别");
                metadata["detected_languages"] = serde_json::json!(["zh"]);
                serde_json::to_vec(&metadata).unwrap()
            };
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",response.len()).unwrap();
            stream.write_all(&response).unwrap();
        }
    });
    let result = retry(endpoint, "voice-archive".into(), 9, saved).await;
    server.join().unwrap();
    assert_eq!(result.unwrap(), "本机录音重新识别");
    assert_eq!(store.list().unwrap().len(), 1, "retry must not create another archive entry");
}
