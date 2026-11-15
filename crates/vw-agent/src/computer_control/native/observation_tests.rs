use super::*;
use crate::computer_control::policy::{ControlFeature, GrantScope};
use std::sync::atomic::{AtomicUsize, Ordering};
use vw_config_types::computer_control::{ComputerControlConfig, GrantDecision};

#[derive(Default)]
struct FakePlatform {
    calls: Arc<AtomicUsize>,
    missing_ax: bool,
    missing_screen: bool,
    changed: Arc<AtomicBool>,
    restarted: Arc<AtomicBool>,
    wrong_bundle: bool,
    cancel_on_read: Option<CancellationToken>,
}

#[async_trait::async_trait]
impl ObservationPlatform for FakePlatform {
    fn request_permission(&self, _: NativePermission) -> Result<(), NativeObservationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn permissions(&self) -> NativePermissions {
        NativePermissions {
            accessibility: !self.missing_ax,
            screen_recording: !self.missing_screen,
        }
    }
    fn applications(&self) -> Result<Vec<ApplicationInstance>, NativeObservationError> {
        Ok(vec![
            instance("com.example.ControlFixture", 100),
            instance("com.example.Other", 200),
            instance("com.google.Chrome", 300),
            instance("com.microsoft.Excel", 400),
            instance("com.microsoft.Word", 500),
        ])
    }
    fn identity(&self, pid: u32) -> Result<ApplicationInstance, NativeObservationError> {
        if self.changed.load(Ordering::SeqCst) {
            return Err(NativeObservationError::Control(ControlErrorCode::TargetChanged));
        }
        let mut app = instance(
            if self.wrong_bundle { "com.example.Other" } else { "com.example.ControlFixture" },
            pid,
        );
        if self.restarted.load(Ordering::SeqCst) {
            app.start_seconds += 1;
        }
        Ok(app)
    }
    fn read(&self, _: &ApplicationInstance) -> Result<PlatformObservation, NativeObservationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(token) = &self.cancel_on_read {
            token.cancel();
        }
        Ok(PlatformObservation {
            tree: serde_json::json!({"role":"AXWindow","title":"controlled fixture"}),
            window: Some(WindowGeometry {
                window_id: 4,
                x: -400.0,
                y: 20.0,
                width: 800.0,
                height: 600.0,
            }),
        })
    }
    async fn capture(
        &self,
        _: &ApplicationInstance,
        _: WindowGeometry,
    ) -> Result<Vec<u8>, NativeObservationError> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(1600, 1200)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        Ok(bytes.into_inner())
    }
}
fn instance(bundle_id: &str, process_id: u32) -> ApplicationInstance {
    ApplicationInstance {
        bundle_id: bundle_id.into(),
        process_id,
        start_seconds: 10,
        start_microseconds: 20,
    }
}
fn policy() -> Arc<RwLock<ComputerControlPolicy>> {
    let mut policy = ComputerControlPolicy::new(ComputerControlConfig::default());
    policy
        .set_grant(
            &local_user(),
            GrantScope::Application("com.example.ControlFixture".into()),
            Some(GrantDecision::Allow),
        )
        .unwrap();
    Arc::new(RwLock::new(policy))
}
fn observer(platform: FakePlatform) -> NativeObserver {
    let p = policy();
    let version = p.read().version();
    NativeObserver::with_platform(
        p,
        Arc::new(AtomicBool::new(false)),
        ControlSessionId("fixture-session".into()),
        version,
        instance("com.example.ControlFixture", 100).target(),
        CancellationToken::new(),
        Box::new(platform),
    )
    .unwrap()
}
#[test]
fn cc_control_05_enumeration_filters_real_policy_and_specialized_apps() {
    let p = policy();
    let allowed = enumerate_with(&p, &FakePlatform::default()).unwrap();
    assert_eq!(allowed, vec![instance("com.example.ControlFixture", 100)]);
    p.write().set_enabled(&local_user(), ControlFeature::NativeApps, true).unwrap();
    assert_eq!(enumerate_with(&p, &FakePlatform::default()).unwrap().len(), 2);
}
#[tokio::test]
async fn cc_control_05_missing_permissions_are_specific_without_reading() {
    let o = observer(FakePlatform { missing_ax: true, ..Default::default() });
    assert_eq!(o.observe(false).await.unwrap_err(), NativeObservationError::AccessibilityRequired);
    let o = observer(FakePlatform { missing_screen: true, ..Default::default() });
    assert!(o.observe(false).await.is_ok());
    assert_eq!(o.observe(true).await.unwrap_err(), NativeObservationError::ScreenRecordingRequired);
}
#[test]
fn cc_control_05_identity_mismatch_is_rejected_before_reading() {
    let p = policy();
    let v = p.read().version();
    let result = NativeObserver::with_platform(
        p,
        Arc::new(AtomicBool::new(false)),
        ControlSessionId("s".into()),
        v,
        ComputerControlTarget::NativeApp {
            bundle_id: "com.example.ControlFixture".into(),
            process_id: 100,
        },
        CancellationToken::new(),
        Box::new(FakePlatform { wrong_bundle: true, ..Default::default() }),
    );
    assert!(matches!(
        result,
        Err(NativeObservationError::Control(ControlErrorCode::TargetChanged))
    ));
}
#[tokio::test]
async fn cc_control_05_revocation_and_lock_reject_subsequent_observation() {
    let o = observer(FakePlatform::default());
    assert!(o.observe(false).await.is_ok());
    o.locked.store(true, Ordering::SeqCst);
    assert_eq!(
        o.observe(false).await.unwrap_err(),
        NativeObservationError::Control(ControlErrorCode::Unsupported)
    );
    o.locked.store(false, Ordering::SeqCst);
    o.policy
        .write()
        .set_grant(
            &local_user(),
            GrantScope::Application("com.example.ControlFixture".into()),
            Some(GrantDecision::Deny),
        )
        .unwrap();
    assert_eq!(
        o.observe(false).await.unwrap_err(),
        NativeObservationError::Control(ControlErrorCode::StalePermission)
    );
}
#[tokio::test]
async fn cc_control_05_cancel_during_read_does_not_deliver_content() {
    let token = CancellationToken::new();
    let mut o =
        observer(FakePlatform { cancel_on_read: Some(token.clone()), ..Default::default() });
    o.cancelled = token;
    assert_eq!(
        o.observe(false).await.unwrap_err(),
        NativeObservationError::Control(ControlErrorCode::Cancelled)
    );
}
#[tokio::test]
async fn cc_control_05_screenshot_coordinates_and_session_media_are_bounded() {
    let o = observer(FakePlatform::default());
    let observed = o.observe(true).await.unwrap();
    let ControlOutput::Observation { text, screenshots } = observed else { panic!("wrong output") };
    let metadata: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(metadata["screenshot_geometry"]["width"], 800.0);
    assert_eq!(metadata["screenshot_geometry"]["pixel_width"], 1600);
    assert_eq!(screenshots[0].dimensions(), (1600, 1200));
    assert!(o.resolve_screenshot(&screenshots[0]).unwrap().starts_with("data:image/png;base64,"));
    let other = observer(FakePlatform::default());
    assert!(other.resolve_screenshot(&screenshots[0]).is_err());
    o.observe(false).await.unwrap();
    assert!(o.resolve_screenshot(&screenshots[0]).is_err());
    let g = ScreenshotGeometry::new(
        WindowGeometry { window_id: 4, x: -400.0, y: 20.0, width: 800.0, height: 600.0 },
        1600,
        1200,
    )
    .unwrap();
    assert_eq!(g.screen_point(800, 600).unwrap(), (0.0, 320.0));
    assert!(g.screen_point(1600, 0).is_err());
}
#[test]
fn cc_control_05_invalid_geometry_and_oversized_images_fail_closed() {
    let w = WindowGeometry { window_id: 4, x: 0.0, y: 0.0, width: f64::NAN, height: 10.0 };
    assert!(ScreenshotGeometry::new(w, 10, 10).is_err());
    assert!(bounded_png(&vec![0; 16 * 1024 * 1024 + 1]).is_err());
    assert!(bounded_png(b"not an image").is_err());
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(4096, 256).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    let (png, width, height) = bounded_png(&bytes.into_inner()).unwrap();
    assert_eq!((width, height), (2048, 128));
    assert_eq!(
        (
            image::load_from_memory(&png).unwrap().width(),
            image::load_from_memory(&png).unwrap().height()
        ),
        (2048, 128)
    );
}

#[tokio::test]
async fn cc_control_05_exited_or_restarted_process_is_rejected() {
    let changed = Arc::new(AtomicBool::new(false));
    let restarted = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let o = observer(FakePlatform {
        changed: changed.clone(),
        restarted: restarted.clone(),
        calls: calls.clone(),
        ..Default::default()
    });
    assert!(o.observe(false).await.is_ok());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    restarted.store(true, Ordering::SeqCst);
    assert_eq!(
        o.observe(false).await.unwrap_err(),
        NativeObservationError::Control(ControlErrorCode::TargetChanged)
    );
    restarted.store(false, Ordering::SeqCst);
    changed.store(true, Ordering::SeqCst);
    assert_eq!(
        o.observe(false).await.unwrap_err(),
        NativeObservationError::Control(ControlErrorCode::TargetChanged)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
#[test]
fn cc_control_05_unauthorized_target_is_rejected_before_platform_read() {
    let p = Arc::new(RwLock::new(ComputerControlPolicy::new(ComputerControlConfig::default())));
    let calls = Arc::new(AtomicUsize::new(0));
    let result = NativeObserver::with_platform(
        p,
        Arc::new(AtomicBool::new(false)),
        ControlSessionId("s".into()),
        PermissionVersion(0),
        instance("com.example.ControlFixture", 100).target(),
        CancellationToken::new(),
        Box::new(FakePlatform { calls: calls.clone(), ..Default::default() }),
    );
    assert!(matches!(
        result,
        Err(NativeObservationError::Control(ControlErrorCode::PermissionDenied))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn cc_control_05_explicit_permission_request_does_not_infer_grant_or_prompt_on_probe() {
    let platform = FakePlatform { missing_ax: true, missing_screen: true, ..Default::default() };
    assert!(!platform.permissions().accessibility);
    assert_eq!(platform.calls.load(Ordering::SeqCst), 0);
    let result = request_permission_with(&platform, NativePermission::Accessibility).unwrap();
    assert!(!result.accessibility);
    assert_eq!(platform.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        NativePermission::Accessibility.settings_url(),
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
    );
    assert!(NativePermission::ScreenRecording.settings_url().ends_with("Privacy_ScreenCapture"));
    let granted = FakePlatform::default();
    assert!(
        request_permission_with(&granted, NativePermission::Accessibility).unwrap().accessibility
    );
    assert_eq!(granted.calls.load(Ordering::SeqCst), 0);
}

#[cfg(target_os = "macos")]
#[test]
fn cc_control_05_empty_ax_children_skip_invalid_range_copy() {
    use super::macos_observation::array_read_limit;
    assert_eq!(array_read_limit(0, 0).unwrap(), None);
    assert_eq!(array_read_limit(0, 500).unwrap(), Some(256));
    assert_eq!(array_read_limit(-25205, 0).unwrap(), None);
    assert_eq!(
        array_read_limit(-25211, 0).unwrap_err(),
        NativeObservationError::AccessibilityRequired
    );
    assert_eq!(
        array_read_limit(0, -1).unwrap_err(),
        NativeObservationError::Control(ControlErrorCode::BackendFailure)
    );
    assert_eq!(
        array_read_limit(-25200, 0).unwrap_err(),
        NativeObservationError::Control(ControlErrorCode::BackendFailure)
    );
}

#[cfg(target_os = "macos")]
#[test]
fn cc_control_05_optional_children_failure_is_explicit_without_masking_denials() {
    use super::macos_observation::children_read_failure;
    assert!(
        children_read_failure(NativeObservationError::Control(ControlErrorCode::BackendFailure))
            .unwrap()
    );
    for error in [
        NativeObservationError::AccessibilityRequired,
        NativeObservationError::Control(ControlErrorCode::TargetChanged),
        NativeObservationError::Control(ControlErrorCode::PermissionDenied),
        NativeObservationError::Control(ControlErrorCode::Cancelled),
    ] {
        assert_eq!(children_read_failure(error.clone()).unwrap_err(), error);
    }
}
