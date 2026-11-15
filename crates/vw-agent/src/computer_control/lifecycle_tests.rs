use super::*;
use super::super::policy::ControlFeature;
use vw_config_types::computer_control::ComputerControlConfig;

fn fixture() -> (ControlHost, SessionTicket, ComputerControlRequest) {
    let user = format!("uid:{}", unsafe { libc::geteuid() });
    let mut policy = ComputerControlPolicy::new(ComputerControlConfig::default());
    policy.set_enabled(&user, ControlFeature::NativeApps, true).unwrap();
    let host = ControlHost::new(Arc::new(RwLock::new(policy)), Arc::new(AtomicBool::new(false)));
    let target = ComputerControlTarget::NativeApp {
        bundle_id: "com.apple.TextEdit".into(), process_id: 10,
    };
    let ticket = host.issue(target.clone(), Duration::from_secs(60)).unwrap();
    let request = ComputerControlRequest {
        request_id: ControlRequestId("lifecycle".into()),
        session_id: ticket.session_id.clone(), permission_version: ticket.permission_version,
        target, action: ComputerControlAction::Stop,
    };
    (host, ticket, request)
}

#[test]
fn cc_control_20_stop_invalidates_ticket() {
    let (host, ticket, _) = fixture();
    host.stop(&ticket.session_id);
    assert!(host.attach(&ticket.session_id, &ticket.credential).is_err());
}

#[test]
fn cc_control_20_stop_cancels_attached_session() {
    let (host, ticket, _) = fixture();
    let (_, cancel) = host.attach(&ticket.session_id, &ticket.credential).unwrap();
    host.stop(&ticket.session_id);
    assert!(cancel.is_cancelled());
}

#[test]
fn cc_control_20_repeated_stop_is_idempotent() {
    let (host, ticket, _) = fixture();
    host.stop(&ticket.session_id);
    host.stop(&ticket.session_id);
    assert_eq!(host.session_count(), 0);
}

#[test]
fn cc_control_20_stop_preserves_other_session() {
    let (host, ticket, request) = fixture();
    let other = host.issue(request.target, Duration::from_secs(60)).unwrap();
    host.stop(&ticket.session_id);
    let (_, cancel) = host.attach(&other.session_id, &other.credential).unwrap();
    assert!(!cancel.is_cancelled());
    assert_eq!(host.session_count(), 1);
}

#[test]
fn cc_control_20_revoked_permission_still_allows_authenticated_stop() {
    let (host, ticket, request) = fixture();
    host.attach(&ticket.session_id, &ticket.credential).unwrap();
    host.policy.write().set_enabled(&host.local_user, ControlFeature::NativeApps, false).unwrap();
    assert_eq!(host.authorize(&ticket.session_id, 1, &request), Ok(()));
}

#[test]
fn cc_control_20_stop_rejects_cross_session_identity() {
    let (host, ticket, mut request) = fixture();
    request.session_id = ControlSessionId("other".into());
    assert_eq!(host.authorize(&ticket.session_id, 1, &request), Err(ControlErrorCode::PermissionDenied));
}

#[test]
fn cc_control_20_stopped_session_rejects_queued_action() {
    let (host, ticket, request) = fixture();
    host.stop(&ticket.session_id);
    assert_eq!(host.authorize(&ticket.session_id, 1, &request), Err(ControlErrorCode::Disconnected));
}

#[test]
fn cc_control_20_stop_rejects_replayed_sequence() {
    let (host, ticket, request) = fixture();
    assert_eq!(host.authorize(&ticket.session_id, 1, &request), Ok(()));
    assert_eq!(host.authorize(&ticket.session_id, 1, &request), Err(ControlErrorCode::InvalidRequest));
}
