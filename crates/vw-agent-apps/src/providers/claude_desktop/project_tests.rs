use super::{publication::*, *};
use crate::providers::{
    desktop_mapping::catalog::{DesktopMappingMember, DesktopModelCatalog},
    live::commit::{ExecutionHostId, HOOK, PendingTarget, Phase, ProviderTransactions},
    mode_state::StackState,
};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

#[test]
fn aggregation_t46_desktop_project_publish_and_recovery() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let project = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
    let other = ProjectScope::new(tenant.clone(), "other".into()).unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let app = AppId::ClaudeDesktop;
    let mut files = ClaudeDesktopFiles::open(home.path()).unwrap();
    tx.commit_project_checked(
        &project,
        &mut files.guard,
        &tx.state(app).unwrap(),
        true,
        &[],
        PendingTarget::default(),
        || Ok(()),
    )
    .unwrap();
    drop(files);
    let before = tx.state(app).unwrap();
    let mapping = vw_api_types::app_providers::desktop_mapping::DesktopMapping {
        mode: vw_api_types::app_providers::desktop_mapping::DesktopMappingMode::Proxy,
        routes: [(
            "claude-sonnet-5".into(),
            vw_api_types::app_providers::desktop_mapping::DesktopModelRoute {
                model: "upstream-model".into(),
                label_override: None,
                supports_1m: false,
            },
        )]
        .into(),
    };
    let catalog = DesktopModelCatalog::new(
        DesktopMappingMember {
            provider_id: "alpha",
            key: "alpha",
            name: "Alpha",
            mapping: &mapping,
        },
        &[],
    )
    .unwrap();
    let gateway = ClaudeDesktopProfile {
        name: "Gateway".into(),
        config: ClaudeDesktopConfig::ThirdParty {
            base_url: "http://127.0.0.1:31911/claude-desktop".into(),
            auth_token: "fixture-private-token".into(),
        },
    };
    let target = || DesktopTarget::Route {
        gateway: &gateway,
        catalog: &catalog,
        stack: Some(StackState {
            enabled: true,
            members: vec!["alpha".into()],
            keys: [("alpha".into(), "alpha".into())].into(),
        }),
    };
    run(home.path(), DesktopInstallation::Installed, &tx, &tenant, &before, target(), || true)
        .unwrap();
    let active = tx.state(app).unwrap();
    assert_eq!(active.project_scope.as_ref(), Some(&project));
    assert!(active.stack.enabled);
    HOOK.with_borrow_mut(|hook| {
        *hook =
            Some(Box::new(
                |_, _, phase| {
                    if phase == Phase::FilePublished { Err(WriteError::Io) } else { Ok(()) }
                },
            ))
    });
    let result = run(
        home.path(),
        DesktopInstallation::Installed,
        &tx,
        &tenant,
        &active,
        DesktopTarget::Restore(None),
        || true,
    );
    HOOK.with_borrow_mut(|hook| *hook = None);
    assert_eq!(result, Err(DesktopPublicationError::TargetFailed));
    assert!(tx.pending(app).unwrap().is_some());
    let mut files = ClaudeDesktopFiles::open(home.path()).unwrap();
    assert!(tx.recover_project(&other, &mut files.guard).is_err());
    assert!(tx.pending(app).unwrap().is_some());
    tx.recover_project(&project, &mut files.guard).unwrap();
    drop(files);
    let restored = tx.state(app).unwrap();
    assert_eq!(restored.project_scope.as_ref(), Some(&project));
    assert!(!restored.stack.enabled);
    assert!(tx.pending(app).unwrap().is_none());
    assert_eq!(
        get_status(home.path(), DesktopInstallation::Installed, Some(&tx), &tenant).unwrap().mode,
        DesktopMode::Official
    );
}
