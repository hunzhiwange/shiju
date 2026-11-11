//! Mapping publication consumes the shared saved-record participant and native journal.
use super::{
    failure,
    native::{APP, Context, installation},
};
use crate::{
    app_providers::AppProviderStore,
    gateway::{api::handlers::app_routing::RoutingControl, settings_store::GatewaySettingsStore},
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::Value;
use std::path::Path;
use vw_agent_apps::providers::{claude_desktop::publication::DesktopInstallation, desktop_mapping};
use vw_api_types::{
    agent_apps::aggregation::{AggregationQuery, DesktopMappingChange, DesktopMappingView},
    id::TenantScope,
};
fn unavailable<T>(_: T) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn invalid<T>(_: T) -> StatusCode {
    StatusCode::UNPROCESSABLE_ENTITY
}
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn save(
    policy: Option<&Path>,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    id: String,
    request: DesktopMappingChange,
    control: Option<Extension<RoutingControl>>,
) -> Result<Response, StatusCode> {
    vw_agent_apps::providers::validate_identity(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if request.expected_revision.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let query = AggregationQuery {
        target_host_id: request.target_host_id.clone(),
        project_id: request.project_id.clone(),
    };
    let context = Context::read(policy, tenant, db, &headers, &query)?;
    let attached = context.state.mode.as_ref().is_some_and(|mode| mode.attached);
    let (home, authorized_root, principal) = super::super::agent_apps::authorize_aggregation_write(
        policy,
        &headers,
        &context.project,
        &query.target_host_id,
        APP,
        attached,
    )?;
    if home != context.home
        || authorized_root.as_ref() != Some(&context.root)
        || principal != context.principal
    {
        return Err(StatusCode::FORBIDDEN);
    }
    if installation(policy, &headers, &context, &query.target_host_id)?
        != DesktopInstallation::Installed
    {
        return Ok(failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"));
    }
    if let Some(pending) = context.tx.pending(APP).map_err(unavailable)? {
        let store = AppProviderStore::new(context.db.clone()).map_err(unavailable)?;
        let mut guard = if pending.files.is_empty() {
            vw_agent_apps::providers::live::write_guard::lock_app(APP, &context.home)
                .map_err(unavailable)?
        } else {
            vw_agent_apps::providers::claude_desktop::ClaudeDesktopFiles::open(&context.home)
                .map_err(unavailable)?
                .guard
        };
        if pending.provider_record.is_some() {
            context
                .tx
                .recover_project_record(&context.project, &mut guard, &store)
                .map_err(|_| StatusCode::BAD_GATEWAY)?;
        } else {
            context
                .tx
                .recover_project(&context.project, &mut guard)
                .map_err(|_| StatusCode::BAD_GATEWAY)?;
        }
        // Recovery forwards a previously authorized durable intent, then requires
        // a fresh resource revision before accepting the new mapping request.
        return Ok(failure(StatusCode::CONFLICT, "revision_conflict"));
    }
    let revision = context.revision(&query.target_host_id, Some(&id))?;
    if revision != request.expected_revision {
        return Err(StatusCode::CONFLICT);
    }
    let key = format!("{}:{id}", serde_json::to_string(&APP).map_err(unavailable)?);
    let raw = GatewaySettingsStore::read_existing(
        &context.db,
        context.project.tenant_scope(),
        "app_providers.records",
        &key,
    )
    .map_err(unavailable)?
    .ok_or(StatusCode::NOT_FOUND)?;
    let before: Value = serde_json::from_str(&raw).map_err(invalid)?;
    let after = desktop_mapping::with_mapping(&before, &request.mapping).map_err(invalid)?;
    let routing = if attached {
        let control = control.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
        let internal = super::super::aggregation_resync::provider_headers(headers.clone())?;
        let (root, owner) =
            control.authorize_mode(&internal, &context.project, &query.target_host_id, APP)?;
        if owner != context.principal || root != context.root {
            return Err(StatusCode::FORBIDDEN);
        }
        Some(control.routing)
    } else {
        None
    };
    context.unchanged(&query.target_host_id, Some(&id), &revision)?;
    let execution = crate::gateway::aggregation_resync::RecordPublicationContext {
        database: context.db.clone(),
        project: context.project.clone(),
        host: query.target_host_id.clone(),
        app: APP,
        home: context.home.clone(),
        root: context.root.clone(),
        routing,
        expected: context.state.clone(),
        principal: context.principal.clone(),
        revision: revision.clone(),
        failure: Default::default(),
    };
    if !execution.replace(&key, &raw, &after.to_string())? {
        return Err(StatusCode::CONFLICT);
    }
    let next = context.tx.state(APP).map_err(unavailable)?;
    let context = Context { state: next, ..context };
    let revision = context.revision(&query.target_host_id, Some(&id))?;
    let committed =
        AppProviderStore::preview_record(&context.db, context.project.tenant_scope(), APP, &id)
            .map_err(unavailable)?;
    if committed != after {
        let mut bound = after.clone();
        bound["shijuPublicationBinding"] =
            serde_json::json!({"host":query.target_host_id,"project":context.project,"app":APP});
        if committed != bound {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }
    let form = desktop_mapping::form_view(&committed).map_err(invalid)?;
    Ok(Json(DesktopMappingView { revision, form }).into_response())
}
