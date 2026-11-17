//! State matrices over the real catalog, Tool executors and shared connection pool.
use std::{fs, path::Path, sync::Arc, time::Duration};
use serde_json::json;
use crate::mcp::McpCatalogStore;
use vw_api_transport::ProjectScope;
use vw_api_mcp_management::{McpCatalogScope, McpRecordRef, McpRuntimeSelection, McpFailureCode, McpConnectionDefinition};
use tool_types::Tool;
use tool_catalog::catalog::ToolCatalog;
use plugin_mcp::mcp_connections::McpSessionConnections;
fn connection(script: &Path, calls: &Path, label: &str) -> McpConnectionDefinition {
    McpConnectionDefinition::Stdio { command: "python3".into(), args: vec![script.display().to_string(), calls.display().to_string(), label.into()], env: Default::default(), cwd: None }
}
pub fn update_delete(
    catalog: &McpCatalogStore, scope: &McpCatalogScope, project: &ProjectScope,
    first: &McpRecordRef, script: &Path, calls: &Path, runtime: &tokio::runtime::Runtime,
    pool: &Arc<McpSessionConnections>, discover: impl Fn() -> Option<Arc<dyn Tool>>,
) -> anyhow::Result<()> {
        let id = "mcp__managed__read";
        let select = |record| { catalog.select_for_runtime(project, scope, &McpRuntimeSelection { project: project.clone(), record }).unwrap(); };
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
    Ok(())
}

pub fn disable_reselect(
    catalog: &McpCatalogStore, scope: &McpCatalogScope, project: &ProjectScope, other: &ProjectScope,
    selection: &McpRuntimeSelection, root: &Path, id: &str, runtime: &tokio::runtime::Runtime,
    pool: &Arc<McpSessionConnections>, discover: impl Fn() -> Option<Arc<dyn Tool>>,
) -> anyhow::Result<()> {
    let cached = discover().expect("selected loopback server should expose its tool");
    let before = runtime.block_on(cached.call(json!({})))?;
    assert!(before.is_success());
    let live = ToolCatalog::default();
    live.replace(vec![cached.clone()], [id.to_owned()])?;

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
    Ok(())
}
