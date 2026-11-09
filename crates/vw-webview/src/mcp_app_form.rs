//! Hold the engine's original form navigation until the host confirms its HTTPS origin.
use vw_api_types::mcp_app_network::McpAppNetworkPolicy;

#[cfg(all(not(coverage), target_os = "macos"))]
#[path = "mcp_app_form/macos.rs"]
mod platform;
#[cfg(all(not(coverage), unix, not(target_os = "macos")))]
#[path = "mcp_app_form/gtk.rs"]
mod platform;

pub(super) fn allowed_origins(policy: &McpAppNetworkPolicy) -> Vec<String> {
    if cfg!(any(target_os = "windows", coverage)) {
        return Vec::new();
    }
    policy
        .connect_domains
        .iter()
        .filter(|origin| {
            // GTK's action API does not identify the destination frame. Keep
            // the form destinations disjoint from ordinary iframe navigations.
            cfg!(target_os = "macos") || !policy.frame_domains.contains(origin)
        })
        .cloned()
        .collect()
}

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

    use crate::mcp_app::{frame_origin_allowed, https_origin, internal_navigation_allowed};

    #[derive(Debug, PartialEq, Eq)]
    pub(crate) enum FormNavigation {
        Allow,
        Deny,
        Confirm(String),
    }

    pub(crate) struct FormPolicy {
        pub(crate) parent: Option<crate::external_protocol::Parent>,
        pub(crate) frame_domains: Vec<String>,
        pub(crate) origins: Vec<String>,
        pub(crate) active: Arc<AtomicBool>,
        pub(crate) submitted: Rc<Cell<bool>>,
        pub(crate) pending: Rc<Cell<bool>>,
    }

    impl FormPolicy {
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
            crate::external_protocol::confirm_mcp_form(parent, &origin, move |approved| {
                pending.set(false);
                let approved = approved && active.load(Ordering::Acquire);
                submitted.set(approved);
                if approved {
                    crate::mcp_app::revoke_document();
                } else {
                    eprintln!("MCP app form submission cancelled");
                }
                decide(approved);
            });
        }
    }
}

#[cfg(all(not(coverage), unix))]
pub(crate) use supported::{FormNavigation, FormPolicy};

#[cfg(all(not(coverage), target_os = "macos"))]
pub(super) fn install(
    webview: &wry::WebView,
    policy: FormPolicy,
) -> wry::Result<objc2::rc::Retained<platform::FormDelegate>> {
    platform::install(webview, policy)
}

#[cfg(all(not(coverage), unix, not(target_os = "macos")))]
pub(super) fn install(webview: &wry::WebView, policy: FormPolicy) -> wry::Result<()> {
    platform::install(webview, policy);
    Ok(())
}
