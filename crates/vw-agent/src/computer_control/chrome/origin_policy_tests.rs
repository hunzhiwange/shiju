use super::bridge::ChromeTarget;
use super::origin_policy::{DocumentBinding, canonical_origin};
use crate::computer_control::policy::{ComputerControlPolicy, ControlFeature, GrantScope};
use vw_api_types::computer_control::*;
use vw_config_types::computer_control::{ComputerControlConfig, GrantDecision};

fn target(url: &str) -> ChromeTarget {
    ChromeTarget {
        tab_id: 7,
        document_id: "document-1".into(),
        navigation_version: 0,
        url: url.into(),
    }
}
fn policy() -> ComputerControlPolicy {
    let mut p = ComputerControlPolicy::new(ComputerControlConfig::default());
    p.set_enabled("uid:10", ControlFeature::Chrome, true).unwrap();
    p.set_grant(
        "uid:10",
        GrantScope::ChromeOrigin("https://example.test".into()),
        Some(GrantDecision::Allow),
    )
    .unwrap();
    p
}
#[test]
fn cc_control_10_origin_normalizes_scheme_host_and_default_port() {
    assert_eq!(
        canonical_origin("HTTPS://ExAmPlE.test:443/path?q=1").unwrap(),
        "https://example.test"
    );
    assert_eq!(
        canonical_origin("http://example.test:8080/path").unwrap(),
        "http://example.test:8080"
    );
}
#[test]
fn cc_control_10_credentials_and_non_http_origins_are_denied() {
    for url in [
        "file:///tmp/x",
        "data:text/plain,hi",
        "https://user:pass@example.test/",
        "javascript:alert(1)",
    ] {
        assert!(canonical_origin(url).is_err());
    }
}
#[test]
fn cc_control_10_cross_origin_current_document_is_denied() {
    let mut binding = DocumentBinding::default();
    assert_eq!(
        binding.check(&target("https://other.test/"), "https://example.test"),
        Err(ControlErrorCode::TargetChanged)
    );
}
#[test]
fn cc_control_10_same_origin_new_document_invalidates_old_handle() {
    let mut binding = DocumentBinding::default();
    let t = target("https://example.test/a");
    binding.check(&t, "https://example.test").unwrap();
    let mut next = t.clone();
    next.document_id = "document-2".into();
    assert_eq!(binding.check(&next, "https://example.test"), Err(ControlErrorCode::TargetChanged));
    // Once invalidated, returning to the old URL/document cannot revive this lease.
    assert_eq!(binding.check(&t, "https://example.test"), Err(ControlErrorCode::TargetChanged));
}
#[test]
fn cc_control_10_navigation_version_invalidates_same_url() {
    let mut binding = DocumentBinding::default();
    let t = target("https://example.test/a");
    binding.check(&t, "https://example.test").unwrap();
    let mut next = t;
    next.navigation_version += 1;
    assert_eq!(binding.check(&next, "https://example.test"), Err(ControlErrorCode::TargetChanged));
}
#[test]
fn cc_control_10_destination_origin_requires_its_own_grant() {
    let p = policy();
    let dst = ComputerControlTarget::ChromeTab {
        tab_id: 7,
        origin: canonical_origin("https://other.test/redir").unwrap(),
    };
    assert_eq!(
        p.authorize("uid:10", &dst, false, p.version()),
        Err(ControlErrorCode::PermissionDenied)
    );
}
#[test]
fn cc_control_10_revocation_rejects_cached_permission_version() {
    let mut p = policy();
    let version = p.version();
    p.set_grant("uid:10", GrantScope::ChromeOrigin("https://example.test".into()), None).unwrap();
    let t = ComputerControlTarget::ChromeTab { tab_id: 7, origin: "https://example.test".into() };
    assert_eq!(p.authorize("uid:10", &t, false, version), Err(ControlErrorCode::StalePermission));
    assert_eq!(
        p.authorize("uid:10", &t, false, p.version()),
        Err(ControlErrorCode::PermissionDenied)
    );
}
#[test]
fn cc_control_10_native_app_cannot_bypass_chrome_site_denial() {
    let mut p = policy();
    p.set_enabled("uid:10", ControlFeature::NativeApps, true).unwrap();
    for bundle in ["com.google.Chrome", "com.google.Chrome.canary"] {
        assert_eq!(
            p.authorize(
                "uid:10",
                &ComputerControlTarget::NativeApp { bundle_id: bundle.into(), process_id: 42 },
                false,
                p.version()
            ),
            Err(ControlErrorCode::PermissionDenied)
        );
    }
}

fn host_fixture() -> (
    std::sync::Arc<crate::computer_control::host::ControlHost>,
    std::sync::Arc<parking_lot::RwLock<ComputerControlPolicy>>,
    ComputerControlRequest,
) {
    use crate::computer_control::host::ControlHost;
    use std::sync::{Arc, atomic::AtomicBool};
    let user = format!("uid:{}", unsafe { libc::geteuid() });
    let mut p = ComputerControlPolicy::new(ComputerControlConfig::default());
    p.set_enabled(&user, ControlFeature::Chrome, true).unwrap();
    p.set_grant(
        &user,
        GrantScope::ChromeOrigin("https://example.test".into()),
        Some(GrantDecision::Allow),
    )
    .unwrap();
    let policy = Arc::new(parking_lot::RwLock::new(p));
    let host = Arc::new(ControlHost::new(policy.clone(), Arc::new(AtomicBool::new(false))));
    let target =
        ComputerControlTarget::ChromeTab { tab_id: 7, origin: "https://example.test".into() };
    let ticket = host.issue(target.clone(), std::time::Duration::from_secs(60)).unwrap();
    let request = ComputerControlRequest {
        request_id: ControlRequestId("cc10-request".into()),
        session_id: ticket.session_id,
        permission_version: ticket.permission_version,
        target,
        action: ComputerControlAction::Chrome { command: ChromeCommand::Snapshot },
    };
    (host, policy, request)
}
async fn bridge_fixture(
    id: ControlSessionId,
) -> (super::bridge::ChromeBridge, tokio::net::UnixStream) {
    use crate::computer_control::host::{read_frame, write_frame};
    use serde_json::{Value, json};
    let (native, mut extension) = tokio::net::UnixStream::pair().unwrap();
    let pairing =
        tokio::spawn(async move { super::bridge::ChromeBridge::pair(native, id).await.unwrap() });
    write_frame(&mut extension,&json!({"type":"hello","version":1,"extensionId":super::bridge::EXTENSION_ID,"challenge":"cc10"})).await.unwrap();
    let _: Value = read_frame(&mut extension).await.unwrap();
    (pairing.await.unwrap(), extension)
}
async fn reply(extension: &mut tokio::net::UnixStream, expected: &str, data: serde_json::Value) {
    use crate::computer_control::host::{read_frame, write_frame};
    use serde_json::{Value, json};
    let request: Value = read_frame(extension).await.unwrap();
    assert_eq!(request["action"], expected);
    write_frame(extension,&json!({"type":"response","version":1,"sessionId":request["sessionId"],"id":request["id"],"ok":true,"data":data})).await.unwrap();
}
#[tokio::test]
async fn cc_control_10_redirect_after_read_discards_page_content() {
    use serde_json::json;
    let (host, _, request) = host_fixture();
    let (bridge, mut extension) = bridge_fixture(request.session_id.clone()).await;
    let boundary = tokio::spawn(async move {
        reply(&mut extension, "enumerate", json!([target("https://example.test/start")])).await;
        reply(&mut extension, "read", json!({"text":"must not be delivered"})).await;
        reply(&mut extension, "enumerate", json!([target("https://unapproved.test/redirect")]))
            .await;
        extension
    });
    assert_eq!(bridge.execute(request, &host).await, Err(ControlErrorCode::TargetChanged));
    let _extension = boundary.await.unwrap();
}
#[tokio::test]
async fn cc_control_10_revocation_during_enumeration_prevents_read() {
    use crate::computer_control::host::{read_frame, write_frame};
    use serde_json::{Value, json};
    let (host, policy, request) = host_fixture();
    let (bridge, mut extension) = bridge_fixture(request.session_id.clone()).await;
    let boundary = tokio::spawn(async move {
        let r: Value = read_frame(&mut extension).await.unwrap();
        policy
            .write()
            .set_grant(
                &format!("uid:{}", unsafe { libc::geteuid() }),
                GrantScope::ChromeOrigin("https://example.test".into()),
                None,
            )
            .unwrap();
        write_frame(&mut extension,&json!({"type":"response","version":1,"sessionId":r["sessionId"],"id":r["id"],"ok":true,"data":[target("https://example.test/start")]})).await.unwrap();
        extension
    });
    assert_eq!(bridge.execute(request, &host).await, Err(ControlErrorCode::StalePermission));
    let mut extension = boundary.await.unwrap();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(30),
            read_frame::<_, Value>(&mut extension)
        )
        .await
        .is_err()
    );
}
#[test]
fn cc_control_10_upload_download_need_distinct_single_use_approval() {
    let (host, _, mut request) = host_fixture();
    request.action = ComputerControlAction::Chrome {
        command: ChromeCommand::Upload {
            selector: "#file".into(),
            file_name: "fixture.txt".into(),
            content_base64: "eA==".into(),
        },
    };
    assert_eq!(host.consume_transfer(&request), Err(ControlErrorCode::PermissionDenied));
    host.approve_transfer(&request).unwrap();
    let mut download = request.clone();
    download.action = ComputerControlAction::Chrome {
        command: ChromeCommand::Download {
            selector: "#link".into(),
            url: "https://example.test/file".into(),
        },
    };
    assert_eq!(host.consume_transfer(&download), Err(ControlErrorCode::PermissionDenied));
    // A mismatched attempt also consumes the permit, preventing replay.
    assert_eq!(host.consume_transfer(&request), Err(ControlErrorCode::PermissionDenied));
    host.approve_transfer(&request).unwrap();
    host.consume_transfer(&request).unwrap();
    assert_eq!(host.consume_transfer(&request), Err(ControlErrorCode::PermissionDenied));
}
#[tokio::test]
async fn cc_control_10_approved_upload_crosses_real_host_and_frames_once() {
    use serde_json::json;
    let (host, _, mut request) = host_fixture();
    request.action = ComputerControlAction::Chrome {
        command: ChromeCommand::Upload {
            selector: "#file".into(),
            file_name: "fixture.txt".into(),
            content_base64: "eA==".into(),
        },
    };
    host.approve_transfer(&request).unwrap();
    let (bridge, mut extension) = bridge_fixture(request.session_id.clone()).await;
    let boundary = tokio::spawn(async move {
        reply(&mut extension, "enumerate", json!([target("https://example.test/start")])).await;
        reply(&mut extension, "upload", json!({"uploaded":true})).await;
        reply(&mut extension, "enumerate", json!([target("https://example.test/start")])).await;
        extension
    });
    assert_eq!(bridge.execute(request.clone(), &host).await, Ok(ControlOutput::Acknowledged));
    assert_eq!(host.consume_transfer(&request), Err(ControlErrorCode::PermissionDenied));
    let _extension = boundary.await.unwrap();
}
#[test]
fn cc_control_10_upload_rejects_paths_invalid_bytes_and_oversize() {
    use super::bridge::validate_upload;
    assert_eq!(
        validate_upload("#file", "../secret", "eA=="),
        Err(ControlErrorCode::InvalidRequest)
    );
    assert_eq!(
        validate_upload("#file", "fixture.txt", "not base64"),
        Err(ControlErrorCode::InvalidRequest)
    );
    assert_eq!(
        validate_upload("#file", "fixture.txt", &"A".repeat(349529)),
        Err(ControlErrorCode::InvalidRequest)
    );
    validate_upload("#file", "fixture.txt", "eA==").unwrap();
}

#[tokio::test]
async fn cc_control_10_cross_origin_download_cannot_use_site_or_transfer_grant() {
    use crate::computer_control::host::read_frame;
    use serde_json::{Value, json};
    let (host, _, mut request) = host_fixture();
    request.action = ComputerControlAction::Chrome {
        command: ChromeCommand::Download {
            selector: "#download".into(),
            url: "https://other.test/file".into(),
        },
    };
    host.approve_transfer(&request).unwrap();
    let (bridge, mut extension) = bridge_fixture(request.session_id.clone()).await;
    let boundary = tokio::spawn(async move {
        reply(&mut extension, "enumerate", json!([target("https://example.test/start")])).await;
        extension
    });
    assert_eq!(bridge.execute(request, &host).await, Err(ControlErrorCode::PermissionDenied));
    let mut extension = boundary.await.unwrap();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(30),
            read_frame::<_, Value>(&mut extension)
        )
        .await
        .is_err()
    );
}
