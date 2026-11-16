//! Await domain owners inline; projection failures never undo a committed receive.
#![cfg(target_os = "macos")]
use std::path::Path;
use axum::http::HeaderMap;
use agent_database::GlobalDatabase;
use agent_app_services::shared_snapshot::SnapshotSelection;
use vw_api_app_operations::DataManagementTarget;
use vw_agent_apps::providers::live::commit::{ExecutionHostId, ProviderTransactions};

pub async fn refresh(database: &GlobalDatabase, policy: Option<&Path>, headers: &HeaderMap,
    target: &DataManagementTarget, selection: &SnapshotSelection, control: Option<&super::lifecycle_api::RoutingControl>,
) -> Result<(), ()> {
    let mut failed = false;
    // Every domain gets an attempt even if a previous domain fails. Callbacks
    // read current saved intent, never replay imported bytes or preimages.
    for (app, id) in &selection.providers {
        let direct = (|| {
            let (_, root, _) = app_access_service::access::authorize_aggregation_write(policy, headers, &target.scope, &target.target_host_id, *app, false).map_err(|_| ())?;
            let Some(root) = root else { return Ok(false); };
            let Some(tx) = ProviderTransactions::open_existing(&root, ExecutionHostId::new(&target.target_host_id).map_err(|_| ())?).map_err(|_| ())? else { return Ok(false); };
            let state = tx.state(*app).map_err(|_| ())?;
            Ok(state.stack.is_empty() && state.pointer.as_ref().is_some_and(|p| p.tenant == *target.scope.tenant_scope() && p.id == *id))
        })();
        if direct == Ok(true) {
            failed |= gateway_app_providers::sync_saved_provider(database.clone(), policy.map(Path::to_path_buf),
                headers.clone(), target, *app, id, super::provider_publication_host::control(None)).await.is_err();
            continue;
        }
        let result = (|| {
            let (home, root, principal) = app_access_service::access::authorize_aggregation_write(
                policy, headers, &target.scope, &target.target_host_id, *app, false).map_err(|_| ())?;
            if principal != target.principal_id { return Err(()); }
            let root = root.ok_or(())?;
            let Some(tx) = ProviderTransactions::open_existing(&root, ExecutionHostId::new(&target.target_host_id).map_err(|_| ())?).map_err(|_| ())?
                else { return Ok(()); };
            let state = tx.state(*app).map_err(|_| ())?;
            if state.stack.is_empty() {
                // No saved native selection requires publication for an inactive record.
                return if state.pointer.as_ref().is_some_and(|p| p.tenant == *target.scope.tenant_scope() && p.id == *id) { Err(()) } else { Ok(()) };
            }
            state.require_project(&target.scope).map_err(|_| ())?;
            let snapshot = gateway_aggregation::aggregation_read_snapshot::load(database, &target.scope, &target.target_host_id,
                *app, &home, &state, None).map_err(|_| ())?;
            let revision = gateway_aggregation::aggregation_read_snapshot::revision(&target.scope, &target.target_host_id,
                *app, &principal, &state, &snapshot).map_err(|_| ())?;
            let attached = state.stack.enabled && state.mode.as_ref().is_some_and(|m| m.attached);
            let routing = if attached {
                let control = control.ok_or(())?;
                let (authorized_root, authorized_principal) = control.authorize_mode(headers, &target.scope, &target.target_host_id, *app).map_err(|_| ())?;
                if authorized_root != root || authorized_principal != principal { return Err(()); }
                Some(control.routing.clone())
            } else { control.map(|c| c.routing.clone()) };
            let context = agent_gateway::aggregation_resync::RecordPublicationContext {
                database: database.clone(), project: target.scope.clone(), host: target.target_host_id.clone(),
                app: *app, home, root, routing, expected: state, principal, revision, failure: Default::default(),
            };
            let row = app_provider_services::AppProviderStore::preview_record(database, target.scope.tenant_scope(), *app, id).map_err(|_| ())?;
            let raw = serde_json::to_string(&row).map_err(|_| ())?;
            let key = format!("{}:{id}", serde_json::to_string(app).map_err(|_| ())?);
            if !context.republish_saved(&key, &raw).map_err(|_| ())? { return Err(()); }
            Ok(())
        })();
        failed |= result.is_err();
    }
    if !selection.mcp.is_empty() {
        failed |= gateway_mcp::server::mcp_management_resync::sync_saved_bindings(database.clone(), policy.map(Path::to_path_buf),
            headers.clone(), target, &selection.mcp).await.is_err();
    }
    let store = agent_app_services::prompts_store::PromptsStore::new(database.clone()).map_err(|_| ())?;
    let mut apps = Vec::new();
    for (app, _) in &selection.prompts { if !apps.contains(app) { apps.push(*app); } }
    for app in apps {
        let result = (|| {
            let context = app_access_service::access::authorize_prompts_activation(policy, headers, &target.scope, &target.target_host_id, app).map_err(|_| ())?;
            if context.2 != target.principal_id { return Err(()); }
            let Some(tx) = ProviderTransactions::open_existing(&context.3, ExecutionHostId::new(&target.target_host_id).map_err(|_| ())?).map_err(|_| ())?
                else { return Err(()); };
            tx.state(app).map_err(|_| ())?.require_project(&target.scope).map_err(|_| ())?;
            let environment = std::env::vars_os().filter_map(|(k,v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect();
            store.sync_to_live(agent_app_services::prompts_store::PromptScope { project: &target.scope, host: &target.target_host_id, app },
                &target.target_host_id, &context.0, None, &environment, &tx, || {
                    let fresh = app_access_service::access::authorize_prompts_activation(policy, headers, &target.scope, &target.target_host_id, app)
                        .map_err(|_| vw_agent_apps::providers::live::write_guard::WriteError::Conflict)?;
                    if fresh != context { return Err(vw_agent_apps::providers::live::write_guard::WriteError::Conflict); }
                    Ok(())
                }).map_err(|_| ())?;
            Ok(())
        })();
        failed |= result.is_err();
    }
    // SkillUpdate publishes all saved copies/links under its resource leases and
    // installs their new fingerprints atomically with the configuration transaction.
    if failed { Err(()) } else { Ok(()) }
}

struct ProjectionHost(Option<super::lifecycle_api::RoutingControl>);
impl gateway_device_sync::device_sync_projection::ProjectionHost for ProjectionHost {
    fn refresh<'a>(&'a self, database: &'a GlobalDatabase, policy: Option<&'a Path>,
        headers: &'a HeaderMap, target: &'a DataManagementTarget, selection: &'a SnapshotSelection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ()>> + Send + 'a>> {
        Box::pin(refresh(database, policy, headers, target, selection, self.0.as_ref()))
    }
}

pub async fn inject(
    control: Option<axum::Extension<super::lifecycle_api::RoutingControl>>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    request.extensions_mut().insert(gateway_device_sync::device_sync_projection::DownloadRouting(
        std::sync::Arc::new(ProjectionHost(control.map(|axum::Extension(c)| c)))));
    next.run(request).await
}
