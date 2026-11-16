//! Capability-restoration assertions shared with the actual host ClientTool adapter.
use crate::runtime::{SessionScope, dynamic_calls::{ClientToolHost, DynamicCallOwner}};
use session_store::dynamic_tool_store::PersistentToolPolicy;
use tool_assembly::catalog::ToolCatalog;
use tool_types::ToolSpec;
use std::sync::Arc;
use serde_json::json;
use anyhow::Result;
pub fn dynamic_tools_restore_client_specs_do_not_restore_host_capabilities(
    store: session_store::SessionStore,
    scope: vw_api_transport::ProjectScope,
    home: &std::path::Path,
    client_tool: impl Fn(ToolSpec, Arc<ClientToolHost>) -> Result<Arc<dyn tool_types::Tool>>,
) -> Result<()> {
    let runtime_scope = SessionScope::new(scope.tenant_scope().clone(), home)?;
    let (host, _requests) = ClientToolHost::register(DynamicCallOwner::new(
        runtime_scope.clone(),
        "session",
        "authenticated-host",
    )?);
    let mut spec = ToolSpec::new("client_echo", "Echo", json!({"type":"object"}));
    spec.source = vw_api_tools::ToolSource::Client;
    spec.exposure = vw_api_tools::ToolExposure::Deferred;
    store.replace_dynamic_tools(
        &scope,
        "session",
        &ToolCatalog::default(),
        vec![client_tool(spec.clone(), host)?],
        [spec.id.clone()],
        PersistentToolPolicy::trusted_host(),
    )?;
    let saved = store.load_dynamic_tools(&scope, "session")?.unwrap();
    assert!(session_store::dynamic_tool_store::rebind(&saved, vec![], [spec.id.clone()]).is_err());
    assert!(session_store::dynamic_tool_store::rebind(&saved, vec![], Vec::<String>::new())?.snapshot().bind(&spec.id).is_err());
    let (new_host, _new_requests) = ClientToolHost::register(DynamicCallOwner::new(
        runtime_scope,
        "session",
        "new-authenticated-host",
    )?);
    let bound = session_store::dynamic_tool_store::rebind(&saved, vec![client_tool(spec.clone(), new_host)?], [spec.id.clone()])?;
    assert!(bound.snapshot().bind(&spec.id).is_ok());
    use tool_assembly::discovery::{ToolDiscoveryCatalog, ToolExposureState};
    let discovery = ToolDiscoveryCatalog::new(bound.snapshot().specs());
    let mut old_loaded = ToolExposureState::default();
    old_loaded.record_loaded(&discovery, [spec.id.as_str()]);
    assert_eq!(discovery.exposed_specs(&old_loaded, true).len(), 1);
    assert!(discovery.exposed_specs(&ToolExposureState::default(), true).is_empty());
    Ok(())
}

use agent_database::GlobalDatabase;
use agent_project::{store::ProjectStore, Info, TimeInfo};
use session_store::SessionStore;
use vw_api_transport::{ProjectId, ProjectScope, TenantScope};

pub fn fixture(
    database: GlobalDatabase,
    tenant: &str,
) -> Result<(SessionStore, ProjectScope)> {
    fixture_project(database, tenant, "project")
}

fn fixture_project(
    database: GlobalDatabase,
    tenant: &str,
    project: &str,
) -> Result<(SessionStore, ProjectScope)> {
    let projects = ProjectStore::new(database.clone())?;
    let scope = ProjectScope::new(TenantScope::new("platform", tenant)?, ProjectId::from(project))?;
    projects.create(
        scope.tenant_scope(),
        &Info {
            id: project.into(),
            worktree: format!("/work/{tenant}/{project}"),
            vcs: None,
            name: None,
            icon: None,
            commands: None,
            time: TimeInfo { created: 1, updated: 1, initialized: None },
            sandboxes: vec![],
        },
    )?;
    let store = SessionStore::new(database)?;
    for id in ["session", "other"] {
        store.save_session(
            &scope,
            &serde_json::from_value(serde_json::json!({
                "id": id, "title": "session", "messages": [], "message_ids": [],
                "calls": [], "steps": [], "created_ms": 1, "updated_ms": 1
            }))?,
        )?;
    }
    Ok((store, scope))
}
