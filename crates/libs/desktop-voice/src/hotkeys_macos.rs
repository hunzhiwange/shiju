//! macOS listen-only tap. Carbon reserves only the two configured combinations.
//! All registrations are created/dropped on the UI thread; CF objects stay on their worker.
use super::hotkeys::{ALT, COMMAND, CONTROL, Config, Input, MODIFIERS, SHIFT, Shortcut};
use super::permissions::{Permission, PermissionBackend, PermissionStatus, SystemPermissions};
use block2::RcBlock;
use objc2::{
    MainThreadMarker, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject},
};
use objc2_foundation::NSString;
use std::{
    ffi::c_void,
    ptr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime},
};
type Ref = *const c_void;
#[repr(C)]
struct HotkeyId {
    signature: u32,
    id: u32,
}
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn RegisterEventHotKey(
        key: u32,
        modifiers: u32,
        id: HotkeyId,
        target: Ref,
        options: u32,
        out: *mut Ref,
    ) -> i32;
    fn UnregisterEventHotKey(key: Ref) -> i32;
    fn GetApplicationEventTarget() -> Ref;
    fn IsSecureEventInputEnabled() -> bool;
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventTapCreate(
        location: u32,
        place: u32,
        options: u32,
        mask: u64,
        callback: unsafe extern "C" fn(Ref, u32, Ref, *mut c_void) -> Ref,
        data: *mut c_void,
    ) -> Ref;
    fn CGEventGetIntegerValueField(event: Ref, field: u32) -> i64;
    fn CGEventGetFlags(event: Ref) -> u64;
    fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
    fn CGSessionCopyCurrentDictionary() -> Ref;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFRunLoopDefaultMode: Ref;
    static kCFBooleanTrue: Ref;
    fn CFMachPortCreateRunLoopSource(allocator: Ref, port: Ref, order: isize) -> Ref;
    fn CFMachPortInvalidate(port: Ref);
    fn CFRunLoopGetCurrent() -> Ref;
    fn CFRunLoopAddSource(runloop: Ref, source: Ref, mode: Ref);
    fn CFRunLoopRemoveSource(runloop: Ref, source: Ref, mode: Ref);
    fn CFRunLoopRunInMode(mode: Ref, seconds: f64, return_after_source: bool) -> i32;
    fn CFDictionaryGetValue(dict: Ref, key: Ref) -> Ref;
    fn CFRelease(value: Ref);
}
struct Reservations(Vec<Ref>);
impl Reservations {
    fn new(keys: impl Iterator<Item = Shortcut>) -> Result<Self, String> {
        if MainThreadMarker::new().is_none() {
            return Err("快捷键必须在主线程注册。".into());
        }
        let mut handles = Self(Vec::new());
        for (index, key) in keys.enumerate() {
            let modifiers = u32::from(key.modifiers & COMMAND != 0) * (1 << 8)
                | u32::from(key.modifiers & SHIFT != 0) * (1 << 9)
                | u32::from(key.modifiers & ALT != 0) * (1 << 11)
                | u32::from(key.modifiers & CONTROL != 0) * (1 << 12);
            let mut handle = ptr::null();
            // SAFETY: main-thread-only Carbon calls, valid output pointer, exclusive reservation.
            let result = unsafe {
                RegisterEventHotKey(
                    u32::from(key.key),
                    modifiers,
                    HotkeyId { signature: u32::from_be_bytes(*b"SJvc"), id: index as u32 },
                    GetApplicationEventTarget(),
                    1,
                    &mut handle,
                )
            };
            if result != 0 {
                return Err(format!("快捷键 {} 无法注册或已被占用（{result}）。", key.label()));
            }
            handles.0.push(handle);
        }
        Ok(handles)
    }
}
impl Drop for Reservations {
    fn drop(&mut self) {
        // SAFETY: Reservations never leave the main thread; each returned handle is owned once.
        for handle in self.0.drain(..) {
            unsafe {
                UnregisterEventHotKey(handle);
            }
        }
    }
}
struct Observer {
    center: Retained<AnyObject>,
    token: Retained<AnyObject>,
}
impl Observer {
    fn new(
        center: *mut AnyObject,
        name: &str,
        unsafe_state: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        // SAFETY: framework-provided Objective-C objects; center copies the block. No UI access
        // occurs in notifications, which can arrive from a different thread.
        unsafe {
            let center = Retained::retain(center).ok_or("无法监听系统休眠状态。")?;
            let callback = RcBlock::new(move |_: *mut AnyObject| {
                unsafe_state.store(true, Ordering::SeqCst);
            });
            let token: *mut AnyObject = msg_send![&*center, addObserverForName: &*NSString::from_str(name), object: ptr::null::<AnyObject>(), queue: ptr::null::<AnyObject>(), usingBlock: &*callback];
            let token = Retained::retain(token).ok_or("无法监听系统安全状态。")?;
            Ok(Self { center, token })
        }
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        // SAFETY: retained center/token pair is removed before either reference is released.
        unsafe {
            let _: () = msg_send![&*self.center, removeObserver: &*self.token];
        }
    }
}
pub(super) struct Listener {
    pub config: Config,
    rx: Receiver<(Input, Instant)>,
    stop: Arc<AtomicBool>,
    fault: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    _reservations: Reservations,
    _observers: Vec<Observer>,
}
impl Listener {
    pub(super) fn check(config: &Config, current: Option<&Self>) -> Result<(), String> {
        config.validate()?;
        if !config.enabled {
            return Ok(());
        }
        if SystemPermissions.query(Permission::InputMonitoring) != PermissionStatus::Granted {
            return Err("请先在 macOS 隐私与安全性中允许输入监控。".into());
        }
        let old = current.map(|l| [l.config.dictation, l.config.chat]).unwrap_or_default();
        let _probe = Reservations::new(
            [config.dictation, config.chat]
                .into_iter()
                .flatten()
                .filter(|key| !old.contains(&Some(*key))),
        )?;
        Ok(())
    }
    pub(super) fn new(config: Config) -> Result<Self, String> {
        Self::check(&config, None)?;
        let reservations =
            Reservations::new([config.dictation, config.chat].into_iter().flatten())?;
        let stop = Arc::new(AtomicBool::new(false));
        let fault = Arc::new(AtomicBool::new(false));
        let mut observers = Vec::new();
        // SAFETY: checked classes exist only on macOS. NSString names are NSWorkspace / loginwindow
        // notification names; observation never requests or modifies OS permissions.
        unsafe {
            let workspace_class = AnyClass::get(c"NSWorkspace").ok_or("NSWorkspace 不可用。")?;
            let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
            let center: *mut AnyObject = msg_send![workspace, notificationCenter];
            observers.push(Observer::new(
                center,
                "NSWorkspaceWillSleepNotification",
                fault.clone(),
            )?);
            observers.push(Observer::new(
                center,
                "NSWorkspaceSessionDidResignActiveNotification",
                fault.clone(),
            )?);
            let distributed =
                AnyClass::get(c"NSDistributedNotificationCenter").ok_or("系统锁屏通知不可用。")?;
            let center: *mut AnyObject = msg_send![distributed, defaultCenter];
            observers.push(Observer::new(center, "com.apple.screenIsLocked", fault.clone())?);
        }
        let (tx, rx) = mpsc::sync_channel(32);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker_stop = stop.clone();
        let worker_fault = fault.clone();
        let worker_config = config.clone();
        let worker = thread::Builder::new()
            .name("voice-hotkeys".into())
            .spawn(move || {
                run(worker_config, tx, worker_stop, worker_fault, ready_tx);
            })
            .map_err(|_| "无法启动全局快捷键监听。")?;
        let mut listener = Self {
            config,
            rx,
            stop,
            fault,
            worker: Some(worker),
            _reservations: reservations,
            _observers: observers,
        };
        if !ready_rx.recv_timeout(Duration::from_secs(2)).unwrap_or(false) {
            listener.stop.store(true, Ordering::SeqCst);
            if let Some(worker) = listener.worker.take() {
                let _ = worker.join();
            }
            return Err("无法创建输入监控监听；请检查 macOS 权限后重新启用。".into());
        }
        Ok(listener)
    }
    pub(super) fn poll(&self) -> Vec<(Input, Instant)> {
        if self.fault.load(Ordering::SeqCst) {
            return vec![(Input::Unsafe, Instant::now())];
        }
        self.rx.try_iter().collect()
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
struct Context {
    config: Config,
    tx: SyncSender<(Input, Instant)>,
    fault: Arc<AtomicBool>,
    dictation_down: bool,
    chat_down: bool,
}
impl Context {
    fn send(&self, event: Input) {
        if self.tx.try_send((event, Instant::now())).is_err() {
            self.fault.store(true, Ordering::SeqCst);
        }
    }
}
unsafe extern "C" fn callback(_: Ref, kind: u32, event: Ref, data: *mut c_void) -> Ref {
    // SAFETY: run owns Context for the lifetime of this source and invokes it on the same thread.
    let context = unsafe { &mut *data.cast::<Context>() };
    if kind >= u32::MAX - 1 {
        context.fault.store(true, Ordering::SeqCst);
        return event;
    }
    if event.is_null() {
        return event;
    }
    // Inspect only key code/flags/repeat. Never read Unicode, text, or unrelated key payloads.
    let key = unsafe { CGEventGetIntegerValueField(event, 9) } as u16;
    let raw_flags = unsafe { CGEventGetFlags(event) } as u32;
    if raw_flags & (1 << 23) != 0 {
        return event;
    } // Fn is deliberately unsupported.
    let flags = raw_flags & MODIFIERS;
    let repeat = unsafe { CGEventGetIntegerValueField(event, 8) } != 0;
    if kind == 10 && key == 53 {
        context.send(Input::Escape);
        return event;
    }
    for (shortcut, is_chat) in [(context.config.dictation, false), (context.config.chat, true)] {
        let Some(shortcut) = shortcut else {
            continue;
        };
        let was_down = if is_chat { context.chat_down } else { context.dictation_down };
        if kind == 10 && key == shortcut.key && flags == shortcut.modifiers {
            if is_chat {
                context.chat_down = true;
                context.send(Input::Chat(repeat || was_down));
            } else {
                context.dictation_down = true;
                context.send(Input::Press(repeat || was_down));
            }
        } else if kind == 11 && key == shortcut.key && was_down {
            if is_chat {
                context.chat_down = false;
            } else {
                context.dictation_down = false;
                context.send(Input::Release);
            }
        } else if kind == 12 && was_down && flags != shortcut.modifiers {
            // Releasing any part of the chord is a normal release, even if the
            // letter's key-up arrives later. Do not disable the listener for it.
            if is_chat {
                context.chat_down = false;
            } else {
                context.dictation_down = false;
                context.send(Input::Release);
            }
        }
    }
    event
}
fn console_active() -> bool {
    // SAFETY: copy-returned CF dictionary is released once; NSString and CFString are toll-free.
    unsafe {
        if IsSecureEventInputEnabled() {
            return false;
        }
        let dict = CGSessionCopyCurrentDictionary();
        if dict.is_null() {
            return false;
        }
        let key = NSString::from_str("kCGSSessionOnConsoleKey");
        let active =
            CFDictionaryGetValue(dict, (&*key as *const NSString).cast()) == kCFBooleanTrue;
        CFRelease(dict);
        active
    }
}
fn run(
    config: Config,
    tx: SyncSender<(Input, Instant)>,
    stop: Arc<AtomicBool>,
    fault: Arc<AtomicBool>,
    ready: SyncSender<bool>,
) {
    let mut context =
        Context { config, tx, fault: fault.clone(), dictation_down: false, chat_down: false };
    // SAFETY: session (not privileged HID) listen-only tap. Context outlives callback/source;
    // all CF objects are created, invalidated and released on this worker thread.
    unsafe {
        let port = CGEventTapCreate(
            1,
            0,
            1,
            (1 << 10) | (1 << 11) | (1 << 12),
            callback,
            (&mut context as *mut Context).cast(),
        );
        if port.is_null() {
            let _ = ready.send(false);
            return;
        }
        let source = CFMachPortCreateRunLoopSource(ptr::null(), port, 0);
        if source.is_null() {
            CFMachPortInvalidate(port);
            CFRelease(port);
            let _ = ready.send(false);
            return;
        }
        let runloop = CFRunLoopGetCurrent();
        CFRunLoopAddSource(runloop, source, kCFRunLoopDefaultMode);
        let _ = ready.send(true);
        let mut last = SystemTime::now();
        while !stop.load(Ordering::SeqCst) && !fault.load(Ordering::SeqCst) {
            CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.05, false);
            let elapsed = last.elapsed().unwrap_or(Duration::MAX);
            last = SystemTime::now();
            let missing_up = (context.dictation_down
                && context.config.dictation.is_some_and(|key| !CGEventSourceKeyState(1, key.key)))
                || (context.chat_down
                    && context.config.chat.is_some_and(|key| !CGEventSourceKeyState(1, key.key)));
            if elapsed > Duration::from_secs(1)
                || missing_up
                || !console_active()
                || SystemPermissions.query(Permission::InputMonitoring) != PermissionStatus::Granted
            {
                fault.store(true, Ordering::SeqCst);
            }
        }
        CFRunLoopRemoveSource(runloop, source, kCFRunLoopDefaultMode);
        CFMachPortInvalidate(port);
        CFRelease(source);
        CFRelease(port);
    }
}

#[cfg(test)]
#[path = "hotkeys_macos_tests.rs"]
mod tests;
