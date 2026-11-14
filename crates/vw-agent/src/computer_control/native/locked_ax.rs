//! Same-process-instance AX commands. No global events, activation or authentication.
use super::*;
use std::ffi::c_void;
type Ref = *const c_void;
#[repr(C)]
#[derive(Default)]
struct Range { location: isize, length: isize }
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> Ref;
    fn AXUIElementCopyAttributeValue(element: Ref, key: Ref, value: *mut Ref) -> i32;
    fn AXUIElementIsAttributeSettable(element: Ref, key: Ref, value: *mut u8) -> i32;
    fn AXUIElementSetAttributeValue(element: Ref, key: Ref, value: Ref) -> i32;
    fn AXUIElementGetPid(element: Ref, pid: *mut i32) -> i32;
    fn AXUIElementGetTypeID() -> usize;
    fn AXUIElementCopyElementAtPosition(app: Ref, x: f32, y: f32, value: *mut Ref) -> i32;
    fn AXUIElementPerformAction(element: Ref, action: Ref) -> i32;
    fn AXUIElementSetMessagingTimeout(element: Ref, timeout: f32) -> i32;
    fn AXValueGetTypeID() -> usize;
    fn AXValueGetValue(value: Ref, kind: u32, output: *mut c_void) -> bool;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: Ref);
    fn CFGetTypeID(value: Ref) -> usize;
    fn CFEqual(a: Ref, b: Ref) -> u8;
    fn CFStringGetTypeID() -> usize;
    fn CFStringCreateWithCharacters(allocator: Ref, chars: *const u16, length: isize) -> Ref;
    fn CFStringGetLength(value: Ref) -> isize;
    fn CFStringGetCharacters(value: Ref, range: Range, chars: *mut u16);
}
fn unsupported() -> NativeObservationError { ControlErrorCode::Unsupported.into() }
fn failed() -> NativeObservationError { ControlErrorCode::BackendFailure.into() }
struct Owned(Ref);
impl Drop for Owned {
    fn drop(&mut self) { unsafe { CFRelease(self.0) } }
}
impl Owned {
    fn new(value: Ref) -> Result<Self, NativeObservationError> {
        if value.is_null() { Err(failed()) } else { Ok(Self(value)) }
    }
    fn string(value: &str) -> Result<Self, NativeObservationError> {
        let chars: Vec<_> = value.encode_utf16().collect();
        // SAFETY: CF copies the valid buffer; Owned releases the retained result.
        Self::new(unsafe { CFStringCreateWithCharacters(std::ptr::null(), chars.as_ptr(), chars.len() as isize) })
    }
    fn attribute(&self, name: &str) -> Result<Self, NativeObservationError> {
        let key = Self::string(name)?;
        let mut value = std::ptr::null();
        // SAFETY: Live references and writable output, Copy ownership on success.
        let code = unsafe { AXUIElementCopyAttributeValue(self.0, key.0, &mut value) };
        if code != 0 { return Err(unsupported()); }
        Self::new(value)
    }
    fn optional_attribute(&self, name: &str) -> Result<Option<Self>, NativeObservationError> {
        let key = Self::string(name)?;
        let mut value = std::ptr::null();
        // SAFETY: Same Copy ownership as attribute. Only documented absent/unsupported
        // attribute errors are optional; messaging and permission errors fail closed.
        match unsafe { AXUIElementCopyAttributeValue(self.0, key.0, &mut value) } {
            0 => Self::new(value).map(Some),
            -25205 | -25212 => Ok(None),
            _ => Err(failed()),
        }
    }
    fn units(&self) -> Result<Vec<u16>, NativeObservationError> {
        // SAFETY: Verify CF type and bound length before allocation and extraction.
        unsafe {
            if CFGetTypeID(self.0) != CFStringGetTypeID() { return Err(unsupported()); }
            let length = CFStringGetLength(self.0);
            if !(0..=65536).contains(&length) { return Err(unsupported()); }
            let mut units = vec![0; length as usize];
            CFStringGetCharacters(self.0, Range { location: 0, length }, units.as_mut_ptr());
            Ok(units)
        }
    }
    fn belongs_to(&self, pid: u32) -> Result<(), NativeObservationError> {
        let mut actual = 0;
        // SAFETY: AX type is checked before calling AX routines on copied objects.
        if unsafe { CFGetTypeID(self.0) != AXUIElementGetTypeID()
            || AXUIElementGetPid(self.0, &mut actual) != 0 } || actual <= 0 || actual as u32 != pid {
            return Err(ControlErrorCode::TargetChanged.into());
        }
        Ok(())
    }
}

/// Called while the observer gate is held; every effect has a fresh live guard.
/// AX timeout is an ambiguous outcome: never retry automatically.
pub(super) fn execute(observer: &NativeObserver, input: &NativeInput) -> Result<(), NativeObservationError> {
    observer.check(false)?;
    let state = locked_session::lock_state();
    let check = || -> Result<(), NativeObservationError> {
        observer.check(false)?;
        if locked_session::lock_state() != state { return Err(ControlErrorCode::Cancelled.into()); }
        Ok(())
    };
    // SAFETY: PID is revalidated against process start time by observer.check.
    let app = Owned::new(unsafe { AXUIElementCreateApplication(observer.app.process_id as i32) })?;
    if unsafe { AXUIElementSetMessagingTimeout(app.0, 0.2) } != 0 { return Err(failed()); }
    match input {
        NativeInput::TypeText { text } => {
            if text.is_empty() || text.len() > 16384 || text.chars().any(char::is_control) {
                return Err(ControlErrorCode::InvalidRequest.into());
            }
            let element = app.attribute("AXFocusedUIElement")?;
            element.belongs_to(observer.app.process_id)?;
            let role = element.attribute("AXRole")?.units()?;
            if role != "AXTextField".encode_utf16().collect::<Vec<_>>()
                && role != "AXTextArea".encode_utf16().collect::<Vec<_>>() { return Err(unsupported()); }
            // Ordinary text controls may have no subrole. Other read errors close.
            // Check secure subtype before requesting any text value.
            let subrole = match element.optional_attribute("AXSubrole")? {
                Some(value) => value.units()?,
                None => Vec::new(),
            };
            if subrole == "AXSecureTextField".encode_utf16().collect::<Vec<_>>() { return Err(unsupported()); }
            let key = Owned::string("AXSelectedText")?;
            let mut settable = 0;
            if unsafe { AXUIElementIsAttributeSettable(element.0, key.0, &mut settable) } != 0 || settable == 0 {
                return Err(unsupported());
            }
            let before = element.attribute("AXValue")?.units()?;
            let selection = element.attribute("AXSelectedTextRange")?;
            let mut range = Range::default();
            if unsafe { CFGetTypeID(selection.0) != AXValueGetTypeID()
                || !AXValueGetValue(selection.0, 4, (&mut range as *mut Range).cast()) }
                || range.location < 0 || range.length < 0 { return Err(unsupported()); }
            let start = range.location as usize;
            let end = start.checked_add(range.length as usize).ok_or_else(unsupported)?;
            if end > before.len() { return Err(unsupported()); }
            let mut expected = before[..start].to_vec();
            expected.extend(text.encode_utf16());
            expected.extend_from_slice(&before[end..]);
            if expected.len() > 65536 { return Err(unsupported()); }
            let replacement = Owned::string(text)?;
            check()?;
            let focused = app.attribute("AXFocusedUIElement")?;
            if unsafe { CFEqual(focused.0, element.0) } == 0
                || element.attribute("AXValue")?.units()? != before { return Err(ControlErrorCode::TargetChanged.into()); }
            let current = element.attribute("AXSelectedTextRange")?;
            if unsafe { CFEqual(current.0, selection.0) } == 0 { return Err(ControlErrorCode::TargetChanged.into()); }
            check()?;
            if unsafe { AXUIElementSetAttributeValue(element.0, key.0, replacement.0) } != 0 { return Err(failed()); }
            check()?;
            if element.attribute("AXValue")?.units()? != expected { return Err(failed()); }
        }
        NativeInput::Click { x, y, button: MouseButton::Left } => {
            let (window, geometry) = observer.media.lock().as_ref()
                .map(|s| (s.window, s.geometry)).ok_or_else(unsupported)?;
            let (x, y) = geometry.screen_point(u32::try_from(*x).map_err(|_| unsupported())?, u32::try_from(*y).map_err(|_| unsupported())?)?;
            if observer.backend.read(&observer.app)?.window != Some(window) { return Err(ControlErrorCode::TargetChanged.into()); }
            let mut value = std::ptr::null();
            if unsafe { AXUIElementCopyElementAtPosition(app.0, x as f32, y as f32, &mut value) } != 0 { return Err(unsupported()); }
            let element = Owned::new(value)?;
            element.belongs_to(observer.app.process_id)?;
            let action = Owned::string("AXPress")?;
            check()?;
            if observer.backend.read(&observer.app)?.window != Some(window) {
                return Err(ControlErrorCode::TargetChanged.into());
            }
            let mut current = std::ptr::null();
            if unsafe { AXUIElementCopyElementAtPosition(app.0, x as f32, y as f32, &mut current) } != 0 {
                return Err(unsupported());
            }
            let current = Owned::new(current)?;
            current.belongs_to(observer.app.process_id)?;
            if unsafe { CFEqual(current.0, element.0) } == 0 {
                return Err(ControlErrorCode::TargetChanged.into());
            }
            check()?;
            if unsafe { AXUIElementPerformAction(element.0, action.0) } != 0 { return Err(failed()); }
            check()?;
            // Require a fresh, same-instance observation after the application action.
            observer.backend.read(&observer.app)?;
        }
        _ => return Err(unsupported()),
    }
    check()
}
