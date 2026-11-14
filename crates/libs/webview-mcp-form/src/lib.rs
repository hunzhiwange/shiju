//! Hold the engine's original form navigation until the host confirms its HTTPS origin.

#[cfg(all(not(coverage), target_os = "macos"))]
#[path = "macos.rs"]
mod platform;
#[cfg(all(not(coverage), unix, not(target_os = "macos")))]
#[path = "gtk.rs"]
mod platform;

#[cfg(all(not(coverage), unix))]
mod supported {
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use webview_mcp_policy::{frame_origin_allowed, https_origin, internal_navigation_allowed};

    #[derive(Debug, PartialEq, Eq)]
    pub(crate) enum FormNavigation {
        Allow,
        Deny,
        Confirm(String),
    }

    pub struct FormPolicy {
        pub(crate) parent: Option<webview_external_protocol::Parent>,
        pub(crate) frame_domains: Vec<String>,
        pub(crate) origins: Vec<String>,
        pub(crate) active: Arc<AtomicBool>,
        pub(crate) submitted: Rc<Cell<bool>>,
        pub(crate) pending: Rc<Cell<bool>>,
        pub(crate) revoke_document: fn(),
    }

    impl FormPolicy {
        pub fn new(
            parent: Option<webview_external_protocol::Parent>,
            frame_domains: Vec<String>,
            origins: Vec<String>,
            active: Arc<AtomicBool>,
            submitted: Rc<Cell<bool>>,
            revoke_document: fn(),
        ) -> Self {
            Self {
                parent,
                frame_domains,
                origins,
                active,
                submitted,
                pending: Rc::new(Cell::new(false)),
                revoke_document,
            }
        }

        pub(crate) fn decide(
            &self,
            url: &str,
            form: bool,
            redirect: bool,
            app_target: bool,
        ) -> FormNavigation {
            // Once approved, only the original in-flight request can commit.
            // A redirected request, including a 307 replay, needs a new load.
            if !self.active.load(Ordering::Acquire) || self.submitted.get() || self.pending.get() {
                return FormNavigation::Deny;
            }
            if form {
                if !redirect
                    && app_target
                    && let Some(origin) = https_origin(url)
                    && self.origins.contains(&origin)
                {
                    return FormNavigation::Confirm(origin);
                }
                eprintln!("MCP app form submission blocked by navigation policy");
                return FormNavigation::Deny;
            }
            if internal_navigation_allowed(url) || frame_origin_allowed(&self.frame_domains, url) {
                FormNavigation::Allow
            } else {
                FormNavigation::Deny
            }
        }

        pub(crate) fn confirm(&self, origin: String, decide: impl FnOnce(bool) + 'static) {
            let Some(parent) = &self.parent else {
                decide(false);
                return;
            };
            self.pending.set(true);
            let pending = self.pending.clone();
            let active = self.active.clone();
            let submitted = self.submitted.clone();
            let revoke_document = self.revoke_document;
            webview_external_protocol::confirm_mcp_form(parent, &origin, move |approved| {
                pending.set(false);
                let approved = approved && active.load(Ordering::Acquire);
                submitted.set(approved);
                if approved {
                    revoke_document();
                } else {
                    eprintln!("MCP app form submission cancelled");
                }
                decide(approved);
            });
        }
    }
}

#[cfg(all(not(coverage), unix))]
pub use supported::FormPolicy;
#[cfg(all(not(coverage), unix))]
use supported::FormNavigation;

#[cfg(all(not(coverage), target_os = "macos"))]
pub fn install(
    webview: &wry::WebView,
    policy: FormPolicy,
) -> wry::Result<objc2::rc::Retained<platform::FormDelegate>> {
    platform::install(webview, policy)
}

#[cfg(all(not(coverage), unix, not(target_os = "macos")))]
pub fn install(webview: &wry::WebView, policy: FormPolicy) -> wry::Result<()> {
    platform::install(webview, policy);
    Ok(())
}

#[cfg(all(test, not(coverage), unix))]
#[path = "form_tests.rs"]
mod tests;
