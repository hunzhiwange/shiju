use block2::{Block, RcBlock};
use objc2::{
    DeclaredClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject, Sel},
    sel,
};
use objc2_web_kit::{
    WKNavigationAction, WKNavigationActionPolicy, WKNavigationDelegate, WKNavigationType, WKWebView,
};
use wry::{WebView, WebViewExtMacOS};

use super::{FormNavigation, FormPolicy};
use crate::mcp_app::{APP_URL, WRAPPER_URL, internal_navigation_allowed};

pub(crate) struct DelegateState {
    policy: FormPolicy,
    original: Retained<ProtocolObject<dyn WKNavigationDelegate>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = DelegateState]
    pub(crate) struct FormDelegate;

    unsafe impl NSObjectProtocol for FormDelegate {
        #[unsafe(method(respondsToSelector:))]
        fn responds_to_selector(&self, selector: Sel) -> bool {
            let responds: bool = unsafe { msg_send![super(self), respondsToSelector: selector] };
            responds || self.ivars().original.respondsToSelector(selector)
        }
    }

    impl FormDelegate {
        #[unsafe(method_id(forwardingTargetForSelector:))]
        fn forwarding_target(&self, _selector: Sel) -> Retained<AnyObject> {
            Retained::from(&*self.ivars().original)
        }
    }

    unsafe impl WKNavigationDelegate for FormDelegate {
        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        fn decide_navigation(
            &self,
            _webview: &WKWebView,
            action: &WKNavigationAction,
            decision: &Block<dyn Fn(WKNavigationActionPolicy)>,
        ) {
            // These are transient WebKit snapshots used only on the UI thread.
            let (url, target, source, is_form, download) = unsafe {
                (
                    action.request().URL().and_then(|url| url.absoluteString()),
                    action.targetFrame(),
                    action.sourceFrame(),
                    action.navigationType() == WKNavigationType::FormSubmitted,
                    action.respondsToSelector(sel!(shouldPerformDownload)) && action.shouldPerformDownload(),
                )
            };
            let (Some(url), Some(target)) = (url, target) else {
                decision.call((WKNavigationActionPolicy::Cancel,));
                return;
            };
            let url = url.to_string();
            if download {
                decision.call((WKNavigationActionPolicy::Cancel,));
                return;
            }
            let policy = &self.ivars().policy;
            if unsafe { target.isMainFrame() } {
                decision.call((if url == WRAPPER_URL
                    && policy.decide(&url, false, false, false) == FormNavigation::Allow
                {
                    WKNavigationActionPolicy::Allow
                } else {
                    WKNavigationActionPolicy::Cancel
                },));
                return;
            }
            let target_is_app = unsafe { target.request() }.URL()
                .and_then(|url| url.absoluteString()).is_some_and(|url| url.to_string() == APP_URL);
            let source_is_app = source.is_some_and(|frame| {
                unsafe { frame.request() }.URL()
                    .and_then(|url| url.absoluteString()).is_some_and(|url| url.to_string() == APP_URL)
            });
            if target_is_app && !is_form && !internal_navigation_allowed(&url) {
                decision.call((WKNavigationActionPolicy::Cancel,));
                return;
            }

            match policy.decide(&url, is_form, false, target_is_app && source_is_app) {
                FormNavigation::Allow => decision.call((WKNavigationActionPolicy::Allow,)),
                FormNavigation::Deny => decision.call((WKNavigationActionPolicy::Cancel,)),
                FormNavigation::Confirm(origin) => {
                    let decision = decision.copy();
                    policy.confirm(origin, move |approved| decide(&decision, approved));
                }
            }
        }

    }
);

fn decide(decision: &RcBlock<dyn Fn(WKNavigationActionPolicy)>, approved: bool) {
    decision.call((if approved {
        WKNavigationActionPolicy::Allow
    } else {
        WKNavigationActionPolicy::Cancel
    },));
}

pub(super) fn install(
    webview: &WebView,
    policy: FormPolicy,
) -> wry::Result<Retained<FormDelegate>> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| std::io::Error::other("MCP forms require the main thread"))?;
    let native = webview.webview();
    let original = unsafe { native.navigationDelegate() }
        .ok_or_else(|| std::io::Error::other("Missing Wry MCP navigation delegate"))?;
    let object = mtm.alloc::<FormDelegate>().set_ivars(DelegateState { policy, original });
    // WKWebView holds a weak delegate reference. The event loop owns it.
    let delegate: Retained<FormDelegate> = unsafe { msg_send![super(object), init] };
    unsafe { native.setNavigationDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
    Ok(delegate)
}
