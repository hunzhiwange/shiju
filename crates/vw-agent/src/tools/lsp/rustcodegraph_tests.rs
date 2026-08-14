//! RustCodeGraph 库内查询适配器的真实索引与结果映射测试。

use super::rustcodegraph::SUPPORTED_OPERATIONS;
use crate::tools::RustCodeGraphTool;
use crate::tools::traits::Tool;
use rustcodegraph::sync::watcher::__emit_watch_event_for_tests;
use rustcodegraph::{CodeGraph, InitOptions, WatchOptions};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn indexed_workspace() -> tempfile::TempDir {
    let workspace = tempfile::tempdir().unwrap();
    let source_dir = workspace.path().join("src");
    std::fs::create_dir(&source_dir).unwrap();
    std::fs::write(
        source_dir.join("lib.rs"),
        "pub fn indexed_entry() -> usize { indexed_leaf() }\n\
         fn indexed_leaf() -> usize { 7 }\n",
    )
    .unwrap();

    let mut graph = CodeGraph::init(workspace.path(), InitOptions { index: true }).unwrap();
    graph.close();
    workspace
}

fn test_tool(workspace: &Path) -> RustCodeGraphTool {
    RustCodeGraphTool::new(workspace.to_path_buf())
        .expect("pre-indexed test workspace should be available")
}

fn test_tool_with_short_debounce(workspace: &Path) -> RustCodeGraphTool {
    RustCodeGraphTool::new_with_watch_options(
        workspace.to_path_buf(),
        WatchOptions {
            debounce_ms: Some(20),
            max_debounce_ms: Some(100),
            min_sync_interval_ms: Some(0),
        },
    )
    .expect("pre-indexed test workspace should be available")
}

#[tokio::test]
async fn rustcodegraph_execute_explore_returns_indexed_flow() {
    let workspace = indexed_workspace();

    let result = test_tool(workspace.path())
        .call(json!({
            "operation": "explore",
            "query": "indexed_entry indexed_leaf"
        }))
        .await
        .unwrap();

    assert!(result.is_success());
    assert_eq!(result.data["operation"], "explore");
    assert_eq!(result.data["library_tool"], "rustcodegraph_explore");
    assert!(result.model_text().contains("indexed_entry"));
    assert!(result.model_text().contains("indexed_leaf"));
    assert_eq!(
        result.render_hint.as_ref().and_then(|hint| hint.title.as_deref()),
        Some("RustCodeGraph")
    );
}

#[test]
fn rustcodegraph_schema_exposes_supported_operations_without_project_path() {
    let workspace = indexed_workspace();
    let schema = test_tool(workspace.path()).parameters_schema();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"]["operation"]["enum"], json!(SUPPORTED_OPERATIONS));
    assert_eq!(schema["required"], json!(["operation"]));
    assert!(schema["properties"].get("projectPath").is_none());
    assert!(schema["properties"].get("project_path").is_none());
}

#[test]
fn rustcodegraph_spec_is_read_only_strict_and_concurrent() {
    let workspace = indexed_workspace();
    let tool = test_tool(workspace.path());
    let spec = tool.spec();

    assert_eq!(spec.id, "rustcodegraph");
    assert_eq!(spec.display_name, "rustcodegraph");
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
    assert_eq!(spec.input_schema["additionalProperties"], false);
    assert!(spec.description.contains("Use `explore` first"));
}

#[tokio::test]
async fn rustcodegraph_preserves_recoverable_empty_results() {
    let workspace = indexed_workspace();

    let result = test_tool(workspace.path())
        .call(json!({
            "operation": "search",
            "query": "symbol_that_does_not_exist_anywhere"
        }))
        .await
        .unwrap();

    assert!(result.is_success());
    assert!(result.model_text().contains("No results found"));
    assert_eq!(result.data["is_error"], false);
    assert_eq!(result.telemetry.as_ref().map(|telemetry| telemetry.success), Some(true));
}

#[tokio::test]
async fn rustcodegraph_first_call_syncs_preexisting_edits() {
    let workspace = indexed_workspace();
    let tool = test_tool(workspace.path());
    std::fs::write(
        workspace.path().join("src/lib.rs"),
        "pub fn added_before_first_query() -> usize { 11 }\n",
    )
    .unwrap();

    let result = tool
        .call(json!({
            "operation": "search",
            "query": "added_before_first_query"
        }))
        .await
        .unwrap();

    assert!(result.is_success());
    assert!(result.model_text().contains("added_before_first_query"));
    assert!(!result.model_text().contains("No results found"));
}

#[tokio::test]
async fn rustcodegraph_observes_workspace_edits_without_restart() {
    let workspace = indexed_workspace();
    let tool = test_tool_with_short_debounce(workspace.path());
    tool.call(json!({
        "operation": "search",
        "query": "indexed_entry"
    }))
    .await
    .unwrap();

    std::fs::write(
        workspace.path().join("src/lib.rs"),
        "pub fn added_after_first_query() -> usize { 13 }\n",
    )
    .unwrap();

    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let result = tool
            .call(json!({
                "operation": "search",
                "query": "added_after_first_query"
            }))
            .await
            .unwrap();
        if result.model_text().contains("added_after_first_query")
            && !result.model_text().contains("No results found")
        {
            break;
        }
        assert!(Instant::now() < deadline, "watcher did not refresh the index");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[test]
fn rustcodegraph_rejects_cross_workspace_paths() {
    let parent = indexed_workspace();
    let nested_workspace = parent.path().join("nested-workspace");
    std::fs::create_dir(&nested_workspace).unwrap();

    let error = RustCodeGraphTool::new(nested_workspace.clone())
        .err()
        .expect("a parent index must not be discovered");
    assert!(error.to_string().contains("not initialized"));
    assert!(!nested_workspace.join(".rustcodegraph").exists());

    let tool = test_tool(parent.path());
    for input in [
        json!({ "operation": "node", "file": "../outside.rs" }),
        json!({ "operation": "node", "file": absolute_outside_path(parent.path()) }),
        json!({ "operation": "files", "path": "../outside" }),
        json!({ "operation": "files", "pattern": "../../**/*.rs" }),
        json!({
            "operation": "search",
            "query": "indexed_entry",
            "projectPath": parent.path().to_string_lossy()
        }),
    ] {
        let error = tool.validate_input(input).unwrap_err();
        assert!(
            error.to_string().contains("relative to the current workspace")
                || error.to_string().contains("unknown field `projectPath`"),
            "unexpected validation error: {error}"
        );
    }

    tool.validate_input(json!({ "operation": "node", "file": "lib.rs" }))
        .unwrap();
    tool.validate_input(json!({ "operation": "files", "pattern": "**/*.rs" }))
        .unwrap();
}

#[tokio::test]
async fn rustcodegraph_drop_stops_owned_watcher() {
    let workspace = indexed_workspace();
    let tool = test_tool_with_short_debounce(workspace.path());
    tool.call(json!({
        "operation": "search",
        "query": "indexed_entry"
    }))
    .await
    .unwrap();

    assert!(__emit_watch_event_for_tests(workspace.path(), "src/lib.rs"));
    drop(tool);
    assert!(!__emit_watch_event_for_tests(workspace.path(), "src/lib.rs"));
}

fn absolute_outside_path(workspace: &Path) -> PathBuf {
    workspace.parent().unwrap().join("outside.rs")
}
