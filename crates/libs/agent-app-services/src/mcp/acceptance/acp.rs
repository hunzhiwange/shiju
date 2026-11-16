//! ACP mount contracts using the host's actual SessionBackend and Tool handles.
use std::{fs, path::Path, sync::Arc};
use acp_types::{McpCatalogMount, SessionMountRevision, SetSessionMountsRequest};
use acp_server::server::SessionBackend;
use serde_json::json;
use crate::mcp::McpCatalogStore;
use tool_types::Tool;
use tool_catalog::catalog::ToolCatalog;
use plugin_mcp::mcp_connections::McpSessionConnections;
use vw_api_mcp_management::{McpCatalogScope, McpConnectionDefinition, McpCatalogRecord};
fn http_draft(id: &str, url: &str) -> vw_agent_apps::mcp::catalog::McpCatalogDraft {
    vw_agent_apps::mcp::catalog::McpCatalogDraft {
        id: id.into(), name: None, description: None, homepage: None, documentation: None, tags: Vec::new(),
        connection: McpConnectionDefinition::StreamableHttp { url: url.into(), headers: Default::default() },
    }
}
const ACP_SERVER: &str = r#"
import json, sys
from pathlib import Path

Path(sys.argv[1]).touch()
for line in sys.stdin:
    request = json.loads(line)
    if 'id' not in request:
        continue
    method = request['method']
    if method == 'initialize':
        result = {'protocolVersion': request['params']['protocolVersion'], 'capabilities': {'tools': {}}, 'serverInfo': {'name': 'fixture', 'version': '1'}}
    elif method == 'tools/list':
        result = {'tools': [{'name': 'read', 'description': 'Read fixture', 'inputSchema': {'type': 'object'}}]}
    elif method == 'tools/call':
        result = {'content': [{'type': 'text', 'text': 'from managed session'}]}
    else:
        result = {}
    print(json.dumps({'jsonrpc': '2.0', 'id': request['id'], 'result': result}), flush=True)
"#;

pub fn add_server(
    store: &McpCatalogStore,
    scope: &McpCatalogScope,
    root: &Path,
    name: &str,
) -> McpCatalogMount {
    let script = root.join("server.py");
    fs::write(
        &script,
        ACP_SERVER,
    )
    .unwrap();
    let mut draft = http_draft(name, "unused");
    draft.connection = McpConnectionDefinition::Stdio {
        command: "python3".into(),
        args: vec![script.display().to_string(), root.join(name).display().to_string()],
        env: Default::default(),
        cwd: None,
    };
    McpCatalogMount { scope: scope.clone(), record: store.create(scope, draft).unwrap().record }
}

fn request(id: &str, revision: u64, mounts: Vec<McpCatalogMount>) -> SetSessionMountsRequest {
        serde_json::from_value::<SetSessionMountsRequest>(json!({
            "sessionId": id,
            "expectedRevision": revision,
            "skills": [],
            "mcpServers": [],
            "mcpCatalog": mounts
        }))
        .unwrap()
}
pub async fn select_catalog(backend: &impl SessionBackend, id: &str, selected: &McpCatalogMount,
    rejected: [McpCatalogMount; 2], root: &Path) {
    let update = |revision, mounts| request(id, revision, mounts);
    for rejected in rejected {
        let error = backend
            .set_session_mounts(update(0, vec![selected.clone(), rejected.clone()]))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("PermissionDenied"));
        assert!(!root.join(rejected.record.id.0).exists());
    }
    assert!(!root.join("selected").exists(), "reject the whole invalid selection");

    let response = backend.set_session_mounts(update(0, vec![selected.clone()])).await.unwrap();
    assert_eq!(
        response.mcp_servers,
        ["selected"],
        "the selected catalog revision must mount through the ACP backend"
    );
    assert_eq!(response.revision, SessionMountRevision(1));
    assert!(root.join("selected").exists());
    assert!(!root.join("native-runtime").exists());

}
pub async fn revoke_cached(catalog: &McpCatalogStore, project_scope: &McpCatalogScope,
    selected: &McpCatalogMount, root: &Path, tool: &Arc<dyn Tool>, pool: &McpSessionConnections) -> McpCatalogRecord {
    let live = ToolCatalog::default();
    live.replace(vec![tool.clone()], [tool.name().to_owned()]).unwrap();
    let definition = catalog.connection_for_revision(&project_scope, &selected.record).unwrap();
    let current = catalog.update_connection(&project_scope, &selected.record, definition).unwrap();
    fs::remove_file(root.join("selected")).unwrap();
    assert!(tool.call(json!({})).await.is_err(), "a cached executor must reject the old revision");
    pool.refresh_tool_catalog(&live).await.unwrap();
    assert!(live.snapshot().tools().is_empty());
    pool.refresh_tool_catalog(&live).await.unwrap();
    assert!(live.snapshot().tools().is_empty());
    current
}
pub async fn replace_and_clear(backend: &impl SessionBackend, id: &str, selected: McpCatalogMount,
    current: McpCatalogMount, replacement_tool: impl FnOnce() -> Arc<dyn Tool>) {
    let update = |revision, mounts| request(id, revision, mounts);
    let error = backend.set_session_mounts(update(1, vec![selected])).await.unwrap_err();
    assert!(error.to_string().contains("StaleRevision"));
    let response = backend.set_session_mounts(update(1, vec![current])).await.unwrap();
    assert_eq!(response.mcp_servers, ["selected"]);
    let replacement = replacement_tool();
    assert!(replacement.call(json!({})).await.unwrap().is_success());
    let cleared = backend.set_session_mounts(update(2, vec![])).await.unwrap();
    assert!(cleared.mcp_servers.is_empty());
    assert!(replacement.call(json!({})).await.is_err());
}
