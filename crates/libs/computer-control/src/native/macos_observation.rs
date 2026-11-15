//! Public macOS APIs only. AppKit objects stay inside an autorelease pool;
//! CF Create/Copy objects use RAII. No AX value is read from a foreign PID.

use super::*;
use std::{
    ffi::{CStr, c_char, c_void},
    ptr,
    time::{Duration, Instant},
};
type Object = *const c_void;

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}
#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> Object;
    fn sel_registerName(name: *const c_char) -> Object;
    fn objc_msgSend();
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFBooleanTrue: Object;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    fn CFDictionaryCreate(
        allocator: Object,
        keys: *const Object,
        values: *const Object,
        count: isize,
        key_callbacks: Object,
        value_callbacks: Object,
    ) -> Object;
    fn CFRelease(value: Object);
    fn CFGetTypeID(value: Object) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFArrayGetTypeID() -> usize;
    fn CFDictionaryGetTypeID() -> usize;
    fn CFNumberGetTypeID() -> usize;
    fn CFStringCreateWithCString(allocator: Object, bytes: *const c_char, encoding: u32) -> Object;
    fn CFStringGetLength(value: Object) -> isize;
    fn CFStringGetCString(value: Object, buffer: *mut c_char, size: isize, encoding: u32) -> bool;
    fn CFArrayGetCount(value: Object) -> isize;
    fn CFArrayGetValueAtIndex(value: Object, index: isize) -> Object;
    fn CFDictionaryGetValue(dict: Object, key: Object) -> Object;
    fn CFNumberGetValue(value: Object, kind: i32, output: *mut c_void) -> bool;
}
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    static kAXTrustedCheckOptionPrompt: Object;
    fn AXIsProcessTrustedWithOptions(options: Object) -> bool;
    fn AXIsProcessTrusted() -> bool;
    fn AXUIElementCreateApplication(pid: i32) -> Object;
    fn AXUIElementCopyAttributeValue(element: Object, name: Object, result: *mut Object) -> i32;
    fn AXUIElementCopyAttributeValues(
        element: Object,
        name: Object,
        index: isize,
        count: isize,
        result: *mut Object,
    ) -> i32;
    fn AXUIElementGetAttributeValueCount(element: Object, name: Object, count: *mut isize) -> i32;
    fn AXUIElementGetPid(element: Object, pid: *mut i32) -> i32;
    fn AXUIElementGetTypeID() -> usize;
    fn AXUIElementSetMessagingTimeout(element: Object, timeout: f32) -> i32;
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    static kCGWindowOwnerPID: Object;
    static kCGWindowLayer: Object;
    static kCGWindowSharingState: Object;
    static kCGWindowNumber: Object;
    static kCGWindowBounds: Object;
    fn CGRequestScreenCaptureAccess() -> bool;
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGWindowListCopyWindowInfo(options: u32, relative: u32) -> Object;
    fn CGRectMakeWithDictionaryRepresentation(dict: Object, rect: *mut Rect) -> bool;
}
#[derive(Default)]
#[repr(C)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
struct Owned(Object);
impl Owned {
    fn new(value: Object) -> Result<Self, NativeObservationError> {
        if value.is_null() { Err(ControlErrorCode::BackendFailure.into()) } else { Ok(Self(value)) }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: Owned is constructed only for nonnull Create/Copy references.
        unsafe {
            CFRelease(self.0);
        }
    }
}
struct Pool(*mut c_void);
impl Pool {
    fn new() -> Self {
        // SAFETY: Balanced per-call autorelease scope, never crossing await/thread.
        Self(unsafe { objc_autoreleasePoolPush() })
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
        // SAFETY: The pool is dropped on the same thread that created it.
        unsafe {
            objc_autoreleasePoolPop(self.0);
        }
    }
}
fn key(name: &CStr) -> Owned {
    // SAFETY: Static terminated UTF-8 key, default allocator.
    Owned(unsafe { CFStringCreateWithCString(ptr::null(), name.as_ptr(), 0x08000100) })
}
// All objc calls use only known framework selectors and exact method ABIs.
unsafe fn message(object: Object, selector: &CStr) -> Object {
    let send: unsafe extern "C" fn(Object, Object) -> Object =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { send(object, sel_registerName(selector.as_ptr())) }
}
fn string(value: Object) -> Option<String> {
    // SAFETY: Callers supply live CF objects. Type checked before string operations.
    unsafe {
        if value.is_null()
            || CFGetTypeID(value) != CFStringGetTypeID()
            || CFStringGetLength(value) > 1024
        {
            return None;
        }
        let mut bytes = [0u8; 4097];
        if !CFStringGetCString(value, bytes.as_mut_ptr().cast(), bytes.len() as isize, 0x08000100) {
            return None;
        }
        Some(CStr::from_ptr(bytes.as_ptr().cast()).to_string_lossy().into_owned())
    }
}
fn dictionary(dict: Object, name: Object) -> Object {
    // SAFETY: Type checked before dictionary operations; borrowed result never escapes owner.
    unsafe {
        if dict.is_null() || CFGetTypeID(dict) != CFDictionaryGetTypeID() {
            return ptr::null();
        }
        CFDictionaryGetValue(dict, name)
    }
}
fn integer(value: Object) -> Option<i64> {
    // SAFETY: Live object, checked CFNumber type, correctly sized SInt64 output.
    unsafe {
        if value.is_null() || CFGetTypeID(value) != CFNumberGetTypeID() {
            return None;
        }
        let mut number = 0i64;
        CFNumberGetValue(value, 4, (&mut number as *mut i64).cast()).then_some(number)
    }
}
fn ax_error(status: i32) -> NativeObservationError {
    match status {
        -25211 => NativeObservationError::AccessibilityRequired,
        -25202 => ControlErrorCode::TargetChanged.into(),
        _ => ControlErrorCode::BackendFailure.into(),
    }
}
fn attribute(
    element: Object,
    name: &CStr,
    array: bool,
) -> Result<Option<Owned>, NativeObservationError> {
    let mut value = ptr::null();
    // SAFETY: Live AX element, static name; Copy returns an owned CF reference.
    let status = unsafe {
        if array {
            let name = key(name);
            let mut count = 0;
            let status = AXUIElementGetAttributeValueCount(element, name.0, &mut count);
            let Some(limit) = array_read_limit(status, count)? else {
                return Ok(None);
            };
            AXUIElementCopyAttributeValues(element, name.0, 0, limit, &mut value)
        } else {
            AXUIElementCopyAttributeValue(element, key(name).0, &mut value)
        }
    };
    match status {
        0 if !value.is_null() => Ok(Some(Owned(value))),
        -25205 | -25212 => Ok(None), // unsupported attribute / no value
        _ => Err(ax_error(status)),
    }
}
pub(super) fn array_read_limit(
    status: i32,
    count: isize,
) -> Result<Option<isize>, NativeObservationError> {
    match status {
        -25205 | -25212 => Ok(None),
        0 if count == 0 => Ok(None),
        0 if count > 0 => Ok(Some(count.min(256))),
        _ => Err(ax_error(status)),
    }
}
// A broken optional child collection is partial data, not an invented empty list.
// Authorization and target failures remain fatal and are checked again on delivery.
pub(super) fn children_read_failure(
    error: NativeObservationError,
) -> Result<bool, NativeObservationError> {
    match error {
        NativeObservationError::Control(ControlErrorCode::BackendFailure) => Ok(true),
        other => Err(other),
    }
}
fn ax_text(element: Object, name: &CStr) -> Result<Option<String>, NativeObservationError> {
    Ok(attribute(element, name, false)?.and_then(|value| string(value.0)))
}
fn children(array: Object, limit: usize) -> Vec<Object> {
    // SAFETY: CFArray type checked; borrowed elements remain within array lifetime.
    unsafe {
        if array.is_null() || CFGetTypeID(array) != CFArrayGetTypeID() {
            return Vec::new();
        }
        (0..CFArrayGetCount(array).min(limit as isize))
            .map(|i| CFArrayGetValueAtIndex(array, i))
            .collect()
    }
}
fn tree(
    element: Object,
    pid: u32,
    depth: usize,
    remaining: &mut usize,
    deadline: Instant,
) -> Result<serde_json::Value, NativeObservationError> {
    if *remaining == 0 || depth > 10 || Instant::now() >= deadline {
        return Ok(serde_json::json!({"truncated":true}));
    }
    *remaining -= 1;
    let mut actual = 0;
    // SAFETY: Verify AX type and process ownership before reading any attributes.
    unsafe {
        if element.is_null()
            || CFGetTypeID(element) != AXUIElementGetTypeID()
            || AXUIElementGetPid(element, &mut actual) != 0
            || actual as u32 != pid
        {
            return Err(ControlErrorCode::TargetChanged.into());
        }
        AXUIElementSetMessagingTimeout(element, 0.2);
    }
    let role = ax_text(element, c"AXRole")?
        .ok_or(NativeObservationError::Control(ControlErrorCode::BackendFailure))?;
    let subrole = ax_text(element, c"AXSubrole")?;
    if role == "AXSecureTextField" || subrole.as_deref() == Some("AXSecureTextField") {
        return Ok(serde_json::json!({"role":role,"redacted":true}));
    }
    let mut result = serde_json::json!({"role":role});
    for (key, name) in
        [("title", c"AXTitle"), ("description", c"AXDescription"), ("value", c"AXValue")]
    {
        if let Some(text) = ax_text(element, name)? {
            result[key] = text.into();
        }
    }
    let child_array = match attribute(element, c"AXChildren", true) {
        Ok(array) => array,
        Err(error) => {
            result["children_unavailable"] = children_read_failure(error)?.into();
            None
        }
    };
    if let Some(array) = child_array {
        let mut nodes = Vec::new();
        for child in children(array.0, *remaining) {
            nodes.push(tree(child, pid, depth + 1, remaining, deadline)?);
            if *remaining == 0 || Instant::now() >= deadline {
                break;
            }
        }
        result["children"] = nodes.into();
    }
    Ok(result)
}

pub(super) struct MacOsObservation;
impl MacOsObservation {
    fn instance(pid: u32) -> Result<ApplicationInstance, NativeObservationError> {
        if pid == 0 || pid > i32::MAX as u32 {
            return Err(ControlErrorCode::TargetChanged.into());
        }
        let _pool = Pool::new();
        // SAFETY: proc_pidinfo fills the SDK-defined BSD info; exact returned size
        // and effective UID are checked. AppKit objects are retained by this pool.
        unsafe {
            let mut bsd: libc::proc_bsdinfo = std::mem::zeroed();
            let size = std::mem::size_of_val(&bsd) as i32;
            if libc::proc_pidinfo(
                pid as i32,
                libc::PROC_PIDTBSDINFO,
                0,
                (&mut bsd as *mut libc::proc_bsdinfo).cast(),
                size,
            ) != size
                || bsd.pbi_uid != libc::geteuid()
            {
                return Err(ControlErrorCode::TargetChanged.into());
            }
            let send: unsafe extern "C" fn(Object, Object, i32) -> Object =
                std::mem::transmute(objc_msgSend as *const ());
            let app = send(
                objc_getClass(c"NSRunningApplication".as_ptr()),
                sel_registerName(c"runningApplicationWithProcessIdentifier:".as_ptr()),
                pid as i32,
            );
            if app.is_null() {
                return Err(ControlErrorCode::TargetChanged.into());
            }
            let bundle_id = string(message(app, c"bundleIdentifier"))
                .ok_or(NativeObservationError::Control(ControlErrorCode::TargetChanged))?;
            Ok(ApplicationInstance {
                bundle_id,
                process_id: pid,
                start_seconds: bsd.pbi_start_tvsec,
                start_microseconds: bsd.pbi_start_tvusec,
            })
        }
    }
    fn windows(pid: u32) -> Result<Vec<WindowGeometry>, NativeObservationError> {
        // SAFETY: Create/Copy ownership and field types checked by helpers. Options
        // are OnScreenOnly | ExcludeDesktopElements; no foreign titles are read.
        let list = Owned::new(unsafe { CGWindowListCopyWindowInfo(1 | 16, 0) })?;
        // SAFETY: Framework-owned immutable CFString keys have process lifetime.
        let (owner, layer, sharing, number, bounds_key) = unsafe {
            (
                kCGWindowOwnerPID,
                kCGWindowLayer,
                kCGWindowSharingState,
                kCGWindowNumber,
                kCGWindowBounds,
            )
        };
        let mut windows = Vec::new();
        for dict in children(list.0, 4096) {
            if integer(dictionary(dict, owner)) != Some(i64::from(pid))
                || integer(dictionary(dict, layer)) != Some(0)
                || integer(dictionary(dict, sharing)).unwrap_or(0) == 0
            {
                continue;
            }
            let id = integer(dictionary(dict, number))
                .and_then(|v| u32::try_from(v).ok())
                .ok_or(NativeObservationError::Control(ControlErrorCode::BackendFailure))?;
            let bounds = dictionary(dict, bounds_key);
            let mut r = Rect::default();
            // SAFETY: Bounds must be a dictionary; the output is a repr(C) CGRect.
            if bounds.is_null()
                || unsafe {
                    CFGetTypeID(bounds) != CFDictionaryGetTypeID()
                        || !CGRectMakeWithDictionaryRepresentation(bounds, &mut r)
                }
            {
                continue;
            }
            let window =
                WindowGeometry { window_id: id, x: r.x, y: r.y, width: r.width, height: r.height };
            ScreenshotGeometry::new(window, 1, 1)?;
            windows.push(window);
        }
        Ok(windows)
    }
}
#[async_trait::async_trait]
impl ObservationPlatform for MacOsObservation {
    fn request_permission(
        &self,
        permission: NativePermission,
    ) -> Result<(), NativeObservationError> {
        // SAFETY: Explicit trusted UI action only. Framework constants and typed
        // callbacks live for the process; the owned options dictionary is released.
        // AX's prompt is asynchronous; callers requery instead of assuming success.
        unsafe {
            match permission {
                NativePermission::Accessibility => {
                    let keys = [kAXTrustedCheckOptionPrompt];
                    let values = [kCFBooleanTrue];
                    let options = Owned::new(CFDictionaryCreate(
                        ptr::null(),
                        keys.as_ptr(),
                        values.as_ptr(),
                        1,
                        ptr::addr_of!(kCFTypeDictionaryKeyCallBacks),
                        ptr::addr_of!(kCFTypeDictionaryValueCallBacks),
                    ))?;
                    let _ = AXIsProcessTrustedWithOptions(options.0);
                }
                NativePermission::ScreenRecording => {
                    let _ = CGRequestScreenCaptureAccess();
                }
            }
        }
        Ok(())
    }

    fn permissions(&self) -> NativePermissions {
        // SAFETY: Nonprompting public queries. These results apply to this process only.
        unsafe {
            NativePermissions {
                accessibility: AXIsProcessTrusted(),
                screen_recording: CGPreflightScreenCaptureAccess(),
            }
        }
    }
    fn identity(&self, pid: u32) -> Result<ApplicationInstance, NativeObservationError> {
        Self::instance(pid)
    }
    fn applications(&self) -> Result<Vec<ApplicationInstance>, NativeObservationError> {
        let _pool = Pool::new();
        // SAFETY: Known NSWorkspace/NSArray selectors, typed objc_msgSend ABIs.
        unsafe {
            let workspace = message(objc_getClass(c"NSWorkspace".as_ptr()), c"sharedWorkspace");
            let apps = message(workspace, c"runningApplications");
            let send_count: unsafe extern "C" fn(Object, Object) -> usize =
                std::mem::transmute(objc_msgSend as *const ());
            let count = send_count(apps, sel_registerName(c"count".as_ptr())).min(4096);
            let send_index: unsafe extern "C" fn(Object, Object, usize) -> Object =
                std::mem::transmute(objc_msgSend as *const ());
            let send_pid: unsafe extern "C" fn(Object, Object) -> i32 =
                std::mem::transmute(objc_msgSend as *const ());
            let mut result = Vec::new();
            for i in 0..count {
                let app = send_index(apps, sel_registerName(c"objectAtIndex:".as_ptr()), i);
                let pid = send_pid(app, sel_registerName(c"processIdentifier".as_ptr()));
                if let Ok(app) = Self::instance(pid as u32) {
                    result.push(app);
                }
            }
            Ok(result)
        }
    }
    fn read(
        &self,
        app: &ApplicationInstance,
    ) -> Result<PlatformObservation, NativeObservationError> {
        if !self.permissions().accessibility {
            return Err(NativeObservationError::AccessibilityRequired);
        }
        if Self::instance(app.process_id)? != *app {
            return Err(ControlErrorCode::TargetChanged.into());
        }
        // SAFETY: Positive PID from kernel identity. AX root is a Copy/Create object.
        let root = Owned::new(unsafe { AXUIElementCreateApplication(app.process_id as i32) })?;
        unsafe {
            AXUIElementSetMessagingTimeout(root.0, 0.2);
        }
        let windows = attribute(root.0, c"AXWindows", true)?;
        let mut remaining = 256;
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut nodes = Vec::new();
        if let Some(array) = windows {
            for window in children(array.0, 32) {
                nodes.push(tree(window, app.process_id, 0, &mut remaining, deadline)?);
                if remaining == 0 || Instant::now() >= deadline {
                    break;
                }
            }
        }
        Ok(PlatformObservation {
            tree: serde_json::json!({"windows":nodes,"truncated":remaining==0 || Instant::now()>=deadline}),
            window: Self::windows(app.process_id)?.into_iter().next(),
        })
    }
    async fn capture(
        &self,
        app: &ApplicationInstance,
        window: WindowGeometry,
    ) -> Result<Vec<u8>, NativeObservationError> {
        if !self.permissions().screen_recording {
            return Err(NativeObservationError::ScreenRecordingRequired);
        }
        if Self::instance(app.process_id)? != *app
            || !Self::windows(app.process_id)?.contains(&window)
        {
            return Err(ControlErrorCode::TargetChanged.into());
        }
        let fail = || NativeObservationError::Control(ControlErrorCode::BackendFailure);
        let dir = tempfile::tempdir().map_err(|_| fail())?;
        let path = dir.path().join("window.png");
        // This public macOS utility captures one window backing store, excluding
        // attached windows (-a), shadows (-o), and other apps. Never fall back to
        // desktop/rectangle capture. No shell, clipboard, cursor or UI interaction.
        let status = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::process::Command::new("/usr/sbin/screencapture")
                .args(["-x", "-o", "-a", "-t", "png", &format!("-l{}", window.window_id)])
                .arg(&path)
                .env_clear()
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .status(),
        )
        .await
        .map_err(|_| fail())?
        .map_err(|_| fail())?;
        if !status.success() {
            return Err(fail());
        }
        if Self::instance(app.process_id)? != *app
            || !Self::windows(app.process_id)?.contains(&window)
        {
            return Err(ControlErrorCode::TargetChanged.into());
        }
        if std::fs::metadata(&path).map_err(|_| fail())?.len() > 16 * 1024 * 1024 {
            return Err(fail());
        }
        std::fs::read(path).map_err(|_| fail())
    }
}
