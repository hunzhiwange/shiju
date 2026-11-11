//! macOS 权限调用隔离；查询不弹窗，申请只由显式用户动作调用。
use anyhow::{Context, Result, bail};
use block2::RcBlock;
use objc2::{
    msg_send,
    runtime::{AnyClass, AnyObject, Bool},
};
use objc2_foundation::NSString;

use super::permissions::{Permission, PermissionStatus};

#[link(name = "AVFoundation", kind = "framework")]
unsafe extern "C" {
    static AVMediaTypeAudio: *const NSString;
}
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOHIDCheckAccess(request_type: u32) -> u32;
    fn IOHIDRequestAccess(request_type: u32) -> bool;
}
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

pub(super) fn query(permission: Permission) -> PermissionStatus {
    // SAFETY: Frameworks are linked above; these zero-pointer queries do not
    // prompt. Signatures/constants follow the macOS SDK headers. NSString audio
    // is a process-lifetime AVFoundation constant; the class is checked first.
    unsafe {
        match permission {
            Permission::Microphone => {
                let Some(class) = AnyClass::get(c"AVCaptureDevice") else {
                    return PermissionStatus::Unsupported;
                };
                let status: isize =
                    msg_send![class, authorizationStatusForMediaType: &*AVMediaTypeAudio];
                match status {
                    0 => PermissionStatus::NotDetermined,
                    1 => PermissionStatus::Restricted,
                    2 => PermissionStatus::Denied,
                    3 => PermissionStatus::Granted,
                    _ => PermissionStatus::Unknown,
                }
            }
            // IOHIDRequestTypeListenEvent = 1; do not request event posting.
            Permission::InputMonitoring => match IOHIDCheckAccess(1) {
                0 => PermissionStatus::Granted,
                1 => PermissionStatus::Denied,
                _ => PermissionStatus::Unknown,
            },
            Permission::Accessibility => {
                if AXIsProcessTrusted() != 0 {
                    PermissionStatus::Granted
                } else {
                    PermissionStatus::Unknown
                }
            }
            Permission::ScreenRecording => {
                if CGPreflightScreenCaptureAccess() {
                    PermissionStatus::Granted
                } else {
                    PermissionStatus::Unknown
                }
            }
        }
    }
}

pub(super) fn request(permission: Permission) -> Result<()> {
    // SAFETY: Only the selected permission API is invoked. AVFoundation copies
    // the heap block for its asynchronous callback; it captures no application
    // data. No capture/listener/injection is started by these requests.
    unsafe {
        match permission {
            Permission::Microphone => {
                let class =
                    AnyClass::get(c"AVCaptureDevice").context("AVCaptureDevice unavailable")?;
                // Without a usage description macOS terminates the process.
                // Development CLI/harness binaries must fail clearly instead.
                let bundle_class = AnyClass::get(c"NSBundle").context("NSBundle unavailable")?;
                let bundle: *mut AnyObject = msg_send![bundle_class, mainBundle];
                let key = NSString::from_str("NSMicrophoneUsageDescription");
                let description: *mut AnyObject =
                    msg_send![bundle, objectForInfoDictionaryKey: &*key];
                if description.is_null() {
                    bail!(
                        "Microphone permission requires a bundled app with NSMicrophoneUsageDescription"
                    );
                }
                let callback = RcBlock::new(|_granted: Bool| {});
                let _: () = msg_send![class, requestAccessForMediaType: &*AVMediaTypeAudio, completionHandler: &*callback];
            }
            Permission::InputMonitoring => {
                let _ = IOHIDRequestAccess(1);
            }
            // AX has a boolean query only. Opening the specific settings pane
            // lets the user grant access without starting an insertion operation.
            Permission::Accessibility => return open_settings(permission),
            Permission::ScreenRecording => {
                let _ = CGRequestScreenCaptureAccess();
            }
        }
    }
    Ok(())
}

pub(super) fn open_settings(permission: Permission) -> Result<()> {
    let pane = match permission {
        Permission::Microphone => "Privacy_Microphone",
        Permission::InputMonitoring => "Privacy_ListenEvent",
        Permission::Accessibility => "Privacy_Accessibility",
        Permission::ScreenRecording => "Privacy_ScreenCapture",
    };
    open::that(format!("x-apple.systempreferences:com.apple.preference.security?{pane}"))
        .context("Cannot open macOS privacy settings")
}
