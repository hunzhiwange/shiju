//! ScreenCaptureKit single-window snapshots; never a display stream or a disk file.
use super::permissions::{Permission, PermissionBackend, PermissionStatus, SystemPermissions};
use super::screen_context::{Target, Ticket};
use block2::RcBlock;
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, Bool},
};
use objc2_foundation::NSString;
use std::{ffi::c_void, sync::mpsc, time::Duration};

type Ref = *const c_void;
unsafe extern "C" {
    fn dlopen(path: *const std::ffi::c_char, mode: i32) -> *mut c_void;
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(options: u32, relative: u32) -> *mut AnyObject;
    fn CGImageGetWidth(image: Ref) -> usize;
    fn CGImageGetHeight(image: Ref) -> usize;
    fn CGImageGetBytesPerRow(image: Ref) -> usize;
    fn CGImageGetBitsPerPixel(image: Ref) -> usize;
    fn CGImageGetBitsPerComponent(image: Ref) -> usize;
    fn CGImageGetBitmapInfo(image: Ref) -> u32;
    fn CGImageGetDataProvider(image: Ref) -> Ref;
    fn CGDataProviderCopyData(provider: Ref) -> Ref;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFDataGetLength(data: Ref) -> isize;
    fn CFDataGetBytePtr(data: Ref) -> *const u8;
    fn CFRelease(value: Ref);
}
fn permitted() -> bool {
    SystemPermissions.query(Permission::ScreenRecording) == PermissionStatus::Granted
}
unsafe fn class(name: &'static std::ffi::CStr) -> Result<&'static AnyClass, String> {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let available = AVAILABLE.get_or_init(|| {
        // Load only this fixed system path after a sharing action. Keep the handle
        // for the process lifetime: SCK owns asynchronous callbacks and ObjC classes.
        // A hard framework link would prevent older supported desktops from starting.
        unsafe {
            !dlopen(
                c"/System/Library/Frameworks/ScreenCaptureKit.framework/ScreenCaptureKit".as_ptr(),
                1,
            )
            .is_null()
                && AnyClass::get(c"SCScreenshotManager").is_some()
        }
    });
    if !available {
        return Err("屏幕分享需要 macOS 14 或更新版本。".into());
    }
    AnyClass::get(name).ok_or_else(|| "无法读取系统屏幕分享能力。".into())
}
unsafe fn string(object: *mut AnyObject) -> String {
    if object.is_null() {
        return String::new();
    }
    // SAFETY: caller supplies NSString properties from the SDK.
    unsafe { (&*(object as *const NSString)).to_string() }
}
/// Metadata query only. Do not ask for permission or acquire pixels here.
pub(super) fn foreground() -> Result<Target, String> {
    if !permitted() {
        return Err("请先允许屏幕录制；未采集任何屏幕内容。".into());
    }
    // SAFETY: AppKit properties are queried on the UI thread; CG returns a +1 CFArray,
    // toll-free bridged to NSArray, and dictionary values are borrowed until it drops.
    unsafe {
        let workspace: *mut AnyObject = msg_send![class(c"NSWorkspace")?, sharedWorkspace];
        let app: *mut AnyObject = msg_send![workspace, frontmostApplication];
        if app.is_null() {
            return Err("没有前台应用可供分享。".into());
        }
        let pid: i32 = msg_send![app, processIdentifier];
        let app_name: *mut AnyObject = msg_send![app, localizedName];
        let app_name = string(app_name);
        let windows = Retained::from_raw(CGWindowListCopyWindowInfo(1 | 16, 0))
            .ok_or("无法读取前台窗口。")?;
        let count: usize = msg_send![&*windows, count];
        for index in 0..count {
            let info: *mut AnyObject = msg_send![&*windows, objectAtIndex: index];
            let get = |key: &str| -> *mut AnyObject {
                let key = NSString::from_str(key);
                msg_send![info, objectForKey: &*key]
            };
            let owner = get("kCGWindowOwnerPID");
            let layer = get("kCGWindowLayer");
            let number = get("kCGWindowNumber");
            if owner.is_null() || layer.is_null() || number.is_null() {
                continue;
            }
            let owner: i32 = msg_send![owner, intValue];
            let layer: i32 = msg_send![layer, intValue];
            if owner == pid && layer == 0 {
                let window: u32 = msg_send![number, unsignedIntValue];
                let title = string(get("kCGWindowName"));
                let label = format!(
                    "{app_name} · {} · 窗口 {window}",
                    if title.is_empty() { "无标题" } else { &title }
                );
                return Ok(Target { pid, window, label });
            }
        }
    }
    Err("前台应用没有可分享的普通窗口。".into())
}
/// Called on a blocking worker. Asynchronous callbacks copy only this bounded frame.
pub(super) fn capture(ticket: &Ticket) -> Result<Vec<u8>, String> {
    ticket.capture(|target| {
        if !permitted() { return Err("屏幕录制权限不可用。".into()); }
        let (tx, rx) = mpsc::sync_channel(1);
        let target = target.clone();
        let ticket = ticket.clone();
        // SAFETY: SCK copies the blocks; captured values are owned. Window/filter/config
        // stay live through scheduling. The image is copied inside its callback lifetime.
        unsafe {
            let manager = class(c"SCScreenshotManager")?;
            let content = class(c"SCShareableContent")?;
            let callback = RcBlock::new(move |content: *mut AnyObject, error: *mut AnyObject| {
                let result = (|| {
                    if !error.is_null() || content.is_null() || !ticket.is_live() || !permitted() {
                        return Err("屏幕分享已取消或权限不可用。".into());
                    }
                    let windows: *mut AnyObject = msg_send![content, windows];
                    let count: usize = msg_send![windows, count];
                    let mut selected = std::ptr::null_mut();
                    for index in 0..count {
                        let window: *mut AnyObject = msg_send![windows, objectAtIndex: index];
                        let id: u32 = msg_send![window, windowID];
                        let app: *mut AnyObject = msg_send![window, owningApplication];
                        if app.is_null() { continue; }
                        let pid: i32 = msg_send![app, processID];
                        if id == target.window && pid == target.pid { selected = window; break; }
                    }
                    if selected.is_null() { return Err("已选择的窗口已关闭。".into()); }
                    let filter: *mut AnyObject = msg_send![class(c"SCContentFilter")?, alloc];
                    let filter: *mut AnyObject = msg_send![filter, initWithDesktopIndependentWindow: selected];
                    let filter = Retained::from_raw(filter).ok_or("无法创建窗口过滤器。")?;
                    let config: Retained<AnyObject> = msg_send![class(c"SCStreamConfiguration")?, new];
                    let _: () = msg_send![&*config, setPixelFormat: 0x42475241u32];
                    let _: () = msg_send![&*config, setWidth: 1024usize];
                    let _: () = msg_send![&*config, setHeight: 1024usize];
                    let _: () = msg_send![&*config, setScalesToFit: Bool::YES];
                    let _: () = msg_send![&*config, setShowsCursor: Bool::NO];
                    let _: () = msg_send![&*config, setIgnoreShadowsSingleWindow: Bool::YES];
                    // Explicitly exclude child windows on systems exposing this selector.
                    let selector = objc2::sel!(setIncludeChildWindows:);
                    let supported: Bool = msg_send![&*config, respondsToSelector: selector];
                    if supported.as_bool() { let _: () = msg_send![&*config, setIncludeChildWindows: Bool::NO]; }
                    let send = tx.clone();
                    let live = ticket.clone();
                    let image_callback = RcBlock::new(move |image: Ref, error: *mut AnyObject| {
                        let result = if !error.is_null() || image.is_null() || !live.is_live() || !permitted() {
                            Err("屏幕分享已取消或采集失败。".into())
                        } else { copy_image(image) };
                        let _ = send.try_send(result);
                    });
                    if !ticket.is_live() || !permitted() { return Err("屏幕分享已取消。".into()); }
                    let _: () = msg_send![manager, captureImageWithFilter: &*filter, configuration: &*config, completionHandler: &*image_callback];
                    Ok(())
                })();
                if let Err(error) = result { let _ = tx.try_send(Err(error)); }
            });
            let _: () = msg_send![content, getShareableContentExcludingDesktopWindows: Bool::YES, onScreenWindowsOnly: Bool::YES, completionHandler: &*callback];
        }
        rx.recv_timeout(Duration::from_secs(10)).map_err(|_| "屏幕分享超时。".to_owned())?
    })
}
unsafe fn copy_image(image: Ref) -> Result<Vec<u8>, String> {
    // SAFETY: CGImage is borrowed for this callback; CFData is +1 and released below.
    unsafe {
        let w = CGImageGetWidth(image);
        let h = CGImageGetHeight(image);
        let row = CGImageGetBytesPerRow(image);
        let bitmap = CGImageGetBitmapInfo(image);
        if w == 0
            || h == 0
            || w > 1024
            || h > 1024
            || row < w * 4
            || row > 8192
            || CGImageGetBitsPerComponent(image) != 8
            || CGImageGetBitsPerPixel(image) != 32
            || !matches!(bitmap & 0x7000, 0 | 0x2000)
        {
            return Err("不支持系统返回的屏幕像素格式。".into());
        }
        let provider = CGImageGetDataProvider(image);
        if provider.is_null() {
            return Err("屏幕帧没有可读像素。".into());
        }
        let data = CGDataProviderCopyData(provider);
        if data.is_null() {
            return Err("无法读取屏幕帧。".into());
        }
        let len = CFDataGetLength(data);
        if len < (row * h) as isize {
            CFRelease(data);
            return Err("屏幕帧不完整。".into());
        }
        let bytes = std::slice::from_raw_parts(CFDataGetBytePtr(data), row * h);
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let p = &bytes[y * row + x * 4..][..4];
                if bitmap & 0x7000 == 0x2000 {
                    rgba.extend_from_slice(&[p[2], p[1], p[0], 255]);
                } else {
                    rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
                }
            }
        }
        CFRelease(data);
        super::screen_context::encode(w as u32, h as u32, rgba)
    }
}
