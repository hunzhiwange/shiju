use super::permissions::*;
use anyhow::Result;
use std::cell::{Cell, RefCell};

struct OsBoundary {
    status: Cell<PermissionStatus>,
    requests: RefCell<Vec<Permission>>,
    settings: RefCell<Vec<Permission>>,
}
impl PermissionBackend for &OsBoundary {
    fn query(&self, _: Permission) -> PermissionStatus {
        self.status.get()
    }
    fn request(&self, permission: Permission) -> Result<()> {
        self.requests.borrow_mut().push(permission);
        Ok(())
    }
    fn open_settings(&self, permission: Permission) -> Result<()> {
        self.settings.borrow_mut().push(permission);
        Ok(())
    }
}
fn boundary(status: PermissionStatus) -> OsBoundary {
    OsBoundary {
        status: Cell::new(status),
        requests: RefCell::new(vec![]),
        settings: RefCell::new(vec![]),
    }
}
#[test]
fn voice_task_03_permission_denied_prevents_capture() {
    for status in [
        PermissionStatus::Unknown,
        PermissionStatus::NotDetermined,
        PermissionStatus::Denied,
        PermissionStatus::Restricted,
        PermissionStatus::Unsupported,
    ] {
        let os = boundary(status);
        let permissions = VoicePermissions::new(&os);
        for capability in [
            VoiceCapability::Recording,
            VoiceCapability::Transcription,
            VoiceCapability::GlobalKeyListening,
            VoiceCapability::TextInsertion,
            VoiceCapability::ScreenSharing,
        ] {
            let started = Cell::new(false);
            let result = permissions.start(capability, || {
                started.set(true);
                Ok(())
            });
            assert!(
                result.is_err(),
                "ungranted permission must prevent capability start: {status:?}"
            );
            assert!(!started.get());
        }
        assert!(os.requests.borrow().is_empty());
    }
}
#[test]
fn voice_task_03_permission_request_targets_single_capability() {
    for permission in [
        Permission::Microphone,
        Permission::SpeechRecognition,
        Permission::InputMonitoring,
        Permission::Accessibility,
        Permission::ScreenRecording,
    ] {
        let os = boundary(PermissionStatus::NotDetermined);
        let permissions = VoicePermissions::new(&os);
        assert_eq!(permissions.refresh(permission), PermissionStatus::NotDetermined);
        assert!(os.requests.borrow().is_empty());
        permissions.request_from_user_action(permission).unwrap();
        assert_eq!(
            *os.requests.borrow(),
            vec![permission],
            "only the selected permission is requested"
        );
        assert_eq!(permissions.refresh(permission), PermissionStatus::NotDetermined);
        os.status.set(PermissionStatus::Granted);
        assert_eq!(permissions.refresh(permission), PermissionStatus::Granted);
        permissions.start(VoiceCapability::Recording, || Ok(42)).unwrap();
        permissions.open_settings_from_user_action(permission).unwrap();
        assert_eq!(*os.settings.borrow(), vec![permission]);
    }
}

#[test]
fn voice_task_03_capabilities_require_only_their_own_permission() {
    assert_eq!(VoiceCapability::Recording.permission(), Permission::Microphone);
    assert_eq!(VoiceCapability::Transcription.permission(), Permission::SpeechRecognition);
    assert_eq!(VoiceCapability::GlobalKeyListening.permission(), Permission::InputMonitoring);
    assert_eq!(VoiceCapability::TextInsertion.permission(), Permission::Accessibility);
    assert_eq!(VoiceCapability::ScreenSharing.permission(), Permission::ScreenRecording);
    let os = boundary(PermissionStatus::Granted);
    let permissions = VoicePermissions::new(&os);
    assert_eq!(permissions.start(VoiceCapability::Recording, || Ok(42)).unwrap(), 42);
    os.status.set(PermissionStatus::Denied);
    assert!(permissions.start(VoiceCapability::Recording, || Ok(42)).is_err());
}

#[test]
fn voice_task_03_denied_guides_to_settings_without_requesting_again() {
    for status in [
        PermissionStatus::Denied,
        PermissionStatus::Restricted,
        PermissionStatus::Unsupported,
        PermissionStatus::Granted,
    ] {
        let os = boundary(status);
        let permissions = VoicePermissions::new(&os);
        let result = permissions.request_from_user_action(Permission::Microphone);
        assert!(os.requests.borrow().is_empty());
        if status == PermissionStatus::Denied {
            result.unwrap();
            assert_eq!(*os.settings.borrow(), vec![Permission::Microphone]);
        } else {
            assert!(os.settings.borrow().is_empty());
            assert_eq!(
                result.is_err(),
                matches!(status, PermissionStatus::Restricted | PermissionStatus::Unsupported)
            );
        }
    }
}

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

#[test]
fn local_voice_transcription_permission_does_not_block_capture() {
    struct SeparatePermissions;
    impl PermissionBackend for SeparatePermissions {
        fn query(&self, permission: Permission) -> PermissionStatus {
            match permission {
                Permission::Microphone => PermissionStatus::Granted,
                Permission::SpeechRecognition => PermissionStatus::Denied,
                _ => PermissionStatus::Unsupported,
            }
        }
        fn request(&self, _: Permission) -> Result<()> {
            panic!("starting capabilities must not request permission")
        }
        fn open_settings(&self, _: Permission) -> Result<()> {
            panic!("starting capabilities must not open settings")
        }
    }
    let permissions = VoicePermissions::new(SeparatePermissions);
    assert_eq!(permissions.start(VoiceCapability::Recording, || Ok(42)).unwrap(), 42);
    assert!(permissions.start(VoiceCapability::Transcription, || Ok(42)).is_err());
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

#[test]
fn voice_moon_permission_delayed_grant_waits_for_system() {
    struct Delayed(Cell<usize>);
    impl PermissionBackend for Delayed {
        fn query(&self, _: Permission) -> PermissionStatus {
            let n = self.0.get();
            self.0.set(n + 1);
            if n >= 3 { PermissionStatus::Granted } else { PermissionStatus::NotDetermined }
        }
        fn request(&self, _: Permission) -> Result<()> {
            Ok(())
        }
        fn open_settings(&self, _: Permission) -> Result<()> {
            panic!("unexpected settings")
        }
    }
    assert_eq!(
        VoicePermissions::new(Delayed(Cell::new(0)))
            .wait_from_user_action(
                Permission::Microphone,
                std::time::Duration::from_secs(1),
                || false
            )
            .unwrap(),
        PermissionStatus::Granted
    );
}

#[test]
fn voice_moon_permission_terminal_cancel_and_timeout() {
    use std::time::Duration;
    for status in [
        PermissionStatus::Granted,
        PermissionStatus::Denied,
        PermissionStatus::Restricted,
        PermissionStatus::Unsupported,
    ] {
        let os = boundary(status);
        assert_eq!(
            VoicePermissions::new(&os)
                .wait_from_user_action(Permission::Microphone, Duration::ZERO, || false)
                .unwrap(),
            status
        );
        assert!(os.requests.borrow().is_empty());
    }
    let os = boundary(PermissionStatus::NotDetermined);
    let p = VoicePermissions::new(&os);
    assert!(p.wait_from_user_action(Permission::Microphone, Duration::ZERO, || true).is_err());
    assert!(os.requests.borrow().is_empty());
    assert!(p.wait_from_user_action(Permission::Microphone, Duration::ZERO, || false).is_err());
}

#[test]
fn voice_moon_permission_cancel_discards_late_grant() {
    let os = boundary(PermissionStatus::NotDetermined);
    let checks = Cell::new(0);
    let result = VoicePermissions::new(&os).wait_from_user_action(
        Permission::Microphone,
        std::time::Duration::from_secs(1),
        || {
            let n = checks.get();
            checks.set(n + 1);
            if n > 0 {
                os.status.set(PermissionStatus::Granted);
            }
            n > 0
        },
    );
    assert!(result.is_err());
    assert_eq!(*os.requests.borrow(), vec![Permission::Microphone]);
}
