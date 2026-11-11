use super::host::{ControlHost, HostMessage, HostReply, SessionTicket, read_frame, write_frame};
use super::policy::{ComputerControlPolicy, ControlFeature};
use parking_lot::RwLock;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::net::UnixStream;
use vw_api_types::computer_control::*;
use vw_config_types::computer_control::ComputerControlConfig;

fn target() -> ComputerControlTarget {
    ComputerControlTarget::NativeApp { bundle_id: "com.apple.TextEdit".into(), process_id: 10 }
}
fn fixture() -> Arc<ControlHost> {
    let mut policy = ComputerControlPolicy::new(ComputerControlConfig::default());
    // The OS identity is derived by the host; no wire field can override it.
    policy
        .set_enabled(
            &format!("uid:{}", unsafe { libc::geteuid() }),
            ControlFeature::NativeApps,
            true,
        )
        .unwrap();
    Arc::new(ControlHost::new(Arc::new(RwLock::new(policy)), Arc::new(AtomicBool::new(false))))
}
fn request(ticket: &SessionTicket) -> ComputerControlRequest {
    ComputerControlRequest {
        request_id: ControlRequestId("request-1".into()),
        session_id: ticket.session_id.clone(),
        permission_version: ticket.permission_version,
        target: target(),
        action: ComputerControlAction::NativeObserve { screenshot: false },
    }
}
async fn connect(
    host: Arc<ControlHost>,
    ticket: &SessionTicket,
    count: Arc<AtomicUsize>,
    wait: bool,
) -> (UnixStream, tokio::task::JoinHandle<()>) {
    let (mut client, server) = UnixStream::pair().unwrap();
    let task = tokio::spawn(async move {
        let _ = host
            .serve(server, move |_, _| {
                count.fetch_add(1, Ordering::SeqCst);
                async move {
                    if wait {
                        std::future::pending::<()>().await;
                    }
                    Ok(ControlOutput::Acknowledged)
                }
            })
            .await;
    });
    write_frame(
        &mut client,
        &HostMessage::Hello {
            session_id: ticket.session_id.clone(),
            credential: ticket.credential.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(read_frame::<_, HostReply>(&mut client).await.unwrap(), HostReply::Ready);
    (client, task)
}
async fn send(client: &mut UnixStream, req: ComputerControlRequest, sequence: u64) -> HostReply {
    write_frame(client, &HostMessage::Action { sequence, request: req }).await.unwrap();
    read_frame(client).await.unwrap()
}

#[tokio::test]
async fn cc_control_04_real_ipc_handshake_action_and_replay_rejection() {
    let host = fixture();
    let ticket = host.issue(target(), Duration::from_secs(10)).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let (mut client, task) = connect(host.clone(), &ticket, count.clone(), false).await;
    assert!(matches!(
        send(&mut client, request(&ticket), 1).await,
        HostReply::Completed { outcome: ControlOutcome::Succeeded { .. }, .. }
    ));
    assert_eq!(
        send(&mut client, request(&ticket), 1).await,
        HostReply::Rejected { error: ControlErrorCode::InvalidRequest }
    );
    task.await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(host.session_count(), 0);
}

#[tokio::test]
async fn cc_control_04_wrong_credential_never_dispatches() {
    let host = fixture();
    let ticket = host.issue(target(), Duration::from_secs(10)).unwrap();
    let (mut client, server) = UnixStream::pair().unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    let task = tokio::spawn(async move {
        host.serve(server, move |_, _| {
            c.fetch_add(1, Ordering::SeqCst);
            async { Ok(ControlOutput::Acknowledged) }
        })
        .await
    });
    write_frame(
        &mut client,
        &HostMessage::Hello { session_id: ticket.session_id, credential: "wrong".into() },
    )
    .await
    .unwrap();
    assert_eq!(
        read_frame::<_, HostReply>(&mut client).await.unwrap(),
        HostReply::Rejected { error: ControlErrorCode::PermissionDenied }
    );
    task.await.unwrap().unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cc_control_04_cross_session_target_version_and_action_rejected() {
    for case in 0..4 {
        let host = fixture();
        let ticket = host.issue(target(), Duration::from_secs(10)).unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let (mut client, task) = connect(host, &ticket, count.clone(), false).await;
        let mut req = request(&ticket);
        let error = match case {
            0 => {
                req.session_id = ControlSessionId("other".into());
                ControlErrorCode::PermissionDenied
            }
            1 => {
                req.target = ComputerControlTarget::NativeApp {
                    bundle_id: "com.apple.Preview".into(),
                    process_id: 11,
                };
                ControlErrorCode::TargetChanged
            }
            2 => {
                req.permission_version = PermissionVersion(0);
                ControlErrorCode::StalePermission
            }
            _ => {
                req.action = ComputerControlAction::Word { command: WordCommand::ReadDocument };
                ControlErrorCode::InvalidRequest
            }
        };
        assert_eq!(send(&mut client, req, 1).await, HostReply::Rejected { error });
        task.await.unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn cc_control_04_expired_handshake_rejected() {
    let host = fixture();
    let ticket = host.issue(target(), Duration::from_millis(1)).unwrap();
    tokio::time::sleep(Duration::from_millis(5)).await;
    let (mut client, server) = UnixStream::pair().unwrap();
    let task = tokio::spawn(async move {
        host.serve(server, |_, _| async { panic!("expired request dispatched") }).await
    });
    write_frame(
        &mut client,
        &HostMessage::Hello { session_id: ticket.session_id, credential: ticket.credential },
    )
    .await
    .unwrap();
    assert_eq!(
        read_frame::<_, HostReply>(&mut client).await.unwrap(),
        HostReply::Rejected { error: ControlErrorCode::PermissionDenied }
    );
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn cc_control_04_disconnect_and_stop_cancel_pending_execution() {
    for stop in [false, true] {
        let host = fixture();
        let ticket = host.issue(target(), Duration::from_secs(10)).unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let (mut client, task) = connect(host.clone(), &ticket, count.clone(), true).await;
        write_frame(&mut client, &HostMessage::Action { sequence: 1, request: request(&ticket) })
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while count.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        if stop {
            let mut req = request(&ticket);
            req.action = ComputerControlAction::Stop;
            write_frame(&mut client, &HostMessage::Action { sequence: 2, request: req })
                .await
                .unwrap();
            assert!(matches!(
                read_frame::<_, HostReply>(&mut client).await.unwrap(),
                HostReply::Completed {
                    outcome: ControlOutcome::Rejected { error: ControlErrorCode::Cancelled },
                    ..
                }
            ));
        }
        drop(client);
        tokio::time::timeout(Duration::from_secs(2), task).await.unwrap().unwrap();
        assert_eq!(host.session_count(), 0);
    }
}

#[tokio::test]
async fn cc_control_04_expiry_cancels_pending_request() {
    let host = fixture();
    let ticket = host.issue(target(), Duration::from_secs(2)).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let (mut client, task) = connect(host.clone(), &ticket, count, true).await;
    let reply = send(&mut client, request(&ticket), 1).await;
    assert!(matches!(
        reply,
        HostReply::Completed {
            outcome: ControlOutcome::Rejected { error: ControlErrorCode::Cancelled },
            ..
        }
    ));
    task.await.unwrap();
    assert_eq!(host.session_count(), 0);
}

#[tokio::test]
async fn cc_control_04_ticket_cannot_attach_to_second_connection() {
    let host = fixture();
    let ticket = host.issue(target(), Duration::from_secs(10)).unwrap();
    let (client, first) =
        connect(host.clone(), &ticket, Arc::new(AtomicUsize::new(0)), false).await;
    let (mut other, server) = UnixStream::pair().unwrap();
    let second = tokio::spawn(async move {
        host.serve(server, |_, _| async { panic!("second connection dispatched") }).await
    });
    write_frame(
        &mut other,
        &HostMessage::Hello { session_id: ticket.session_id, credential: ticket.credential },
    )
    .await
    .unwrap();
    assert_eq!(
        read_frame::<_, HostReply>(&mut other).await.unwrap(),
        HostReply::Rejected { error: ControlErrorCode::PermissionDenied }
    );
    second.await.unwrap().unwrap();
    drop(client);
    first.await.unwrap();
}

#[tokio::test]
async fn cc_control_04_live_revocation_prevents_dispatch() {
    let mut policy = ComputerControlPolicy::new(ComputerControlConfig::default());
    // SAFETY: geteuid has no preconditions.
    let user = format!("uid:{}", unsafe { libc::geteuid() });
    policy.set_enabled(&user, ControlFeature::NativeApps, true).unwrap();
    let policy = Arc::new(RwLock::new(policy));
    let host = Arc::new(ControlHost::new(policy.clone(), Arc::new(AtomicBool::new(false))));
    let ticket = host.issue(target(), Duration::from_secs(10)).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let (mut client, task) = connect(host, &ticket, count.clone(), false).await;
    policy.write().set_enabled(&user, ControlFeature::NativeApps, false).unwrap();
    assert_eq!(
        send(&mut client, request(&ticket), 1).await,
        HostReply::Rejected { error: ControlErrorCode::StalePermission }
    );
    task.await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cc_control_04_trusted_stop_and_abort_cancel_backend_token() {
    for abort in [false, true] {
        let host = fixture();
        let ticket = host.issue(target(), Duration::from_secs(10)).unwrap();
        let (mut client, server) = UnixStream::pair().unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let serving = host.clone();
        let task = tokio::spawn(async move {
            serving
                .serve(server, move |_, token| {
                    tx.try_send(token).unwrap();
                    async {
                        std::future::pending::<Result<ControlOutput, ControlErrorCode>>().await
                    }
                })
                .await
        });
        write_frame(
            &mut client,
            &HostMessage::Hello {
                session_id: ticket.session_id.clone(),
                credential: ticket.credential.clone(),
            },
        )
        .await
        .unwrap();
        assert_eq!(read_frame::<_, HostReply>(&mut client).await.unwrap(), HostReply::Ready);
        write_frame(&mut client, &HostMessage::Action { sequence: 1, request: request(&ticket) })
            .await
            .unwrap();
        let token = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
        if abort {
            task.abort();
        } else {
            host.stop(&ticket.session_id);
        }
        let _ = tokio::time::timeout(Duration::from_secs(2), task).await.unwrap();
        assert!(token.is_cancelled());
        assert_eq!(host.session_count(), 0);
    }
}

#[tokio::test]
async fn cc_control_04_invalid_initial_sequence_never_dispatches() {
    let host = fixture();
    let ticket = host.issue(target(), Duration::from_secs(10)).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let (mut client, task) = connect(host, &ticket, count.clone(), false).await;
    assert_eq!(
        send(&mut client, request(&ticket), 0).await,
        HostReply::Rejected { error: ControlErrorCode::InvalidRequest }
    );
    task.await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 0);
}
