use super::macos_input::{InputEvent, InputPlatform};
use super::*;
use crate::computer_control::policy::GrantScope;
use vw_config_types::computer_control::{ComputerControlConfig, GrantDecision};

#[derive(Default)]
struct State {
    events: Vec<InputEvent>,
    focus: u32,
    exited: bool,
    restarted: bool,
    moved: bool,
    missing_ax: bool,
    cancel_after: Option<CancellationToken>,
    focus_after: bool,
    revoke_after: Option<Arc<RwLock<ComputerControlPolicy>>>,
}
#[derive(Clone)]
struct Os(Arc<Mutex<State>>);
fn app() -> ApplicationInstance {
    ApplicationInstance {
        bundle_id: "com.example.InputFixture".into(),
        process_id: 100,
        start_seconds: 10,
        start_microseconds: 20,
    }
}
fn window() -> WindowGeometry {
    WindowGeometry { window_id: 4, x: -400.0, y: 20.0, width: 800.0, height: 600.0 }
}
#[async_trait::async_trait]
impl ObservationPlatform for Os {
    fn request_permission(&self, _: NativePermission) -> Result<(), NativeObservationError> {
        unreachable!()
    }
    fn permissions(&self) -> NativePermissions {
        NativePermissions { accessibility: !self.0.lock().missing_ax, screen_recording: true }
    }
    fn identity(&self, _: u32) -> Result<ApplicationInstance, NativeObservationError> {
        let s = self.0.lock();
        if s.exited {
            return Err(ControlErrorCode::TargetChanged.into());
        }
        let mut a = app();
        if s.restarted {
            a.start_seconds += 1;
        }
        Ok(a)
    }
    fn applications(&self) -> Result<Vec<ApplicationInstance>, NativeObservationError> {
        Ok(vec![app()])
    }
    fn read(&self, _: &ApplicationInstance) -> Result<PlatformObservation, NativeObservationError> {
        let mut w = window();
        if self.0.lock().moved {
            w.x += 1.0;
        }
        Ok(PlatformObservation { tree: serde_json::json!({}), window: Some(w) })
    }
    async fn capture(
        &self,
        _: &ApplicationInstance,
        _: WindowGeometry,
    ) -> Result<Vec<u8>, NativeObservationError> {
        unreachable!()
    }
}
impl InputPlatform for Os {
    fn focused_pid(&self) -> Result<u32, NativeObservationError> {
        Ok(self.0.lock().focus)
    }
    fn activate(&self, _: &ApplicationInstance) -> Result<(), NativeObservationError> {
        self.0.lock().focus = 100;
        Ok(())
    }
    fn emit(
        &self,
        _: &ApplicationInstance,
        event: &InputEvent,
    ) -> Result<(), NativeObservationError> {
        let mut s = self.0.lock();
        s.events.push(event.clone());
        if let Some(t) = &s.cancel_after {
            t.cancel();
        }
        if let Some(policy) = &s.revoke_after {
            policy
                .write()
                .set_grant(
                    &local_user(),
                    GrantScope::Application(app().bundle_id),
                    Some(GrantDecision::Deny),
                )
                .unwrap();
        }
        if s.focus_after {
            s.focus = 200;
        }
        Ok(())
    }
}
fn fixture() -> (NativeObserver, Os) {
    let os = Os(Arc::new(Mutex::new(State { focus: 100, ..Default::default() })));
    let mut p = ComputerControlPolicy::new(ComputerControlConfig::default());
    p.set_grant(
        &local_user(),
        GrantScope::Application(app().bundle_id),
        Some(GrantDecision::Allow),
    )
    .unwrap();
    let version = p.version();
    let o = NativeObserver::with_platform(
        Arc::new(RwLock::new(p)),
        Arc::new(AtomicBool::new(false)),
        ControlSessionId("input-session".into()),
        version,
        app().target(),
        CancellationToken::new(),
        Box::new(os.clone()),
    )
    .unwrap();
    (o, os)
}
fn text() -> NativeInput {
    NativeInput::TypeText { text: "中文😀".into() }
}
fn err(code: ControlErrorCode) -> NativeObservationError {
    code.into()
}
#[tokio::test]
async fn cc_control_06_changed_focus_rejects_without_input() {
    let (o, os) = fixture();
    os.0.lock().focus = 200;
    assert_eq!(o.execute_input_with(&text(), &os).await, Err(err(ControlErrorCode::TargetChanged)));
    assert!(os.0.lock().events.is_empty());
}
#[tokio::test]
async fn cc_control_06_unicode_uses_complete_utf16_scalars() {
    let (o, os) = fixture();
    o.execute_input_with(&text(), &os).await.unwrap();
    assert_eq!(
        os.0.lock().events,
        vec![
            InputEvent::Unicode(vec![0x4e2d]),
            InputEvent::Unicode(vec![0x6587]),
            InputEvent::Unicode(vec![0xd83d, 0xde00])
        ]
    );
}
#[tokio::test]
async fn cc_control_06_exit_and_recycled_pid_reject_without_input() {
    for restarted in [false, true] {
        let (o, os) = fixture();
        {
            let mut s = os.0.lock();
            s.restarted = restarted;
            s.exited = !restarted;
        }
        assert_eq!(
            o.execute_input_with(&text(), &os).await,
            Err(err(ControlErrorCode::TargetChanged))
        );
        assert!(os.0.lock().events.is_empty());
    }
}
#[tokio::test]
async fn cc_control_06_cancel_and_focus_change_stop_remaining_text() {
    for cancel in [true, false] {
        let (o, os) = fixture();
        if cancel {
            os.0.lock().cancel_after = Some(o.cancelled.clone());
        } else {
            os.0.lock().focus_after = true;
        }
        assert_eq!(
            o.execute_input_with(&text(), &os).await,
            Err(err(if cancel {
                ControlErrorCode::Cancelled
            } else {
                ControlErrorCode::TargetChanged
            }))
        );
        assert_eq!(os.0.lock().events.len(), 1);
        assert!(o.execute_input_with(&text(), &os).await.is_err());
        assert_eq!(os.0.lock().events.len(), 1);
    }
}
#[tokio::test]
async fn cc_control_06_revocation_permission_and_lock_reject_without_input() {
    let (o, os) = fixture();
    os.0.lock().missing_ax = true;
    assert_eq!(
        o.execute_input_with(&text(), &os).await,
        Err(NativeObservationError::AccessibilityRequired)
    );
    os.0.lock().missing_ax = false;
    o.locked.store(true, Ordering::SeqCst);
    assert_eq!(o.execute_input_with(&text(), &os).await, Err(err(ControlErrorCode::Unsupported)));
    o.locked.store(false, Ordering::SeqCst);
    o.policy
        .write()
        .set_grant(
            &local_user(),
            GrantScope::Application(app().bundle_id),
            Some(GrantDecision::Deny),
        )
        .unwrap();
    assert_eq!(
        o.execute_input_with(&text(), &os).await,
        Err(err(ControlErrorCode::StalePermission))
    );
    assert!(os.0.lock().events.is_empty());
}
#[tokio::test]
async fn cc_control_06_click_requires_current_screenshot_and_valid_coordinates() {
    let (o, os) = fixture();
    let click = NativeInput::Click { x: 800, y: 600, button: MouseButton::Left };
    assert!(o.execute_input_with(&click, &os).await.is_err());
    *o.media.lock() = Some(StoredScreenshot {
        reference: ScreenshotReference::new(
            "fixture".into(),
            ScreenshotMimeType::Png,
            1,
            1600,
            1200,
        )
        .unwrap(),
        png: vec![1],
        window: window(),
        geometry: ScreenshotGeometry::new(window(), 1600, 1200).unwrap(),
    });
    for (x, y) in [(-1, 0), (1600, 0), (0, 1200)] {
        assert_eq!(
            o.execute_input_with(&NativeInput::Click { x, y, button: MouseButton::Left }, &os)
                .await,
            Err(err(ControlErrorCode::InvalidRequest))
        );
    }
    o.execute_input_with(&click, &os).await.unwrap();
    assert_eq!(
        os.0.lock().events,
        vec![InputEvent::Click { window: window(), x: 0.0, y: 320.0, button: MouseButton::Left }]
    );
    os.0.lock().moved = true;
    assert_eq!(o.execute_input_with(&click, &os).await, Err(err(ControlErrorCode::TargetChanged)));
    assert_eq!(os.0.lock().events.len(), 1);
}
#[tokio::test]
async fn cc_control_06_activate_scroll_and_known_keys_use_typed_events() {
    let (o, os) = fixture();
    os.0.lock().focus = 200;
    o.execute_input_with(&NativeInput::Activate, &os).await.unwrap();
    o.execute_input_with(&NativeInput::Scroll { delta_x: 0, delta_y: -3 }, &os).await.unwrap();
    o.execute_input_with(
        &NativeInput::Press { key: "A".into(), modifiers: vec![KeyModifier::Meta] },
        &os,
    )
    .await
    .unwrap();
    assert_eq!(
        os.0.lock().events,
        vec![
            InputEvent::Scroll { window: window(), x: 0.0, y: 320.0, delta_x: 0, delta_y: -3 },
            InputEvent::Key { code: 0, flags: 1 << 20 },
        ]
    );
    for key in ["arbitrary script", "中文"] {
        assert_eq!(
            o.execute_input_with(&NativeInput::Press { key: key.into(), modifiers: vec![] }, &os)
                .await,
            Err(err(ControlErrorCode::InvalidRequest))
        );
    }
    assert_eq!(os.0.lock().events.len(), 2);
}

#[tokio::test]
async fn cc_control_06_revocation_during_input_stops_remaining_scalars() {
    let (o, os) = fixture();
    os.0.lock().revoke_after = Some(o.policy.clone());
    assert_eq!(
        o.execute_input_with(&text(), &os).await,
        Err(err(ControlErrorCode::StalePermission))
    );
    assert_eq!(os.0.lock().events.len(), 1);
}
#[tokio::test]
async fn cc_control_06_cancelled_waiter_cannot_input_after_gate_releases() {
    let (o, os) = fixture();
    let gate = o.observe_gate.lock().await;
    let input = text();
    let pending = o.execute_input_with(&input, &os);
    tokio::pin!(pending);
    tokio::select! { biased; _ = &mut pending => panic!("gate should block input"), _ = tokio::task::yield_now() => {} }
    o.cancelled.cancel();
    drop(gate);
    assert_eq!(pending.await, Err(err(ControlErrorCode::Cancelled)));
    assert!(os.0.lock().events.is_empty());
}

#[tokio::test]
async fn cc_control_06_invalid_payloads_are_rejected_without_input() {
    let (o, os) = fixture();
    for input in [
        NativeInput::TypeText { text: String::new() },
        NativeInput::TypeText { text: "\0".into() },
        NativeInput::TypeText { text: "x".repeat(16 * 1024 + 1) },
        NativeInput::Scroll { delta_x: i32::MIN, delta_y: 0 },
        NativeInput::Scroll { delta_x: 0, delta_y: 0 },
        NativeInput::Press {
            key: "A".into(),
            modifiers: vec![KeyModifier::Meta, KeyModifier::Meta],
        },
    ] {
        assert_eq!(
            o.execute_input_with(&input, &os).await,
            Err(err(ControlErrorCode::InvalidRequest))
        );
    }
    assert!(os.0.lock().events.is_empty());
}
