//! `notebook_edit` 工具测试。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
    use crate::app::agent::tools::FileSnapshot;
    use crate::app::agent::tools::ToolUseContext;
    use crate::app::agent::tools::context::scope_tool_use_context;
    use serde_json::json;
    use std::sync::Arc;
    use vw_api_types::tools::ToolResultContentDto;

    fn test_security(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace,
            ..SecurityPolicy::default()
        })
    }

    fn notebook_text(value: serde_json::Value) -> String {
        let mut text = serde_json::to_string_pretty(&value).unwrap();
        text.push('\n');
        text
    }

    fn seed_read_state(
        context: &Arc<ToolUseContext>,
        workspace: &std::path::Path,
        path: &str,
        content: &str,
    ) {
        let read_state = context.read_state_handle();
        read_state.lock().unwrap_or_else(|error| error.into_inner()).note_read(
            Some(workspace),
            path,
            content.len(),
            false,
            None,
            None,
            Some(FileSnapshot::from_text(content)),
        );
    }

    fn sample_notebook() -> serde_json::Value {
        json!({
            "cells": [
                {
                    "cell_type": "markdown",
                    "id": "cell-1",
                    "metadata": {"id": "cell-1", "language": "markdown"},
                    "source": ["# Title\n"]
                },
                {
                    "cell_type": "code",
                    "id": "cell-2",
                    "metadata": {"id": "cell-2", "language": "python"},
                    "source": ["print('old')\n"],
                    "outputs": [],
                    "execution_count": null
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        })
    }

    #[test]
    fn notebook_edit_name() {
        let tool = NotebookEditTool::new(test_security(std::env::temp_dir()));
        assert_eq!(tool.name(), "notebook_edit");
    }

    #[test]
    fn notebook_edit_description_and_schema_advertise_notebook_only_contract() {
        let tool = NotebookEditTool::new(test_security(std::env::temp_dir()));

        assert!(tool.description().contains("Existing notebooks must be read first"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], json!("object"));
        assert_eq!(schema["required"], json!(["path", "edit_type"]));
        assert_eq!(schema["properties"]["edit_type"]["enum"], json!(["insert", "edit", "delete"]));
        assert_eq!(
            schema["properties"]["position"]["enum"],
            json!(["before", "after", "start", "end"])
        );
        assert!(
            schema["properties"]["cell"]["description"]
                .as_str()
                .unwrap_or_default()
                .contains("完整 cell JSON")
        );
    }

    #[test]
    fn normalize_document_initializes_null_metadata_and_rejects_scalars() {
        let mut null_metadata = NotebookDocument::default();
        NotebookEditTool::normalize_document(&mut null_metadata).unwrap();
        assert_eq!(null_metadata.metadata, json!({}));

        let mut scalar_metadata =
            NotebookDocument { metadata: json!("invalid"), ..NotebookDocument::default() };
        let error = NotebookEditTool::normalize_document(&mut scalar_metadata).unwrap_err();
        assert_eq!(error.to_string(), "Notebook metadata must be a JSON object");
    }

    #[test]
    fn prepare_cell_forces_id_and_clears_non_code_execution_state() {
        let cell = NotebookCell {
            cell_type: "markdown".into(),
            metadata: Value::Null,
            source: json!("# heading"),
            outputs: vec![json!({"stale": true})],
            execution_count: json!(9),
            ..NotebookCell::default()
        };

        let prepared = NotebookEditTool::prepare_cell(cell, Some("forced-id".into())).unwrap();
        assert_eq!(prepared.id.as_deref(), Some("forced-id"));
        assert_eq!(prepared.metadata["id"], json!("forced-id"));
        assert!(prepared.outputs.is_empty());
        assert!(prepared.execution_count.is_null());
    }

    #[test]
    fn cell_selectors_reject_conflicts_and_invalid_insert_combinations() {
        let cells = vec![
            NotebookCell { id: Some("first".into()), ..NotebookCell::default() },
            NotebookCell { id: Some("second".into()), ..NotebookCell::default() },
        ];

        let conflict =
            NotebookEditTool::resolve_cell_index(&cells, Some("first"), Some(2)).unwrap_err();
        assert!(conflict.to_string().contains("different notebook cells"));

        let invalid_start = NotebookEditTool::resolve_insert_index(
            &cells,
            Some("first"),
            None,
            Some(InsertPosition::Start),
        )
        .unwrap_err();
        assert!(invalid_start.to_string().contains("position=start"));

        let (end_index, position) =
            NotebookEditTool::resolve_insert_index(&cells, None, None, None).unwrap();
        assert_eq!(end_index, cells.len());
        assert_eq!(position.as_str(), "end");
    }

    #[test]
    fn notebook_cell_helpers_cover_language_metadata_and_validation_edges() {
        assert_eq!(
            NotebookEditTool::notebook_cell_language(&json!({"language_info": {"name": "python"}})),
            Some("python".to_string())
        );
        assert_eq!(
            NotebookEditTool::notebook_cell_language(&json!({"vscode": {"languageId": "rust"}})),
            Some("rust".to_string())
        );
        assert_eq!(NotebookEditTool::notebook_cell_language(&json!({"language": 3})), None);

        let descriptor = NotebookEditTool::cell_descriptor(
            &NotebookCell {
                cell_type: "markdown".into(),
                metadata: json!({"id": "metadata-id", "language": "markdown"}),
                source: json!("body"),
                ..NotebookCell::default()
            },
            4,
        );
        assert_eq!(descriptor.cell_number, 5);
        assert_eq!(descriptor.cell_id.as_deref(), Some("metadata-id"));
        assert_eq!(descriptor.language.as_deref(), Some("markdown"));

        let code_cell = NotebookCell {
            cell_type: "code".into(),
            metadata: json!({"id": "kept-id"}),
            source: json!(["print('ok')\n"]),
            outputs: vec![json!({"output_type": "stream"})],
            execution_count: json!(7),
            ..NotebookCell::default()
        };
        let prepared_code = NotebookEditTool::prepare_cell(code_cell, None).unwrap();
        assert_eq!(prepared_code.id.as_deref(), Some("kept-id"));
        assert_eq!(prepared_code.outputs.len(), 1);
        assert_eq!(prepared_code.execution_count, json!(7));

        let empty_type = NotebookEditTool::prepare_cell(
            NotebookCell {
                cell_type: "  ".into(),
                source: json!("body"),
                ..NotebookCell::default()
            },
            None,
        )
        .unwrap_err();
        assert!(empty_type.to_string().contains("cell.cell_type"));

        let scalar_metadata = NotebookEditTool::prepare_cell(
            NotebookCell {
                cell_type: "markdown".into(),
                metadata: json!("bad"),
                source: json!("body"),
                ..NotebookCell::default()
            },
            None,
        )
        .unwrap_err();
        assert!(scalar_metadata.to_string().contains("cell.metadata"));

        let invalid_source = NotebookEditTool::prepare_cell(
            NotebookCell {
                cell_type: "markdown".into(),
                metadata: json!({}),
                source: json!(["ok", 1]),
                ..NotebookCell::default()
            },
            None,
        )
        .unwrap_err();
        assert!(invalid_source.to_string().contains("cell.source"));
    }

    #[test]
    fn cell_selector_helpers_report_missing_targets_and_before_after_positions() {
        let cells = vec![
            NotebookCell { id: Some("first".into()), ..NotebookCell::default() },
            NotebookCell { id: Some("second".into()), ..NotebookCell::default() },
        ];

        let missing_id =
            NotebookEditTool::resolve_cell_index(&cells, Some("missing"), None).unwrap_err();
        assert!(missing_id.to_string().contains("Notebook cell id not found"));

        let invalid_number =
            NotebookEditTool::resolve_cell_index(&cells, None, Some(0)).unwrap_err();
        assert!(invalid_number.to_string().contains("out of range"));

        let missing_selector =
            NotebookEditTool::resolve_cell_index(&cells, None, None).unwrap_err();
        assert!(missing_selector.to_string().contains("requires cell_id or cell_number"));

        let (before_index, before_position) = NotebookEditTool::resolve_insert_index(
            &cells,
            Some("second"),
            Some(2),
            Some(InsertPosition::Before),
        )
        .unwrap();
        assert_eq!(before_index, 1);
        assert_eq!(before_position.as_str(), "before");

        let (after_index, after_position) =
            NotebookEditTool::resolve_insert_index(&cells, Some("first"), None, None).unwrap();
        assert_eq!(after_index, 1);
        assert_eq!(after_position.as_str(), "after");

        let invalid_end = NotebookEditTool::resolve_insert_index(
            &cells,
            Some("first"),
            None,
            Some(InsertPosition::End),
        )
        .unwrap_err();
        assert!(invalid_end.to_string().contains("position=end"));
    }

    #[tokio::test]
    async fn notebook_edit_inserts_cell_at_end() {
        let dir = std::env::temp_dir().join("vibewindow_test_notebook_edit_insert");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let document: NotebookDocument = serde_json::from_value(sample_notebook()).unwrap();
        let original = NotebookEditTool::serialize_document(&document).unwrap();
        tokio::fs::write(dir.join("demo.ipynb"), &original).await.unwrap();

        let tool = NotebookEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "notebook-edit-insert",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "demo.ipynb", &original);

        let result = scope_tool_use_context(
            context.clone(),
            tool.call(json!({
                "path": "demo.ipynb",
                "edit_type": "insert",
                "position": "end",
                "cell": {
                    "cell_type": "code",
                    "metadata": {"language": "python"},
                    "source": ["print('new')\n"],
                    "outputs": [],
                    "execution_count": null
                }
            })),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["kind"], json!("insert"));
        assert_eq!(result.data["cell"]["cell_number"], json!(3));
        assert_eq!(
            result.render_hint.as_ref().and_then(|hint| hint.kind.as_deref()),
            Some("notebook_edit")
        );
        assert!(matches!(
            result.content_blocks.first(),
            Some(ToolResultContentDto::StructuredPatch { hunks }) if !hunks.is_empty()
        ));

        let written: serde_json::Value =
            serde_json::from_str(&tokio::fs::read_to_string(dir.join("demo.ipynb")).await.unwrap())
                .unwrap();
        assert_eq!(written["cells"].as_array().unwrap().len(), 3);
        assert!(written["cells"][2]["id"].as_str().is_some_and(|value| !value.is_empty()));

        let mut snapshot = context.read_state_snapshot();
        let entry = snapshot.get(Some(dir.as_path()), "demo.ipynb").expect("read state missing");
        assert_eq!(
            entry.snapshot,
            Some(FileSnapshot::from_text(
                &tokio::fs::read_to_string(dir.join("demo.ipynb")).await.unwrap()
            ))
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn notebook_edit_inserts_before_selected_cell_using_compatible_aliases() {
        let dir = std::env::temp_dir().join("vibewindow_test_notebook_edit_insert_before");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let original = notebook_text(sample_notebook());
        tokio::fs::write(dir.join("demo.ipynb"), &original).await.unwrap();

        let tool = NotebookEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "notebook-edit-insert-before",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "demo.ipynb", &original);

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "filePath": "demo.ipynb",
                "operation": "insert",
                "cell_number": 2,
                "position": "before",
                "cell": {
                    "cell_type": "markdown",
                    "metadata": {"id": "inserted-id", "vscode": {"languageId": "markdown"}},
                    "source": "Inserted note",
                    "outputs": [{"stale": true}],
                    "execution_count": 42
                }
            })),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["kind"], json!("insert"));
        assert_eq!(result.data["position"], json!("before"));
        assert_eq!(result.data["cell"]["cell_number"], json!(2));
        assert_eq!(result.data["cell"]["cell_id"], json!("inserted-id"));
        assert_eq!(result.data["cell"]["language"], json!("markdown"));
        assert_eq!(result.render_hint.as_ref().unwrap().metadata["operation"], json!("insert"));

        let written: serde_json::Value =
            serde_json::from_str(&tokio::fs::read_to_string(dir.join("demo.ipynb")).await.unwrap())
                .unwrap();
        assert_eq!(written["cells"][1]["id"], json!("inserted-id"));
        assert_eq!(written["cells"][1]["outputs"], json!([]));
        assert!(written["cells"][1]["execution_count"].is_null());
        assert_eq!(written["cells"][2]["id"], json!("cell-2"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn notebook_edit_replaces_cell_by_id() {
        let dir = std::env::temp_dir().join("vibewindow_test_notebook_edit_replace");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let original = notebook_text(sample_notebook());
        tokio::fs::write(dir.join("demo.ipynb"), &original).await.unwrap();

        let tool = NotebookEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "notebook-edit-replace",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "demo.ipynb", &original);

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "path": "demo.ipynb",
                "edit_type": "edit",
                "cell_id": "cell-2",
                "cell": {
                    "cell_type": "code",
                    "metadata": {"language": "python"},
                    "source": ["print('updated')\n"],
                    "outputs": [],
                    "execution_count": null
                }
            })),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["kind"], json!("edit"));
        assert_eq!(result.data["cell"]["cell_id"], json!("cell-2"));

        let written: serde_json::Value =
            serde_json::from_str(&tokio::fs::read_to_string(dir.join("demo.ipynb")).await.unwrap())
                .unwrap();
        assert_eq!(written["cells"][1]["source"][0], json!("print('updated')\n"));
        assert_eq!(written["cells"][1]["id"], json!("cell-2"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn notebook_edit_reports_unchanged_replacement_without_patch_blocks() {
        let dir = std::env::temp_dir().join("vibewindow_test_notebook_edit_no_change");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let document: NotebookDocument = serde_json::from_value(sample_notebook()).unwrap();
        let original = NotebookEditTool::serialize_document(&document).unwrap();
        tokio::fs::write(dir.join("demo.ipynb"), &original).await.unwrap();

        let tool = NotebookEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "notebook-edit-no-change",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "demo.ipynb", &original);

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "path": "demo.ipynb",
                "editType": "edit",
                "cell_id": "cell-2",
                "cell": {
                    "cell_type": "code",
                    "id": "cell-2",
                    "metadata": {"id": "cell-2", "language": "python"},
                    "source": ["print('old')\n"],
                    "outputs": [],
                    "execution_count": null
                }
            })),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["kind"], json!("edit"));
        assert_eq!(result.data["changed"], json!(false));
        assert!(result.content_blocks.is_empty());
        assert!(
            result
                .model_result
                .as_str()
                .unwrap_or_default()
                .contains("did not change notebook contents")
        );
        assert_eq!(tokio::fs::read_to_string(dir.join("demo.ipynb")).await.unwrap(), original);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn notebook_edit_deletes_cell_by_number() {
        let dir = std::env::temp_dir().join("vibewindow_test_notebook_edit_delete");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let original = notebook_text(sample_notebook());
        tokio::fs::write(dir.join("demo.ipynb"), &original).await.unwrap();

        let tool = NotebookEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "notebook-edit-delete",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "demo.ipynb", &original);

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "path": "demo.ipynb",
                "edit_type": "delete",
                "cell_number": 1
            })),
        )
        .await
        .unwrap();

        assert!(result.is_success(), "error: {:?}", result.error_text());
        assert_eq!(result.data["kind"], json!("delete"));
        assert_eq!(result.data["total_cells"], json!(1));

        let written: serde_json::Value =
            serde_json::from_str(&tokio::fs::read_to_string(dir.join("demo.ipynb")).await.unwrap())
                .unwrap();
        assert_eq!(written["cells"].as_array().unwrap().len(), 1);
        assert_eq!(written["cells"][0]["id"], json!("cell-2"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn notebook_edit_rejects_missing_required_cell_payloads() {
        let dir = std::env::temp_dir().join("vibewindow_test_notebook_edit_missing_cell");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let original = notebook_text(sample_notebook());
        tokio::fs::write(dir.join("demo.ipynb"), &original).await.unwrap();

        let tool = NotebookEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "notebook-edit-missing-cell",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "demo.ipynb", &original);

        let insert_result = scope_tool_use_context(
            context.clone(),
            tool.call(json!({
                "path": "demo.ipynb",
                "edit_type": "insert",
                "position": "end"
            })),
        )
        .await
        .unwrap();
        assert!(!insert_result.is_success());
        assert!(
            insert_result
                .error_text()
                .unwrap_or_default()
                .contains("insert requires a cell payload")
        );

        let edit_result = scope_tool_use_context(
            context,
            tool.call(json!({
                "path": "demo.ipynb",
                "edit_type": "edit",
                "cell_number": 1
            })),
        )
        .await
        .unwrap();
        assert!(!edit_result.is_success());
        assert!(
            edit_result.error_text().unwrap_or_default().contains("edit requires a cell payload")
        );

        assert_eq!(tokio::fs::read_to_string(dir.join("demo.ipynb")).await.unwrap(), original);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn notebook_edit_execute_returns_legacy_success_and_failure_results() {
        let dir = std::env::temp_dir().join("vibewindow_test_notebook_edit_execute");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let original = notebook_text(sample_notebook());
        tokio::fs::write(dir.join("demo.ipynb"), &original).await.unwrap();

        let tool = NotebookEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "notebook-edit-execute",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "demo.ipynb", &original);

        let invalid_parameters = tool.execute(json!({"path": "demo.ipynb"})).await.unwrap_err();
        assert!(invalid_parameters.to_string().contains("Missing or invalid parameters"));

        let missing_path = tool.execute(json!({"path": "", "edit_type": "delete"})).await.unwrap();
        assert!(!missing_path.success);
        assert_eq!(missing_path.error.as_deref(), Some("Missing path"));

        let result = scope_tool_use_context(
            context,
            tool.execute(json!({
                "path": "demo.ipynb",
                "edit_type": "delete",
                "cellId": "cell-1"
            })),
        )
        .await
        .unwrap();

        assert!(result.success, "error: {:?}", result.error);
        assert!(result.output.contains("Deleted cell 1"));

        let written: serde_json::Value =
            serde_json::from_str(&tokio::fs::read_to_string(dir.join("demo.ipynb")).await.unwrap())
                .unwrap();
        assert_eq!(written["cells"].as_array().unwrap().len(), 1);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn notebook_edit_rejects_non_notebook_paths() {
        let dir = std::env::temp_dir().join("vibewindow_test_notebook_edit_reject_plain");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("demo.txt"), "hello\n").await.unwrap();

        let tool = NotebookEditTool::new(test_security(dir.clone()));
        let context = Arc::new(ToolUseContext::new(
            "notebook-edit-reject-plain",
            Some(dir.to_string_lossy().to_string()),
        ));
        seed_read_state(&context, dir.as_path(), "demo.txt", "hello\n");

        let result = scope_tool_use_context(
            context,
            tool.call(json!({
                "path": "demo.txt",
                "edit_type": "delete",
                "cell_number": 1
            })),
        )
        .await
        .unwrap();

        assert!(!result.is_success());
        assert!(
            result
                .error_text()
                .unwrap_or_default()
                .contains("NotebookEdit only supports .ipynb files")
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
