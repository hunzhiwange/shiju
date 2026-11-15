use super::bridge::{ChromeBridge, EXTENSION_ID};
use crate::computer_control::host::{read_frame, write_frame};
use serde_json::{Value, json};
use tokio::net::UnixStream;
use vw_api_types::computer_control::ControlSessionId;

async fn paired() -> (ChromeBridge, UnixStream) {
    let (bridge, mut extension) = UnixStream::pair().unwrap();
    let task = tokio::spawn(async move {
        ChromeBridge::pair(bridge, ControlSessionId("session".into())).await.unwrap()
    });
    write_frame(
        &mut extension,
        &json!({"type":"hello","version":1,"extensionId":EXTENSION_ID,"challenge":"challenge"}),
    )
    .await
    .unwrap();
    let reply: Value = read_frame(&mut extension).await.unwrap();
    assert_eq!(reply["challenge"], "challenge");
    assert_eq!(reply["sessionId"], "session");
    (task.await.unwrap(), extension)
}

#[tokio::test]
async fn cc_control_09_pair_and_correlate_real_frames() {
    let (bridge, mut extension) = paired().await;
    let task = tokio::spawn(async move {
        let request: Value = read_frame(&mut extension).await.unwrap();
        assert_eq!(request["action"], "enumerate");
        write_frame(&mut extension, &json!({"type":"response","version":1,"sessionId":"session","id":request["id"],"ok":true,"data":[]})).await.unwrap();
        extension
    });
    assert_eq!(bridge.enumerate().await.unwrap(), json!([]));
    let _extension = task.await.unwrap();
    assert!(bridge.is_available());
}

#[tokio::test]
async fn cc_control_09_wrong_extension_rejected() {
    let (server, mut extension) = UnixStream::pair().unwrap();
    write_frame(
        &mut extension,
        &json!({"type":"hello","version":1,"extensionId":"wrong","challenge":"challenge"}),
    )
    .await
    .unwrap();
    assert!(ChromeBridge::pair(server, ControlSessionId("session".into())).await.is_err());
}

#[tokio::test]
async fn cc_control_09_oversize_rejected_before_allocation() {
    use tokio::io::AsyncWriteExt;
    let (server, mut extension) = UnixStream::pair().unwrap();
    extension.write_u32(1_048_577).await.unwrap();
    assert!(ChromeBridge::pair(server, ControlSessionId("session".into())).await.is_err());
}

#[tokio::test]
async fn cc_control_09_response_mismatch_disconnects() {
    let (bridge, mut extension) = paired().await;
    let task = tokio::spawn(async move {
        let _: Value = read_frame(&mut extension).await.unwrap();
        write_frame(&mut extension, &json!({"type":"response","version":1,"sessionId":"session","id":"other","ok":true,"data":[]})).await.unwrap();
    });
    assert!(bridge.enumerate().await.is_err());
    task.await.unwrap();
    assert!(!bridge.is_available());
}

#[tokio::test]
async fn cc_control_09_idle_disconnect_updates_availability() {
    let (bridge, extension) = paired().await;
    drop(extension);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while bridge.is_available() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(bridge.enumerate().await.is_err());
}

#[tokio::test]
async fn cc_control_09_authenticated_host_and_live_revocation() {
    use crate::computer_control::{
        host::{ControlHost, HostMessage, HostReply},
        policy::{ComputerControlPolicy, ControlFeature, GrantScope},
    };
    use parking_lot::RwLock;
    use std::sync::{Arc, atomic::AtomicBool};
    use std::time::Duration;
    use vw_api_types::computer_control::*;
    use vw_config_types::computer_control::{ComputerControlConfig, GrantDecision};
    let user = format!("uid:{}", unsafe { libc::geteuid() });
    let mut policy = ComputerControlPolicy::new(ComputerControlConfig::default());
    policy.set_enabled(&user, ControlFeature::Chrome, true).unwrap();
    policy
        .set_grant(
            &user,
            GrantScope::ChromeOrigin("https://example.com".into()),
            Some(GrantDecision::Allow),
        )
        .unwrap();
    let policy = Arc::new(RwLock::new(policy));
    let host = Arc::new(ControlHost::new(policy.clone(), Arc::new(AtomicBool::new(false))));
    let target =
        ComputerControlTarget::ChromeTab { tab_id: 1, origin: "https://example.com".into() };
    let ticket = host.issue(target.clone(), Duration::from_secs(10)).unwrap();
    let (native, mut extension) = UnixStream::pair().unwrap();
    let id = ticket.session_id.clone();
    let pair = tokio::spawn(async move { ChromeBridge::pair(native, id).await.unwrap() });
    write_frame(
        &mut extension,
        &json!({"type":"hello","version":1,"extensionId":EXTENSION_ID,"challenge":"challenge"}),
    )
    .await
    .unwrap();
    let _: Value = read_frame(&mut extension).await.unwrap();
    let bridge = pair.await.unwrap();
    let (mut client, server) = UnixStream::pair().unwrap();
    let serve = tokio::spawn(async move { bridge.serve(&host, server).await });
    write_frame(
        &mut client,
        &HostMessage::Hello {
            session_id: ticket.session_id.clone(),
            credential: ticket.credential,
        },
    )
    .await
    .unwrap();
    assert_eq!(read_frame::<_, HostReply>(&mut client).await.unwrap(), HostReply::Ready);
    let extension_task = tokio::spawn(async move {
        for action in ["enumerate", "read", "enumerate"] {
            let request: Value = read_frame(&mut extension).await.unwrap();
            assert_eq!(request["action"], action);
            let data = if action == "enumerate" {
                json!([{"tabId":1,"documentId":"document","navigationVersion":0,"url":"https://example.com/fixture"}])
            } else {
                json!({"text":"真实帧回读"})
            };
            write_frame(&mut extension, &json!({"type":"response","version":1,"sessionId":request["sessionId"],"id":request["id"],"ok":true,"data":data})).await.unwrap();
        }
        extension
    });
    let request = ComputerControlRequest {
        request_id: ControlRequestId("request".into()),
        session_id: ticket.session_id,
        permission_version: ticket.permission_version,
        target,
        action: ComputerControlAction::Chrome { command: ChromeCommand::Snapshot },
    };
    write_frame(&mut client, &HostMessage::Action { sequence: 1, request: request.clone() })
        .await
        .unwrap();
    assert_eq!(
        read_frame::<_, HostReply>(&mut client).await.unwrap(),
        HostReply::Completed {
            request_id: request.request_id.clone(),
            outcome: ControlOutcome::Succeeded {
                output: ControlOutput::Observation {
                    text: "真实帧回读".into(),
                    screenshots: vec![]
                }
            }
        }
    );
    let _extension = extension_task.await.unwrap();
    policy.write().set_enabled(&user, ControlFeature::Chrome, false).unwrap();
    write_frame(&mut client, &HostMessage::Action { sequence: 2, request }).await.unwrap();
    assert_eq!(
        read_frame::<_, HostReply>(&mut client).await.unwrap(),
        HostReply::Rejected { error: ControlErrorCode::StalePermission }
    );
    serve.await.unwrap().unwrap();
}

#[tokio::test]
async fn cc_control_09_python_subprocess_split_frames_pair() {
    use std::{os::unix::fs::PermissionsExt, process::Stdio};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::UnixListener,
        process::Command,
    };
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let script = root.path().join("chrome_native_host.py");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/computer_control/chrome_native_host.py"
        ),
        &script,
    )
    .unwrap();
    let path = root.path().join("chrome.sock");
    let listener = UnixListener::bind(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let mut child = Command::new("python3")
        .arg(script)
        .arg(format!("chrome-extension://{EXTENSION_ID}/"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = child.stdout.take().unwrap();
    let frame = serde_json::to_vec(
        &json!({"type":"hello","version":1,"extensionId":EXTENSION_ID,"challenge":"split"}),
    )
    .unwrap();
    let length = (frame.len() as u32).to_ne_bytes();
    input.write_all(&length[..2]).await.unwrap();
    input.write_all(&length[2..]).await.unwrap();
    input.write_all(&frame).await.unwrap();
    let (stream, _) = tokio::time::timeout(std::time::Duration::from_secs(3), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let bridge = ChromeBridge::pair(stream, ControlSessionId("subprocess".into())).await.unwrap();
    let mut prefix = [0; 4];
    output.read_exact(&mut prefix).await.unwrap();
    let mut body = vec![0; u32::from_ne_bytes(prefix) as usize];
    output.read_exact(&mut body).await.unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&body).unwrap()["challenge"], "split");
    drop(bridge);
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(3), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}

/// Opt-in real browser verification. The driver starts an isolated Chrome profile,
/// loads the actual extension and clicks its action; no port or Chrome API mocks.
#[tokio::test]
#[ignore = "requires macOS Chrome, Node and chrome_native_host.py --install"]
async fn cc_control_09_real_chrome_native_profile() {
    use std::{os::unix::fs::PermissionsExt, process::Stdio};
    use tokio::{net::UnixListener, process::Command};
    let directory =
        std::path::PathBuf::from(std::env::var("HOME").unwrap()).join(".shiju/computer-control");
    let endpoint = directory.join("chrome.sock");
    assert!(!endpoint.exists(), "a real host is already using the endpoint");
    let listener = UnixListener::bind(&endpoint).unwrap();
    std::fs::set_permissions(&endpoint, std::fs::Permissions::from_mode(0o600)).unwrap();
    struct SocketCleanup(std::path::PathBuf);
    impl Drop for SocketCleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _cleanup = SocketCleanup(endpoint);
    let mut driver = Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/computer_control/test_chrome_native_host.py"
        ))
        .arg("--live-driver")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let (stream, _) = tokio::time::timeout(std::time::Duration::from_secs(30), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let bridge =
        ChromeBridge::pair(stream, ControlSessionId("cc09-live-profile".into())).await.unwrap();
    let mut targets = json!([]);
    for _ in 0..20 {
        targets = bridge.enumerate().await.unwrap();
        if !targets.as_array().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert_eq!(targets.as_array().unwrap().len(), 1);
    assert!(targets[0]["url"].as_str().unwrap().starts_with("http://127.0.0.1:"));
    println!("real native messaging target={targets}");
    drop(bridge);
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(10), driver.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}

#[tokio::test]
async fn cc_control_09_cancelling_request_closes_native_connection() {
    use std::sync::Arc;
    let (bridge, mut extension) = paired().await;
    let bridge = Arc::new(bridge);
    let caller = bridge.clone();
    let pending = tokio::spawn(async move { caller.enumerate().await });
    let _: Value = read_frame(&mut extension).await.unwrap();
    pending.abort();
    let _ = pending.await;
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while bridge.is_available() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(read_frame::<_, Value>(&mut extension).await.is_err());
}

#[tokio::test]
async fn cc_control_09_truncated_frame_rejected() {
    use tokio::io::AsyncWriteExt;
    let (server, mut extension) = UnixStream::pair().unwrap();
    extension.write_u32(20).await.unwrap();
    extension.write_all(b"{}").await.unwrap();
    drop(extension);
    assert!(ChromeBridge::pair(server, ControlSessionId("session".into())).await.is_err());
}
