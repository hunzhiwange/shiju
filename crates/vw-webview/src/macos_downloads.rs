use block2::Block;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject, Sel};
use objc2::{DeclaredClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSHTTPURLResponse, NSString};
use objc2_web_kit::{
    WKNavigationDelegate, WKNavigationResponse, WKNavigationResponsePolicy, WKWebView,
};

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Retained<ProtocolObject<dyn WKNavigationDelegate>>]
    pub(crate) struct AttachmentNavigationDelegate;

    unsafe impl NSObjectProtocol for AttachmentNavigationDelegate {
        #[unsafe(method(respondsToSelector:))]
        fn responds_to_selector(&self, selector: Sel) -> bool {
            let responds: bool = unsafe { msg_send![super(self), respondsToSelector: selector] };
            responds || self.ivars().respondsToSelector(selector)
        }
    }

    impl AttachmentNavigationDelegate {
        #[unsafe(method_id(forwardingTargetForSelector:))]
        fn forwarding_target(&self, _selector: Sel) -> Retained<AnyObject> {
            Retained::from(&**self.ivars())
        }
    }

    unsafe impl WKNavigationDelegate for AttachmentNavigationDelegate {
        #[unsafe(method(webView:decidePolicyForNavigationResponse:decisionHandler:))]
        fn decide_response(
            &self,
            webview: &WKWebView,
            response: &WKNavigationResponse,
            decision: &Block<dyn Fn(WKNavigationResponsePolicy)>,
        ) {
            // Wry's default policy only checks the MIME type. PDFs and images can still
            // be explicit attachments. Let Wry handle the resulting WKDownload.
            let attachment = unsafe { response.response() }
                .downcast::<NSHTTPURLResponse>()
                .ok()
                .and_then(|response| response.valueForHTTPHeaderField(&NSString::from_str("Content-Disposition")))
                .is_some_and(|value| is_attachment(&value.to_string()));
            if attachment {
                decision.call((WKNavigationResponsePolicy::Download,));
            } else {
                unsafe {
                    self.ivars().webView_decidePolicyForNavigationResponse_decisionHandler(
                        webview, response, decision,
                    );
                }
            }
        }
    }
);

pub(crate) fn enable_attachments(
    webview: &wry::WebView,
) -> Option<Retained<AttachmentNavigationDelegate>> {
    use wry::WebViewExtMacOS;

    let mtm = MainThreadMarker::new()?;
    let webview = webview.webview();
    // WKDownload and its navigation policies were added in macOS 11.3.
    objc2::runtime::AnyClass::get(c"WKDownload")?;
    let original = unsafe { webview.navigationDelegate() }?;
    if !original
        .respondsToSelector(sel!(webView:decidePolicyForNavigationResponse:decisionHandler:))
    {
        return None;
    }
    let delegate = mtm.alloc::<AttachmentNavigationDelegate>().set_ivars(original);
    let delegate: Retained<AttachmentNavigationDelegate> =
        unsafe { msg_send![super(delegate), init] };
    unsafe { webview.setNavigationDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
    // WKWebView's delegate reference is weak. The caller keeps this guard alive.
    Some(delegate)
}

fn is_attachment(value: &str) -> bool {
    let disposition = value.split(';').next().unwrap_or_default();
    disposition.trim().eq_ignore_ascii_case("attachment")
}

#[cfg(test)]
#[path = "macos_downloads_tests.rs"]
mod tests;
