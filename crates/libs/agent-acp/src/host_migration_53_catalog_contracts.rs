use std::fs;

use serde_json::json;
use acp_types::{McpCatalogMount, SessionMountRevision, SetSessionMountsRequest};
use acp_server::server::SessionBackend;
use vw_agent_apps::mcp::catalog::McpCatalogDraft;
use {
    vw_api_transport::ProjectScope,
    vw_api_transport::TenantScope,
    vw_api_mcp_management::McpCatalogScope,
    vw_api_mcp_management::McpConnectionDefinition,
    vw_api_mcp_management::McpFailureCode,
};

use crate::protocol::{VibeWindowBackend, Host};
use crate::catalog_mounts::CatalogMountSnapshot;
use acp_server::server::BackendError;
use agent_app_services::mcp::McpCatalogStore;
use agent_database::GlobalDatabase;

#[async_trait::async_trait]
pub trait CatalogPorts {
    type Host: Host + Default;
    fn home() -> &'static std::path::Path;
    async fn read_catalog_mounts(project: &ProjectScope, id: &str) -> Result<CatalogMountSnapshot, BackendError>;
}

pub async fn cc_mcp_t29_read_mounts_checks_exact_identity_and_exposes_only_committed_references<P: CatalogPorts>() {
    let Some(root) = std::env::var_os("VW_T29_READ_ROOT").map(std::path::PathBuf::from) else {
        let fixture = tempfile::tempdir().unwrap();
        fs::create_dir(fixture.path().join("home")).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "acp::catalog_mounts_tests::cc_mcp_t29_read_mounts_checks_exact_identity_and_exposes_only_committed_references", "--nocapture"])
            .env("VW_T29_READ_ROOT", fixture.path())
            .env("VW_MODEL_RECOVERY_TEST_ROOT", fixture.path().join("home"))
            .env("VIBEWINDOW_TEST_ROOT", fixture.path().join("home"))
            .env("VW_ACP_MOUNT_RECOVERY_DB_ROOT", fixture.path().join("home"))
            .output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    };
    let workspace = root.join("workspace");
    fs::create_dir(&workspace).unwrap();
    let tenant = TenantScope::default();
    let (info, _) = agent_project::from_directory(&tenant, &workspace).await.unwrap();
    let project = ProjectScope::new(tenant.clone(), info.id.into()).unwrap();
    let scope = McpCatalogScope::Project { scope: project.clone() };
    let script = root.join("server.py");
    let marker = root.join("started");
    fs::write(&script, r#"
import json,sys
from pathlib import Path
Path(sys.argv[1]).touch()
for line in sys.stdin:
    r=json.loads(line)
    if 'id' not in r: continue
    if r.get('method')=='initialize':
        result={'protocolVersion':r['params']['protocolVersion'],'capabilities':{'tools':{}},'serverInfo':{'name':'fixture','version':'1'}}
    elif r.get('method')=='tools/list':
        result={'tools':[]}
    else: result={}
    print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
"#).unwrap();
    let connection = McpConnectionDefinition::Stdio {
        command: "python3".into(),
        args: vec![script.display().to_string(), marker.display().to_string()],
        env: Default::default(),
        cwd: None,
    };
    let catalog =
        McpCatalogStore::new(GlobalDatabase::for_home(P::home()).unwrap())
            .unwrap();
    let selected = catalog
        .create(
            &scope,
            McpCatalogDraft {
                id: "selected".into(),
                name: None,
                description: None,
                homepage: None,
                documentation: None,
                tags: vec![],
                connection: connection.clone(),
            },
        )
        .unwrap();
    let backend = VibeWindowBackend::<P::Host>::new(tenant.clone());
    let session = backend.create_session(workspace.clone()).await.unwrap();
    backend
        .set_session_mounts(SetSessionMountsRequest {
            session_id: session.id.clone().into(),
            expected_revision: SessionMountRevision(0),
            skills: vec![],
            mcp_servers: vec![],
            mcp_catalog: vec![McpCatalogMount {
                scope: scope.clone(),
                record: selected.record.clone(),
            }],
        })
        .await
        .unwrap();
    assert!(marker.exists());
    fs::remove_file(&marker).unwrap();
    let snapshot = P::read_catalog_mounts(&project, &session.id).await.unwrap();
    assert_eq!(snapshot.revision, SessionMountRevision(1));
    assert_eq!(snapshot.project, project);
    assert_eq!(snapshot.session_id, session.id);
    assert_eq!(snapshot.catalog.len(), 1);
    assert_eq!(snapshot.catalog[0].mount.record, selected.record);
    assert!(snapshot.catalog[0].failure.is_none());
    let serialized = serde_json::to_value(snapshot).unwrap();
    assert!(!serialized.to_string().contains("python3"));
    assert!(!serialized.to_string().contains(workspace.to_str().unwrap()));
    assert!(!marker.exists(), "reading must not restart the MCP process");
    let foreign = ProjectScope::new(
        TenantScope::new("foreign", "company").unwrap(),
        project.project_id().clone(),
    )
    .unwrap();
    let other_project = ProjectScope::new(tenant.clone(), "other-project".into()).unwrap();
    for rejected in [&foreign, &other_project] {
        assert!(P::read_catalog_mounts(rejected, &session.id).await.is_err());
    }
    assert!(P::read_catalog_mounts(&project, "missing-session").await.is_err());
    let updated = catalog.update_connection(&scope, &selected.record, connection).unwrap();
    let stale = P::read_catalog_mounts(&project, &session.id).await.unwrap();
    assert_eq!(stale.catalog[0].failure, Some(McpFailureCode::StaleRevision));
    catalog.delete(&scope, &updated.record).unwrap();
    let deleted = P::read_catalog_mounts(&project, &session.id).await.unwrap();
    assert_eq!(deleted.catalog[0].failure, Some(McpFailureCode::NotFound));
    let second = backend.create_session(workspace.clone()).await.unwrap();
    assert!(P::read_catalog_mounts(&project, &second.id).await.unwrap().catalog.is_empty());
    // A native session with no ACP commit has no mount snapshot.
    let native =
        P::Host::default().create_session(session_port::CreateSessionInput {
            scope: session_port::SessionScope::new(tenant, workspace.clone()).unwrap(),
            parent_id: None,
            title: None,
            permission: None,
        })
        .await
        .unwrap();
    assert!(P::read_catalog_mounts(&project, &native.info.id).await.is_err());
    let session_scope =
        session_port::SessionScope::new(project.tenant_scope().clone(), &workspace)
            .unwrap();
    let mut persisted =
        crate::mount_store::load(&session_scope, &session.id).await.unwrap().unwrap();
    persisted.mcp_catalog[0].scope =
        McpCatalogScope::Tenant { scope: foreign.tenant_scope().clone() };
    agent_storage::write_atomic(
        &["acp_session_mounts", &crate::mount_store::key(&session_scope, &session.id)],
        &persisted,
    )
    .await
    .unwrap();
    assert!(
        P::read_catalog_mounts(&project, &session.id).await.is_err(),
        "foreign persisted references must be rejected"
    );
    assert_eq!(serialized["catalog"][0]["mount"]["record"]["id"], json!("selected"));
}
