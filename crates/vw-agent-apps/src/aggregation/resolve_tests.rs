use vw_api_types::{
    agent_apps::AppId::{Claude, ClaudeDesktop, Codex, Gemini},
    app_providers::desktop_mapping::{DesktopMapping, DesktopMappingMode, DesktopModelRoute},
    id::TenantScope,
};

use crate::providers::{
    desktop_mapping::catalog::{DesktopMappingMember, DesktopModelCatalog},
    live::commit::{AppState, Mode, ModeState},
    mode_state::StackState,
};

use super::resolve::{PublishedModel, ResolveError, Resolved, resolve};

fn state(tenant: &TenantScope) -> AppState {
    AppState {
        stack: StackState {
            enabled: true,
            members: ["primary", "relay-a", "relay-b"].map(String::from).into(),
            keys: ["primary", "relay-a", "relay-b", "removed"]
                .into_iter()
                .map(|id| (id.into(), id.into()))
                .collect(),
        },
        stack_tenant: Some(tenant.clone()),
        mode: Some(ModeState {
            mode: Mode::Proxy,
            attached: true,
            proxy_route: Some("primary".into()),
            contract: None,
        }),
        mode_tenant: Some(tenant.clone()),
        ..Default::default()
    }
}

fn row<'a>(id: &'a str, provider_id: &'a str, upstream_model: &'a str) -> PublishedModel<'a> {
    PublishedModel { id, provider_id, upstream_model }
}

#[test]
fn aggregation_t10_resolve_preserves_contract() {
    let tenant = TenantScope::new("platform", "company").unwrap();
    let state = state(&tenant);
    let claude = [
        row("ccs-claude-relay-a--vendor/model[1M]", "relay-a", "vendor/model[1M]"),
        row("ccs-claude-relay-b--vendor/model", "relay-b", "vendor/model"),
    ];
    for id in
        [claude[0].id, "ccs-claude-relay-a--vendor/model", "ccs-claude-relay-a--vendor/model[1m]"]
    {
        assert_eq!(resolve(Claude, &tenant, &state, id, claude), Ok(Resolved::Exact(claude[0])));
    }
    assert_eq!(
        resolve(Claude, &tenant, &state, claude[1].id, claude),
        Ok(Resolved::Exact(claude[1]))
    );

    let codex = [
        row("ccs-relay-a/vendor/model", "relay-a", "vendor/model"),
        row("ccs-relay-b/vendor/model", "relay-b", "vendor/model"),
    ];
    for expected in codex {
        assert_eq!(
            resolve(Codex, &tenant, &state, expected.id, codex),
            Ok(Resolved::Exact(expected))
        );
    }
    assert_eq!(resolve(Codex, &tenant, &state, "vendor/model", codex), Ok(Resolved::Plain));
}

#[test]
fn aggregation_t10_refuses_stale_unknown_and_malformed_ids() {
    let tenant = TenantScope::new("platform", "company").unwrap();
    let state = state(&tenant);
    let catalog = [
        row("ccs-claude-removed--model", "removed", "model"),
        row("ccs-claude-relay-a--mismatch", "primary", "model"),
        row("ccs-claude-relay-a--invalid-row", "relay-a", ""),
    ];
    for (app, id, error) in [
        (Claude, catalog[0].id, ResolveError::RemovedMember),
        (Claude, catalog[1].id, ResolveError::InvalidCatalog),
        (Claude, catalog[2].id, ResolveError::InvalidCatalog),
        (Claude, "ccs-claude-relay-a--retired", ResolveError::UnpublishedModel),
        (Claude, "ccs-claude-unknown--model", ResolveError::UnknownMember),
        (Claude, "ccs-claude-relay-a--", ResolveError::InvalidModelId),
        (Codex, "ccs-relay-a/no-longer-listed", ResolveError::UnpublishedModel),
        (Codex, "ccs-relay-a", ResolveError::InvalidModelId),
        (Gemini, "model", ResolveError::UnsupportedApplication),
    ] {
        assert_eq!(resolve(app, &tenant, &state, id, catalog), Err(error), "{id}");
    }
}

#[test]
fn aggregation_t10_requires_owned_active_aggregation_for_reserved_ids() {
    let tenant = TenantScope::new("platform", "company").unwrap();
    let other = TenantScope::new("platform", "other").unwrap();
    let mut state = state(&tenant);
    let published = row("ccs-relay-a/model", "relay-a", "model");
    let resolve_for = |tenant: &TenantScope, state: &AppState| {
        resolve(Codex, tenant, state, published.id, [published])
    };
    assert_eq!(resolve_for(&other, &state), Err(ResolveError::Forbidden));
    state.stack.enabled = false;
    assert_eq!(resolve_for(&tenant, &state), Err(ResolveError::Inactive));
    state.stack.enabled = true;
    state.mode.as_mut().unwrap().mode = Mode::Direct;
    assert_eq!(resolve_for(&tenant, &state), Err(ResolveError::Inactive));
    state.mode.as_mut().unwrap().mode = Mode::Proxy;
    state.mode.as_mut().unwrap().attached = false;
    assert_eq!(resolve_for(&tenant, &state), Err(ResolveError::Inactive));
}

#[test]
fn aggregation_t10_desktop_uses_the_published_collision_winner() {
    let tenant = TenantScope::new("platform", "company").unwrap();
    let mut state = state(&tenant);
    let mapping = |model: &str| DesktopMapping {
        mode: DesktopMappingMode::Proxy,
        routes: [(
            "claude-sonnet-5".into(),
            DesktopModelRoute { model: model.into(), label_override: None, supports_1m: false },
        )]
        .into(),
    };
    let primary = mapping("default-model");
    let first = mapping("model-4710");
    let second = mapping("model-911");
    let member = |id: &'static str, mapping| DesktopMappingMember {
        provider_id: id,
        key: id,
        name: id,
        mapping,
    };
    let catalog = DesktopModelCatalog::new(
        member("primary", &primary),
        &[member("relay-a", &first), member("relay-b", &second)],
    )
    .unwrap();
    let id = "claude-fable-ccs-957548";
    let dispatch = |state: &AppState, request: &str| {
        resolve(
            ClaudeDesktop,
            &tenant,
            state,
            request,
            catalog.models().iter().map(PublishedModel::from),
        )
    };
    assert_eq!(
        dispatch(&state, &format!("{id}[1m]")),
        Ok(Resolved::Exact(row(id, "relay-a", "model-4710")))
    );
    assert_eq!(dispatch(&state, "claude-fable-ccs-000000"), Err(ResolveError::UnpublishedModel));
    for malformed in
        ["claude-fable-ccs-95754", "CLAUDE-FABLE-CCS-957548", "anthropic/claude-fable-ccs-957548"]
    {
        assert_eq!(dispatch(&state, malformed), Err(ResolveError::InvalidModelId));
    }
    state.stack.members.retain(|member| member != "relay-a");
    assert_eq!(dispatch(&state, id), Err(ResolveError::RemovedMember));
}
