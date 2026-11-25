use super::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use vw_gateway_client::media::{MediaModel, MediaModels, MediaSubmit};

#[test]
fn media_picker_filters_operations_and_keeps_selection_with_normal_send() {
    use desktop_chat_input::input_panel::media_shortcuts::MediaModelCatalog;
    use desktop_session::message::chat::{
        request::ChatSendBehavior,
        send_input::{SendInputEffect, prepare_send_input},
    };

    let mut app = App::new().0;
    app.session_state.active_session_id = Some("media-picker-session".into());
    app.session_state.input_popovers.set_media_shortcuts(true);
    let runtime = app.current_session_runtime_mut();
    runtime.model = "chat-provider/chat-model".into();
    runtime.auto_model = false;
    runtime.input_editor = iced::widget::text_editor::Content::with_text("保留这段草稿");

    let operation = MediaOperation::VideoGenerate;
    let _ = update(&mut app, Event::OpenModelPicker(operation));
    let key = app.media_jobs.picker.as_ref().unwrap().key.clone();
    let video = MediaModel {
        deployment_id: "custom:video".into(),
        model_id: "video-model".into(),
        display_name: "Video Model".into(),
        operations: vec![operation],
    };
    let image = MediaModel {
        deployment_id: "shiju:image".into(),
        model_id: "image-model".into(),
        display_name: "Image Model".into(),
        operations: vec![MediaOperation::ImageGenerate],
    };
    let _ = update(
        &mut app,
        Event::ModelPickerLoaded(key, Ok(MediaModels { models: vec![image, video.clone()] })),
    );
    let MediaModelCatalog::Ready(models) = model_picker(&app).unwrap().catalog else {
        panic!("expected filtered media models");
    };
    assert_eq!(models, [video.clone()]);

    let selection = MediaModelSelection {
        operation,
        deployment_id: video.deployment_id,
        model_id: video.model_id,
    };
    let _ = update(&mut app, Event::ModelPicked(Some(selection.clone())));
    assert!(model_picker(&app).is_none());
    let runtime = app.current_session_runtime_mut();
    assert!(runtime.input_editor.text().contains("保留这段草稿"));
    runtime.input_editor = iced::widget::text_editor::Content::with_text("生成海边日落的视频");
    let effect =
        prepare_send_input(runtime, &mut vec![], None, None, false, ChatSendBehavior::Guide, 1);
    let SendInputEffect::Send(item) = effect else {
        panic!("expected a normal chat request");
    };
    assert_eq!(item.send_behavior, ChatSendBehavior::Queue);
    assert_eq!(item.model.as_deref(), Some("chat-provider/chat-model"));
    assert_eq!(item.media_model.unwrap().selection, selection);
    assert!(runtime.pending_media_model.is_none());
}

#[test]
fn media_job_ui_restores_scoped_results_and_never_regresses_a_canceled_job() {
    let folder = tempfile::tempdir().unwrap();
    let a = scope("session-a");
    let b = scope("session-b");
    let id = MediaJobId::new_v4();
    let index = folder.path().join("identity").join(&a.session_id);
    let other = folder.path().join("identity").join(&b.session_id);
    let reference = JobRef { id, operation: MediaOperation::VideoGenerate };
    remember_job(&index, reference).unwrap();
    assert_eq!(read_job_refs(&index).unwrap(), [reference]);
    assert!(read_job_refs(&other).unwrap().is_empty());

    let mut entry = JobEntry::new(reference);
    let running = snapshot(
        &a,
        id,
        reference.operation,
        MediaJobState::Running { progress_percent: Some(45) },
    );
    assert!(!entry.accept(&b, running.clone()));
    assert!(entry.accept(&a, running.clone()));
    assert!(entry.accept(&a, MediaJob { state: MediaJobState::Canceled, ..running.clone() }));
    assert!(!entry.accept(&a, running));
    assert_eq!(entry.snapshot.unwrap().state, MediaJobState::Canceled);
}

#[test]
fn media_job_ui_projects_image_preview_playback_save_and_text() {
    for (mime, primary, extension) in [
        ("image/png", AssetAction::Preview, "png"),
        ("audio/mpeg", AssetAction::Play, "mp3"),
        ("video/mp4", AssetAction::Play, "mp4"),
    ] {
        let asset = asset(mime);
        assert_eq!(asset_actions(&asset), [primary, AssetAction::Save]);
        assert_eq!(asset_extension(&asset.mime), extension);
    }
    for operation in [
        MediaOperation::ImageUnderstand,
        MediaOperation::VideoUnderstand,
        MediaOperation::AudioTranscribe,
    ] {
        let reference = JobRef { id: MediaJobId::new_v4(), operation };
        let request_scope = scope("result-session");
        let mut entry = JobEntry::new(reference);
        assert!(entry.accept(
            &request_scope,
            snapshot(
                &request_scope,
                reference.id,
                operation,
                MediaJobState::Succeeded {
                    result: MediaResult { text: Some("返回的文本".into()), assets: vec![] },
                }
            )
        ));
        assert_eq!(entry.result().unwrap().text.as_deref(), Some("返回的文本"));
    }
}

#[tokio::test]
async fn media_job_ui_fixture_transports_identity_and_scopes_submit_progress_cancel_and_output() {
    let request_scope = scope("fixture-session");
    let id = MediaJobId::new_v4();
    let completed = MediaJobId::new_v4();
    let operation = MediaOperation::VideoGenerate;
    let output = asset("video/mp4");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let session = request_scope.clone();
    let media = output.clone();
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let job_path = |id| format!("/api/media/jobs/{id}?session_id=fixture-session");
        let responses = [
            (
                "GET",
                "/api/media/models?session_id=fixture-session".into(),
                "application/json",
                serde_json::to_vec(&MediaModels {
                    models: vec![MediaModel {
                        deployment_id: "direct".into(),
                        model_id: "video".into(),
                        display_name: "Video".into(),
                        operations: vec![operation],
                    }],
                })
                .unwrap(),
            ),
            (
                "POST",
                "/api/media/jobs".into(),
                "application/json",
                serde_json::to_vec(&snapshot(&session, id, operation, MediaJobState::Submitted))
                    .unwrap(),
            ),
            (
                "GET",
                job_path(id),
                "application/json",
                serde_json::to_vec(&snapshot(
                    &session,
                    id,
                    operation,
                    MediaJobState::Running { progress_percent: Some(40) },
                ))
                .unwrap(),
            ),
            (
                "DELETE",
                job_path(id),
                "application/json",
                serde_json::to_vec(&snapshot(&session, id, operation, MediaJobState::Canceled))
                    .unwrap(),
            ),
            (
                "GET",
                job_path(completed),
                "application/json",
                serde_json::to_vec(&snapshot(
                    &session,
                    completed,
                    operation,
                    MediaJobState::Succeeded {
                        result: MediaResult { text: None, assets: vec![media] },
                    },
                ))
                .unwrap(),
            ),
            (
                "GET",
                format!("/api/media/jobs/{completed}/assets/output?session_id=fixture-session"),
                "video/mp4",
                b"clip".to_vec(),
            ),
        ];
        for (method, path, mime, response) in responses {
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("fixture accept failed: {error}"),
                }
            };
            socket.set_nonblocking(false).unwrap();
            socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut reader = BufReader::new(&mut socket);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert_eq!(line.trim_end(), format!("{method} {path} HTTP/1.1"));
            let mut headers = String::new();
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                assert!(!line.is_empty());
                if line == "\r\n" {
                    break;
                }
                headers.push_str(&line.to_ascii_lowercase());
            }
            assert!(headers.contains("authorization: bearer fixture-key\r\n"));
            assert!(headers.contains("x-company-id: fixture-company\r\n"));
            assert!(headers.contains("x-user-id: fixture-user\r\n"));
            if method == "POST" {
                let len = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                let mut bytes = vec![0; len];
                reader.read_exact(&mut bytes).unwrap();
                let request: MediaSubmit = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(
                    (
                        request.session_id,
                        request.request_id,
                        request.deployment_id,
                        request.model_id
                    ),
                    ("fixture-session".into(), id, "direct".into(), "video".into())
                );
            }
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response.len()).unwrap();
            socket.write_all(&response).unwrap();
        }
    });
    let identity = gateway_endpoint::GatewayEndpoint::new("127.0.0.1", port)
        .with_auth(gateway_endpoint::GatewayAuth { skey: Some("fixture-key".into()) })
        .with_tenant_scope(
            vw_api_transport::id::TenantScope::new("fixture-platform", "fixture-company").unwrap(),
        )
        .with_user_id(vw_api_transport::id::UserId::new("fixture-user").unwrap());
    let client = GatewayClient::new(identity).unwrap();
    let models = client.media_models(&request_scope).await.unwrap();
    let mut entry = JobEntry::new(JobRef { id, operation });
    let model = &models.models[0];
    let request = MediaSubmit {
        request_id: id,
        session_id: request_scope.session_id.clone(),
        operation,
        deployment_id: model.deployment_id.clone(),
        model_id: model.model_id.clone(),
        text: Some("Moonlight".into()),
        inputs: vec![],
    };
    let folder = tempfile::tempdir().unwrap();
    let index = folder.path().join("media-ref-index");
    let reference = JobRef { id, operation };
    assert!(entry.accept(&request_scope, client.media_submit(&request).await.unwrap()));
    remember_job(&index, reference).unwrap();
    assert_eq!(read_job_refs(&index).unwrap(), [reference]);
    assert!(entry.accept(
        &request_scope,
        request_update(&client, &request_scope, reference, false).await.unwrap()
    ));
    assert_eq!(
        entry.snapshot.as_ref().unwrap().state,
        MediaJobState::Running { progress_percent: Some(40) }
    );
    assert!(entry.accept(
        &request_scope,
        request_update(&client, &request_scope, reference, true).await.unwrap()
    ));
    assert_eq!(entry.snapshot.unwrap().state, MediaJobState::Canceled);
    let job = request_update(&client, &request_scope, JobRef { id: completed, operation }, false)
        .await
        .unwrap();
    let MediaJobState::Succeeded { result } = job.state else { panic!("expected result") };
    assert_eq!(result.assets, [output.clone()]);
    let file = download_asset(&client, &request_scope, completed, &output, None).await.unwrap();
    assert_eq!(std::fs::read(file.path()).unwrap(), b"clip");
    server.join().unwrap();
}

fn scope(session_id: &str) -> MediaSessionQuery {
    MediaSessionQuery { session_id: session_id.into() }
}

fn snapshot(
    scope: &MediaSessionQuery,
    id: MediaJobId,
    operation: MediaOperation,
    state: MediaJobState,
) -> MediaJob {
    MediaJob {
        id,
        operation,
        session_id: scope.session_id.clone(),
        created_at_ms: 1,
        updated_at_ms: 2,
        state,
    }
}

fn asset(mime: &str) -> MediaAsset {
    let source = if mime.starts_with("image/") {
        MediaAssetSource::Image { image_id: "a".repeat(64) }
    } else {
        MediaAssetSource::Stream
    };
    MediaAsset {
        id: "output".into(),
        mime: mime.into(),
        byte_len: 4,
        width: None,
        height: None,
        duration_ms: None,
        source,
    }
}
