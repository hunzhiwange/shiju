use super::*;
use recordings::Store;
#[tokio::test]
async fn voice_task_14_recordings_retry_reads_stored_wav_over_loopback() {
    use std::io::{BufRead, Read, Write};
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("archive")).unwrap();
    let audio = audio_capture::Recording { generation: 1, pcm: vec![1234; 480] }
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
            // Accepted sockets can inherit the listener's nonblocking mode on macOS.
            stream.set_nonblocking(false).unwrap();
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
                serde_json::to_vec(&config_voice::VoiceConfig {
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
    let result = retry(
        Preferences {
            engine: desktop_voice::preferences::VoiceEngine::Cloud,
            ..Default::default()
        },
        endpoint,
        Some("voice-archive".into()),
        9,
        saved,
        desktop_voice::local::Cancellation::default(),
    )
    .await;
    server.join().unwrap();
    assert_eq!(result.unwrap(), "本机录音重新识别");
    assert_eq!(store.list().unwrap().len(), 1, "retry must not create another archive entry");
}
