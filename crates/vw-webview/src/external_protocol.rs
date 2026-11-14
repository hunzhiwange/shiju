use std::cell::RefCell;
use std::rc::Rc;

use webview_core::navigation::ExternalTarget;

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
    } else if open(target.scheme(), target.as_str()) {
        Outcome::Opened
    } else {
        Outcome::Unavailable
    }
}

#[cfg(all(not(coverage), unix))]
pub(crate) use platform::Parent;

#[cfg(all(not(coverage), unix))]
pub(crate) fn confirm_mcp_form(parent: &Parent, origin: &str, done: impl FnOnce(bool) + 'static) {
    parent.show(
        "提交 MCP 表单",
        &format!("是否向 {origin} 发送表单数据？提交后将显示隔离的响应页面。"),
        true,
        done,
    );
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
