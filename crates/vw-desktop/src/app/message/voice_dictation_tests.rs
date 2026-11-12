use super::*;
use crate::app::voice::preferences::{Preferences, VoiceEngine};
#[test]
fn voice_task_07_local_default_and_legacy_preferences_use_local_on_macos() {
    let preferences: Preferences = serde_json::from_str("{}").unwrap();
    assert_eq!(
        preferences.engine,
        if cfg!(target_os = "macos") { VoiceEngine::Local } else { VoiceEngine::Cloud }
    );
    assert!(preferences.local_language.is_none());
}
#[tokio::test]
async fn voice_task_07_local_route_does_not_require_gateway_or_cloud_provider() {
    let preferences = Preferences {
        engine: VoiceEngine::Local,
        local_language: Some("zh-CN".into()),
        ..Default::default()
    };
    let result =
        select_route(&preferences, GatewayEndpoint::new("unreachable.invalid", 9), None, 1).await;
    assert!(
        matches!(result, Ok(Route::Local { language: Some(ref language) }) if language=="zh-CN")
    );
}

fn capability(permission: PermissionStatus) -> Capabilities {
    Capabilities {
        language: "zh-CN".into(),
        supports_on_device: true,
        available: true,
        permission,
        voices: vec![],
    }
}
#[test]
fn voice_task_07_local_permission_gate_never_opens_capture_without_speech() {
    let opened = std::cell::Cell::new(false);
    for status in
        [PermissionStatus::NotDetermined, PermissionStatus::Denied, PermissionStatus::Restricted]
    {
        assert!(
            gated_capture(&capability(status), || {
                opened.set(true);
                Ok(())
            })
            .is_err()
        );
        assert!(!opened.get());
    }
    gated_capture(&capability(PermissionStatus::Granted), || {
        opened.set(true);
        Ok(())
    })
    .unwrap();
    assert!(opened.get());
    let mut offline = capability(PermissionStatus::Granted);
    offline.supports_on_device = false;
    assert!(gated_capture(&offline, || Ok(())).is_err());
}
#[tokio::test]
async fn voice_task_07_local_native_cancel_discards_late_success() {
    let token = Cancellation::default();
    let signal = token.clone();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(recognize_local_with(
        vec![123; 480],
        Some("zh-CN".into()),
        token,
        move |pcm, language, cancel| {
            assert_eq!(pcm, &[123; 480]);
            assert_eq!(language, Some("zh-CN"));
            let _ = entered_tx.send(());
            while !cancel.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(local::Transcript { text: "late native result".into(), language: "zh-CN".into() })
        },
    ));
    entered_rx.await.unwrap();
    signal.cancel();
    assert!(task.await.unwrap().unwrap_err().contains("取消"));
}
#[tokio::test]
async fn voice_task_07_local_timeout_drop_cancels_native_worker() {
    let token = Cancellation::default();
    let observed = token.clone();
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let mut work =
        Box::pin(recognize_local_with(vec![123; 480], None, token, move |_, _, cancel| {
            let _ = entered_tx.send(());
            while !cancel.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
            let _ = done_tx.send(());
            Err(LocalSpeechError::Cancelled)
        }));
    tokio::select! { _=&mut work=>panic!("native task ended before timeout"), _=entered_rx=>{} }
    assert!(tokio::time::timeout(Duration::from_millis(40), work).await.is_err());
    assert!(observed.is_cancelled());
    tokio::time::timeout(Duration::from_secs(1), done_rx).await.unwrap().unwrap();
}
#[tokio::test]
async fn voice_task_07_local_archive_retry_passes_pcm_without_gateway() {
    let root = tempfile::tempdir().unwrap();
    let store = crate::app::voice::recordings::Store::open(&root.path().join("archive")).unwrap();
    let wav = crate::app::voice::capture::Recording { generation: 1, pcm: vec![1234; 480] }
        .wav()
        .unwrap();
    store.save(true, &wav).unwrap();
    let saved = store.read(&store.list().unwrap()[0].id).unwrap();
    let preferences = Preferences {
        engine: VoiceEngine::Local,
        local_language: Some("zh-CN".into()),
        ..Default::default()
    };
    let result = retry_with_local(
        preferences,
        GatewayEndpoint::new("not-a-gateway.invalid", 9),
        None,
        1,
        saved,
        Cancellation::default(),
        |pcm, language, _| async move {
            assert_eq!(pcm, vec![1234; 480]);
            assert_eq!(language.as_deref(), Some("zh-CN"));
            Ok("本机重识别".into())
        },
    )
    .await
    .unwrap();
    assert_eq!(result, "本机重识别");
    assert_eq!(store.list().unwrap().len(), 1);
}
#[tokio::test]
async fn voice_task_07_local_archive_over_limit_never_calls_native_or_uploads() {
    let wav = crate::app::voice::capture::Recording { generation: 1, pcm: vec![1234; 24_000 * 61] }
        .wav()
        .unwrap();
    let preferences = Preferences { engine: VoiceEngine::Local, ..Default::default() };
    let result = retry_with_local(
        preferences,
        GatewayEndpoint::new("not-a-gateway.invalid", 9),
        None,
        1,
        wav,
        Cancellation::default(),
        |_, _, _| async { panic!("over-limit audio reached native recognition") },
    )
    .await;
    assert!(result.unwrap_err().contains("60"));
}
#[test]
fn voice_task_07_local_limits_and_explicit_cloud_are_distinct() {
    assert_eq!(Route::Local { language: None }.capture_limit(), Duration::from_secs(60));
    let p: Preferences =
        serde_json::from_str(r#"{"engine":"cloud","local_language":null}"#).unwrap();
    assert_eq!(p.engine, VoiceEngine::Cloud);
    assert!(serde_json::from_str::<Preferences>(r#"{"engine":"unknown"}"#).is_err());
}
#[test]
fn voice_task_07_local_context_change_cancels_native_and_rejects_completion() {
    use crate::app::App;
    use crate::app::message::voice::{self, Event};
    use crate::app::message::voice_draft::Phase;
    let mut app = App::new().0;
    app.active_session_id = Some("local-original".into());
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("原草稿");
    let scope = voice::scope(&app).unwrap();
    let generation = app.voice.draft.start(scope, "原草稿".into()).unwrap();
    app.voice.draft.phase = Phase::Transcribing;
    let token = Cancellation::default();
    app.voice.native_cancel = Some(token.clone());
    app.active_session_id = Some("other".into());
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("其它草稿");
    voice::invalidate(&mut app);
    assert!(token.is_cancelled());
    let _task = voice::update(&mut app, Event::Completed(generation, Ok("迟到识别".into())));
    assert_eq!(app.current_session_runtime_ref().unwrap().input_editor.text(), "其它草稿");
    assert!(!app.current_session_runtime_ref().unwrap().is_requesting);
}

#[tokio::test]
async fn voice_task_07_local_native_success_does_not_cancel_completed_token() {
    let token = Cancellation::default();
    let observed = token.clone();
    let text = recognize_local_with(vec![123; 480], None, token, |_, _, _| {
        Ok(local::Transcript { text: "本机结果".into(), language: "zh-CN".into() })
    })
    .await
    .unwrap();
    assert_eq!(text, "本机结果");
    assert!(!observed.is_cancelled());
}

#[tokio::test]
async fn voice_task_07_local_cancelled_prepare_never_requests_permissions() {
    let token = Cancellation::default();
    token.cancel();
    let (_tx, rx) = tokio::sync::watch::channel(false);
    let preferences = Preferences { engine: VoiceEngine::Local, ..Default::default() };
    let result =
        prepare(preferences, GatewayEndpoint::new("not-a-gateway.invalid", 9), None, 1, rx, token)
            .await;
    assert!(result.err().unwrap().contains("取消"));
}
