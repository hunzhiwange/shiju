use super::{ExternalProtocols, Outcome, hand_off};
use webview_core::navigation::{ExternalTarget, Navigation, navigation};

fn external(url: &str) -> ExternalTarget {
    let Navigation::External(target) = navigation(url, false) else {
        panic!("external link did not reach host confirmation: {url}");
    };
    target
}

#[test]
fn external_protocol_hands_off_confirmed_links() {
    let requests = ExternalProtocols::default();
    for (url, scheme, popup) in [
        ("mailto:someone@example.test?subject=Hello", "mailto", None),
        ("tel:+15550101000", "tel", Some(7)),
        ("shiju-test://callback/path?code=synthetic", "shiju-test", Some(12)),
    ] {
        assert!(requests.queue(external(url), popup));
        // One target=_blank click can reach both native callbacks.
        assert!(!requests.queue(external(url), popup));
        let (target, parent, confirmation) = requests.take().unwrap();
        assert_eq!(parent, popup);
        assert!(!requests.queue(external(url), popup));
        let mut launches = 0;
        assert_eq!(
            hand_off(target, true, |actual_scheme, actual_url| {
                launches += 1;
                assert_eq!((actual_scheme, actual_url), (scheme, url));
                true
            }),
            Outcome::Opened
        );
        assert_eq!(launches, 1);
        drop(confirmation);
    }
}

#[test]
fn external_protocol_rejects_unsafe_and_unconfirmed_links() {
    let requests = ExternalProtocols::default();
    let url = "shiju-unregistered:payload";
    assert!(requests.queue(external(url), None));
    let (target, _, confirmation) = requests.take().unwrap();
    assert_eq!(
        hand_off(target, false, |_, _| panic!("cancelled request reached system")),
        Outcome::Cancelled
    );
    assert!(!requests.queue(external(url), None));
    drop(confirmation);
    assert!(requests.queue(external(url), None));
    let (target, _, _confirmation) = requests.take().unwrap();
    assert_eq!(hand_off(target, true, |_, _| false), Outcome::Unavailable);
}
