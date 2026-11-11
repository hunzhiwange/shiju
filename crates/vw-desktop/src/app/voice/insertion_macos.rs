//! macOS AX selected-text replacement; never synthesizes keys or touches the pasteboard.
use super::insertion::{Backend, Snapshot, Target};
use super::permissions::{Permission, PermissionBackend, PermissionStatus, SystemPermissions};
use objc2::{
    msg_send,
    runtime::{AnyClass, AnyObject},
};
use std::{ffi::c_void, ptr};

type Ref = *const c_void;
#[repr(C)]
#[derive(Default)]
struct Range {
    location: isize,
    length: isize,
}
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> Ref;
    fn AXUIElementCopyAttributeValue(element: Ref, attribute: Ref, value: *mut Ref) -> i32;
    fn AXUIElementIsAttributeSettable(element: Ref, attribute: Ref, settable: *mut u8) -> i32;
    fn AXUIElementSetAttributeValue(element: Ref, attribute: Ref, value: Ref) -> i32;
    fn AXUIElementGetPid(element: Ref, pid: *mut i32) -> i32;
    fn AXUIElementSetMessagingTimeout(element: Ref, seconds: f32) -> i32;
    fn AXUIElementGetTypeID() -> usize;
    fn AXValueGetTypeID() -> usize;
    fn AXValueGetValue(value: Ref, kind: u32, result: *mut c_void) -> bool;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: Ref);
    fn CFEqual(a: Ref, b: Ref) -> u8;
    fn CFGetTypeID(value: Ref) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFBooleanGetValue(value: Ref) -> u8;
    fn CFStringCreateWithCharacters(allocator: Ref, characters: *const u16, length: isize) -> Ref;
    fn CFStringGetLength(value: Ref) -> isize;
    fn CFStringGetCharacters(value: Ref, range: Range, buffer: *mut u16);
}
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn IsSecureEventInputEnabled() -> bool;
}

struct Owned(Ref);
impl Owned {
    fn new(value: Ref) -> Result<Self, String> {
        if value.is_null() { Err("无法读取可编辑目标。".into()) } else { Ok(Self(value)) }
    }
    fn string(value: &str) -> Result<Self, String> {
        let units: Vec<u16> = value.encode_utf16().collect();
        // SAFETY: CF copies the valid UTF-16 buffer. +1 is released by Owned.
        Self::new(unsafe {
            CFStringCreateWithCharacters(ptr::null(), units.as_ptr(), units.len() as isize)
        })
    }
    fn attribute(&self, name: &str) -> Result<Self, String> {
        let attribute = Self::string(name)?;
        let mut value = ptr::null();
        // SAFETY: self/attribute are live CF objects; copy returns a retained object.
        let code = unsafe { AXUIElementCopyAttributeValue(self.0, attribute.0, &mut value) };
        if code != 0 {
            return Err(format!("无法核验目标属性 {name}（系统错误 {code}）。"));
        }
        Self::new(value)
    }
    fn subrole(&self) -> Result<String, String> {
        let name = Self::string("AXSubrole")?;
        let mut value = ptr::null();
        // AXSubrole is optional for ordinary text controls (SDK AXAttributeConstants).
        // Only documented unsupported/no-value codes mean absence; all other failures close.
        match unsafe { AXUIElementCopyAttributeValue(self.0, name.0, &mut value) } {
            0 => Self::new(value)?.text(),
            -25205 | -25212 => Ok(String::new()),
            _ => Err("无法核验目标的安全字段类型。".into()),
        }
    }
    fn text(&self) -> Result<String, String> {
        // SAFETY: Check the runtime type before calling CFString functions.
        unsafe {
            if CFGetTypeID(self.0) != CFStringGetTypeID() {
                return Err("目标文本类型无效。".into());
            }
            let length = CFStringGetLength(self.0);
            if !(0..=1_048_576).contains(&length) {
                return Err("目标文本过长，无法安全核验。".into());
            }
            let mut units = vec![0; length as usize];
            CFStringGetCharacters(self.0, Range { location: 0, length }, units.as_mut_ptr());
            String::from_utf16(&units).map_err(|_| "目标文本不是有效 Unicode。".into())
        }
    }
    fn element(&self, name: &str) -> Result<Self, String> {
        let value = self.attribute(name)?;
        // SAFETY: CF type queries are valid on any live CF object.
        if unsafe { CFGetTypeID(value.0) != AXUIElementGetTypeID() } {
            return Err("目标辅助功能元素类型无效。".into());
        }
        Ok(value)
    }
    fn settable(&self, name: &str) -> Result<bool, String> {
        let name = Self::string(name)?;
        let mut yes = 0;
        // SAFETY: The out pointer and both AX/CF objects live for the call.
        if unsafe { AXUIElementIsAttributeSettable(self.0, name.0, &mut yes) } != 0 {
            return Err("无法验证目标是否允许选择区写入。".into());
        }
        Ok(yes != 0)
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.0);
        }
    }
}
// AX references are CF objects, used synchronously by the UI lifecycle, without callbacks.
// App can move between executor setup and event handling; ownership remains exclusive.
unsafe impl Send for Owned {}
unsafe impl Sync for Owned {}

pub(crate) struct NativeBackend {
    application: Owned,
    window: Owned,
    element: Owned,
    pid: i32,
}
fn frontmost_pid() -> Result<i32, String> {
    // SAFETY: AppKit is linked by the desktop; checked class and nil receiver are handled.
    unsafe {
        let class = AnyClass::get(c"NSWorkspace").ok_or("无法读取前台应用。")?;
        let workspace: *mut AnyObject = msg_send![class, sharedWorkspace];
        let application: *mut AnyObject = msg_send![workspace, frontmostApplication];
        if application.is_null() {
            return Err("没有前台应用。".into());
        }
        Ok(msg_send![application, processIdentifier])
    }
}
pub(crate) fn capture() -> Result<Option<Target<NativeBackend>>, String> {
    let pid = frontmost_pid()?;
    if pid == std::process::id() as i32 {
        return Ok(None);
    }
    authorized()?;
    // SAFETY: Create returns +1; no user data is read before authorization/secure-input checks.
    let application = Owned::new(unsafe { AXUIElementCreateApplication(pid) })?;
    unsafe {
        AXUIElementSetMessagingTimeout(application.0, 0.25);
    }
    let window = application.element("AXFocusedWindow")?;
    let element = application.element("AXFocusedUIElement")?;
    let backend = NativeBackend { application, window, element, pid };
    Target::capture(backend).map(Some)
}
fn authorized() -> Result<(), String> {
    if SystemPermissions.query(Permission::Accessibility) != PermissionStatus::Granted {
        return Err("请在跨应用听写试用中检查辅助功能授权。".into());
    }
    // SAFETY: This is a non-prompting system query.
    if unsafe { IsSecureEventInputEnabled() } {
        return Err("安全输入已开启，拒绝跨应用听写。".into());
    }
    Ok(())
}
impl Backend for NativeBackend {
    fn snapshot(&self) -> Result<Snapshot, String> {
        authorized()?;
        if frontmost_pid()? != self.pid {
            return Err("前台应用已变化。".into());
        }
        let app = &self.application;
        let window = app.element("AXFocusedWindow")?;
        let element = app.element("AXFocusedUIElement")?;
        let mut pid = 0;
        // SAFETY: All references are retained and type-checked AX elements.
        if unsafe {
            AXUIElementGetPid(element.0, &mut pid) != 0
                || pid != self.pid
                || CFEqual(window.0, self.window.0) == 0
                || CFEqual(element.0, self.element.0) == 0
        } {
            return Err("听写目标、窗口或焦点已变化。".into());
        }
        let role = element.attribute("AXRole")?.text()?;
        let subrole = element.subrole()?;
        if !matches!(role.as_str(), "AXTextField" | "AXTextArea" | "AXComboBox")
            || subrole == "AXSecureTextField"
        {
            return Err("目标不是可核验的安全可编辑文本框。".into());
        }
        let enabled = element.attribute("AXEnabled")?;
        if unsafe {
            CFGetTypeID(enabled.0) != CFBooleanGetTypeID() || CFBooleanGetValue(enabled.0) == 0
        } {
            return Err("目标已禁用。".into());
        }
        // Query write capability before reading normal text. Never query a password value.
        let editable = element.settable("AXSelectedText")?;
        let range = element.attribute("AXSelectedTextRange")?;
        let mut selection = Range::default();
        if unsafe {
            CFGetTypeID(range.0) != AXValueGetTypeID()
                || !AXValueGetValue(range.0, 4, (&mut selection as *mut Range).cast())
        } {
            return Err("目标不支持可核验的选择区。".into());
        }
        let value = element.attribute("AXValue")?.text()?;
        Ok(Snapshot {
            pid,
            element: self.element.0 as usize,
            window: self.window.0 as usize,
            selection: (selection.location as i64, selection.length as i64),
            value,
            editable,
            secure: false,
        })
    }
    fn replace_selection(&self, text: &str) -> Result<(), String> {
        authorized()?;
        let attribute = Owned::string("AXSelectedText")?;
        let value = Owned::string(text)?;
        // SAFETY: Only the retained, revalidated selected-text attribute is written.
        // No focus manipulation, AXValue overwrite, Enter, clipboard or paste fallback.
        if unsafe { AXUIElementSetAttributeValue(self.element.0, attribute.0, value.0) } != 0 {
            return Err("目标拒绝选择区写入，请显式复制结果。".into());
        }
        Ok(())
    }
}
