use std::{fs, path::Path, sync::Arc, time::Duration};

use serde_json::json;
use vw_api_types::{
    ProjectScope, TenantScope,
    mcp_management::{
        McpCatalogScope, McpConnectionDefinition, McpFailureCode, McpRuntimeSelection,
    },
};

use super::{McpCatalogStore, catalog_create_tests::http_draft, runtime_tests::register_project};
use crate::{
    global_database::GlobalDatabase,
    project::instance,
    session::ui_store::with_tenant_scope_sync,
    tools::{Tool, ToolRuntimeContext, catalog::ToolCatalog, execution_tools_for_context},
};

fn connection(script: &Path, calls: &Path, label: &str) -> McpConnectionDefinition {
    McpConnectionDefinition::Stdio {
        command: "python3".into(),
        args: vec![script.display().to_string(), calls.display().to_string(), label.into()],
        env: Default::default(),
        cwd: None,
    }
}

#[test]
fn cc_mcp_t28_update_and_delete_revoke_cached_tools_without_interrupting_admitted_call()
-> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let workspace = fixture.path().canonicalize()?;
    fs::write(workspace.join(".vwacprc.json"), r#"{"mcpServers":[]}"#)?;
    let script = workspace.join("server.py");
    let calls = workspace.join("calls");
    fs::write(
        &script,
        r#"
import json
import os
import sys
import time

for line in sys.stdin:
    request = json.loads(line)
    if "id" not in request:
        continue
    method = request["method"]
    if method == "initialize":
        result = {
            "protocolVersion": request["params"]["protocolVersion"],
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "fixture", "version": "1"},
        }
    elif method == "tools/list":
        result = {"tools": [{"name": "read", "inputSchema": {"type": "object"}, "annotations": {"readOnlyHint": True}}]}
    elif method == "tools/call":
        with open(sys.argv[1], "a") as calls:
            calls.write(sys.argv[2] + "\n")
        if request["params"]["arguments"].get("wait"):
            while not os.path.exists(sys.argv[1] + ".release"):
                time.sleep(0.01)
        result = {"content": [{"type": "text", "text": sys.argv[2]}]}
    else:
        result = {}
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
"#,
    )?;

    let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
    let tenant = TenantScope::new("mcp-runtime-revocation", "company")?;
    let project = ProjectScope::new(tenant.clone(), "managed-project".into())?;
    let project_context = register_project(&database, &project, &workspace)?;
    let scope = McpCatalogScope::Project { scope: project.clone() };
    let catalog = McpCatalogStore::new(database).unwrap();
    let mut draft = http_draft("managed", "unused");
    draft.connection = connection(&script, &calls, "first");
    let first = catalog.create(&scope, draft).unwrap().record;
    let select = |record| {
        catalog
            .select_for_runtime(
                &project,
                &scope,
                &McpRuntimeSelection { project: project.clone(), record },
            )
            .unwrap();
    };
    select(first.clone());
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    with_tenant_scope_sync(tenant, || {
        instance::provide_context_sync(project_context, || -> anyhow::Result<()> {
            let context =
                ToolRuntimeContext::new("revocation", Some(workspace.display().to_string()));
            let pool = crate::tools::mcp_connections_for_context(&context);
            let id = "mcp__managed__read";
            let discover = || -> Option<Arc<dyn Tool>> {
                execution_tools_for_context(&context)
                    .into_iter()
                    .find(|tool| tool.name() == id)
                    .map(Arc::from)
            };
            let old = discover().expect("selected tool should be discovered");
            assert!(runtime.block_on(old.call(json!({})))?.model_text().contains("first"));
            let live = ToolCatalog::default();
            live.replace(vec![old.clone()], [id.to_owned()])?;

            let second = runtime.block_on(async {
                let call = old.call(json!({"wait": true}));
                let revoke = async {
                    tokio::time::timeout(Duration::from_secs(5), async {
                        while fs::read_to_string(&calls)?.lines().count() < 2 {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        Ok::<_, std::io::Error>(())
                    })
                    .await??;
                    let second = catalog
                        .update_connection(&scope, &first, connection(&script, &calls, "second"))
                        .unwrap()
                        .record;
                    let error = old.call(json!({})).await.expect_err(
                        "a cached tool must reject a new call after its catalog revision changes",
                    );
                    assert!(format!("{error:#}").contains("runtime selection was revoked"));
                    pool.refresh_tool_catalog(&live).await?;
                    assert!(!live.snapshot().authorizes(id));
                    fs::write(calls.with_extension("release"), "")?;
                    Ok::<_, anyhow::Error>(second)
                };
                let (finished, second) = tokio::try_join!(call, revoke)?;
                assert!(finished.model_text().contains("first"));
                Ok::<_, anyhow::Error>(second)
            })?;
            assert!(discover().is_none());

            select(second.clone());
            let current = discover().expect("the newly selected revision should be discovered");
            assert!(runtime.block_on(current.call(json!({})))?.model_text().contains("second"));
            assert!(runtime.block_on(old.call(json!({}))).is_err());
            live.replace(vec![current.clone()], [id.to_owned()])?;

            catalog.delete(&scope, &second).unwrap();
            assert!(runtime.block_on(current.call(json!({}))).is_err());
            runtime.block_on(pool.refresh_tool_catalog(&live))?;
            assert!(!live.snapshot().authorizes(id));
            assert!(discover().is_none());
            assert_eq!(fs::read_to_string(calls)?, "first\nfirst\nsecond\n");
            crate::plugins::close_session_mcp_connections("revocation", &workspace);
            Ok(())
        })
    })
}

#[test]
fn cc_mcp_t28_disable_revokes_loopback_tools_until_a_fresh_selection() -> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let workspace = fixture.path().canonicalize()?;
    fs::write(workspace.join(".vwacprc.json"), r#"{"mcpServers":[]}"#)?;
    let script = workspace.join("server.py");
    fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t18/http_server.py"))?;
    let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
    let tenant = TenantScope::new("mcp-runtime-disable", "company")?;
    let project = ProjectScope::new(tenant.clone(), "managed-project".into())?;
    let other = ProjectScope::new(tenant.clone(), "other-project".into())?;
    let project_context = register_project(&database, &project, &workspace)?;
    let scope = McpCatalogScope::Tenant { scope: tenant.clone() };
    let catalog = McpCatalogStore::new(database).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    with_tenant_scope_sync(tenant, || {
        instance::provide_context_sync(project_context, || -> anyhow::Result<()> {
            let context = ToolRuntimeContext::new("disable", Some(workspace.display().to_string()));
            let pool = crate::tools::mcp_connections_for_context(&context);
            for mode in ["http", "sse"] {
                let root = workspace.join(mode);
                fs::create_dir(&root)?;
                let (mut child, port) = runtime.block_on(async {
                    let child = tokio::process::Command::new("python3")
                        .arg(&script)
                        .arg(&root)
                        .arg(mode)
                        .kill_on_drop(true)
                        .spawn()?;
                    let port = tokio::time::timeout(Duration::from_secs(5), async {
                        loop {
                            if let Ok(value) = fs::read_to_string(root.join("port"))
                                && let Ok(port) = value.parse::<std::num::NonZeroU16>()
                            {
                                break port;
                            }
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                    })
                    .await?;
                    Ok::<_, anyhow::Error>((child, port))
                })?;
                let url = format!("http://127.0.0.1:{port}/mcp");
                let mut draft = http_draft(mode, &url);
                if mode == "sse" {
                    draft.connection =
                        McpConnectionDefinition::Sse { url, headers: Default::default() };
                }
                let record = catalog.create(&scope, draft).unwrap().record;
                let selection = McpRuntimeSelection { project: project.clone(), record };
                catalog.select_for_runtime(&project, &scope, &selection).unwrap();
                let id = format!("mcp__{mode}__pid");
                let discover = || -> Option<Arc<dyn Tool>> {
                    execution_tools_for_context(&context)
                        .into_iter()
                        .find(|tool| tool.name() == id)
                        .map(Arc::from)
                };
                let cached = discover().expect("selected loopback server should expose its tool");
                let before = runtime.block_on(cached.call(json!({})))?;
                assert!(before.is_success());
                let live = ToolCatalog::default();
                live.replace(vec![cached.clone()], [id.clone()])?;

                assert_eq!(
                    catalog.disable_for_runtime(&other, &scope, &selection),
                    Err(McpFailureCode::PermissionDenied)
                );
                catalog.disable_for_runtime(&project, &scope, &selection).unwrap();
                catalog.disable_for_runtime(&project, &scope, &selection).unwrap();
                let error = runtime.block_on(cached.call(json!({}))).unwrap_err();
                assert!(format!("{error:#}").contains("runtime selection was revoked"));
                runtime.block_on(pool.refresh_tool_catalog(&live))?;
                assert!(!live.snapshot().authorizes(&id));
                assert!(discover().is_none());

                catalog.select_for_runtime(&project, &scope, &selection).unwrap();
                let fresh = discover().expect("reenabled server should expose a new executor");
                let error = runtime.block_on(cached.call(json!({}))).unwrap_err();
                assert!(format!("{error:#}").contains("runtime selection was revoked"));
                let after = runtime.block_on(fresh.call(json!({})))?;
                assert!(after.is_success());
                assert_ne!(before.model_text(), after.model_text());
                assert_eq!(fs::read_to_string(root.join("starts"))?.lines().count(), 2);

                catalog.disable_for_runtime(&project, &scope, &selection).unwrap();
                assert!(discover().is_none());
                runtime.block_on(child.kill())?;
            }
            crate::plugins::close_session_mcp_connections("disable", &workspace);
            Ok(())
        })
    })
}
