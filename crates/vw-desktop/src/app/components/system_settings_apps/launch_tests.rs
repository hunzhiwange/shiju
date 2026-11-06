use super::launch::*;
use vw_gateway_client::vw_api_types::agent_apps::*;
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
fn cc_apps_desktop_launch_state_message_scope_and_redacted_diagnostics() {
    use crate::app::message::{SettingsMessage, settings::AppsCatalogMessage};
    let mut app = crate::app::App::new().0;
    let client = crate::app::config::gateway_client().unwrap();
    let context = crate::app::state::AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    app.apps_catalog.context = Some(context.clone());
    app.apps_catalog.host = "中文主机".into();
    app.apps_catalog.token = "isolated-fixture-grant".into();
    app.apps_catalog.launch = draft();
    app.apps_catalog.launch.args = vec!["private-material".into(), "中文第二参数".into()];
    assert!(!format!("{:?}", app.apps_catalog.launch).contains("private-material"));
    let route = |app: &mut crate::app::App, value| {
        crate::app::message::settings::update(
            app,
            SettingsMessage::AppsCatalog(AppsCatalogMessage::Launch(value)),
        )
    };
    let _ = route(&mut app, LaunchMessage::AddArg);
    let _ = route(&mut app, LaunchMessage::Args(2, "可移除参数".into()));
    assert_eq!(app.apps_catalog.launch.args[2], "可移除参数");
    let _ = route(&mut app, LaunchMessage::RemoveArg(2));
    let _ = route(&mut app, LaunchMessage::Submit);
    assert!(app.apps_catalog.launch.busy);
    assert_eq!(
        app.apps_catalog.launch.request.as_ref().unwrap().args,
        ["private-material", "中文第二参数"]
    );
    let generation = app.apps_catalog.launch.generation;
    let cg = app.apps_catalog.generation;
    let reply = |host: &str, catalog_generation| LaunchMessage::Reply {
        context: context.clone(),
        host: host.into(),
        catalog_generation,
        generation,
        result: Ok(LaunchResult {
            session_id: None,
            installation_id: AppInstallationId("remote-instance".into()),
        }),
    };
    let _ = route(&mut app, reply("wrong-host", cg));
    let _ = route(&mut app, reply("中文主机", cg + 1));
    assert!(!app.apps_catalog.launch.accepted);
    let _ = route(&mut app, reply("中文主机", cg));
    assert!(app.apps_catalog.launch.accepted);
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::AppsCatalog(AppsCatalogMessage::HostInput("other".into())),
    );
    let _ = route(&mut app, reply("中文主机", cg));
    assert!(app.apps_catalog.launch.request.is_none());
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
fn cc_apps_desktop_launch_state_identity_reply_scope_and_no_forged_selection() {
    use crate::app::message::{SettingsMessage, settings::AppsCatalogMessage};
    let mut app = crate::app::App::new().0;
    let client = crate::app::config::gateway_client().unwrap();
    let context = crate::app::state::AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    app.apps_catalog.context = Some(context.clone());
    app.apps_catalog.host = "主机".into();
    app.apps_catalog.launch = draft();
    app.apps_catalog.launch.identities_loading = true;
    app.apps_catalog.launch.identities_ready = false;
    app.apps_catalog.launch.selected = None;
    let route = |app: &mut crate::app::App, value| {
        crate::app::message::settings::update(
            app,
            SettingsMessage::AppsCatalog(AppsCatalogMessage::Launch(value)),
        )
    };
    let scope = ReplyScope {
        context,
        host: "主机".into(),
        catalog_generation: app.apps_catalog.generation,
        generation: app.apps_catalog.launch.generation,
    };
    let identity = LaunchIdentity {
        kind: LaunchIdentityKind::OfficialAccount,
        id: "account".into(),
        label: "中文账号".into(),
    };
    let mut stale = scope.clone();
    stale.host = "其他主机".into();
    let _ = route(
        &mut app,
        LaunchMessage::Identities { scope: stale, result: Ok(vec![identity.clone()]) },
    );
    assert!(app.apps_catalog.launch.identities_loading);
    let _ =
        route(&mut app, LaunchMessage::Identities { scope, result: Ok(vec![identity.clone()]) });
    assert!(app.apps_catalog.launch.identities_ready);
    assert!(
        app.apps_catalog.launch.selected.is_none(),
        "loading must not implicitly select an account"
    );
    let mut forged = identity.clone();
    forged.id = "missing".into();
    let _ = route(&mut app, LaunchMessage::Select(LaunchChoice::Identity(forged)));
    assert!(app.apps_catalog.launch.selected.is_none());
    let _ = route(&mut app, LaunchMessage::Select(LaunchChoice::Identity(identity.clone())));
    assert_eq!(app.apps_catalog.launch.selected, Some(LaunchChoice::Identity(identity)));
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
    let tenant = vw_gateway_client::vw_api_types::id::TenantScope::new(
        "fixture-platform",
        "fixture-company",
    )
    .unwrap();
    let credentials =
        vw_shared::local_apps::initialize(home.path(), "fixture-platform", "fixture-company")
            .unwrap();
    let directory =
        |tenant: &vw_gateway_client::vw_api_types::id::TenantScope, token: &str, project| {
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
        vw_gateway_client::vw_api_types::id::TenantScope::new("fixture-platform", "other-company")
            .unwrap();
    assert!(directory(&other, &credentials.token, None).is_none());
    let remote = vw_gateway_client::GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "remote.example",
        42617,
    ))
    .unwrap();
    assert!(
        super::local::launch_directory(
            &remote,
            vw_shared::local_apps::HOST_ID,
            &credentials.token,
            None
        )
        .is_none()
    );
}
