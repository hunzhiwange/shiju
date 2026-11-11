use super::preview::*;
use crate::providers::{
    live::{
        commit::{ExecutionHostId, Mode, ModeState, PendingTarget, ProviderTransactions},
        write_guard::lock_app,
    },
    mode_state::StackState,
};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

fn snapshot(path: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut result = vec![];
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(snapshot(&path));
        } else {
            result.push((path.clone(), fs::read(path).unwrap()));
        }
    }
    result.sort();
    result
}

#[test]
fn aggregation_t13_preview_preserves_contract() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let live = tempfile::tempdir().unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    tx.commit(
        &tenant,
        &mut guard,
        &[],
        PendingTarget {
            mode: Some(ModeState {
                mode: Mode::Direct,
                attached: false,
                proxy_route: Some("beta".into()),
                contract: None,
            }),
            stack: Some(StackState {
                enabled: false,
                members: vec!["alpha".into(), "beta".into()],
                keys: [("alpha".into(), "alpha".into()), ("beta".into(), "beta".into())].into(),
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let alpha =
        json!({"name":"Alpha", "meta":{"stackModels":[{"model":"same"},{"model":"same[1M]"}]}});
    let beta = json!({"name":"Beta", "settingsConfig":{"env":{"ANTHROPIC_MODEL":"other"}}});
    let providers = [
        Provider {
            id: "alpha",
            name: "Alpha",
            official_service: None,
            catalog: Catalog::Claude(&alpha),
        },
        Provider {
            id: "beta",
            name: "Beta",
            official_service: None,
            catalog: Catalog::Claude(&beta),
        },
    ];
    let before = snapshot(root.path());
    let state = tx.state(AppId::Claude).unwrap();
    let output = preview(
        AppId::Claude,
        &tenant,
        &state,
        PreviewInput {
            candidate_default: None,
            providers: &providers,
            pending_tenant: None,
            failover_enabled: false,
            direct_restorable: true,
            catalog_policy: CatalogPolicy::Preserve,
        },
    )
    .unwrap();
    assert_eq!(output.default_provider_id.as_deref(), Some("beta"));
    assert_eq!(
        output.members.iter().map(|m| m.provider_id.as_str()).collect::<Vec<_>>(),
        ["beta", "alpha"]
    );
    assert_eq!(output.members[1].models.len(), 1);
    assert!(output.members[1].models[0].supports_1m);
    assert_eq!(output.notices, [Notice::ModelReselect]);
    assert!(output.issues.is_empty());
    assert_eq!(snapshot(root.path()), before);
    assert!(tx.state(AppId::Claude).unwrap() == state);
    assert!(tx.pending(AppId::Claude).unwrap().is_none());
}

fn state(tenant: &TenantScope) -> crate::providers::live::commit::AppState {
    crate::providers::live::commit::AppState {
        stack: StackState {
            enabled: false,
            members: vec!["alpha".into(), "beta".into()],
            keys: [("alpha".into(), "alpha".into()), ("beta".into(), "beta".into())].into(),
        },
        stack_tenant: Some(tenant.clone()),
        mode: Some(ModeState {
            mode: Mode::Direct,
            attached: false,
            proxy_route: Some("alpha".into()),
            contract: None,
        }),
        mode_tenant: Some(tenant.clone()),
        ..Default::default()
    }
}
fn input<'a>(providers: &'a [Provider<'a>]) -> PreviewInput<'a> {
    PreviewInput {
        candidate_default: None,
        providers,
        pending_tenant: None,
        failover_enabled: false,
        direct_restorable: true,
        catalog_policy: CatalogPolicy::Preserve,
    }
}

#[test]
fn aggregation_t13_refuses_foreign_pending_and_unsupported_snapshots() {
    let tenant = TenantScope::new("platform", "company").unwrap();
    let other = TenantScope::new("platform", "other").unwrap();
    let state = state(&tenant);
    assert_eq!(preview(AppId::Claude, &other, &state, input(&[])), Err(PreviewError::Forbidden));
    for (owner, error) in
        [(&tenant, PreviewError::RecoveryPending), (&other, PreviewError::Forbidden)]
    {
        let mut request = input(&[]);
        request.pending_tenant = Some(owner);
        assert_eq!(preview(AppId::Claude, &tenant, &state, request), Err(error));
    }
    assert_eq!(
        preview(AppId::Pi, &tenant, &state, input(&[])),
        Err(PreviewError::UnsupportedTarget)
    );
}

#[test]
fn aggregation_t13_reports_conflicts_without_allocating_keys_or_switching_modes() {
    let tenant = TenantScope::new("platform", "company").unwrap();
    let state = state(&tenant);
    let record = json!({"name":"New", "meta":{"stackModels":[]}});
    let providers = [Provider {
        id: "new",
        name: "New",
        official_service: None,
        catalog: Catalog::Claude(&record),
    }];
    let mut request = input(&providers);
    request.candidate_default = Some("new");
    request.failover_enabled = true;
    request.direct_restorable = false;
    let output = preview(AppId::Claude, &tenant, &state, request).unwrap();
    for code in [
        IssueCode::InvalidDefault,
        IssueCode::InvalidMember,
        IssueCode::FailoverEnabled,
        IssueCode::UnrestorableDirect,
    ] {
        assert!(output.issues.iter().any(|issue| issue.code == code && issue.blocking));
    }
    assert!(!state.stack.keys.values().any(|id| id == "new"));
    assert!(state.mode.unwrap().mode == Mode::Direct);
}

#[test]
fn aggregation_t13_codex_uses_native_catalog_and_preserve_notice() {
    use super::codex_catalog::CodexCatalogRow;
    let tenant = TenantScope::new("platform", "company").unwrap();
    let state = state(&tenant);
    let config = "model = 'native-model'\n".parse().unwrap();
    let providers = ["alpha", "beta"].map(|id| Provider {
        id,
        name: id,
        official_service: None,
        catalog: Catalog::Codex(CodexCatalogRow { catalog: None, config: &config }),
    });
    let output = preview(AppId::Codex, &tenant, &state, input(&providers)).unwrap();
    assert!(output.issues.is_empty());
    assert_eq!(output.members[0].models[0].id, "native-model");
    assert_eq!(output.members[1].models[0].id, "ccs-beta/native-model");
    assert!(output.notices.contains(&Notice::CodexRouteOwnsCatalog));
    let mut request = input(&providers);
    request.catalog_policy = CatalogPolicy::UseManaged;
    assert!(
        !preview(AppId::Codex, &tenant, &state, request)
            .unwrap()
            .notices
            .contains(&Notice::CodexRouteOwnsCatalog)
    );
}

#[test]
fn aggregation_t13_desktop_collision_keeps_native_first_winner() {
    use vw_api_types::app_providers::desktop_mapping::{
        DesktopMapping, DesktopMappingMode, DesktopModelRoute,
    };
    let tenant = TenantScope::new("platform", "company").unwrap();
    let mut state = state(&tenant);
    state.stack.members = vec!["primary".into(), "relay-a".into(), "relay-b".into()];
    state.stack.keys = state.stack.members.iter().map(|id| (id.clone(), id.clone())).collect();
    state.mode.as_mut().unwrap().proxy_route = Some("primary".into());
    let mapping = |model: &str| DesktopMapping {
        mode: DesktopMappingMode::Proxy,
        routes: [(
            "claude-sonnet-5".into(),
            DesktopModelRoute { model: model.into(), label_override: None, supports_1m: false },
        )]
        .into(),
    };
    let mappings = [mapping("default-model"), mapping("model-4710"), mapping("model-911")];
    let providers = ["primary", "relay-a", "relay-b"]
        .into_iter()
        .zip(&mappings)
        .map(|(id, mapping)| Provider {
            id,
            name: id,
            official_service: None,
            catalog: Catalog::Desktop(mapping),
        })
        .collect::<Vec<_>>();
    let output = preview(AppId::ClaudeDesktop, &tenant, &state, input(&providers)).unwrap();
    assert_eq!(output.members[1].models[0].id, "claude-fable-ccs-957548");
    assert!(output.members[2].models.is_empty());
    assert!(output.issues.iter().any(|issue| issue.code == IssueCode::ModelCollision
        && !issue.blocking
        && issue.provider_id.as_deref() == Some("relay-b")));
}

#[test]
fn aggregation_t13_missing_default_and_invalid_catalog_are_blocking() {
    let tenant = TenantScope::new("platform", "company").unwrap();
    let mut state = state(&tenant);
    state.mode.as_mut().unwrap().proxy_route = None;
    let output = preview(AppId::Claude, &tenant, &state, input(&[])).unwrap();
    assert!(
        output.issues.iter().any(|issue| issue.code == IssueCode::MissingDefault && issue.blocking)
    );
    state.mode.as_mut().unwrap().proxy_route = Some("alpha".into());
    let record = json!({"name":"Alpha","meta":{"stackModels":"broken"}});
    let providers = [Provider {
        id: "alpha",
        name: "Alpha",
        official_service: None,
        catalog: Catalog::Claude(&record),
    }];
    let output = preview(AppId::Claude, &tenant, &state, input(&providers)).unwrap();
    assert!(
        output.issues.iter().any(|issue| issue.code == IssueCode::InvalidDefault && issue.blocking)
    );
}

#[test]
fn aggregation_t13_official_default_requires_native_catalog_and_restore_proof() {
    use vw_api_types::agent_apps_auth::AuthService;
    let tenant = TenantScope::new("platform", "company").unwrap();
    let state = state(&tenant);
    let config = toml_edit::DocumentMut::new();
    let missing = [Provider {
        id: "alpha",
        name: "Official",
        official_service: Some(AuthService::CodexOauth),
        catalog: Catalog::CodexOfficial { native: &[], config: &config },
    }];
    assert_eq!(
        preview(AppId::Codex, &tenant, &state, input(&missing)),
        Err(PreviewError::UnsupportedTarget)
    );
    let native =
        [json!({"slug":"official","display_name":"Official"}).as_object().unwrap().clone()];
    let providers = [Provider {
        id: "alpha",
        name: "Official",
        official_service: Some(AuthService::CodexOauth),
        catalog: Catalog::CodexOfficial { native: &native, config: &config },
    }];
    let mut request = input(&providers);
    request.direct_restorable = false;
    let output = preview(AppId::Codex, &tenant, &state, request).unwrap();
    assert_eq!(output.members[0].models[0].id, "official");
    assert!(
        output
            .issues
            .iter()
            .any(|issue| issue.code == IssueCode::UnrestorableDirect && issue.blocking)
    );
}
