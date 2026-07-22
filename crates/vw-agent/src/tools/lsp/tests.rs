//! LSP 工具输入校验、请求映射、结果封装和缺失 server 路径测试。
//!
//! 当前测试避免依赖本机安装语言服务器，使用纯函数和临时目录覆盖确定性行为。

use super::super::*;
use super::format::{LspFormattedResponse, LspPayload};
use super::{Args, SUPPORTED_OPERATIONS, ValidatedArgs};
use crate::app::agent::security::SecurityPolicy;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn test_tool(workspace: &Path) -> LspTool {
    let mut security = SecurityPolicy::default();
    security.workspace_dir = workspace.to_path_buf();
    LspTool::new(Arc::new(security))
}

fn empty_args() -> Args {
    Args { operation: None, file_path: None, uri: None, line: None, character: None, query: None }
}

fn validated_args(operation: &str) -> ValidatedArgs {
    ValidatedArgs {
        operation: operation.to_string(),
        requested_path: "src/demo.rs".to_string(),
        absolute_path: PathBuf::from("/workspace/src/demo.rs"),
        line: Some(3),
        character: Some(5),
        query: Some("needle".to_string()),
    }
}

#[test]
fn lsp_schema_lists_supported_operations_and_position_constraints() {
    let schema = LspTool::schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"]["operation"]["enum"], json!(SUPPORTED_OPERATIONS));
    assert!(schema["properties"]["filePath"].is_object());
    assert!(schema["properties"]["uri"].is_object());
    assert_eq!(schema["properties"]["line"]["minimum"], 1);
    assert_eq!(schema["properties"]["character"]["minimum"], 1);
    assert!(schema["properties"]["query"].is_object());
}

#[test]
fn effective_file_path_prefers_explicit_path_and_decodes_file_uri() {
    let mut args = empty_args();
    args.file_path = Some("explicit.rs".to_string());
    args.uri = Some("file:///workspace/ignored%20file.rs".to_string());
    assert_eq!(LspTool::effective_file_path(&args).as_deref(), Some("explicit.rs"));

    args.file_path = None;
    assert_eq!(LspTool::effective_file_path(&args).as_deref(), Some("/workspace/ignored file.rs"));

    args.uri = Some("relative%20file.rs".to_string());
    assert_eq!(LspTool::effective_file_path(&args).as_deref(), Some("relative file.rs"));
}

#[test]
fn requires_position_distinguishes_symbol_operations() {
    for operation in [
        "goToDefinition",
        "findReferences",
        "hover",
        "goToImplementation",
        "prepareCallHierarchy",
        "incomingCalls",
        "outgoingCalls",
    ] {
        assert!(LspTool::requires_position(operation), "{operation}");
    }

    for operation in ["documentSymbol", "workspaceSymbol", "unknown"] {
        assert!(!LspTool::requires_position(operation), "{operation}");
    }
}

#[test]
fn resolve_requested_path_joins_relative_and_preserves_absolute_paths() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());
    assert_eq!(tool.resolve_requested_path("src/demo.rs"), workspace.path().join("src/demo.rs"));

    let absolute = workspace.path().join("absolute.rs");
    assert_eq!(tool.resolve_requested_path(absolute.to_string_lossy().as_ref()), absolute);
}

#[test]
fn lsp_spec_declares_read_only_strict_concurrent_contract() {
    let workspace = tempfile::tempdir().unwrap();
    let spec = test_tool(workspace.path()).spec();

    assert_eq!(spec.id, "lsp");
    assert_eq!(spec.display_name, "lsp");
    assert_eq!(spec.name, "lsp");
    assert!(!spec.description.trim().is_empty());
    assert_eq!(spec.input_schema, spec.parameters);
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}

#[test]
fn validate_input_rejects_missing_operation() {
    let workspace = tempfile::tempdir().unwrap();
    let error =
        test_tool(workspace.path()).validate_input(json!({ "filePath": "demo.rs" })).unwrap_err();

    assert!(error.to_string().contains("Missing operation"));
}

#[test]
fn validate_input_rejects_invalid_parameter_shape() {
    let workspace = tempfile::tempdir().unwrap();
    let error = test_tool(workspace.path()).validate_input(json!([])).unwrap_err();

    assert!(error.to_string().contains("Missing or invalid parameters"));
}

#[test]
fn validate_input_rejects_unsupported_operation() {
    let workspace = tempfile::tempdir().unwrap();
    let error = test_tool(workspace.path())
        .validate_input(json!({
            "operation": "rename",
            "filePath": "demo.rs"
        }))
        .unwrap_err();

    assert!(error.to_string().contains("Unsupported LSP operation: rename"));
}

#[test]
fn validate_input_rejects_missing_file_path() {
    let workspace = tempfile::tempdir().unwrap();
    let error = test_tool(workspace.path())
        .validate_input(json!({ "operation": "documentSymbol" }))
        .unwrap_err();

    assert!(error.to_string().contains("Missing filePath"));
}

#[test]
fn validate_input_accepts_path_alias_and_uri_for_position_free_operations() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());
    let path_input = json!({ "operation": "documentSymbol", "path": "demo.rs" });
    let uri_input = json!({ "operation": "workspaceSymbol", "uri": "file:///demo.rs" });

    assert_eq!(tool.validate_input(path_input.clone()).unwrap(), path_input);
    assert_eq!(tool.validate_input(uri_input.clone()).unwrap(), uri_input);
}

#[tokio::test]
async fn lsp_rejects_missing_required_position() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());

    let error = tool
        .validate_input(json!({
            "operation": "hover",
            "filePath": "demo.rs"
        }))
        .unwrap_err();

    assert!(error.to_string().contains("Missing line or character"));
}

#[tokio::test]
async fn validate_args_rejects_zero_position_before_file_access() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());
    let mut args = empty_args();
    args.operation = Some("hover".to_string());
    args.file_path = Some("missing.rs".to_string());
    args.line = Some(0);
    args.character = Some(1);

    let error = tool.validate_args(&args).await.unwrap_err();

    assert!(error.to_string().contains("line and character must be >= 1"));
}

#[tokio::test]
async fn validate_args_rejects_parent_directory_traversal() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());
    let mut args = empty_args();
    args.operation = Some("documentSymbol".to_string());
    args.file_path = Some("../escape.rs".to_string());

    let error = tool.validate_args(&args).await.unwrap_err();

    assert!(error.to_string().contains("Path not allowed by security policy"));
}

#[tokio::test]
async fn validate_args_trims_values_and_canonicalizes_existing_file() {
    let workspace = tempfile::tempdir().unwrap();
    let file_path = workspace.path().join("demo.rs");
    tokio::fs::write(&file_path, "fn main() {}").await.unwrap();
    let tool = test_tool(workspace.path());
    let mut args = empty_args();
    args.operation = Some(" workspaceSymbol ".to_string());
    args.file_path = Some(" demo.rs ".to_string());
    args.query = Some(" needle ".to_string());

    let validated = tool.validate_args(&args).await.unwrap();

    assert_eq!(validated.operation, "workspaceSymbol");
    assert_eq!(validated.requested_path, "demo.rs");
    assert_eq!(validated.absolute_path, file_path.canonicalize().unwrap());
    assert_eq!(validated.query.as_deref(), Some("needle"));
}

#[tokio::test]
async fn validate_args_rejects_directory_target() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());
    let mut args = empty_args();
    args.operation = Some("documentSymbol".to_string());
    args.file_path = Some(".".to_string());

    let error = tool.validate_args(&args).await.unwrap_err();

    assert!(error.to_string().contains("Path is not a file"));
}

#[tokio::test]
async fn validate_args_rejects_missing_and_unsupported_values_before_file_access() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());

    let error = tool.validate_args(&empty_args()).await.unwrap_err();
    assert_eq!(error.to_string(), "Missing operation");

    let mut args = empty_args();
    args.operation = Some("rename".to_string());
    args.file_path = Some("missing.rs".to_string());
    let error = tool.validate_args(&args).await.unwrap_err();
    assert_eq!(error.to_string(), "Unsupported LSP operation: rename");

    args.operation = Some("documentSymbol".to_string());
    args.file_path = Some("   ".to_string());
    let error = tool.validate_args(&args).await.unwrap_err();
    assert_eq!(error.to_string(), "Missing filePath");
}

#[tokio::test]
async fn lsp_execute_maps_backend_failure_to_failed_tool_result() {
    let workspace = tempfile::tempdir().unwrap();
    let file_path = workspace.path().join("demo.txt");
    tokio::fs::write(&file_path, "hello").await.unwrap();

    let result = test_tool(workspace.path())
        .execute(json!({
            "operation": "hover",
            "filePath": file_path.to_string_lossy(),
            "line": 1,
            "character": 1
        }))
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result.output.contains("No LSP server available for file type: .txt"));
    assert_eq!(result.error.as_deref(), Some(result.output.as_str()));
}

#[test]
fn method_and_params_maps_position_operations_to_zero_based_coordinates() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());

    for (operation, expected_method) in [
        ("goToDefinition", "textDocument/definition"),
        ("hover", "textDocument/hover"),
        ("goToImplementation", "textDocument/implementation"),
    ] {
        let args = validated_args(operation);
        let (method, params) = tool.method_and_params(&args, "file:///workspace/src/demo.rs");
        assert_eq!(method, expected_method);
        assert_eq!(params["textDocument"]["uri"], "file:///workspace/src/demo.rs");
        assert_eq!(params["position"], json!({ "line": 2, "character": 4 }));
    }
}

#[test]
fn method_and_params_adds_reference_declaration_context() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());
    let args = validated_args("findReferences");

    let (method, params) = tool.method_and_params(&args, "file:///workspace/src/demo.rs");

    assert_eq!(method, "textDocument/references");
    assert_eq!(params["position"], json!({ "line": 2, "character": 4 }));
    assert_eq!(params["context"]["includeDeclaration"], json!(true));
}

#[test]
fn method_and_params_maps_document_and_workspace_symbols() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());

    let document_args = validated_args("documentSymbol");
    let (method, params) = tool.method_and_params(&document_args, "file:///workspace/src/demo.rs");
    assert_eq!(method, "textDocument/documentSymbol");
    assert_eq!(params, json!({ "textDocument": { "uri": "file:///workspace/src/demo.rs" } }));

    let workspace_args = validated_args("workspaceSymbol");
    let (method, params) = tool.method_and_params(&workspace_args, "ignored");
    assert_eq!(method, "workspace/symbol");
    assert_eq!(params, json!({ "query": "needle" }));

    let mut empty_query_args = validated_args("workspaceSymbol");
    empty_query_args.query = None;
    let (_, params) = tool.method_and_params(&empty_query_args, "ignored");
    assert_eq!(params, json!({ "query": "" }));
}

#[test]
fn method_and_params_prepares_all_call_hierarchy_operations() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());

    for operation in ["prepareCallHierarchy", "incomingCalls", "outgoingCalls"] {
        let args = validated_args(operation);
        let (method, params) = tool.method_and_params(&args, "file:///workspace/src/demo.rs");
        assert_eq!(method, "textDocument/prepareCallHierarchy", "{operation}");
        assert_eq!(params["position"], json!({ "line": 2, "character": 4 }));
    }
}

#[test]
fn method_and_params_falls_back_to_hover_with_default_position() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());
    let mut args = validated_args("unreachable");
    args.line = None;
    args.character = None;

    let (method, params) = tool.method_and_params(&args, "file:///workspace/src/demo.rs");

    assert_eq!(method, "textDocument/hover");
    assert_eq!(params["position"], json!({ "line": 0, "character": 0 }));
}

#[test]
fn success_result_maps_every_payload_kind_and_common_metadata() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());
    let args = validated_args("hover");
    let payloads = [
        (LspPayload::Locations { items: Vec::new() }, "locations"),
        (
            LspPayload::Hover { contents: "docs".to_string(), line: Some(3), character: Some(5) },
            "hover",
        ),
        (LspPayload::DocumentSymbols { items: Vec::new() }, "document_symbols"),
        (LspPayload::CallHierarchyItems { items: Vec::new() }, "call_hierarchy_items"),
        (LspPayload::CallHierarchyCalls { items: Vec::new() }, "call_hierarchy_calls"),
        (LspPayload::Message { message: "done".to_string() }, "message"),
    ];

    for (payload, expected_kind) in payloads {
        let result = tool.success_result(
            &args,
            LspFormattedResponse {
                result_text: "formatted".to_string(),
                summary: "summary".to_string(),
                result_count: 2,
                file_count: 1,
                payload,
            },
            "rust-analyzer",
            "rust",
        );

        assert!(result.is_success());
        assert_eq!(result.model_text(), "formatted");
        assert_eq!(result.data["success"], json!(true));
        assert_eq!(result.data["server_key"], "rust-analyzer");
        assert_eq!(result.data["language_id"], "rust");
        assert_eq!(result.content_blocks.len(), 2);
        assert_eq!(result.render_metadata()["payload_kind"], expected_kind);
        assert_eq!(result.render_metadata()["result_count"], 2);
    }
}

#[test]
fn failure_result_includes_error_telemetry_and_server_metadata() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = test_tool(workspace.path());
    let args = validated_args("hover");

    let result = tool.failure_result(&args, "backend unavailable", Some("rust-analyzer"));

    assert!(!result.is_success());
    assert_eq!(result.model_text(), "backend unavailable");
    assert_eq!(result.error_text().as_deref(), Some("backend unavailable"));
    assert_eq!(result.data["success"], json!(false));
    assert_eq!(result.data["implemented"], json!(true));
    assert_eq!(result.data["result_count"], 0);
    assert_eq!(result.data["file_count"], 0);
    assert_eq!(result.data["payload"]["kind"], "message");
    assert_eq!(result.data["server_key"], "rust-analyzer");
    assert_eq!(result.content_blocks.len(), 2);
    assert_eq!(result.render_metadata()["success"], json!(false));
    assert_eq!(result.render_metadata()["server_key"], "rust-analyzer");
}

#[tokio::test]
async fn lsp_returns_missing_server_error_for_unsupported_extension() {
    let workspace = tempfile::tempdir().unwrap();
    let file_path = workspace.path().join("demo.txt");
    tokio::fs::write(&file_path, "hello").await.unwrap();

    let tool = test_tool(workspace.path());
    let result = tool
        .call(json!({
            "operation": "hover",
            "filePath": file_path.to_string_lossy(),
            "line": 1,
            "character": 1
        }))
        .await
        .unwrap();

    assert!(!result.is_success());
    assert!(result.model_text().contains("No LSP server available for file type: .txt"));
}
