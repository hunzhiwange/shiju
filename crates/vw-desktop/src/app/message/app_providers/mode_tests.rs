use crate::app::state::app_providers::mode::{Form, Operation, Outcome};
use vw_gateway_client::{
    AgentAppsReadError,
    vw_api_types::app_providers::mode_state::{
        ProviderMode, ProviderModeStatus, RouteRuntimeStatus, StackStatus,
    },
};

fn status(mode: ProviderMode) -> ProviderModeStatus {
    ProviderModeStatus {
        mode,
        direct_provider: Some("直连供应商".into()),
        in_use_provider: Some("路由供应商".into()),
        proxy_route: Some("路由供应商".into()),
        attached: mode == ProviderMode::Proxy,
        has_contract: true,
        stack: StackStatus { enabled: false, active: false, members: vec![] },
        decision: None,
        route_runtime: RouteRuntimeStatus::Unavailable,
    }
}

#[test]
fn cc04_ui_mode_validation_failure_preserves_draft() {
    let observed = status(ProviderMode::Proxy);
    let mut form = Form {
        status: Some(observed.clone()),
        confirming_direct: true,
        pending: Some((3, Operation::RestoreDirect)),
        ..Form::default()
    };
    assert!(
        form.finish(
            3,
            Err(AgentAppsReadError::Http { status: 422, body: serde_json::Value::Null })
        )
        .is_none()
    );
    assert!(form.confirming_direct, "执行端拒绝后必须保留恢复直连的确认草稿");
    assert_eq!(form.status, Some(observed));
    assert!(form.pending.is_none());
    assert!(form.error.is_some());
}

#[test]
fn cc04_ui_mode_stale_response_is_ignored() {
    let observed = status(ProviderMode::Proxy);
    let mut form = Form {
        status: Some(observed.clone()),
        confirming_direct: true,
        pending: Some((9, Operation::RestoreDirect)),
        ..Form::default()
    };
    assert!(form.finish(8, Ok(Outcome::Observed(status(ProviderMode::Direct)))).is_none());
    assert_eq!(form.status, Some(observed), "过期响应不能覆盖最新界面状态");
    assert_eq!(form.pending, Some((9, Operation::RestoreDirect)));
    assert!(form.confirming_direct);
    assert!(form.error.is_none());
}

#[test]
fn cc04_ui_mode_conflict_requires_a_fresh_status_before_retry() {
    let mut form = Form {
        status: Some(status(ProviderMode::Proxy)),
        confirming_direct: true,
        pending: Some((4, Operation::RestoreDirect)),
        ..Form::default()
    };
    assert!(
        form.finish(
            4,
            Err(AgentAppsReadError::Http { status: 409, body: serde_json::Value::Null })
        )
        .is_none()
    );
    assert!(form.confirming_direct);
    assert!(!form.can_restore_direct());
    form.pending = Some((5, Operation::Read));
    assert!(form.finish(5, Ok(Outcome::Observed(status(ProviderMode::Direct)))).is_none());
    assert!(!form.can_restore_direct(), "已恢复直连后不能继续提交");
}
