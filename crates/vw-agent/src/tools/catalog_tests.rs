use super::catalog::ToolCatalog;
use super::{Tool, ToolResult, ToolSpec, ToolUseContext};
use serde_json::{Value, json};
use std::sync::Arc;

struct VersionTool(&'static str);
#[async_trait::async_trait]
impl Tool for VersionTool {
    fn name(&self) -> &str {
        "version_tool"
    }
    fn description(&self) -> &str {
        self.0
    }
    fn parameters_schema(&self) -> Value {
        json!({"type":"object", "description":self.0})
    }
    async fn execute(&self, _: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: self.0.into(), error: None })
    }
}
fn tool(version: &'static str) -> Arc<dyn Tool> {
    Arc::new(VersionTool(version))
}

#[tokio::test]
async fn dynamic_tools_catalog_replacement_pins_inflight_executor() {
    let catalog = ToolCatalog::default();
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let first = catalog
        .replace(
            vec![Arc::new(GatedTool { started: started.clone(), release: release.clone() })],
            ["version_tool".into()],
        )
        .unwrap();
    let context = Arc::new(ToolUseContext::new("catalog", None).with_tool_catalog(first.clone()));
    let pending = first.bind("version_tool").unwrap();
    let active_snapshot = first.clone();
    let active = tokio::spawn(async move {
        active_snapshot
            .bind("version_tool")
            .unwrap()
            .execute(&active_snapshot, json!({}), context)
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(10), started.notified()).await.unwrap();
    let second = catalog.replace(vec![tool("second")], ["version_tool".into()]).unwrap();
    release.notify_one();
    assert!(second.version() > first.version());
    assert_eq!(first.specs()[0].description, "first");
    assert_eq!(second.specs()[0].input_schema["description"], "second");
    let result = active.await.unwrap().unwrap();
    assert_eq!(result.result.model_text(), "first");
    assert!(
        pending.execute(&second, json!({}), Arc::new(ToolUseContext::default())).await.is_err()
    );
    let empty = catalog.replace(vec![], []).unwrap();
    assert!(empty.version() > second.version());
    assert!(empty.bind("version_tool").is_err());
    assert!(catalog.snapshot().specs().is_empty());
}

#[test]
fn dynamic_tools_catalog_rejects_duplicates_without_publication() {
    let catalog = ToolCatalog::default();
    assert!(catalog.replace(vec![tool("first"), tool("second")], ["version_tool".into()]).is_err());
    assert_eq!(catalog.snapshot().version(), 0);
    assert!(catalog.replace(vec![tool("first")], ["unknown".into()]).is_err());
}

#[tokio::test]
async fn dynamic_tools_catalog_revocation_does_not_restore_loaded_ids() {
    let catalog = ToolCatalog::default();
    let old = catalog.replace(vec![tool("first")], ["version_tool".into()]).unwrap();
    let current = catalog.replace(vec![tool("second")], []).unwrap();
    let context = ToolUseContext::new("catalog", None)
        .with_tool_exposure_snapshot(["version_tool".into()])
        .with_tool_catalog(current.clone());
    assert!(!context.tool_is_exposed("version_tool"));
    assert!(current.bind("version_tool").is_err());
    assert!(
        old.bind("version_tool")
            .unwrap()
            .execute(&current, json!({}), Arc::new(context))
            .await
            .is_err()
    );
}

#[test]
fn dynamic_tools_catalog_rejects_reserved_client_names() {
    struct Reserved(vw_api_types::tools::ToolSource);
    #[async_trait::async_trait]
    impl Tool for Reserved {
        fn name(&self) -> &str {
            "exec"
        }
        fn description(&self) -> &str {
            "reserved"
        }
        fn parameters_schema(&self) -> Value {
            json!({"type":"object"})
        }
        fn spec(&self) -> ToolSpec {
            let mut spec = ToolSpec::new(self.name(), self.description(), self.parameters_schema());
            spec.source = self.0;
            spec
        }
        async fn execute(&self, _: Value) -> anyhow::Result<ToolResult> {
            unreachable!()
        }
    }
    for source in [vw_api_types::tools::ToolSource::Client, vw_api_types::tools::ToolSource::Mcp] {
        assert!(
            ToolCatalog::default()
                .replace(vec![Arc::new(Reserved(source))], ["exec".into()])
                .is_err()
        );
    }
}

#[tokio::test]
async fn dynamic_tools_catalog_same_number_from_another_catalog_is_rejected() {
    let first =
        ToolCatalog::default().replace(vec![tool("first")], ["version_tool".into()]).unwrap();
    let other =
        ToolCatalog::default().replace(vec![tool("second")], ["version_tool".into()]).unwrap();
    assert_eq!(first.version(), other.version());
    assert!(
        first
            .bind("version_tool")
            .unwrap()
            .execute(&other, json!({}), Arc::new(ToolUseContext::new("catalog", None)))
            .await
            .is_err()
    );
    let context = Arc::new(ToolUseContext::new("catalog", None).with_tool_catalog(other));
    assert!(first.bind("version_tool").unwrap().execute(&first, json!({}), context).await.is_err());
}

#[test]
fn dynamic_tools_catalog_session_and_context_share_snapshot_without_granting_metadata() {
    let snapshot = ToolCatalog::default().replace(vec![tool("first")], []).unwrap();
    assert!(snapshot.specs().is_empty());
    assert!(snapshot.tools().is_empty());
    let session = crate::session::runtime::SessionToolset::default().with_catalog(snapshot.clone());
    let context = ToolUseContext::new("catalog", None).with_tool_catalog(session.catalog().clone());
    assert!(Arc::ptr_eq(context.tool_catalog().unwrap(), &snapshot));
    assert!(!context.tool_is_exposed("version_tool"));
    assert!(snapshot.bind("version_tool").is_err());
}

struct GatedTool {
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
#[async_trait::async_trait]
impl Tool for GatedTool {
    fn name(&self) -> &str {
        "version_tool"
    }
    fn description(&self) -> &str {
        "first"
    }
    fn parameters_schema(&self) -> Value {
        VersionTool("first").parameters_schema()
    }
    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        self.started.notify_one();
        self.release.notified().await;
        VersionTool("first").execute(args).await
    }
}
