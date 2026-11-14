//! Real-platform probes, separate from pure state tests.
use super::*;

/// Read-only real-platform probe: never requests permission or opens settings.
#[test]
fn voice_task_03_system_queries_do_not_request_permissions() {
    let permissions = VoicePermissions::new(SystemPermissions);
    for permission in [
        Permission::Microphone,
        Permission::SpeechRecognition,
        Permission::InputMonitoring,
        Permission::Accessibility,
        Permission::ScreenRecording,
    ] {
        let status = permissions.refresh(permission);
        println!("system permission {permission:?}: {status:?}");
        #[cfg(target_os = "macos")]
        assert_ne!(status, PermissionStatus::Unsupported);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(status, PermissionStatus::Unsupported);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn local_voice_missing_usage_description_fails_before_tcc() {
    assert!(!super::permissions_macos::has_usage_description("ShiJuMissingPurposeFixture"));
    if !super::permissions_macos::has_usage_description("NSSpeechRecognitionUsageDescription") {
        // This branch is safe only after observing the missing purpose: it must
        // return an error before invoking Apple's permission API, never prompt.
        assert!(SystemPermissions.request(Permission::SpeechRecognition).is_err());
    }
}
