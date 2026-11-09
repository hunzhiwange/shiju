use super::{ExternalProtocols, ExternalTarget, Navigation, Outcome, hand_off, navigation};

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
    for url in [
        "javascript:alert(1)",
        "file:///tmp/private",
        "ms-settings:privacy",
        "search-ms:query=secret",
        "shell:startup",
        "x-apple.systempreferences:com.apple.settings",
        "chrome://settings",
        "view-source:https://example.test",
        "vibe://localhost/private",
        "mailto:a@example.test\r\nBcc:other@example.test",
        "mailto:a@example.test?subject=%0d%0aBcc:other@example.test",
        "tel:",
        "1bad:payload",
        "C:private.exe",
        "custom://",
    ] {
        assert!(matches!(navigation(url, false), Navigation::Blocked), "accepted {url:?}");
    }
    for url in [
        "https://example.test",
        "http://127.0.0.1:5000",
        "blob:https://example.test/id",
        "data:text/plain,download",
        "about:blank",
    ] {
        assert!(matches!(navigation(url, false), Navigation::InWebView));
        assert!(ExternalTarget::parse(url).is_none());
    }
    assert!(matches!(navigation("vibe://localhost/index.html", true), Navigation::InWebView));

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
