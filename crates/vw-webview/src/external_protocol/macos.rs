use std::cell::Cell;

use block2::RcBlock;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSAlert, NSAlertSecondButtonReturn, NSWindow, NSWorkspace};
use objc2_foundation::{NSString, NSURL};
use tao::platform::macos::WindowExtMacOS;
use wry::{WebView, WebViewExtMacOS};

#[derive(Clone)]
pub(crate) struct Parent(Retained<NSWindow>);

impl Parent {
    pub(super) fn from_window(window: &tao::window::Window) -> Option<Self> {
        // Tao owns this NSWindow on the current UI thread.
        let window = unsafe { Retained::retain(window.ns_window().cast::<NSWindow>()) }?;
        window.isVisible().then_some(Self(window))
    }

    pub(super) fn from_webview(webview: &WebView) -> Option<Self> {
        let window = webview.webview().window()?;
        window.isVisible().then_some(Self(window))
    }

    pub(super) fn show(
        &self,
        title: &str,
        message: &str,
        confirm: bool,
        done: impl FnOnce(bool) + 'static,
    ) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let alert = NSAlert::new(mtm);
        alert.setMessageText(&NSString::from_str(title));
        alert.setInformativeText(&NSString::from_str(message));
        if confirm {
            // Escape cancels regardless of the system language.
            let cancel = alert.addButtonWithTitle(&NSString::from_str("取消"));
            cancel.setKeyEquivalent(&NSString::from_str("\u{1b}"));
            let open = alert.addButtonWithTitle(&NSString::from_str("打开"));
            open.setKeyEquivalent(&NSString::from_str(""));
        } else {
            alert.addButtonWithTitle(&NSString::from_str("好"));
        }
        let done = Cell::new(Some(done));
        let completion = RcBlock::new(move |response| {
            if let Some(done) = done.take() {
                done(confirm && response == NSAlertSecondButtonReturn);
            }
        });
        alert.beginSheetModalForWindow_completionHandler(&self.0, Some(&completion));
    }

    pub(super) fn open(&self, _scheme: &str, url: &str) -> bool {
        let Some(url) = NSURL::URLWithString(&NSString::from_str(url)) else { return false };
        let workspace = NSWorkspace::sharedWorkspace();
        workspace.URLForApplicationToOpenURL(&url).is_some() && workspace.openURL(&url)
    }
}
