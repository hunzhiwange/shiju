use super::launch::*;
use vw_gateway_client::vw_api_types::agent_apps::*;
fn draft() -> LaunchState {
    LaunchState {
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
    assert!(
        state.finish(generation, Ok(LaunchResult { installation_id: request.installation_id }))
    );
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
    assert!(
        !state.finish(generation, Ok(LaunchResult { installation_id: request.installation_id }))
    );
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
        Ok(LaunchResult { installation_id: AppInstallationId("other".into()) }),
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
        result: Ok(LaunchResult { installation_id: AppInstallationId("remote-instance".into()) }),
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
