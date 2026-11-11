//! Additional authorization for existing scoped supplier writes.
use crate::{
    app_providers::AppProviderStore, gateway::aggregation_resync::RecordPublicationContext,
    global_database::GlobalDatabase,
};
use axum::http::{HeaderMap, StatusCode};
use std::path::Path;
use vw_agent_apps::providers::live::commit::{ExecutionHostId, ProviderTransactions};
use vw_api_types::{
    agent_apps::{AppId, aggregation::AggregationWriteQuery},
    id::{ProjectScope, TenantScope},
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn configure(
    store: AppProviderStore,
    db: &GlobalDatabase,
    policy: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    host: &str,
    app: AppId,
    query: AggregationWriteQuery,
    routing: Option<super::app_routing::RoutingControl>,
) -> Result<AppProviderStore, StatusCode> {
    if query.target_host_id.as_deref().is_some_and(|target| target != host) {
        return Err(StatusCode::FORBIDDEN);
    }
    let headers = if query.project_id.is_some() || query.expected_revision.is_some() {
        provider_headers(headers.clone())?
    } else {
        headers.clone()
    };
    let headers = &headers;
    let (home, root) =
        super::agent_apps::provider_editor_context(policy, headers, tenant, host, true)?;
    let Some(tx) = ProviderTransactions::open_existing(
        &root,
        ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?,
    )
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    else {
        return Ok(store);
    };
    let state = tx.state(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if state.stack.is_empty() {
        return Ok(store);
    }
    let project = ProjectScope::new(tenant.clone(), query.project_id.ok_or(StatusCode::CONFLICT)?)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let attached = state.stack.enabled && state.mode.as_ref().is_some_and(|m| m.attached);
    let (authorized_home, authorized_root, principal) =
        super::agent_apps::authorize_aggregation_write(
            policy, headers, &project, host, app, attached,
        )?;
    if authorized_home != home || authorized_root.as_ref() != Some(&root) {
        return Err(StatusCode::FORBIDDEN);
    }
    state.require_project(&project).map_err(|_| StatusCode::FORBIDDEN)?;
    if let Some(pending) = tx.pending(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? {
        if pending.tenant != *tenant || pending.target.project_scope.as_ref() != Some(&project) {
            return Err(StatusCode::FORBIDDEN);
        }
        if query.expected_revision.as_ref().is_none_or(|revision| revision.trim().is_empty()) {
            return Err(StatusCode::CONFLICT);
        }
        if pending.provider_record.is_none() {
            return Err(StatusCode::CONFLICT);
        }
        if pending.files.is_empty() && matches!(app, AppId::Claude | AppId::Codex) {
            let mut guard = vw_agent_apps::providers::live::write_guard::lock_app(app, &home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            tx.recover_project_record(&project, &mut guard, &store)
                .map_err(|_| StatusCode::BAD_GATEWAY)?;
        } else {
            match app {
                AppId::Claude => {
                    let _directory =
                        tx.lock_claude_directory().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                    let selected =
                        tx.claude_directory(tenant).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                    let mut file = vw_agent_apps::providers::claude::ClaudeFile::open_selected(
                        &home,
                        selected.as_ref(),
                        false,
                    )
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                    tx.recover_project_record(&project, &mut file.guard, &store)
                        .map_err(|_| StatusCode::BAD_GATEWAY)?;
                }
                AppId::Codex => {
                    let mut files =
                        vw_agent_apps::providers::codex_catalog::CodexFiles::open(&home)
                            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                    tx.recover_project_record(&project, &mut files.guard, &store)
                        .map_err(|_| StatusCode::BAD_GATEWAY)?;
                }
                _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
            }
        }
        return Err(StatusCode::CONFLICT);
    }
    let snapshot =
        super::aggregation_read_snapshot::load(db, &project, host, app, &home, &state, None)?;
    let revision = super::aggregation_read_snapshot::revision(
        &project, host, app, &principal, &state, &snapshot,
    )?;
    if query.expected_revision.as_deref() != Some(&revision) {
        return Err(StatusCode::CONFLICT);
    }
    let preview = super::aggregation_read_snapshot::project(app, tenant, &state, &snapshot, None)?;
    if attached && preview.issues.iter().any(|i| i.blocking) {
        return Err(StatusCode::CONFLICT);
    }
    let routing = match routing {
        Some(control) => {
            if attached {
                let (route_root, route_principal) =
                    control.authorize_mode(headers, &project, host, app)?;
                if route_root != root || route_principal != principal {
                    return Err(StatusCode::FORBIDDEN);
                }
            }
            Some(control.routing)
        }
        None if attached => return Err(StatusCode::SERVICE_UNAVAILABLE),
        None => None,
    };
    Ok(store.with_resync(RecordPublicationContext {
        database: db.clone(),
        project,
        host: host.into(),
        app,
        home,
        root,
        routing,
        expected: state,
        failure: Default::default(),
        principal,
        revision,
    }))
}

/// Reuse the legacy provider grant with the scoped API's single Bearer identity.
/// Conflicting credentials are rejected instead of authorizing different subjects.
pub(crate) fn provider_headers(mut headers: HeaderMap) -> Result<HeaderMap, StatusCode> {
    let mut values = headers.get_all(axum::http::header::AUTHORIZATION).iter();
    let Some(value) = values.next() else { return Ok(headers) };
    let token = value
        .to_str()
        .ok()
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| v.len() >= 32 && v.len() <= 512 && v.bytes().all(|b| b.is_ascii_graphic()))
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_owned();
    if values.next().is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut legacy = headers.get_all("x-agent-apps-token").iter();
    if legacy.next().is_some_and(|v| v.as_bytes() != token.as_bytes()) || legacy.next().is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    headers.insert("x-agent-apps-token", token.parse().map_err(|_| StatusCode::UNAUTHORIZED)?);
    Ok(headers)
}
