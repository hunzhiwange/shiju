use super::*;
#[test]
fn cc_apps_latest_version_selection_preserves_local_on_failure() {
    let result = select_latest(Some("1.2.3"), Err(UnknownReason::Network));
    assert_eq!(result.local_version.as_deref(), Some("1.2.3"));
    assert_eq!(result.unknown_reason, Some(UnknownReason::Network));
    assert!(!result.update_available);
}
#[test]
fn cc_apps_latest_version_selection_never_downgrades_or_replaces_dev() {
    for local in ["2.0.0", "0.1.0+238.gabcd", "1.0.0+dirty", "unknown"] {
        let result = select_latest(Some(local), Ok("1.2.3".into()));
        assert_eq!(result.local_version.as_deref(), Some(local));
        assert!(result.latest_version.is_none());
        assert!(!result.update_available);
        assert!(result.unknown_reason.is_some());
    }
}
#[test]
fn cc_apps_latest_version_selection_reports_real_updates() {
    let result = select_latest(Some("1.0.0"), Ok("1.2.3".into()));
    assert!(result.update_available);
    assert_eq!(result.latest_version.as_deref(), Some("1.2.3"));
    assert_eq!(
        select_latest(None, Ok("garbage".into())).unknown_reason,
        Some(UnknownReason::InvalidMetadata)
    );
}

#[test]
fn cc_apps_latest_version_selection_metadata_sources_and_preview_policy() {
    use serde_json::json;
    let tags = json!({"latest":"1.0.0","next":"2.0.0-beta.2","beta":"9.0.0"});
    assert_eq!(metadata_version(Source::Npm(true), &tags, Some("1.0.0")).unwrap(), "1.0.0");
    assert_eq!(
        metadata_version(Source::Npm(true), &tags, Some("2.0.0-beta.1")).unwrap(),
        "2.0.0-beta.2"
    );
    assert_eq!(metadata_version(Source::Npm(false), &tags, Some("2.0.0-beta.1")).unwrap(), "1.0.0");
    for next in ["invalid", "9.0.0+dirty", "0.1.0"] {
        assert_eq!(
            metadata_version(
                Source::Npm(true),
                &json!({"latest":"1.0.0","next":next}),
                Some("2.0.0")
            )
            .unwrap(),
            "1.0.0"
        );
    }
    assert_eq!(
        metadata_version(
            Source::Github,
            &json!({"name":"Hermes Agent v0.21.0 (v2026.8.31)", "tag_name":"v2026.8.31"}),
            None
        )
        .unwrap(),
        "v0.21.0"
    );
    for value in [
        json!({"tag_name":"v2026.8.31"}),
        json!({"tag_name":"v1.0.0", "prerelease":true}),
        json!({"message":"rate limited"}),
    ] {
        assert_eq!(
            metadata_version(Source::Github, &value, None),
            Err(UnknownReason::InvalidMetadata)
        );
    }
    assert_eq!(
        metadata_version(Source::Pypi, &json!({"info":{"version":"1.2.3"}}), None).unwrap(),
        "1.2.3"
    );
    assert_eq!(metadata_version(Source::Agy, &json!({"version":"1.2.3"}), None).unwrap(), "1.2.3");
    assert_eq!(agy_platform("windows", "x86_64").as_deref(), Some("windows_amd64"));
    assert_eq!(agy_platform("macos", "aarch64").as_deref(), Some("darwin_arm64"));
    assert_eq!(agy_platform("linux", "unsupported"), None);
}
#[tokio::test]
async fn cc_apps_latest_version_selection_http_failure_and_deadline() {
    use std::io::{Read, Write};
    use std::time::Duration;
    for (status, body, delay, expected) in [
        ("200 OK", r#"{"latest":"1.2.3"}"#, 0, Ok("1.2.3".to_owned())),
        ("503 Unavailable", "{}", 0, Err(UnknownReason::Network)),
        ("200 OK", "broken", 0, Err(UnknownReason::InvalidMetadata)),
        ("200 OK", "{}", 150, Err(UnknownReason::Network)),
    ] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            let mut buffer = [0; 2048];
            let _ = stream.read(&mut buffer);
            std::thread::sleep(Duration::from_millis(delay));
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let result =
            fetch(&client, &url, Source::Npm(false), Some("1.0.0"), Duration::from_millis(80))
                .await;
        assert_eq!(result, expected);
        let selected = select_latest(Some("1.0.0"), result);
        assert_eq!(selected.local_version.as_deref(), Some("1.0.0"));
        server.join().unwrap();
    }
}
