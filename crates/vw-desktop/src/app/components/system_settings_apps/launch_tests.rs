use vw_api_app_identity::{
    AppId,
    AppInstallationId,
    LaunchIdentity,
    LaunchIdentityKind,
    LaunchRequest,
    LaunchResult,
    LaunchSessionPhase,
};
use super::launch::*;
use vw_api_types::agent_apps::*;
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
