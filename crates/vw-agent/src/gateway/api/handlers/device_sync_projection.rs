//! Await domain owners inline; projection failures never undo a committed receive.
#![cfg(target_os = "macos")]
use std::path::Path;
use axum::http::HeaderMap;
use crate::{global_database::GlobalDatabase, agent_apps::shared_snapshot::SnapshotSelection};
use vw_api_types::agent_apps::DataManagementTarget;
use vw_agent_apps::providers::live::commit::{ExecutionHostId, ProviderTransactions};

pub(super) async fn refresh(database: &GlobalDatabase, policy: Option<&Path>, headers: &HeaderMap,
    target: &DataManagementTarget, selection: &SnapshotSelection, control: Option<&super::app_routing::RoutingControl>,
) -> Result<(), ()> {
    let mut failed = false;
    // Every domain gets an attempt even if a previous domain fails. Callbacks
    // read current saved intent, never replay imported bytes or preimages.
    for (app, id) in &selection.providers {
        let direct = (|| {
            let (_, root, _) = super::agent_apps::authorize_aggregation_write(policy, headers, &target.scope, &target.target_host_id, *app, false).map_err(|_| ())?;
            let Some(root) = root else { return Ok(false); };
            let Some(tx) = ProviderTransactions::open_existing(&root, ExecutionHostId::new(&target.target_host_id).map_err(|_| ())?).map_err(|_| ())? else { return Ok(false); };
            let state = tx.state(*app).map_err(|_| ())?;
            Ok(state.stack.is_empty() && state.pointer.as_ref().is_some_and(|p| p.tenant == *target.scope.tenant_scope() && p.id == *id))
        })();
        if direct == Ok(true) {
            failed |= super::app_providers::sync_saved_provider(database.clone(), policy.map(Path::to_path_buf),
                headers.clone(), target, *app, id).await.is_err();
            continue;
        }
        let result = (|| {
            let (home, root, principal) = super::agent_apps::authorize_aggregation_write(
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
            let snapshot = super::aggregation_read_snapshot::load(database, &target.scope, &target.target_host_id,
                *app, &home, &state, None).map_err(|_| ())?;
            let revision = super::aggregation_read_snapshot::revision(&target.scope, &target.target_host_id,
                *app, &principal, &state, &snapshot).map_err(|_| ())?;
            let attached = state.stack.enabled && state.mode.as_ref().is_some_and(|m| m.attached);
            let routing = if attached {
                let control = control.ok_or(())?;
                let (authorized_root, authorized_principal) = control.authorize_mode(headers, &target.scope, &target.target_host_id, *app).map_err(|_| ())?;
                if authorized_root != root || authorized_principal != principal { return Err(()); }
                Some(control.routing.clone())
            } else { control.map(|c| c.routing.clone()) };
            let context = crate::gateway::aggregation_resync::RecordPublicationContext {
                database: database.clone(), project: target.scope.clone(), host: target.target_host_id.clone(),
                app: *app, home, root, routing, expected: state, principal, revision, failure: Default::default(),
            };
            let row = crate::app_providers::AppProviderStore::preview_record(database, target.scope.tenant_scope(), *app, id).map_err(|_| ())?;
            let raw = serde_json::to_string(&row).map_err(|_| ())?;
            let key = format!("{}:{id}", serde_json::to_string(app).map_err(|_| ())?);
            if !context.republish_saved(&key, &raw).map_err(|_| ())? { return Err(()); }
            Ok(())
        })();
        failed |= result.is_err();
    }
    if !selection.mcp.is_empty() {
        failed |= super::mcp_management_resync::sync_saved_bindings(database.clone(), policy.map(Path::to_path_buf),
            headers.clone(), target, &selection.mcp).await.is_err();
    }
    let store = crate::agent_apps::prompts_store::PromptsStore::new(database.clone()).map_err(|_| ())?;
    let mut apps = Vec::new();
    for (app, _) in &selection.prompts { if !apps.contains(app) { apps.push(*app); } }
    for app in apps {
        let result = (|| {
            let context = super::agent_apps::authorize_prompts_activation(policy, headers, &target.scope, &target.target_host_id, app).map_err(|_| ())?;
            if context.2 != target.principal_id { return Err(()); }
            let Some(tx) = ProviderTransactions::open_existing(&context.3, ExecutionHostId::new(&target.target_host_id).map_err(|_| ())?).map_err(|_| ())?
                else { return Err(()); };
            tx.state(app).map_err(|_| ())?.require_project(&target.scope).map_err(|_| ())?;
            let environment = std::env::vars_os().filter_map(|(k,v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect();
            store.sync_to_live(crate::agent_apps::prompts_store::PromptScope { project: &target.scope, host: &target.target_host_id, app },
                &target.target_host_id, &context.0, None, &environment, &tx, || {
                    let fresh = super::agent_apps::authorize_prompts_activation(policy, headers, &target.scope, &target.target_host_id, app)
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
