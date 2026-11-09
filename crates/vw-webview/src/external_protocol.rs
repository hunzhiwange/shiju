use std::cell::RefCell;
use std::rc::Rc;

use url::Url;

#[cfg(all(not(coverage), target_os = "macos"))]
#[path = "external_protocol/macos.rs"]
mod platform;
#[cfg(all(not(coverage), target_os = "windows"))]
#[path = "external_protocol/windows.rs"]
mod platform;
#[cfg(all(not(coverage), unix, not(target_os = "macos")))]
#[path = "external_protocol/unix.rs"]
mod platform;

#[cfg(all(not(coverage), target_os = "windows"))]
pub(crate) use platform::block_native_external_launches;

pub(crate) struct ExternalTarget(Url);

impl ExternalTarget {
    pub(crate) fn parse(input: &str) -> Option<Self> {
        // The URL parser can strip raw controls. Reject them and encoded ASCII
        // controls before any normalization, including mail header delimiters.
        if input.chars().any(|ch| ch.is_control() || ch.is_whitespace() || ch == '\\') {
            return None;
        }
        for (index, byte) in input.bytes().enumerate() {
            if byte == b'%' {
                let hex = input.get(index + 1..index + 3)?;
                if u8::from_str_radix(hex, 16).ok()?.is_ascii_control() {
                    return None;
                }
            }
        }
        let url = Url::parse(input).ok()?;
        let scheme = url.scheme();
        let payload = input.split_once(':')?.1;
        // Windows can treat a single-letter prefix as a drive-relative path.
        if !(2..=64).contains(&scheme.len())
            || matches!(payload, "" | "//")
            || matches!(
                scheme,
                "http"
                    | "https"
                    | "about"
                    | "blob"
                    | "data"
                    | "file"
                    | "filesystem"
                    | "javascript"
                    | "vbscript"
                    | "view-source"
                    | "devtools"
                    | "resource"
                    | "jar"
                    | "wyciwyg"
                    | "chrome"
                    | "edge"
                    | "opera"
                    | "safari"
                    | "brave"
                    | "vivaldi"
                    | "firefox"
                    | "ftp"
                    | "ws"
                    | "wss"
                    | "vibe"
                    | "vibe-mcp"
                    | "shell"
                    | "search"
                    | "search-ms"
                    | "smb"
                    | "afp"
                    | "nfs"
                    | "x-man-page"
                    | "x-scheme-handler"
                    | "intent"
                    | "powershell"
                    | "cmd"
                    | "mshta"
                    | "msdt"
                    | "x-callback-url"
            )
            || [
                "chrome-",
                "edge-",
                "moz-",
                "safari-",
                "webkit-",
                "ms-",
                "microsoft-",
                "windows-",
                "x-ms-",
                "apple-",
                "x-apple",
                "com.apple.",
            ]
            .iter()
            .any(|prefix| scheme.starts_with(prefix))
        {
            return None;
        }
        Some(Self(url))
    }

    pub(crate) fn scheme(&self) -> &str {
        self.0.scheme()
    }
}

pub(crate) enum Navigation {
    InWebView,
    External(ExternalTarget),
    Blocked,
}

pub(crate) fn navigation(url: &str, local_site: bool) -> Navigation {
    if crate::is_page_link_url(url)
        || matches!(url, "about:blank" | "about:srcdoc")
        || (local_site && url.starts_with("vibe://localhost/"))
    {
        Navigation::InWebView
    } else {
        ExternalTarget::parse(url).map_or(Navigation::Blocked, Navigation::External)
    }
}

#[derive(Clone, Default)]
pub(crate) struct ExternalProtocols(Rc<RefCell<Pending>>);

#[derive(Default)]
enum Pending {
    #[default]
    Idle,
    Queued(ExternalTarget, Option<u64>),
    Confirming,
}

impl ExternalProtocols {
    pub(crate) fn queue(&self, target: ExternalTarget, popup: Option<u64>) -> bool {
        let mut pending = self.0.borrow_mut();
        if !matches!(*pending, Pending::Idle) {
            return false;
        }
        *pending = Pending::Queued(target, popup);
        true
    }

    #[cfg(not(coverage))]
    pub(crate) fn request(
        &self,
        target: ExternalTarget,
        popup: Option<u64>,
        proxy: &tao::event_loop::EventLoopProxy<crate::UserEvent>,
    ) {
        if self.queue(target, popup) {
            let _ = proxy.send_event(crate::UserEvent::ConfirmExternal);
        }
    }

    pub(crate) fn take(&self) -> Option<(ExternalTarget, Option<u64>, Confirmation)> {
        let mut pending = self.0.borrow_mut();
        match std::mem::replace(&mut *pending, Pending::Confirming) {
            Pending::Queued(target, popup) => Some((target, popup, Confirmation(self.clone()))),
            previous => {
                *pending = previous;
                None
            }
        }
    }
}

pub(crate) struct Confirmation(ExternalProtocols);

impl Drop for Confirmation {
    fn drop(&mut self) {
        let Self(ExternalProtocols(pending)) = self;
        *pending.borrow_mut() = Pending::Idle;
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Opened,
    Cancelled,
    Unavailable,
}

pub(crate) fn hand_off(
    target: ExternalTarget,
    approved: bool,
    open: impl FnOnce(&str, &str) -> bool,
) -> Outcome {
    if !approved {
        Outcome::Cancelled
    } else if open(target.scheme(), target.0.as_str()) {
        Outcome::Opened
    } else {
        Outcome::Unavailable
    }
}

#[cfg(not(coverage))]
pub(crate) fn dialog_parent(webview: &wry::WebView) -> Option<platform::Parent> {
    platform::Parent::from_webview(webview)
}

#[cfg(not(coverage))]
pub(crate) fn main_dialog_parent(window: &tao::window::Window) -> Option<platform::Parent> {
    platform::Parent::from_window(window)
}

#[cfg(not(coverage))]
pub(crate) fn show_confirmation(
    parent: platform::Parent,
    target: ExternalTarget,
    confirmation: Confirmation,
) {
    let message = format!("是否使用系统应用打开 {}: 链接？取消后保留当前页面。", target.scheme());
    let result_parent = parent.clone();
    parent.show("打开外部应用", &message, true, move |approved| {
        let outcome = hand_off(target, approved, |scheme, url| result_parent.open(scheme, url));
        if outcome == Outcome::Unavailable {
            result_parent.show(
                "无法打开外部应用",
                "系统没有可用的应用处理此协议，或应用启动失败。当前页面未更改。",
                false,
                move |_| drop(confirmation),
            );
        }
    });
}

#[cfg(not(coverage))]
pub(crate) fn show_browser_open_error(window: &tao::window::Window) {
    if let Some(parent) = main_dialog_parent(window) {
        parent.show("无法打开外部浏览器", "系统未能打开此网页。", false, |_| {});
    }
}

#[cfg(test)]
#[path = "external_protocol_tests.rs"]
mod tests;
