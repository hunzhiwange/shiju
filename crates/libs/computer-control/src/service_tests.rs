use crate::{
    host::{ControlHost, HostMessage, HostReply, read_frame, write_frame},
    policy::{ComputerControlPolicy, ControlFeature},
    service::ControlService,
};
use parking_lot::RwLock;
use std::{
    os::unix::fs::PermissionsExt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{io::AsyncWriteExt, net::UnixStream};
use tokio_util::sync::CancellationToken;
use vw_api_computer_control::*;

fn fixture() -> Arc<ControlHost> {
    let mut policy = ComputerControlPolicy::new(Default::default());
    policy
        .set_enabled(
            &format!("uid:{}", unsafe { libc::geteuid() }),
            ControlFeature::NativeApps,
            true,
        )
        .unwrap();
    Arc::new(ControlHost::new_with_lock_state(
        Arc::new(RwLock::new(policy)),
        crate::native::locked_session::LockState::Unlocked,
    ))
}
fn target() -> ComputerControlTarget {
    ComputerControlTarget::NativeApp { bundle_id: "com.apple.TextEdit".into(), process_id: 10 }
}

#[tokio::test]
async fn cc_runtime_02_private_socket_auth_sequence_and_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("service");
    let service = ControlService::bind(&directory).unwrap();
    let socket = service.socket_path().to_owned();
    assert_eq!(std::fs::metadata(&directory).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(std::fs::metadata(&socket).unwrap().permissions().mode() & 0o777, 0o600);
    assert!(ControlService::bind(&directory).is_err());
    let host = fixture();
    let ticket = host.issue(target(), Duration::from_secs(10)).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    let shutdown = CancellationToken::new();
    let task = tokio::spawn(service.run(host.clone(), shutdown.clone(), move |_, _| {
        calls.fetch_add(1, Ordering::SeqCst);
        async { Ok(ControlOutput::Acknowledged) }
    }));
    let mut wrong = UnixStream::connect(&socket).await.unwrap();
    write_frame(
        &mut wrong,
        &HostMessage::Hello { session_id: ticket.session_id.clone(), credential: "wrong".into() },
    )
    .await
    .unwrap();
    assert_eq!(
        read_frame::<_, HostReply>(&mut wrong).await.unwrap(),
        HostReply::Rejected { error: ControlErrorCode::PermissionDenied }
    );
    let mut client = UnixStream::connect(&socket).await.unwrap();
    let hello = serde_json::to_vec(&HostMessage::Hello {
        session_id: ticket.session_id.clone(),
        credential: ticket.credential.clone(),
    })
    .unwrap();
    client.write_u32(hello.len() as u32).await.unwrap();
    for fragment in hello.chunks(3) {
        client.write_all(fragment).await.unwrap();
        tokio::task::yield_now().await;
    }
    assert_eq!(read_frame::<_, HostReply>(&mut client).await.unwrap(), HostReply::Ready);
    let mut replay = UnixStream::connect(&socket).await.unwrap();
    write_frame(
        &mut replay,
        &HostMessage::Hello {
            session_id: ticket.session_id.clone(),
            credential: ticket.credential.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        read_frame::<_, HostReply>(&mut replay).await.unwrap(),
        HostReply::Rejected { error: ControlErrorCode::PermissionDenied }
    );
    let request = ComputerControlRequest {
        request_id: ControlRequestId("r1".into()),
        session_id: ticket.session_id,
        permission_version: ticket.permission_version,
        target: target(),
        action: ComputerControlAction::NativeObserve { screenshot: false },
    };
    for attempt in 0..2 {
        let sequence = 1;
        write_frame(&mut client, &HostMessage::Action { sequence, request: request.clone() })
            .await
            .unwrap();
        let reply: HostReply = read_frame(&mut client).await.unwrap();
        if count.load(Ordering::SeqCst) == 0 {
            panic!("action was not dispatched");
        }
        if attempt == 1 {
            assert_eq!(reply, HostReply::Rejected { error: ControlErrorCode::InvalidRequest });
        } else {
            assert!(matches!(reply, HostReply::Completed { .. }));
        }
    }
    assert_eq!(count.load(Ordering::SeqCst), 1);
    shutdown.cancel();
    task.await.unwrap().unwrap();
    assert!(!socket.exists());
    assert!(!directory.exists());
}

#[tokio::test]
async fn cc_runtime_02_bad_frame_never_dispatches() {
    let root = tempfile::tempdir().unwrap();
    let service = ControlService::bind(&root.path().join("service")).unwrap();
    let socket = service.socket_path().to_owned();
    let shutdown = CancellationToken::new();
    let task = tokio::spawn(
        service
            .run(fixture(), shutdown.clone(), |_, _| async { panic!("invalid frame dispatched") }),
    );
    for length in [0, 1024 * 1024 + 1] {
        let mut client = UnixStream::connect(&socket).await.unwrap();
        client.write_u32(length).await.unwrap();
        assert_eq!(
            read_frame::<_, HostReply>(&mut client).await.unwrap(),
            HostReply::Rejected { error: ControlErrorCode::InvalidRequest }
        );
    }
    shutdown.cancel();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn cc_runtime_02_disconnect_cancels_inflight() {
    let root = tempfile::tempdir().unwrap();
    let service = ControlService::bind(&root.path().join("service")).unwrap();
    let socket = service.socket_path().to_owned();
    let host = fixture();
    let ticket = host.issue(target(), Duration::from_secs(10)).unwrap();
    let shutdown = CancellationToken::new();
    let (sent, mut received) = tokio::sync::mpsc::channel(1);
    let task = tokio::spawn(service.run(host.clone(), shutdown.clone(), move |_, cancel| {
        let sent = sent.clone();
        async move {
            sent.send(cancel).await.unwrap();
            std::future::pending().await
        }
    }));
    let mut client = UnixStream::connect(&socket).await.unwrap();
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
    write_frame(
        &mut client,
        &HostMessage::Action {
            sequence: 1,
            request: ComputerControlRequest {
                request_id: ControlRequestId("wait".into()),
                session_id: ticket.session_id,
                permission_version: ticket.permission_version,
                target: target(),
                action: ComputerControlAction::NativeObserve { screenshot: false },
            },
        },
    )
    .await
    .unwrap();
    let cancel =
        tokio::time::timeout(Duration::from_secs(2), received.recv()).await.unwrap().unwrap();
    drop(client);
    tokio::time::timeout(Duration::from_secs(2), cancel.cancelled()).await.unwrap();
    shutdown.cancel();
    task.await.unwrap().unwrap();
    assert_eq!(host.session_count(), 0);
}
