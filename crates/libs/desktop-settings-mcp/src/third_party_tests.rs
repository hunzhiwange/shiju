use crate::management_context::Context;
use crate::management_state::McpSettingsState;
use crate::management_update::Update;
use crate::third_party::{self, Action, Message, Scope};
use desktop_provider_apps::AgentAppsContext;
use vw_api_app_identity::AppId;
use vw_api_mcp_management::McpAppTarget;
use vw_api_transport::TenantScope;

fn scope() -> Scope {
    Scope {
        context: Context {
            endpoint: AgentAppsContext {
                gateway: "http://localhost:1".into(),
                tenant: TenantScope::new("10000", "100000001").unwrap(),
                project: None,
            },
            host: "host".into(),
            credentials_generation: 1,
        },
        app_id: AppId::Claude,
        generation: 1,
    }
}

#[test]
fn third_party_task_03_scope_and_navigation() {
    let current = scope();
    let mut state = McpSettingsState::default();
    let mut app = Update {
        state: &mut state,
        context: Some(current.context.clone()),
        authorized: true,
        token: String::new(),
        client: Err("isolated".into()),
        targets: Vec::new(),
        effects: Vec::new(),
    };
    let mut previous = current.clone();
    previous.generation = 0;
    let _ = third_party::update(
        &mut app,
        &current,
        Message { scope: previous, action: Action::Refresh },
    );
    assert!(app.effects.is_empty());
    let target = McpAppTarget {
        app_id: AppId::Codex,
        installation_id: vw_api_app_identity::AppInstallationId("fixture".into()),
        target_host_id: current.context.host.clone(),
    };
    let _ = third_party::update(
        &mut app,
        &current,
        Message {
            scope: current.clone(),
            action: Action::Bindings(crate::management_bindings::BindingsMessage::Apply(
                target, true,
            )),
        },
    );
    assert_eq!(app.effects.len(), 1);
    assert!(!app.state.bindings.running);
    assert!(app.state.bindings.context.is_none());
    assert!(app.state.catalog.context.is_none());
}

#[test]
fn third_party_task_03_target_requires_application_and_host() {
    let scope = scope();
    let mut target = McpAppTarget {
        app_id: scope.app_id,
        installation_id: vw_api_app_identity::AppInstallationId("fixture".into()),
        target_host_id: scope.context.host.clone(),
    };
    assert!(scope.contains(&target));
    target.target_host_id = "other-host".into();
    assert!(!scope.contains(&target));
    target.target_host_id = scope.context.host.clone();
    target.app_id = AppId::Codex;
    assert!(!scope.contains(&target));
}
