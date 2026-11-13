#![allow(dead_code)]
// Test-only UI message type declarations let the unchanged hotkey module compile.
// No UI message handler is substituted or exercised. Runtime/prefs/OS code use real sources.
mod app {
    pub fn project_dirs() -> Option<directories::ProjectDirs> {
        directories::ProjectDirs::from_path(
            std::path::PathBuf::from(std::env::var_os("VIBEWINDOW_CONFIG_DIR")?).join("desktop"),
        )
    }
    #[derive(Clone)]
    pub enum Message {
        Settings(message::SettingsMessage),
    }
    pub mod message {
        #[derive(Clone)]
        pub enum SettingsMessage {
            Voice(settings::voice::Event),
        }
        pub mod settings {
            pub mod voice {
                #[derive(Clone)]
                pub enum Event {
                    RecordHotkey(Option<bool>),
                    RecordedHotkey(crate::app::voice::hotkeys::Shortcut),
                }
            }
        }
    }
    pub mod voice {
        // @VOICE_MODULES@
    }
}
use app::voice::{hotkeys::*, preferences::Preferences};
use objc2::{
    msg_send,
    runtime::{AnyClass, AnyObject},
};
use objc2_foundation::NSString;
use std::{
    ffi::c_void,
    process::{Child, Command},
    ptr,
    time::{Duration, Instant},
};
type Ref = *const c_void;
static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
fn stamp() -> u128 {
    START.get().unwrap().elapsed().as_millis()
}
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceCreate(state: i32) -> Ref;
    fn CGEventSourceSetLocalEventsSuppressionInterval(source: Ref, seconds: f64);
    fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
    fn CGEventSetFlags(event: Ref, flags: u64);
    fn CGEventSetIntegerValueField(event: Ref, field: u32, value: i64);
    fn CGEventPost(location: u32, event: Ref);
    fn CGEventSourceFlagsState(state: i32) -> u64;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFRunLoopDefaultMode: Ref;
    fn CFRunLoopRunInMode(mode: Ref, seconds: f64, once: bool) -> i32;
    fn CFRelease(value: Ref);
}
fn foreground() -> i32 {
    unsafe {
        let w: *mut AnyObject = msg_send![AnyClass::get(c"NSWorkspace").unwrap(), sharedWorkspace];
        let a: *mut AnyObject = msg_send![w, frontmostApplication];
        msg_send![a, processIdentifier]
    }
}
fn activate(pid: i32) {
    unsafe {
        let a: *mut AnyObject = msg_send![AnyClass::get(c"NSRunningApplication").unwrap(),runningApplicationWithProcessIdentifier:pid];
        if !a.is_null() {
            let _: bool = msg_send![a,activateWithOptions:2_usize];
        }
    }
}
struct Target {
    child: Child,
    previous: i32,
    source: Ref,
    held: std::collections::BTreeSet<u16>,
}
impl Target {
    fn new() -> Self {
        assert_ne!(
            unsafe { AXIsProcessTrusted() },
            0,
            "OS injection requires already-granted accessibility; no grant is requested"
        );
        assert_eq!(
            unsafe { CGEventSourceFlagsState(1) } & u64::from(MODIFIERS),
            0,
            "user modifiers are active; refusing key injection"
        );
        let previous = foreground();
        let child =
            Command::new(std::env::var_os("SHIJU_HOTKEY_TARGET").expect("probe target path"))
                .spawn()
                .unwrap();
        let source = unsafe { CGEventSourceCreate(1) };
        assert!(!source.is_null());
        if std::env::var_os("SHIJU_HOTKEY_DEFAULT_SUPPRESSION").is_none() {
            unsafe {
                CGEventSourceSetLocalEventsSuppressionInterval(source, 0.0);
            }
        }
        let target = Self { child, previous, source, held: std::collections::BTreeSet::new() };
        let began = Instant::now();
        while foreground() != target.child.id() as i32 && began.elapsed() < Duration::from_secs(4) {
            unsafe {
                CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.02, true);
            }
        }
        assert_eq!(
            foreground(),
            target.child.id() as i32,
            "test input target did not activate; no keys will be sent"
        );
        assert_ne!(foreground(), std::process::id() as i32);
        target
    }
    fn key(&mut self, key: u16, down: bool, repeat: bool) {
        assert_eq!(
            foreground(),
            self.child.id() as i32,
            "focus changed; refusing to send to user's app"
        );
        unsafe {
            let e = CGEventCreateKeyboardEvent(
                if std::env::var_os("SHIJU_HOTKEY_NULL_SOURCE").is_some() {
                    ptr::null()
                } else {
                    self.source
                },
                key,
                down,
            );
            assert!(!e.is_null());
            CGEventSetFlags(
                e,
                if down && key != 53 { u64::from(CONTROL | ALT | COMMAND) } else { 0 },
            );
            CGEventSetIntegerValueField(e, 8, i64::from(repeat));
            eprintln!("POST ms={} key={} down={} repeat={}", stamp(), key, down, repeat);
            CGEventPost(0, e);
            CFRelease(e);
        }
        if down {
            self.held.insert(key);
        } else {
            self.held.remove(&key);
        }
    }
}
impl Drop for Target {
    fn drop(&mut self) {
        for key in std::mem::take(&mut self.held) {
            unsafe {
                let e = CGEventCreateKeyboardEvent(self.source, key, false);
                if !e.is_null() {
                    CGEventSetFlags(e, 0);
                    CGEventPost(0, e);
                    CFRelease(e);
                }
            }
        }
        let restore = foreground() == self.child.id() as i32;
        let _ = self.child.kill();
        let _ = self.child.wait();
        unsafe {
            CFRelease(self.source);
        }
        if restore {
            activate(self.previous);
        }
    }
}
fn pump(runtime: &mut Runtime, ms: u64) -> Vec<Action> {
    let began = Instant::now();
    let mut result = Vec::new();
    while began.elapsed() < Duration::from_millis(ms) {
        unsafe {
            CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.005, true);
        }
        let actions = runtime.poll();
        if !actions.is_empty() {
            eprintln!(
                "POLL ms={} actions={:?} active={}",
                stamp(),
                actions,
                runtime.machine.active
            );
        }
        result.extend(actions);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        runtime.error.is_none(),
        "native runtime stopped: {:?}; actions: {:?}",
        runtime.error,
        result
    );
    result
}
fn sleep_notification() {
    unsafe {
        let w: *mut AnyObject = msg_send![AnyClass::get(c"NSWorkspace").unwrap(), sharedWorkspace];
        let c: *mut AnyObject = msg_send![w, notificationCenter];
        let _: () = msg_send![c,postNotificationName:&*NSString::from_str("NSWorkspaceWillSleepNotification"),object:ptr::null::<AnyObject>()];
    }
}
fn run() {
    let root = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", root.path());
    }
    let path = Preferences::path().unwrap();
    let prefs = Preferences {
        hotkeys: Config {
            enabled: true,
            dictation: Some(Shortcut { key: 0, modifiers: CONTROL | ALT | COMMAND }),
            chat: Some(Shortcut { key: 8, modifiers: CONTROL | ALT | COMMAND }),
        },
        ..Preferences::default()
    };
    prefs.write(&path).unwrap();
    let mut runtime = Runtime::default();
    assert!(runtime.poll().is_empty());
    assert!(runtime.error.is_none(), "{:?}", runtime.error);
    let mut target = Target::new();
    println!("PROBE_BACKGROUND=true TARGET_OWNED=true");
    target.key(0, true, false);
    assert_eq!(pump(&mut runtime, 150), vec![Action::Start]);
    target.key(0, true, true);
    assert!(pump(&mut runtime, 180).is_empty());
    target.key(0, false, false);
    assert_eq!(pump(&mut runtime, 80), vec![Action::Finish]);
    println!("CASE hold_repeat_release PASS");
    target.key(0, true, false);
    assert_eq!(pump(&mut runtime, 35), vec![Action::Start]);
    target.key(0, false, false);
    assert!(pump(&mut runtime, 40).is_empty());
    target.key(0, true, false);
    assert!(pump(&mut runtime, 35).is_empty());
    target.key(0, false, false);
    assert!(pump(&mut runtime, 40).is_empty());
    let after = pump(&mut runtime, 320);
    assert!(after.is_empty(), "double tap timed out; inspect POST/POLL times: {after:?}");
    assert!(runtime.machine.active);
    target.key(0, true, false);
    assert_eq!(pump(&mut runtime, 40), vec![Action::Finish]);
    target.key(0, false, false);
    assert!(pump(&mut runtime, 40).is_empty());
    println!("CASE double_tap_hands_free_finish PASS");
    // Control: a second press after the window must never latch hands-free.
    target.key(0, true, false);
    assert_eq!(pump(&mut runtime, 40), vec![Action::Start]);
    target.key(0, false, false);
    assert!(pump(&mut runtime, 40).is_empty());
    assert_eq!(pump(&mut runtime, 320), vec![Action::Finish]);
    assert!(!runtime.machine.active);
    println!("CASE single_tap_timeout_control PASS");
    // Malformed injected repeat metadata reproduces the old final-wait symptom.
    // This is a diagnostic control, not evidence of what the historical OS delivered.
    target.key(0, true, false);
    assert_eq!(pump(&mut runtime, 35), vec![Action::Start]);
    target.key(0, false, false);
    assert!(pump(&mut runtime, 40).is_empty());
    target.key(0, true, true);
    assert!(pump(&mut runtime, 35).is_empty());
    target.key(0, false, false);
    assert!(pump(&mut runtime, 40).is_empty());
    assert_eq!(pump(&mut runtime, 320), vec![Action::Finish]);
    assert!(!runtime.machine.active);
    println!("CASE repeat_marked_second_press_timeout_control PASS");
    let repetitions: usize = std::env::var("SHIJU_HOTKEY_DOUBLE_TAPS").unwrap().parse().unwrap();
    assert!((1..=100).contains(&repetitions));
    for iteration in 0..repetitions {
        target.key(0, true, false);
        assert_eq!(pump(&mut runtime, 35), vec![Action::Start]);
        target.key(0, false, false);
        assert!(pump(&mut runtime, 40).is_empty());
        target.key(0, true, false);
        assert!(pump(&mut runtime, 35).is_empty());
        target.key(0, false, false);
        assert!(pump(&mut runtime, 40).is_empty());
        assert!(pump(&mut runtime, 320).is_empty(), "double tap iteration {iteration} ended early");
        assert!(runtime.machine.active);
        target.key(0, true, false);
        assert_eq!(pump(&mut runtime, 40), vec![Action::Finish]);
        target.key(0, false, false);
        assert!(pump(&mut runtime, 40).is_empty());
    }
    println!("CASE repeated_double_tap count={repetitions} PASS");
    target.key(8, true, false);
    assert_eq!(pump(&mut runtime, 60), vec![Action::Chat]);
    target.key(8, true, true);
    assert!(pump(&mut runtime, 60).is_empty());
    target.key(8, false, false);
    assert!(pump(&mut runtime, 60).is_empty());
    println!("CASE separate_chat_repeat PASS");
    target.key(0, true, false);
    assert_eq!(pump(&mut runtime, 60), vec![Action::Start]);
    target.key(53, true, false);
    assert_eq!(pump(&mut runtime, 40), vec![Action::Cancel]);
    target.key(53, false, false);
    target.key(0, false, false);
    assert!(pump(&mut runtime, 40).is_empty());
    println!("CASE escape_cancel PASS");
    runtime.stop_session();
    runtime.reload();
    assert!(pump(&mut runtime, 40).is_empty());
    target.key(0, true, false);
    assert!(pump(&mut runtime, 80).is_empty());
    target.key(0, false, false);
    assert!(pump(&mut runtime, 40).is_empty());
    runtime.resume_after_save();
    assert!(pump(&mut runtime, 60).is_empty());
    target.key(0, true, false);
    assert_eq!(pump(&mut runtime, 60), vec![Action::Start]);
    sleep_notification();
    let began = Instant::now();
    let mut cancelled = false;
    while began.elapsed() < Duration::from_millis(300) {
        unsafe {
            CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.005, true);
        }
        cancelled |= runtime.poll().contains(&Action::Cancel);
        if cancelled {
            break;
        }
    }
    assert!(cancelled);
    assert!(!runtime.machine.active);
    assert!(runtime.error.is_some());
    target.key(0, false, false);
    runtime.reload();
    assert!(runtime.poll().is_empty());
    println!("CASE disable_resume_sleep_notification_latch PASS");
    Preferences::disable_hotkeys_at(&path).unwrap();
    runtime.stop_session();
    assert!(!Preferences::read(&path).unwrap().hotkeys.enabled);
    drop(target);
    drop(runtime);
    println!("OS_RUNTIME_CASES=8 AUDIO_CAPTURED=false SERVICE_REQUESTS=0");
}

fn main() {
    START.set(Instant::now()).unwrap();
    run();
}
