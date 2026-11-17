use super::*;

#[test]
fn cc_runtime_05_rejects_model_authority() {
    let request = json!({"target":{"kind":"native_app","bundle_id":"com.example.fixture","process_id":42},"action":{"action":"stop"}});
    assert!(parse(request.clone()).is_ok());
    for key in ["credential", "permission_version", "session_id"] {
        let mut forged = request.clone();
        forged[key] = json!("foreign");
        assert!(parse(forged).is_err(), "accepted {key}");
    }
}

#[tokio::test]
async fn cc_runtime_05_cancelled_service_cannot_launch_or_resolve() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    let backend = service_backend::ServiceBackend::new(
        Default::default(),
        ComputerControlTarget::NativeApp {
            bundle_id: "com.example.fixture".into(),
            process_id: 42,
        },
        cancel,
    );
    assert_eq!(backend.observe(false).await.unwrap_err().code(), ControlErrorCode::Cancelled);
    let reference =
        ScreenshotReference::new("foreign".into(), ScreenshotMimeType::Png, 68, 1, 1).unwrap();
    assert_eq!(backend.resolve(&reference).unwrap_err().code(), ControlErrorCode::Cancelled);
}
