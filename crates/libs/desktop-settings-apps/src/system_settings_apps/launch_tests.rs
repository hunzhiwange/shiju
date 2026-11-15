use vw_api_app_identity::{
    AppId,
    AppInstallationId,
    LaunchRequest,
    LaunchResult,
    LaunchSessionPhase,
};
use super::launch::*;
fn draft() -> LaunchState {
    LaunchState {
        identities_ready: true,
        selected: Some(LaunchChoice::ExistingConfiguration),
        request: Some(LaunchRequest {
            app_id: AppId::Codex,
            installation_id: AppInstallationId("remote-instance".into()),
            cwd: "C:\\远端项目".into(),
            args: vec!["中文材料；$(echo unsafe)".into()],
            skip_permissions: false,
        }),
        ..Default::default()
    }
}
#[test]
fn cc_apps_desktop_launch_state_success_requires_matching_instance() {
    let mut state = draft();
    let (generation, request) = state.begin().unwrap();
    assert!(!request.skip_permissions);
    assert_eq!(request.cwd, "C:\\远端项目");
    assert!(state.begin().is_none());
    assert!(state.finish(
        generation,
        Ok(LaunchResult { session_id: None, installation_id: request.installation_id })
    ));
    assert!(state.accepted);
    assert!(state.begin().is_none());
}

#[test]
fn cc_apps_desktop_launch_state_failure_and_invalidation_preserve_selection() {
    let mut state = draft();
    let (generation, request) = state.begin().unwrap();
    assert!(!state.finish(generation + 1, Err("过期拒绝".into())));
    assert!(state.finish(generation, Err("启动未确认，不会自动重试".into())));
    assert!(!state.accepted);
    assert_eq!(state.request.as_ref(), Some(&request));
    assert!(state.error.is_some());
    state.invalidate();
    assert!(!state.finish(
        generation,
        Ok(LaunchResult { session_id: None, installation_id: request.installation_id })
    ));
    assert!(state.request.is_none());
}

#[test]
fn cc_apps_desktop_launch_state_wrong_response_and_empty_cwd_reject() {
    let mut state = draft();
    state.request.as_mut().unwrap().cwd.clear();
    assert!(state.begin().is_none());
    state.request.as_mut().unwrap().cwd = "/远端目录".into();
    let (generation, _) = state.begin().unwrap();
    state.finish(
        generation,
        Ok(LaunchResult { session_id: None, installation_id: AppInstallationId("other".into()) }),
    );
    assert!(!state.accepted);
    assert!(state.error.is_some());
}

#[test]
fn cc_apps_desktop_launch_state_identity_loading_empty_and_explicit_selection() {
    let mut state = draft();
    state.selected = None;
    assert!(state.begin().is_none(), "identity selection must be explicit");
    state.selected = Some(LaunchChoice::ExistingConfiguration);
    state.identities_ready = false;
    state.identities_loading = true;
    assert!(state.begin().is_none(), "loading must not fall back to host config");
    state.identities_loading = false;
    assert!(state.begin().is_none(), "failed identity read must keep launch disabled");
    state.identities_ready = true;
    assert!(
        state.begin().is_some(),
        "empty identity list still requires explicit existing-config selection"
    );
}

#[test]
fn cc_apps_desktop_launch_state_session_completion_is_not_spawn_success() {
    let mut state = draft();
    let (generation, request) = state.begin().unwrap();
    assert!(state.finish(
        generation,
        Ok(LaunchResult {
            installation_id: request.installation_id,
            session_id: Some("session-fixture".into())
        })
    ));
    assert_eq!(state.phase, Some(LaunchSessionPhase::Running));
    assert_ne!(state.phase, Some(LaunchSessionPhase::Completed));
    state.invalidate();
    assert!(state.session_id.is_none());
}

#[test]
fn cc_apps_desktop_launch_state_explains_disabled_action_and_blocks_uncertain_replay() {
    let mut state = draft();
    state.selected = None;
    assert_eq!(state.blocked_reason(), Some("请选择启动身份。"));
    state.selected = Some(LaunchChoice::ExistingConfiguration);
    state.request.as_mut().unwrap().cwd.clear();
    assert_eq!(state.blocked_reason(), Some("请填写工作目录。"));
    state.request.as_mut().unwrap().cwd = "/fixture/project".into();
    assert!(state.blocked_reason().is_none());
    let (generation, _) = state.begin().unwrap();
    state.finish(generation, Err("启动结果不明".into()));
    assert!(state.blocked_reason().is_some());
    assert!(state.begin().is_none());
}

#[cfg(unix)]
#[test]
fn cc_apps_desktop_launch_state_defaults_only_for_matching_private_local_grant() {
    let home = tempfile::tempdir().unwrap();
    let tenant = vw_api_transport::TenantScope::new(
        "fixture-platform",
        "fixture-company",
    )
    .unwrap();
    let credentials =
        local_app_auth::initialize(home.path(), "fixture-platform", "fixture-company")
            .unwrap();
    let directory =
        |tenant: &vw_api_transport::TenantScope, token: &str, project| {
            super::local::verified_launch_directory(home.path(), tenant, token, project)
        };
    assert_eq!(
        directory(&tenant, &credentials.token, None),
        Some(home.path().to_string_lossy().into_owned())
    );
    assert_eq!(
        directory(&tenant, &credentials.token, Some("/fixture/project")),
        Some("/fixture/project".into())
    );
    assert!(directory(&tenant, "unrelated-credential", None).is_none());
    let other =
        vw_api_transport::TenantScope::new("fixture-platform", "other-company")
            .unwrap();
    assert!(directory(&other, &credentials.token, None).is_none());
    let remote = vw_gateway_client::GatewayClient::new(gateway_endpoint::GatewayEndpoint::new(
        "remote.example",
        42617,
    ))
    .unwrap();
    assert!(
        super::local::launch_directory(
            &remote,
            local_app_auth::HOST_ID,
            &credentials.token,
            None
        )
        .is_none()
    );
}
