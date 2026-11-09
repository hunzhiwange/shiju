use gtk::prelude::Cast;
use webkit2gtk::{
    NavigationPolicyDecision, NavigationPolicyDecisionExt, NavigationType, PolicyDecisionExt,
    PolicyDecisionType, URIRequestExt, WebViewExt,
};
use wry::{WebView, WebViewExtUnix};

use super::{FormNavigation, FormPolicy};

pub(super) fn install(webview: &WebView, policy: FormPolicy) {
    webview.webview().connect_decide_policy(move |_, decision, kind| {
        if kind == PolicyDecisionType::Response {
            return false;
        }
        let navigation = (kind == PolicyDecisionType::NavigationAction)
            .then(|| decision.downcast_ref::<NavigationPolicyDecision>())
            .flatten()
            .and_then(NavigationPolicyDecisionExt::navigation_action);
        let Some(mut action) = navigation else {
            decision.ignore();
            return true;
        };
        let Some(url) = action.request().and_then(|request| request.uri()) else {
            decision.ignore();
            return true;
        };

        // GTK does not expose frame identity here. The outer CSP admits only
        // local app HTML and the disjoint form origins as direct iframe loads.
        // The iframe sandbox rules out ancestor and auxiliary window targets.
        let form = action.navigation_type() == NavigationType::FormSubmitted;
        match policy.decide(&url, form, action.is_redirect(), true) {
            FormNavigation::Allow => decision.use_(),
            FormNavigation::Deny => decision.ignore(),
            FormNavigation::Confirm(origin) => {
                let decision = decision.clone();
                policy.confirm(origin, move |approved| {
                    if approved {
                        decision.use_();
                    } else {
                        decision.ignore();
                    }
                });
            }
        }
        true
    });
}
