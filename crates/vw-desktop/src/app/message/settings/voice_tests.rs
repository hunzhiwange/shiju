use super::*;
#[test]
fn voice_task_13_late_load_cannot_reopen_cancelled_form() {
    let (mut app, _) = App::new();
    app.show_system_settings = true;
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Voice;
    app.voice.settings.endpoint = Some(crate::app::gateway_client_endpoint());
    app.voice.settings.generation = 4;
    app.voice.settings.edit = Some(Edit::new(VoiceConfig::default(), Preferences::default()));
    let _ = update(&mut app, Event::Cancel);
    let _ = update(
        &mut app,
        Event::Loaded(
            4,
            Ok(Loaded {
                config: VoiceConfig::default(),
                local: Preferences::default(),
                devices: vec![],
                warning: None,
                resources: LocalResources::default(),
                cloud_ready: false,
                cloud_error: None,
            }),
        ),
    );
    assert!(app.voice.settings.edit.is_none());
}
#[test]
fn voice_task_13_connection_identity_includes_tenant_and_auth() {
    let endpoint = GatewayEndpoint::new("localhost", 1234);
    let changed =
        endpoint.clone().with_auth(vw_gateway_client::GatewayAuth { skey: Some("fixture".into()) });
    assert!(!same_endpoint(&endpoint, &changed));
    assert!(same_endpoint(&endpoint, &endpoint.clone()));
}

#[test]
fn voice_task_13_local_save_ignores_invalid_cloud_settings() {
    let (mut app, _) = App::new();
    app.show_system_settings = true;
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Voice;
    app.voice.settings.endpoint = Some(crate::app::gateway_client_endpoint());
    let config = VoiceConfig {
        dictation_enabled: true,
        provider_id: Some("invalid provider".into()),
        ..Default::default()
    };
    assert!(config.validate().is_err());
    app.voice.settings.edit = Some(Edit::new(
        config,
        Preferences {
            engine: crate::app::voice::preferences::VoiceEngine::Local,
            ..Default::default()
        },
    ));
    let _task = update(&mut app, Event::Save);
    assert!(
        app.voice.settings.busy,
        "Local save must proceed independently of invalid cloud configuration"
    );
}
#[tokio::test]
async fn voice_task_13_voice_moon_permission_local_offline_load_save_and_cancel_leave_cloud_untouched()
 {
    use crate::app::voice::preferences::VoiceEngine;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("preferences.json");
    let before = Preferences {
        engine: VoiceEngine::Local,
        microphone: Some("fixture-microphone".into()),
        retain_recordings: true,
        ..Default::default()
    };
    before.write(&path).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let loaded = io::load_at(endpoint.clone(), path.clone()).await.unwrap();
    assert_eq!(loaded.local, before);
    assert!(!loaded.cloud_ready);
    let cloud = VoiceConfig { provider_id: Some("invalid provider".into()), ..Default::default() };
    let mut edit = Edit::new(cloud.clone(), loaded.local);
    edit.local.local_language = Some("zh-CN".into());
    edit.local.local_voice = Some("com.apple.voice.fixture".into());
    edit.config.dictionary = vec!["<invalid-cloud-dictionary>".into()];
    let _ = edit.clone().cancel();
    assert_eq!(Preferences::read(&path).unwrap(), before, "cancel must not persist");
    io::save(edit.clone(), endpoint, false, &path).await.unwrap();
    assert_eq!(Preferences::read(&path).unwrap(), edit.local);
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock,
        "Local must not even contact an unpaired gateway"
    );
    assert_eq!(edit.cancel().0, cloud, "Local edits must preserve original cloud snapshot");
}
#[tokio::test]
async fn voice_task_13_cloud_failure_keeps_local_choice_and_rejects_save() {
    use crate::app::voice::preferences::VoiceEngine;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("preferences.json");
    let local = Preferences { engine: VoiceEngine::Cloud, ..Default::default() };
    local.write(&path).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    drop(listener);
    let loaded = io::load_at(endpoint.clone(), path.clone()).await.unwrap();
    assert!(!loaded.cloud_ready);
    assert!(loaded.cloud_error.is_some());
    let mut edit = Edit::new(loaded.config, loaded.local);
    assert!(io::save(edit.clone(), endpoint.clone(), false, &path).await.is_err());
    assert_eq!(Preferences::read(&path).unwrap(), local);
    edit.local.engine = VoiceEngine::Local;
    io::save(edit.clone(), endpoint, false, &path).await.unwrap();
    assert_eq!(Preferences::read(&path).unwrap(), edit.local);
}
#[cfg(target_os = "macos")]
#[test]
fn voice_task_13_route_change_cancels_activity_without_persisting() {
    use crate::app::voice::preferences::VoiceEngine;
    let (mut app, _) = App::new();
    app.show_system_settings = true;
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Voice;
    app.voice.settings.endpoint = Some(crate::app::gateway_client_endpoint());
    app.voice.settings.edit = Some(Edit::new(VoiceConfig::default(), Preferences::default()));
    app.voice.draft.phase = crate::app::message::voice_draft::Phase::Transcribing;
    let old = app.voice.settings.generation;
    let _ = update(&mut app, Event::Engine(VoiceEngine::Local));
    assert_ne!(app.voice.draft.phase, crate::app::message::voice_draft::Phase::Transcribing);
    let _ = update(&mut app, Event::LocalLanguage(Some("zh-CN".into())));
    let _ = update(&mut app, Event::LocalVoice(Some("com.apple.voice.fixture".into())));
    let _ = update(&mut app, Event::Cancel);
    let _ = update(&mut app, Event::CloudLoaded(old, Ok(VoiceConfig::default())));
    assert!(app.voice.settings.edit.is_none(), "late cloud load must not reopen cancelled edit");
}
#[tokio::test]
async fn voice_task_13_cloud_explicit_save_patches_only_cloud_fields() {
    use std::io::{Read, Write};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("preferences.json");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let server = std::thread::spawn(move || {
        let (mut connection, _) = listener.accept().unwrap();
        connection.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut request = Vec::new();
        loop {
            let mut bytes = [0; 2048];
            let size = connection.read(&mut bytes).unwrap();
            assert_ne!(size, 0);
            request.extend_from_slice(&bytes[..size]);
            if let Some(end) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&request[..end]);
                let length: usize = header
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|length| length.trim().parse().unwrap())
                    })
                    .unwrap_or(0);
                if request.len() >= end + 4 + length {
                    break;
                }
            }
        }
        connection.write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n").unwrap();
        String::from_utf8(request).unwrap()
    });
    let mut edit = Edit::new(
        VoiceConfig::default(),
        Preferences { engine: VoiceEngine::Cloud, ..Default::default() },
    );
    edit.config.voice = VoiceName::Cedar;
    edit.local.local_language = Some("zh-CN".into());
    io::save(edit.clone(), endpoint, true, &path).await.unwrap();
    let request = server.join().unwrap();
    assert!(request.starts_with("PATCH /api/voice/config HTTP/1.1"));
    let body: serde_json::Value =
        serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body, serde_json::json!({"voice":"cedar"}));
    assert_eq!(Preferences::read(&path).unwrap(), edit.local);
}

#[test]
fn voice_moon_permission_late_result_does_not_reopen_form() {
    let (mut app, _) = App::new();
    app.show_system_settings = true;
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Voice;
    app.voice.settings.endpoint = Some(crate::app::gateway_client_endpoint());
    app.voice.settings.generation = 4;
    app.voice.settings.edit = Some(Edit::new(VoiceConfig::default(), Preferences::default()));
    let _ = update(&mut app, Event::Cancel);
    let _ = update(
        &mut app,
        Event::PermissionFinished(4, Ok(crate::app::voice::permissions::PermissionStatus::Granted)),
    );
    assert!(app.voice.settings.edit.is_none());
    assert!(app.voice.settings.permission_cancel.is_cancelled());
}
