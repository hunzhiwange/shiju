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
