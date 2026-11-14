//! Process control protocol behavior tests.

use super::*;

#[test]
fn parse_stdin_event_accepts_resize_and_move() {
    assert_eq!(parse_stdin_event(" resize 640 480 "), Some(UserEvent::Resize(640, 480)));
    assert_eq!(parse_stdin_event("move -5 20 trailing"), Some(UserEvent::Move(-5, 20)));
}

#[test]
fn parse_stdin_event_rejects_invalid_commands() {
    assert_eq!(parse_stdin_event(""), None);
    assert_eq!(parse_stdin_event("resize 10"), None);
    assert_eq!(parse_stdin_event("resize wide 10"), None);
    assert_eq!(parse_stdin_event("open https://example.com"), None);
}

#[test]
fn work_browser_protocol_parses_navigation_history_visibility_and_close() {
    assert_eq!(
        parse_stdin_event("navigate https://example.com/path?q=1"),
        Some(UserEvent::OpenUrl("https://example.com/path?q=1".to_string()))
    );
    assert_eq!(parse_stdin_event("back"), Some(UserEvent::Back));
    assert_eq!(parse_stdin_event("forward"), Some(UserEvent::Forward));
    assert_eq!(parse_stdin_event("reload"), Some(UserEvent::Reload));
    assert_eq!(parse_stdin_event("raise"), Some(UserEvent::SetAlwaysOnTop(true)));
    assert_eq!(parse_stdin_event("lower"), Some(UserEvent::SetAlwaysOnTop(false)));
    assert_eq!(parse_stdin_event("show"), Some(UserEvent::Show));
    assert_eq!(parse_stdin_event("hide"), Some(UserEvent::Hide));
    assert_eq!(parse_stdin_event("close"), Some(UserEvent::Close));
    assert_eq!(parse_stdin_event("back unexpected"), None);
    assert_eq!(parse_stdin_event("navigate"), None);
}

#[test]
fn work_browser_protocol_rejects_unsafe_navigation_scheme() {
    assert_eq!(parse_stdin_event("navigate javascript:alert(1)"), None);
    assert_eq!(parse_stdin_event("navigate file:///tmp/private"), None);
    assert_eq!(parse_stdin_event("navigate data:text/html,unsafe"), None);
    assert_eq!(parse_stdin_event("navigate https://"), None);
    assert_eq!(
        parse_stdin_event("navigate HTTP://example.com"),
        Some(UserEvent::OpenUrl("HTTP://example.com".to_string()))
    );
}

#[test]
fn work_browser_navigation_event_encodes_safe_url_for_parent() {
    assert_eq!(
        navigation_event_line("https://example.com/path?q=a%20b#part").as_deref(),
        Some(r#"navigation "https://example.com/path?q=a%20b#part""#)
    );
    assert_eq!(navigation_event_line("javascript:alert(1)"), None);
    assert_eq!(navigation_event_line("file:///tmp/private"), None);
}

#[test]
fn work_browser_navigation_ipc_accepts_only_safe_navigation_messages() {
    assert_eq!(
        parse_navigation_ipc_message(
            r#"{"type":"navigation","url":"https://example.com/client-route"}"#
        )
        .as_deref(),
        Some("https://example.com/client-route")
    );
    assert_eq!(
        parse_navigation_ipc_message(r#"{"type":"navigation","url":"javascript:alert(1)"}"#),
        None
    );
    assert_eq!(
        parse_navigation_ipc_message(r#"{"type":"other","url":"https://example.com"}"#),
        None
    );
    assert_eq!(parse_navigation_ipc_message("not-json"), None);
}
