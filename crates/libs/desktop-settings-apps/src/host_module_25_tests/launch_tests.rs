use super::{launch_fixture, launch_route as route};
use crate::system_settings_apps::launch::{LaunchChoice, LaunchState};
use vw_api_app_identity::{
    AppId, AppInstallationId, LaunchIdentity, LaunchIdentityKind, LaunchRequest, LaunchResult,
    LaunchSessionPhase,
};
type LaunchMessage =
    crate::system_settings_apps::launch::LaunchMessage<desktop_apps_ui::AgentAppsContext>;
type ReplyScope =
    crate::system_settings_apps::launch::ReplyScope<desktop_apps_ui::AgentAppsContext>;

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
    let (mut app, context) = launch_fixture();
    app.context = Some(context.clone());
    app.host = "中文主机".into();
    app.token = "isolated-fixture-grant".into();
    app.launch = draft();
    app.launch.args = vec!["private-material".into(), "中文第二参数".into()];
    assert!(!format!("{:?}", app.launch).contains("private-material"));
    let _ = route(&mut app, LaunchMessage::AddArg);
    let _ = route(&mut app, LaunchMessage::Args(2, "可移除参数".into()));
    assert_eq!(app.launch.args[2], "可移除参数");
    let _ = route(&mut app, LaunchMessage::RemoveArg(2));
    let _ = route(&mut app, LaunchMessage::Submit);
    assert!(app.launch.busy);
    assert_eq!(app.launch.request.as_ref().unwrap().args, ["private-material", "中文第二参数"]);
    let generation = app.launch.generation;
    let cg = app.generation;
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
    assert!(!app.launch.accepted);
    let _ = route(&mut app, reply("中文主机", cg));
    assert!(app.launch.accepted);
    app.input().credentials(Some("other".into()), None);
    let _ = route(&mut app, reply("中文主机", cg));
    assert!(app.launch.request.is_none());
}

#[test]
fn cc_apps_desktop_launch_state_identity_reply_scope_and_no_forged_selection() {
    let (mut app, context) = launch_fixture();
    app.context = Some(context.clone());
    app.host = "主机".into();
    app.launch = draft();
    app.launch.identities_loading = true;
    app.launch.identities_ready = false;
    app.launch.selected = None;
    let scope = ReplyScope {
        context,
        host: "主机".into(),
        catalog_generation: app.generation,
        generation: app.launch.generation,
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
    assert!(app.launch.identities_loading);
    let _ =
        route(&mut app, LaunchMessage::Identities { scope, result: Ok(vec![identity.clone()]) });
    assert!(app.launch.identities_ready);
    assert!(app.launch.selected.is_none(), "loading must not implicitly select an account");
    let mut forged = identity.clone();
    forged.id = "missing".into();
    let _ = route(&mut app, LaunchMessage::Select(LaunchChoice::Identity(forged)));
    assert!(app.launch.selected.is_none());
    let _ = route(&mut app, LaunchMessage::Select(LaunchChoice::Identity(identity.clone())));
    assert_eq!(app.launch.selected, Some(LaunchChoice::Identity(identity)));
}
