use super::*;

fn context() -> Context {
    Context {
        gateway: AgentAppsContext {
            gateway: "http://127.0.0.1:1".into(),
            tenant: Default::default(),
            project: None,
        },
        host: "fixture".into(),
        generation: 1,
        access: 1,
    }
}
fn status(state: RoutingState) -> RoutingStatus {
    RoutingStatus {
        state,
        bound_address: Some("127.0.0.1".parse().unwrap()),
        bound_port: Some(43123),
        started_at: None,
    }
}

#[test]
fn cc_route_088_ui_lifecycle_expected_behavior() {
    let context = context();
    let mut state =
        State { context: Some(context.clone()), serial: 1, busy: true, ..Default::default() };
    assert!(state.finish(
        &context,
        1,
        Ok((status(RoutingState::Running), RoutingHealth { ready: true }))
    ));
    assert_eq!(state.status.as_ref().unwrap().bound_port, Some(43123));
    assert!(can_control(&state, Operation::Stop));
    assert!(!can_control(&state, Operation::Start));
    state.serial = 2;
    state.busy = true;
    assert!(state.finish(&context, 2, Err("停止失败，状态未知".into())));
    assert!(state.status.is_none() && state.health.is_none());
    assert_eq!(state.error.as_deref(), Some("停止失败，状态未知"));
    assert!(!can_control(&state, Operation::Stop));
}

#[test]
fn cc_route_088_stale_host_access_and_serial_are_rejected() {
    let context = context();
    let mut state =
        State { context: Some(context.clone()), serial: 9, busy: true, ..Default::default() };
    let mut other = context.clone();
    other.host = "other".into();
    assert!(!state.finish(
        &other,
        9,
        Ok((status(RoutingState::Running), RoutingHealth { ready: true }))
    ));
    other = context.clone();
    other.access += 1;
    assert!(!state.finish(&other, 9, Err("stale".into())));
    assert!(!state.finish(&context, 8, Err("stale".into())));
    assert!(state.busy && state.status.is_none() && state.error.is_none());
    state.invalidate();
    assert!(!state.finish(
        &context,
        9,
        Ok((status(RoutingState::Running), RoutingHealth { ready: true }))
    ));
}

#[test]
fn cc_route_088_errors_do_not_render_server_payloads() {
    use vw_gateway_client::vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
    let message = error_message(AppRoutingError::Http {
        status: 409,
        body: Some(ApiErrorBody {
            error: ApiErrorDetail {
                code: "address_in_use".into(),
                message: "fixture-sensitive-payload".into(),
                details: None,
            },
        }),
    });
    assert!(message.contains("监听地址已占用"));
    assert!(!message.contains("fixture-sensitive-payload"));
}
