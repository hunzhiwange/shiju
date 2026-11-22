use super::*;
use crate::mcp::catalog_create_tests::http_draft;
use agent_client_protocol::{McpServerHttp, McpServerStdio};
use mcp_client::{FormOwner, RunningMcpPeer};
use plugin_mcp::{
    connection_host::{ConnectionHost, HostFuture, MountLease},
    mcp_connections::McpSessionConnections,
};
use rmcp::model::CallToolRequestParam;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use vw_api_mcp_management::{McpCatalogScope, McpConnectionDefinition};

struct FixtureHost {
    opening: Notify,
    finish_open: Notify,
    hold_selected: bool,
}

impl ConnectionHost for FixtureHost {
    fn request_abort(&self) -> anyhow::Result<Option<CancellationToken>> {
        Ok(None)
    }

    fn runtime_stdio_cwd(&self, server: &McpServerStdio) -> anyhow::Result<Option<PathBuf>> {
        super::super::runtime::managed_stdio_cwd(server)
    }

    fn runtime_headers(
        &self,
        _: &McpServerHttp,
        _: Option<&str>,
    ) -> anyhow::Result<Option<reqwest_012::header::HeaderMap>> {
        unreachable!("stdio fixture")
    }

    fn oauth_access<'a>(
        &'a self,
        _: &'a McpServerHttp,
        _: &'a FormOwner,
    ) -> HostFuture<'a, Option<String>> {
        unreachable!("stdio fixture")
    }

    fn oauth_cancellation(
        &self,
        _: &McpServerHttp,
        _: &FormOwner,
    ) -> anyhow::Result<CancellationToken> {
        unreachable!("stdio fixture")
    }

    fn selection_is_current(
        &self,
        server: &McpServer,
        tenant: Option<&TenantScope>,
    ) -> anyhow::Result<bool> {
        super::super::revocation::runtime_selection_is_current(
            server,
            tenant,
            TenantScope::default(),
            || anyhow::bail!("fixture has no project runtime selection"),
        )
    }

    fn open_peer(
        &self,
        server: McpServer,
        startup: Duration,
        closed: CancellationToken,
        _: Option<FormOwner>,
    ) -> HostFuture<'_, RunningMcpPeer> {
        Box::pin(async move {
            let McpServer::Stdio(server) = server else { unreachable!("stdio fixture") };
            let mut command = tokio::process::Command::new(&server.command);
            command.args(&server.args);
            let peer =
                mcp_client::client::open_isolated_stdio_peer(command, startup, closed).await?;
            if self.hold_selected && server.name == "selected" {
                self.opening.notify_one();
                self.finish_open.notified().await;
            }
            Ok(peer)
        })
    }

    fn discover<'a>(
        &'a self,
        _: McpServer,
        _: acp_config::McpServerManagement,
        _: &'a [&'a str],
        _: Arc<McpSessionConnections>,
    ) -> HostFuture<'a, Vec<Arc<dyn tool_types::Tool>>> {
        unreachable!("test uses the connection pool directly")
    }

    fn revoke_app(&self, _: &TenantScope, _: &str, _: &Path) {}
}

struct ScopedLease(AcpMountLeases);
impl MountLease for ScopedLease {
    fn is_current(&self, name: &str) -> anyhow::Result<bool> {
        self.0.is_current(name, Some((TenantScope::default(), "owner".into())))
    }
}

const SERVER: &str = r#"
import json, sys
for line in sys.stdin:
    request = json.loads(line)
    if 'id' not in request:
        continue
    method = request['method']
    if method == 'initialize':
        result = {'protocolVersion': request['params']['protocolVersion'], 'capabilities': {'tools': {}}, 'serverInfo': {'name': 'fixture', 'version': '1'}}
    elif method == 'tools/list':
        result = {'tools': [{'name': 'read', 'description': 'fixture', 'inputSchema': {'type': 'object'}}]}
    elif method == 'tools/call':
        with open(sys.argv[1], 'a') as output:
            output.write(sys.argv[2] + '\n')
        result = {'content': [{'type': 'text', 'text': sys.argv[2]}]}
    else:
        result = {}
    print(json.dumps({'jsonrpc': '2.0', 'id': request['id'], 'result': result}), flush=True)
"#;

async fn call(
    pool: &McpSessionConnections,
    server: &McpServer,
) -> anyhow::Result<serde_json::Value> {
    let result = pool
        .with_peer(server, Some(Duration::from_secs(10)), |peer| {
            Box::pin(async move {
                Ok(peer
                    .call_tool(CallToolRequestParam { name: "read".into(), arguments: None })
                    .await?)
            })
        })
        .await?;
    Ok(serde_json::to_value(result)?)
}

#[tokio::test]
async fn acp_platform_mcp_revoke_during_peer_startup() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let db = GlobalDatabase::for_home(root.path())?;
    let store = McpCatalogStore::new(db.clone()).unwrap();
    let workspace = root.path().join("项目");
    fs::create_dir(&workspace)?;
    let tenant = TenantScope::default();
    let scope = SessionScope::new(tenant.clone(), &workspace)?;
    let project = McpCatalogScope::Project { scope: ProjectScope::new(tenant, "project".into())? };
    let script = root.path().join("server.py");
    fs::write(&script, SERVER)?;
    let managed_calls = root.path().join("managed-calls");
    let native_calls = root.path().join("native-calls");
    let args = |path: &Path, marker: &str| {
        vec![script.to_str().unwrap().into(), path.to_str().unwrap().into(), marker.into()]
    };
    let mut draft = http_draft("selected", "unused");
    draft.connection = McpConnectionDefinition::Stdio {
        command: "python3".into(),
        args: args(&managed_calls, "managed"),
        env: Default::default(),
        cwd: None,
    };
    let record = store.create(&project, draft).unwrap().record;
    let selected = [McpCatalogMount { scope: project.clone(), record: record.clone() }];
    let resolve =
        |restore| resolve_mounts(&scope, "owner", "project", &selected, restore, || Ok(db.clone()));
    let mut resolved = resolve(false)?;
    let managed = resolved.servers.pop().unwrap();
    let lease = resolved.leases.unwrap();
    assert!(!lease.is_current("selected", Some((TenantScope::default(), "other-session".into())))?);
    let native = McpServer::Stdio(
        McpServerStdio::new("native", "python3").args(args(&native_calls, "native")),
    );
    let fixture_host = |hold_selected| {
        Arc::new(FixtureHost { opening: Notify::new(), finish_open: Notify::new(), hold_selected })
    };
    let initial = McpSessionConnections::new(fixture_host(false))
        .with_acp_mounts(Some(Arc::new(ScopedLease(resolve(false)?.leases.unwrap()))));
    initial.synchronize(&[managed.clone()])?;
    assert_eq!(call(&initial, &managed).await?["content"][0]["text"], "managed");
    initial.shutdown();
    fs::remove_file(&managed_calls)?;

    let host = fixture_host(true);
    let pool = McpSessionConnections::new(host.clone())
        .with_acp_mounts(Some(Arc::new(ScopedLease(lease))));
    pool.synchronize(&[native.clone(), managed.clone()])?;
    assert_eq!(call(&pool, &native).await?["content"][0]["text"], "native");

    let revoke = async {
        tokio::time::timeout(Duration::from_secs(10), host.opening.notified()).await.unwrap();
        let connection = store.connection_for_revision(&project, &record).unwrap();
        store.update_connection(&project, &record, connection).unwrap();
        host.finish_open.notify_one();
    };
    let (late_call, ()) = tokio::join!(call(&pool, &managed), revoke);
    assert!(late_call.unwrap_err().to_string().contains("revoked"));
    assert!(call(&pool, &managed).await.is_err());
    assert!(!managed_calls.exists(), "the revoked peer must receive zero tool calls");
    assert_eq!(call(&pool, &native).await?["content"][0]["text"], "native");
    assert_eq!(fs::read_to_string(native_calls)?, "native\nnative\n");

    assert!(resolve(false).is_err());
    let restored = resolve(true)?;
    assert!(restored.servers.is_empty());
    assert!(restored.leases.is_none());
    pool.shutdown();
    Ok(())
}

#[test]
fn acp_platform_mcp_revoke_keeps_catalog_resolution_scoped_by_source() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let db = GlobalDatabase::for_home(root.path())?;
    let store = McpCatalogStore::new(db.clone()).unwrap();
    let tenant = TenantScope::default();
    let scope = SessionScope::new(tenant.clone(), root.path())?;
    let project =
        McpCatalogScope::Project { scope: ProjectScope::new(tenant.clone(), "project".into())? };
    let tenant_catalog = McpCatalogScope::Tenant { scope: tenant };
    let url = "http://127.0.0.1:41001/mcp";
    let http = store.create(&project, http_draft("shared", url)).unwrap().record;
    let mut draft = http_draft("shared", url);
    draft.connection =
        McpConnectionDefinition::Sse { url: url.into(), headers: Default::default() };
    let sse = store.create(&tenant_catalog, draft).unwrap().record;
    let catalog = [
        McpCatalogMount { scope: project.clone(), record: http.clone() },
        McpCatalogMount { scope: tenant_catalog, record: sse },
    ];
    let resolve = |mounts: &[McpCatalogMount], restore| {
        resolve_mounts(&scope, "owner", "project", mounts, restore, || Ok(db.clone()))
    };
    assert!(resolve(&catalog, false).err().unwrap().to_string().contains("Conflict"));
    let project_mount = resolve(&catalog[..1], false)?;
    let tenant_mount = resolve(&catalog[1..], false)?;
    assert!(
        matches!(&project_mount.servers[..], [McpServer::Http(server)] if server.name == "shared")
    );
    assert!(
        matches!(&tenant_mount.servers[..], [McpServer::Sse(server)] if server.name == "shared")
    );

    store.delete(&project, &http).unwrap();
    assert!(!project_mount.leases.unwrap().is_current("shared", None)?);
    assert!(tenant_mount.leases.unwrap().is_current("shared", None)?);
    assert!(resolve(&catalog[..1], true)?.servers.is_empty());
    assert_eq!(resolve(&catalog[1..], true)?.servers.len(), 1);
    Ok(())
}
