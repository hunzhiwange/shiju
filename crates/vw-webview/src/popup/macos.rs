use std::path::Path;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol, ProtocolObject, Sel};
use objc2::{DeclaredClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSBackingStoreType, NSView, NSWindow, NSWindowDelegate, NSWindowStyleMask};
use objc2_foundation::{NSNotification, NSObject, NSPoint, NSRect, NSSize, NSString};
use objc2_web_kit::{WKUIDelegate, WKUserContentController, WKWebView};
use wry::raw_window_handle::{AppKitWindowHandle, HandleError, HasWindowHandle, WindowHandle};
use wry::{
    NewWindowFeatures, NewWindowResponse, WebView, WebViewBuilder, WebViewBuilderExtMacos,
    WebViewExtMacOS,
};

use crate::macos_downloads::{AttachmentNavigationDelegate, enable_attachments};
use crate::{DEFAULT_TITLE, SAFARI_USER_AGENT};

struct CloseCallbacks {
    original: Retained<ProtocolObject<dyn WKUIDelegate>>,
    close: Box<dyn Fn()>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = CloseCallbacks]
    struct PopupDelegate;

    unsafe impl NSObjectProtocol for PopupDelegate {
        #[unsafe(method(respondsToSelector:))]
        fn responds_to_selector(&self, selector: Sel) -> bool {
            let responds: bool = unsafe { msg_send![super(self), respondsToSelector: selector] };
            responds || self.ivars().original.respondsToSelector(selector)
        }
    }

    impl PopupDelegate {
        #[unsafe(method_id(forwardingTargetForSelector:))]
        fn forwarding_target(&self, _selector: Sel) -> Retained<AnyObject> {
            Retained::from(&*self.ivars().original)
        }
    }

    unsafe impl WKUIDelegate for PopupDelegate {
        #[unsafe(method(webViewDidClose:))]
        fn webview_did_close(&self, _webview: &WKWebView) {
            (self.ivars().close)();
        }
    }

    unsafe impl NSWindowDelegate for PopupDelegate {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSNotification) {
            (self.ivars().close)();
        }
    }
);

struct ContentView(Retained<NSView>);

impl HasWindowHandle for ContentView {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let view = std::ptr::NonNull::from(&*self.0).cast();
        // The view is retained for the lifetime of the returned handle.
        Ok(unsafe { WindowHandle::borrow_raw(AppKitWindowHandle::new(view).into()) })
    }
}

pub(super) struct Host {
    window: Retained<NSWindow>,
    _delegate: Retained<PopupDelegate>,
    _attachments: Option<Retained<AttachmentNavigationDelegate>>,
}

impl Host {
    pub(super) fn build(
        builder: WebViewBuilder<'_>,
        features: NewWindowFeatures,
        on_close: impl Fn() + 'static,
        log_path: &Path,
    ) -> wry::Result<(WebView, Self)> {
        let mtm = MainThreadMarker::new().ok_or(wry::Error::NotMainThread)?;
        let size = features.size.map_or(NSSize::new(960.0, 720.0), |size| {
            NSSize::new(size.width.max(320.0), size.height.max(240.0))
        });
        let rect = NSRect::new(NSPoint::ZERO, size);
        // The window and its delegates are owned by the popup entry.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                rect,
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Miniaturizable
                    | NSWindowStyleMask::Resizable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(DEFAULT_TITLE));
        window.center();
        if let Some(parent) = features.opener.webview.window() {
            window.setLevel(parent.level());
        }
        let content_view =
            ContentView(window.contentView().ok_or(wry::Error::UnsupportedWindowHandle)?);
        let config = features.opener.target_configuration;
        // Keep the supplied session and process relationship. The child has no
        // toolbar IPC and needs its own user scripts instead of the parent's.
        unsafe { config.setUserContentController(&WKUserContentController::new(mtm)) };
        let webview = super::with_downloads(builder, log_path)
            .with_user_agent(SAFARI_USER_AGENT)
            .with_webview_configuration(config)
            .build(&content_view)?;
        let attachments = enable_attachments(&webview);

        let native = webview.webview();
        let original = unsafe { native.UIDelegate() }.ok_or(wry::Error::UnsupportedWindowHandle)?;
        let delegate = mtm
            .alloc::<PopupDelegate>()
            .set_ivars(CloseCallbacks { original, close: Box::new(on_close) });
        let delegate: Retained<PopupDelegate> = unsafe { msg_send![super(delegate), init] };
        unsafe { native.setUIDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
        window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        window.makeKeyAndOrderFront(None);

        Ok((webview, Self { window, _delegate: delegate, _attachments: attachments }))
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.window.setDelegate(None);
        self.window.close();
    }
}

pub(super) fn response(webview: &WebView) -> NewWindowResponse {
    NewWindowResponse::Create { webview: Retained::into_super(webview.webview()) }
}
