use super::dynamic_tool_store::PersistentToolPolicy;
use super::store::SessionStore;
use crate::global_database::GlobalDatabase;
use crate::project::store::ProjectStore;
use crate::tools::catalog::ToolCatalog;
use crate::tools::{Tool, ToolResult};
use anyhow::Result;
use serde_json::{Value, json};
use std::sync::Arc;
use {
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};
use project_types::{Info, TimeInfo};
use session_ui_types::ChatSession;

struct VersionTool(&'static str);
#[async_trait::async_trait]
impl Tool for VersionTool {
    fn name(&self) -> &str {
        "persistent_tool"
    }
    fn description(&self) -> &str {
        self.0
    }
    fn parameters_schema(&self) -> Value {
        json!({"type":"object"})
    }
    async fn execute(&self, _: Value) -> Result<ToolResult> {
        Ok(ToolResult { success: true, output: self.0.into(), error: None })
    }
}
pub(crate) fn tools(value: &'static str) -> Vec<Arc<dyn Tool>> {
    vec![Arc::new(VersionTool(value))]
}

pub(crate) fn fixture(
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
            &ChatSession {
                id: id.into(),
                title: "session".into(),
                messages: vec![],
                message_ids: vec![],
                calls: vec![],
                steps: vec![],
                created_ms: 1,
                updated_ms: 1,
            },
        )?;
    }
    Ok((store, scope))
}

#[test]
fn dynamic_tools_restore_client_specs_do_not_restore_host_capabilities() -> Result<()> {
    use crate::session::runtime::{
        SessionScope,
        dynamic_calls::{ClientToolHost, DynamicCallOwner},
    };
    use crate::tools::{ToolSpec, client_tool::ClientTool};
    let home = tempfile::tempdir()?;
    let (store, scope) =
        fixture(GlobalDatabase::new(home.path().join("db.sqlite3"))?, "client-restore")?;
    let runtime_scope = SessionScope::new(scope.tenant_scope().clone(), home.path())?;
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
        vec![Arc::new(ClientTool::new(spec.clone(), host)?)],
        [spec.id.clone()],
        PersistentToolPolicy::trusted_host(),
    )?;
    let saved = store.load_dynamic_tools(&scope, "session")?.unwrap();
    assert!(super::dynamic_tool_store::rebind(&saved, vec![], [spec.id.clone()]).is_err());
    assert!(super::dynamic_tool_store::rebind(&saved, vec![], Vec::<String>::new())?.snapshot().bind(&spec.id).is_err());
    let (new_host, _new_requests) = ClientToolHost::register(DynamicCallOwner::new(
        runtime_scope,
        "session",
        "new-authenticated-host",
    )?);
    let bound = super::dynamic_tool_store::rebind(&saved, vec![Arc::new(ClientTool::new(spec.clone(), new_host)?)], [spec.id.clone()])?;
    assert!(bound.snapshot().bind(&spec.id).is_ok());
    use crate::tools::discovery::{ToolDiscoveryCatalog, ToolExposureState};
    let discovery = ToolDiscoveryCatalog::new(bound.snapshot().specs());
    let mut old_loaded = ToolExposureState::default();
    old_loaded.record_loaded(&discovery, [spec.id.as_str()]);
    assert_eq!(discovery.exposed_specs(&old_loaded, true).len(), 1);
    assert!(discovery.exposed_specs(&ToolExposureState::default(), true).is_empty());
    Ok(())
}
