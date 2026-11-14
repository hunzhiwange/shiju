#[cfg(all(unix, not(coverage)))]
#[test]
fn mcp_app_form_navigation_requires_exact_origin_and_live_document() {
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use super::{
        APP_URL,
        form::{FormNavigation, FormPolicy},
    };

    let origin = "https://api.example.test:8443";
    let policy = FormPolicy {
        parent: None,
        frame_domains: Vec::new(),
        origins: vec![origin.into(), "https://other.example.test".into()],
        active: Arc::new(AtomicBool::new(true)),
        submitted: Rc::new(Cell::new(false)),
        pending: Rc::new(Cell::new(false)),
    };
    let destination = format!("{origin}/submit");
    assert_eq!(policy.decide(APP_URL, false, false, false), FormNavigation::Allow);
    assert_eq!(
        policy.decide(&destination, true, false, true),
        FormNavigation::Confirm(origin.into())
    );
    for target in [
        "https://api.example.test/submit",
        "http://api.example.test:8443/submit",
        "https://user@api.example.test:8443/submit",
        "https://api.example.test:8443.evil.test/submit",
    ] {
        assert_eq!(policy.decide(target, true, false, true), FormNavigation::Deny);
    }
    assert_eq!(policy.decide(&destination, false, false, true), FormNavigation::Deny);
    assert_eq!(policy.decide(&destination, true, false, false), FormNavigation::Deny);
    assert_eq!(policy.decide(&destination, true, true, true), FormNavigation::Deny);

    policy.pending.set(true);
    assert_eq!(policy.decide(APP_URL, false, false, true), FormNavigation::Deny);
    policy.pending.set(false);
    policy.submitted.set(true);
    assert_eq!(
        policy.decide("https://other.example.test/redirect", true, false, true),
        FormNavigation::Deny
    );
    policy.submitted.set(false);
    policy.active.store(false, Ordering::Release);
    assert_eq!(policy.decide(&destination, true, false, true), FormNavigation::Deny);
}
