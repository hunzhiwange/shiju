//! Target-directed input. Never post to the global event tap or use the clipboard.
use super::*;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum InputEvent {
    Click { window: WindowGeometry, x: f64, y: f64, button: MouseButton },
    Scroll { window: WindowGeometry, x: f64, y: f64, delta_x: i32, delta_y: i32 },
    Key { code: u16, flags: u64 },
    Unicode(Vec<u16>),
}
/// Only the OS boundary is injectable. Each emission is one balanced event pair
/// (or one scroll), directed to the verified PID even if focus races afterward.
pub(super) trait InputPlatform: Send + Sync {
    fn focused_pid(&self) -> Result<u32, NativeObservationError>;
    fn activate(&self, app: &ApplicationInstance) -> Result<(), NativeObservationError>;
    fn emit(
        &self,
        app: &ApplicationInstance,
        event: &InputEvent,
    ) -> Result<(), NativeObservationError>;
}
fn input_platform() -> Result<Box<dyn InputPlatform>, NativeObservationError> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(os::MacOsInput))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(ControlErrorCode::Unsupported.into())
    }
}
impl NativeObserver {
    /// Uses this session's captured process instance, permission version and stop
    /// token. Click coordinates are pixels in its latest screenshot, not desktop pixels.
    pub async fn execute_input(&self, input: &NativeInput) -> Result<(), NativeObservationError> {
        self.execute_input_with(input, input_platform()?.as_ref()).await
    }
    pub(super) async fn execute_input_with(
        &self,
        input: &NativeInput,
        backend: &dyn InputPlatform,
    ) -> Result<(), NativeObservationError> {
        let _gate = self.observe_gate.lock().await;
        self.check(false)?;
        if self.locked.load(Ordering::SeqCst) {
            #[cfg(target_os = "macos")]
            { return super::locked_ax::execute(self, input); }
            #[cfg(not(target_os = "macos"))]
            { return Err(ControlErrorCode::Unsupported.into()); }
        }
        if matches!(input, NativeInput::Activate) {
            // Activation itself is authorized, but is the only action allowed
            // while the target is not frontmost. Never activate another application.
            self.check(false)?;
            backend.activate(&self.app)?;
            for _ in 0..50 {
                self.check(false)?;
                if backend.focused_pid()? == self.app.process_id {
                    return Ok(());
                }
                tokio::select! {
                    biased;
                    _ = self.cancelled.cancelled() => return Err(ControlErrorCode::Cancelled.into()),
                    _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {},
                }
            }
            return Err(ControlErrorCode::TargetChanged.into());
        }
        let events = match input {
            NativeInput::TypeText { text } => {
                if text.is_empty() || text.len() > 16 * 1024 || text.chars().any(|c| c.is_control())
                {
                    return Err(ControlErrorCode::InvalidRequest.into());
                }
                text.chars()
                    .map(|c| InputEvent::Unicode(c.encode_utf16(&mut [0; 2]).to_vec()))
                    .collect::<Vec<_>>()
            }
            NativeInput::Press { key, modifiers } => {
                let code = key_code(key)
                    .ok_or(NativeObservationError::Control(ControlErrorCode::InvalidRequest))?;
                let mut flags = 0;
                for modifier in modifiers {
                    let flag = match modifier {
                        KeyModifier::Shift => 1 << 17,
                        KeyModifier::Control => 1 << 18,
                        KeyModifier::Alt => 1 << 19,
                        KeyModifier::Meta => 1 << 20,
                    };
                    if flags & flag != 0 {
                        return Err(ControlErrorCode::InvalidRequest.into());
                    }
                    flags |= flag;
                }
                vec![InputEvent::Key { code, flags }]
            }
            NativeInput::Click { x, y, button } => {
                let (window, geometry) = self
                    .media
                    .lock()
                    .as_ref()
                    .map(|s| (s.window, s.geometry))
                    .ok_or(NativeObservationError::Control(ControlErrorCode::InvalidRequest))?;
                let (x, y) = geometry.screen_point(
                    u32::try_from(*x).map_err(|_| {
                        NativeObservationError::Control(ControlErrorCode::InvalidRequest)
                    })?,
                    u32::try_from(*y).map_err(|_| {
                        NativeObservationError::Control(ControlErrorCode::InvalidRequest)
                    })?,
                )?;
                if self.backend.read(&self.app)?.window != Some(window) {
                    return Err(ControlErrorCode::TargetChanged.into());
                }
                vec![InputEvent::Click { window, x, y, button: *button }]
            }
            NativeInput::Scroll { delta_x, delta_y } => {
                if delta_x.unsigned_abs() > 10000
                    || delta_y.unsigned_abs() > 10000
                    || (*delta_x == 0 && *delta_y == 0)
                {
                    return Err(ControlErrorCode::InvalidRequest.into());
                }
                let window = self
                    .backend
                    .read(&self.app)?
                    .window
                    .ok_or(NativeObservationError::Control(ControlErrorCode::TargetChanged))?;
                ScreenshotGeometry::new(window, 1, 1)?;
                vec![InputEvent::Scroll {
                    window,
                    x: window.x + window.width / 2.0,
                    y: window.y + window.height / 2.0,
                    delta_x: *delta_x,
                    delta_y: *delta_y,
                }]
            }
            NativeInput::Activate => unreachable!(),
        };
        for event in events {
            // A fresh guard for every scalar/pair: stop, revocation, lock, OS
            // permission, PID recycling and foreground changes halt the remainder.
            self.check(false)?;
            if backend.focused_pid()? != self.app.process_id {
                return Err(ControlErrorCode::TargetChanged.into());
            }
            self.check(false)?;
            backend.emit(&self.app, &event)?;
            tokio::task::yield_now().await;
        }
        self.check(false)?;
        if backend.focused_pid()? != self.app.process_id {
            return Err(ControlErrorCode::TargetChanged.into());
        }
        Ok(())
    }
}
fn key_code(key: &str) -> Option<u16> {
    Some(match key {
        "a" | "A" => 0,
        "s" | "S" => 1,
        "d" | "D" => 2,
        "f" | "F" => 3,
        "h" | "H" => 4,
        "g" | "G" => 5,
        "z" | "Z" => 6,
        "x" | "X" => 7,
        "c" | "C" => 8,
        "v" | "V" => 9,
        "b" | "B" => 11,
        "q" | "Q" => 12,
        "w" | "W" => 13,
        "e" | "E" => 14,
        "r" | "R" => 15,
        "y" | "Y" => 16,
        "t" | "T" => 17,
        "1" => 18,
        "2" => 19,
        "3" => 20,
        "4" => 21,
        "6" => 22,
        "5" => 23,
        "=" => 24,
        "9" => 25,
        "7" => 26,
        "-" => 27,
        "8" => 28,
        "0" => 29,
        "]" => 30,
        "o" | "O" => 31,
        "u" | "U" => 32,
        "[" => 33,
        "i" | "I" => 34,
        "p" | "P" => 35,
        "Enter" => 36,
        "l" | "L" => 37,
        "j" | "J" => 38,
        "'" => 39,
        "k" | "K" => 40,
        ";" => 41,
        "\\" => 42,
        "," => 43,
        "/" => 44,
        "n" | "N" => 45,
        "m" | "M" => 46,
        "." => 47,
        "Tab" => 48,
        "Space" => 49,
        "`" => 50,
        "Backspace" => 51,
        "Escape" => 53,
        "Home" => 115,
        "PageUp" => 116,
        "Delete" => 117,
        "End" => 119,
        "PageDown" => 121,
        "ArrowLeft" => 123,
        "ArrowRight" => 124,
        "ArrowDown" => 125,
        "ArrowUp" => 126,
        _ => return None,
    })
}

#[cfg(target_os = "macos")]
mod os {
    use super::*;
    use std::ffi::{c_char, c_void};
    type Object = *const c_void;
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Point {
        x: f64,
        y: f64,
    }
    #[repr(C)]
    struct Rect {
        origin: Point,
        size: Point,
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventCreateKeyboardEvent(source: Object, code: u16, down: bool) -> Object;
        fn CGEventKeyboardSetUnicodeString(event: Object, length: usize, text: *const u16);
        fn CGEventCreateCopy(event: Object) -> Object;
        fn CGEventSetType(event: Object, kind: u32);
        fn CGMainDisplayID() -> u32;
        fn CGDisplayBounds(display: u32) -> Rect;
        fn CGEventGetIntegerValueField(event: Object, field: u32) -> i64;
        fn CGEventCreateScrollWheelEvent(source: Object, units: u32, count: u32, ...) -> Object;
        fn CGEventSetFlags(event: Object, flags: u64);
        fn CGEventSetIntegerValueField(event: Object, field: u32, value: i64);
        fn CGEventPostToPid(pid: i32, event: Object);
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(object: Object);
    }
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
    struct Pool(*mut c_void);
    impl Pool {
        fn new() -> Self {
            // SAFETY: Balanced pool scoped to this thread and synchronous call.
            Self(unsafe { objc_autoreleasePoolPush() })
        }
    }
    impl Drop for Pool {
        fn drop(&mut self) {
            unsafe {
                objc_autoreleasePoolPop(self.0);
            }
        }
    }
    struct Event(Object);
    impl Event {
        fn new(value: Object) -> Result<Self, NativeObservationError> {
            if value.is_null() {
                Err(ControlErrorCode::BackendFailure.into())
            } else {
                Ok(Self(value))
            }
        }
    }
    impl Drop for Event {
        fn drop(&mut self) {
            // SAFETY: Nonnull owned Create reference, released once.
            unsafe {
                CFRelease(self.0);
            }
        }
    }
    unsafe fn window_event(
        kind: u32,
        window: WindowGeometry,
        x: f64,
        y: f64,
    ) -> Result<Event, NativeObservationError> {
        // Target-directed CGEvents carry window-local Quartz points. NSEvent's
        // public factory preserves destination window metadata that raw events
        // lack. For a foreign window its factory uses screen coordinates; convert
        // through the primary display height without moving the cursor.
        unsafe {
            let height = CGDisplayBounds(CGMainDisplayID()).size.y;
            let point = Point { x: x - window.x, y: height - (y - window.y) };
            let factory: unsafe extern "C" fn(
                Object,
                Object,
                usize,
                Point,
                u64,
                f64,
                isize,
                Object,
                isize,
                isize,
                f32,
            ) -> Object = std::mem::transmute(objc_msgSend as *const ());
            let native = factory(objc_getClass(c"NSEvent".as_ptr()), sel_registerName(c"mouseEventWithType:location:modifierFlags:timestamp:windowNumber:context:eventNumber:clickCount:pressure:".as_ptr()), 1, point, 0, 0.0, window.window_id as isize, std::ptr::null(), 1, 1, 1.0);
            if native.is_null() {
                return Err(ControlErrorCode::BackendFailure.into());
            }
            let send: unsafe extern "C" fn(Object, Object) -> Object =
                std::mem::transmute(objc_msgSend as *const ());
            let cg = send(native, sel_registerName(c"CGEvent".as_ptr()));
            if cg.is_null() {
                return Err(ControlErrorCode::BackendFailure.into());
            }
            let event = Event::new(CGEventCreateCopy(cg))?;
            CGEventSetType(event.0, kind);
            Ok(event)
        }
    }
    pub(super) struct MacOsInput;
    impl InputPlatform for MacOsInput {
        fn focused_pid(&self) -> Result<u32, NativeObservationError> {
            let _pool = Pool::new();
            // SAFETY: Known AppKit selectors and exact objc_msgSend ABIs.
            unsafe {
                let send: unsafe extern "C" fn(Object, Object) -> Object =
                    std::mem::transmute(objc_msgSend as *const ());
                let workspace = send(
                    objc_getClass(c"NSWorkspace".as_ptr()),
                    sel_registerName(c"sharedWorkspace".as_ptr()),
                );
                let app = send(workspace, sel_registerName(c"frontmostApplication".as_ptr()));
                if app.is_null() {
                    return Err(ControlErrorCode::TargetChanged.into());
                }
                let pid: unsafe extern "C" fn(Object, Object) -> i32 =
                    std::mem::transmute(objc_msgSend as *const ());
                u32::try_from(pid(app, sel_registerName(c"processIdentifier".as_ptr())))
                    .map_err(|_| ControlErrorCode::TargetChanged.into())
            }
        }
        fn activate(&self, app: &ApplicationInstance) -> Result<(), NativeObservationError> {
            let _pool = Pool::new();
            // SAFETY: Verified positive PID; fixed selectors and typed ABIs.
            unsafe {
                let send: unsafe extern "C" fn(Object, Object, i32) -> Object =
                    std::mem::transmute(objc_msgSend as *const ());
                let running = send(
                    objc_getClass(c"NSRunningApplication".as_ptr()),
                    sel_registerName(c"runningApplicationWithProcessIdentifier:".as_ptr()),
                    app.process_id as i32,
                );
                if running.is_null() {
                    return Err(ControlErrorCode::TargetChanged.into());
                }
                let activate: unsafe extern "C" fn(Object, Object, usize) -> bool =
                    std::mem::transmute(objc_msgSend as *const ());
                if !activate(running, sel_registerName(c"activateWithOptions:".as_ptr()), 2) {
                    return Err(ControlErrorCode::TargetChanged.into());
                }
            }
            Ok(())
        }
        fn emit(
            &self,
            app: &ApplicationInstance,
            input: &InputEvent,
        ) -> Result<(), NativeObservationError> {
            // SAFETY: CGEvent Create objects owned by Event; UTF-16 slices remain
            // live during copying. No global event tap; post only to checked PID.
            let _pool = Pool::new();
            unsafe {
                let source = std::ptr::null();
                let (down, up, flags) = match input {
                    InputEvent::Click { window, x, y, button } => {
                        let (down, up, button) = match button {
                            MouseButton::Left => (1, 2, 0),
                            MouseButton::Right => (3, 4, 1),
                            MouseButton::Middle => (25, 26, 2),
                        };
                        let down = window_event(down, *window, *x, *y)?;
                        let up = window_event(up, *window, *x, *y)?;
                        for event in [&down, &up] {
                            CGEventSetIntegerValueField(event.0, 3, button);
                        }
                        (down, Some(up), 0)
                    }
                    InputEvent::Scroll { window, x, y, delta_x, delta_y } => {
                        let scroll = Event::new(CGEventCreateScrollWheelEvent(
                            source, 0, 2, *delta_y, *delta_x,
                        ))?;
                        let event = window_event(22, *window, *x, *y)?;
                        // Public scroll deltas, fixed-point/point deltas and units,
                        // generated by CoreGraphics rather than guessed scaling.
                        for field in [11, 12, 93, 94, 96, 97, 88] {
                            CGEventSetIntegerValueField(
                                event.0,
                                field,
                                CGEventGetIntegerValueField(scroll.0, field),
                            );
                        }
                        (event, None, 0)
                    }
                    InputEvent::Key { code, flags } => (
                        Event::new(CGEventCreateKeyboardEvent(source, *code, true))?,
                        Some(Event::new(CGEventCreateKeyboardEvent(source, *code, false))?),
                        *flags,
                    ),
                    InputEvent::Unicode(text) => {
                        let down = Event::new(CGEventCreateKeyboardEvent(source, 0, true))?;
                        let up = Event::new(CGEventCreateKeyboardEvent(source, 0, false))?;
                        for event in [&down, &up] {
                            CGEventKeyboardSetUnicodeString(event.0, text.len(), text.as_ptr());
                        }
                        (down, Some(up), 0)
                    }
                };
                CGEventSetFlags(down.0, flags);
                CGEventPostToPid(app.process_id as i32, down.0);
                if let Some(up) = up {
                    CGEventSetFlags(up.0, flags);
                    CGEventPostToPid(app.process_id as i32, up.0);
                }
            }
            Ok(())
        }
    }
}
