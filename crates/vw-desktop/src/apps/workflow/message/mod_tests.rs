//! 工作流消息分派的测试，覆盖顶层消息到各状态处理器的路由。
//! 用例确保无活动应用或编辑器时的操作保持安全空操作。
#![allow(unused_must_use)]
use super::*;
use crate::apps::workflow::model::create_node_from_type;
use crate::apps::workflow::state::WorkflowVariablePanelKind;
use iced::{Point, Vector};

fn scroll_action() -> text_editor::Action {
    text_editor::Action::Scroll { lines: 1 }
}

fn app_with_workflow() -> App {
    let mut app = App::new().0;
    app.window_size = (1000.0, 800.0);
    let loaded = load_document_from_text(
        None,
        r#"
app:
  name: 消息入口测试
workflow:
  graph:
    nodes:
      - id: start
        position:
          x: 10
          y: 20
        data:
          title: Start
          type: start
      - id: answer
        position:
          x: 260
          y: 20
        data:
          title: Answer
          type: answer
    edges: []
"#
        .to_string(),
    )
    .expect("workflow should load");
    apply_loaded(&mut app, loaded);
    app
}

#[test]
fn workflow_messages_clone_and_debug_cover_signal_variants() {
    use crate::apps::workflow::model::WorkflowHandleKind;

    let endpoint = WorkflowConnectionEndpoint {
        node_id: "node".to_string(),
        handle_id: "handle".to_string(),
        kind: WorkflowHandleKind::Source,
    };
    let run_event = vw_gateway_client::GatewayWorkflowStreamEvent::TextChunk {
        task_id: "task".to_string(),
        workflow_run_id: "run".to_string(),
        text: "chunk".to_string(),
        data: serde_json::Value::Null,
    };
    let messages = vec![
        WorkflowMessage::LoadSavedApps,
        WorkflowMessage::LoadSavedAppsFinished(Err("load failed".to_string())),
        WorkflowMessage::OpenSavedApp("uuid".to_string()),
        WorkflowMessage::OpenSavedAppFinished(Err("open failed".to_string())),
        WorkflowMessage::ShowSavedApps,
        WorkflowMessage::SavedAppSearchChanged("search".to_string()),
        WorkflowMessage::ToggleSavedAppActions("uuid".to_string()),
        WorkflowMessage::CloseSavedAppActions,
        WorkflowMessage::CopySavedAppUuid("uuid".to_string()),
        WorkflowMessage::ClearCopiedSavedAppUuid("uuid".to_string()),
        WorkflowMessage::OpenSavedAppRunPrompt("uuid".to_string()),
        WorkflowMessage::SavedAppRunPromptLoaded { uuid: "uuid".to_string(), result: Ok(vec![]) },
        WorkflowMessage::SavedAppRunQueryChanged("query".to_string()),
        WorkflowMessage::SavedAppRunInputChanged(0, "value".to_string()),
        WorkflowMessage::SavedAppRunInputToggled(1, true),
        WorkflowMessage::CancelSavedAppRunPrompt,
        WorkflowMessage::SubmitSavedAppRunPrompt,
        WorkflowMessage::SavedAppRunEvent { uuid: "uuid".to_string(), event: run_event },
        WorkflowMessage::SavedAppRunFinished { uuid: "uuid".to_string(), result: Ok(()) },
        WorkflowMessage::CancelSavedAppRun,
        WorkflowMessage::RetrySavedAppRunPreview,
        WorkflowMessage::CloseSavedAppRunPreview,
        WorkflowMessage::OpenSavedAppRunNodeDetail(0),
        WorkflowMessage::OpenSavedAppRunOutputDetail,
        WorkflowMessage::SavedAppRunDetailAction(scroll_action()),
        WorkflowMessage::CloseSavedAppRunDetail,
        WorkflowMessage::CopySavedAppRunError,
        WorkflowMessage::ClearSavedAppRunErrorCopied,
        WorkflowMessage::RequestDeleteSavedApp("uuid".to_string()),
        WorkflowMessage::CancelDeleteSavedApp,
        WorkflowMessage::DeleteSavedApp("uuid".to_string()),
        WorkflowMessage::DeleteSavedAppFinished(Ok("uuid".to_string())),
        WorkflowMessage::OpenInlineYaml {
            workflow_yaml: "app:\n  name: Inline\n".to_string(),
            focus_node_id: Some("node".to_string()),
        },
        WorkflowMessage::OpenFile,
        WorkflowMessage::OpenFileFinished(Err("open file failed".to_string())),
        WorkflowMessage::SelectApp("app".to_string()),
        WorkflowMessage::OpenCreateAppEditor,
        WorkflowMessage::OpenEditAppEditor(Some("app".to_string())),
        WorkflowMessage::CloseAppEditor,
        WorkflowMessage::AppEditorNameChanged("name".to_string()),
        WorkflowMessage::AppEditorDescriptionChanged("description".to_string()),
        WorkflowMessage::AppEditorIconChanged("icon".to_string()),
        WorkflowMessage::AppEditorUseIconAsAnswerIconChanged(true),
        WorkflowMessage::AppEditorMaxActiveRequestsChanged("5".to_string()),
        WorkflowMessage::OrganizeActiveApp,
        WorkflowMessage::SubmitAppEditor,
        WorkflowMessage::ToggleQuickInsertPanel,
        WorkflowMessage::InsertSuggestedNode("answer".to_string()),
        WorkflowMessage::OpenCreateNodeEditor("llm".to_string()),
        WorkflowMessage::OpenCreateNodeEditorAt("llm".to_string(), Point::new(1.0, 2.0)),
        WorkflowMessage::CreateContextNode("answer".to_string()),
        WorkflowMessage::OpenDownstreamNodePicker("node".to_string()),
        WorkflowMessage::OpenEditNodeEditor(Some("node".to_string())),
        WorkflowMessage::CloseNodeEditor,
        WorkflowMessage::NodeEditorTabSelected(WorkflowNodeEditorTab::Basic),
        WorkflowMessage::NodeEditorTitleChanged("title".to_string()),
        WorkflowMessage::NodeEditorDescriptionChanged("description".to_string()),
        WorkflowMessage::NodeEditorDescriptionAction(scroll_action()),
        WorkflowMessage::NodeEditorStartAddVariable,
        WorkflowMessage::NodeEditorStartVariableHovered(Some(1)),
        WorkflowMessage::NodeEditorStartRemoveVariable(0),
        WorkflowMessage::NodeEditorStartSelectVariable(0),
        WorkflowMessage::NodeEditorStartCloseVariableEditor,
        WorkflowMessage::NodeEditorStartSubmitVariableEditor,
        WorkflowMessage::NodeEditorStartVariableEditorLabelChanged("label".to_string()),
        WorkflowMessage::NodeEditorStartVariableEditorNameChanged("name".to_string()),
        WorkflowMessage::NodeEditorStartVariableEditorTypeChanged("text-input".to_string()),
        WorkflowMessage::NodeEditorStartVariableEditorRequiredChanged(true),
        WorkflowMessage::NodeEditorStartVariableEditorHiddenChanged(false),
        WorkflowMessage::NodeEditorStartVariableEditorDefaultChanged("default".to_string()),
        WorkflowMessage::NodeEditorStartVariableEditorDefaultAction(scroll_action()),
        WorkflowMessage::NodeEditorStartVariableEditorMaxLengthChanged("42".to_string()),
        WorkflowMessage::NodeEditorStartVariableEditorAddOption,
        WorkflowMessage::NodeEditorStartVariableEditorRemoveOption(0),
        WorkflowMessage::NodeEditorStartVariableEditorOptionChanged(0, "option".to_string()),
        WorkflowMessage::NodeEditorStartVariableEditorToggleFileType("image".to_string()),
        WorkflowMessage::NodeEditorStartVariableEditorExtensionsChanged(".txt".to_string()),
        WorkflowMessage::NodeEditorStartVariableEditorUploadMethodChanged("local".to_string()),
        WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFile,
        WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFileFinished(Ok(None)),
        WorkflowMessage::NodeEditorStartVariableEditorRemoveDefaultFile(0),
        WorkflowMessage::NodeEditorStartVariableEditorOpenDefaultFileUrlInput,
        WorkflowMessage::NodeEditorStartVariableEditorCloseDefaultFileUrlInput,
        WorkflowMessage::NodeEditorStartVariableEditorDefaultFileUrlChanged(
            "https://example.test/file".to_string(),
        ),
        WorkflowMessage::NodeEditorStartVariableEditorSubmitDefaultFileUrl,
        WorkflowMessage::NodeEditorStartVariableLabelChanged(0, "label".to_string()),
        WorkflowMessage::NodeEditorStartVariableNameChanged(0, "name".to_string()),
        WorkflowMessage::NodeEditorStartVariableTypeChanged(0, "text-input".to_string()),
        WorkflowMessage::NodeEditorStartVariableRequiredChanged(0, true),
        WorkflowMessage::NodeEditorStartVariableDefaultChanged(0, "default".to_string()),
        WorkflowMessage::NodeEditorStartVariablePlaceholderChanged(0, "placeholder".to_string()),
        WorkflowMessage::NodeEditorStartVariableHintChanged(0, "hint".to_string()),
        WorkflowMessage::NodeEditorStartVariableMaxLengthChanged(0, "128".to_string()),
        WorkflowMessage::InsertDownstreamNode("node".to_string(), "answer".to_string()),
        WorkflowMessage::InsertDownstreamNodeFromHandle(
            "node".to_string(),
            "source".to_string(),
            "answer".to_string(),
        ),
        WorkflowMessage::NodeEditorShowRawDataEditorChanged(true),
        WorkflowMessage::NodeEditorIfElseAddCase,
        WorkflowMessage::NodeEditorIfElseCaseLogicalOperatorChanged(0, "and".to_string()),
        WorkflowMessage::NodeEditorIfElseAddCondition(0),
        WorkflowMessage::NodeEditorIfElseRemoveCondition(0, 1),
        WorkflowMessage::NodeEditorIfElseConditionSelectorChanged(0, 1, "selector".to_string()),
        WorkflowMessage::NodeEditorIfElseConditionOperatorChanged(0, 1, "contains".to_string()),
        WorkflowMessage::NodeEditorIfElseConditionValueChanged(0, 1, "value".to_string()),
        WorkflowMessage::NodeEditorIfElseConditionVarTypeChanged(0, 1, "string".to_string()),
        WorkflowMessage::NodeEditorKnowledgeQuerySelectorChanged("query".to_string()),
        WorkflowMessage::NodeEditorKnowledgeQueryAttachmentSelectorChanged("file".to_string()),
        WorkflowMessage::NodeEditorKnowledgeDatasetIdsChanged("dataset".to_string()),
        WorkflowMessage::NodeEditorKnowledgeRetrievalModeChanged("semantic".to_string()),
        WorkflowMessage::NodeEditorKnowledgeTopKChanged("3".to_string()),
        WorkflowMessage::NodeEditorKnowledgeScoreThresholdEnabledChanged(true),
        WorkflowMessage::NodeEditorKnowledgeScoreThresholdChanged("0.5".to_string()),
        WorkflowMessage::NodeEditorKnowledgeRerankingEnabledChanged(true),
        WorkflowMessage::NodeEditorKnowledgeSingleModelProviderChanged("provider".to_string()),
        WorkflowMessage::NodeEditorKnowledgeSingleModelNameChanged("model".to_string()),
        WorkflowMessage::NodeEditorKnowledgeSingleModelModeChanged("chat".to_string()),
        WorkflowMessage::NodeEditorToolProviderIdChanged("provider".to_string()),
        WorkflowMessage::NodeEditorToolProviderTypeChanged("builtin".to_string()),
        WorkflowMessage::NodeEditorToolProviderNameChanged("Provider".to_string()),
        WorkflowMessage::NodeEditorToolNameChanged("tool".to_string()),
        WorkflowMessage::NodeEditorToolLabelChanged("Tool".to_string()),
        WorkflowMessage::NodeEditorToolDescriptionChanged("description".to_string()),
        WorkflowMessage::NodeEditorToolCredentialIdChanged("credential".to_string()),
        WorkflowMessage::NodeEditorToolPluginUniqueIdentifierChanged("plugin".to_string()),
        WorkflowMessage::NodeEditorToolParametersAction(scroll_action()),
        WorkflowMessage::NodeEditorToolConfigurationsAction(scroll_action()),
        WorkflowMessage::NodeEditorAgentStrategyProviderChanged("provider".to_string()),
        WorkflowMessage::NodeEditorAgentStrategyNameChanged("strategy".to_string()),
        WorkflowMessage::NodeEditorAgentStrategyLabelChanged("Strategy".to_string()),
        WorkflowMessage::NodeEditorAgentPluginUniqueIdentifierChanged("plugin".to_string()),
        WorkflowMessage::NodeEditorAgentOutputSchemaAction(scroll_action()),
        WorkflowMessage::NodeEditorAgentParametersAction(scroll_action()),
        WorkflowMessage::NodeEditorAgentMemoryEnabledChanged(true),
        WorkflowMessage::NodeEditorAgentMemoryWindowSizeChanged("8".to_string()),
        WorkflowMessage::NodeEditorAgentMemoryPromptAction(scroll_action()),
        WorkflowMessage::NodeEditorLlmProviderChanged("provider".to_string()),
        WorkflowMessage::NodeEditorLlmModelNameChanged("model".to_string()),
        WorkflowMessage::NodeEditorLlmModelModeChanged("chat".to_string()),
        WorkflowMessage::NodeEditorLlmEnableThinkingChanged(true),
        WorkflowMessage::NodeEditorLlmContextEnabledChanged(true),
        WorkflowMessage::NodeEditorLlmContextSelectorChanged("context".to_string()),
        WorkflowMessage::NodeEditorLlmSystemPromptAction(scroll_action()),
        WorkflowMessage::NodeEditorLlmUserPromptAction(scroll_action()),
        WorkflowMessage::NodeEditorLlmVisionEnabledChanged(true),
        WorkflowMessage::NodeEditorAnswerAction(scroll_action()),
        WorkflowMessage::NodeEditorCodeLanguageChanged("python3".to_string()),
        WorkflowMessage::NodeEditorCodeAddInputVariable,
        WorkflowMessage::NodeEditorCodeRemoveInputVariable(0),
        WorkflowMessage::NodeEditorCodeInputVariableNameChanged(0, "input".to_string()),
        WorkflowMessage::NodeEditorCodeInputVariableSelectorChanged(
            0,
            "node".to_string(),
            "var".to_string(),
        ),
        WorkflowMessage::NodeEditorCodeAddOutputVariable,
        WorkflowMessage::NodeEditorCodeRemoveOutputVariable(0),
        WorkflowMessage::NodeEditorCodeOutputNameChanged(0, "output".to_string()),
        WorkflowMessage::NodeEditorCodeOutputTypeChanged(0, "string".to_string()),
        WorkflowMessage::NodeEditorCodeRetryEnabledChanged(true),
        WorkflowMessage::NodeEditorCodeRetryMaxRetriesChanged(3),
        WorkflowMessage::NodeEditorCodeRetryIntervalChanged(100),
        WorkflowMessage::NodeEditorCodeErrorStrategyChanged("fail-branch".to_string()),
        WorkflowMessage::NodeEditorCodeAction(scroll_action()),
        WorkflowMessage::NodeEditorCodeDefaultValueAction(scroll_action()),
        WorkflowMessage::NodeEditorDataAction(scroll_action()),
        WorkflowMessage::NodeEditorScrollCaptured,
        WorkflowMessage::NodeEditorDslOpenContextMenu { x: 1.0, y: 2.0 },
        WorkflowMessage::NodeEditorDslCloseContextMenu,
        WorkflowMessage::NodeEditorDslUndo,
        WorkflowMessage::NodeEditorDslRedo,
        WorkflowMessage::NodeEditorDslContextMenuCopy,
        WorkflowMessage::NodeEditorDslContextMenuCut,
        WorkflowMessage::NodeEditorDslContextMenuPaste,
        WorkflowMessage::NodeEditorDslContextMenuDelete,
        WorkflowMessage::SubmitNodeEditor,
        WorkflowMessage::OpenVariablePanel(WorkflowVariablePanelKind::System),
        WorkflowMessage::CloseVariablePanel,
        WorkflowMessage::OpenCreateEnvironmentVariableEditor,
        WorkflowMessage::OpenEditEnvironmentVariableEditor("env".to_string()),
        WorkflowMessage::OpenCreateConversationVariableEditor,
        WorkflowMessage::OpenEditConversationVariableEditor("conv".to_string()),
        WorkflowMessage::CloseVariableEditor,
        WorkflowMessage::VariableEditorNameChanged("name".to_string()),
        WorkflowMessage::VariableEditorDescriptionChanged("description".to_string()),
        WorkflowMessage::VariableEditorTypeChanged("string".to_string()),
        WorkflowMessage::VariableEditorValueAction(scroll_action()),
        WorkflowMessage::SubmitVariableEditor,
        WorkflowMessage::DeleteEnvironmentVariable("env".to_string()),
        WorkflowMessage::DeleteConversationVariable("conv".to_string()),
        WorkflowMessage::ToggleActionMenu,
        WorkflowMessage::CloseFloatingPanels,
        WorkflowMessage::SaveActiveApp,
        WorkflowMessage::SaveActiveAppAs,
        WorkflowMessage::SaveActiveAppFinished(Ok(None)),
        WorkflowMessage::ExportPng,
        WorkflowMessage::ExportJpeg,
        WorkflowMessage::ExportSvg,
        WorkflowMessage::ExportFinished(Ok(())),
        WorkflowMessage::Reload,
        WorkflowMessage::SelectNode("node".to_string()),
        WorkflowMessage::SelectEdge("edge".to_string()),
        WorkflowMessage::ClearSelection,
        WorkflowMessage::PanBy(Vector::new(1.0, 2.0)),
        WorkflowMessage::Zoom(1.2, Some(Point::new(3.0, 4.0))),
        WorkflowMessage::ZoomSet(0.8),
        WorkflowMessage::ZoomFit,
        WorkflowMessage::ToggleZoomMenu,
        WorkflowMessage::NodeDragStart("node".to_string()),
        WorkflowMessage::NodeDragged("node".to_string(), Vector::new(5.0, 6.0)),
        WorkflowMessage::FinishNodeDrag,
        WorkflowMessage::StartConnection(endpoint.clone(), Point::new(7.0, 8.0)),
        WorkflowMessage::UpdateConnectionCursor(Point::new(9.0, 10.0)),
        WorkflowMessage::FinishConnection(endpoint),
        WorkflowMessage::CancelConnection,
        WorkflowMessage::CancelInteraction,
        WorkflowMessage::OpenCanvasContextMenu(
            WorkflowCanvasContextMenuTarget::Canvas,
            Point::new(11.0, 12.0),
            Point::new(13.0, 14.0),
        ),
        WorkflowMessage::CanvasContextMenuScrollCaptured,
        WorkflowMessage::CloseCanvasContextMenu,
        WorkflowMessage::DuplicateSelectedNode,
        WorkflowMessage::DeleteSelectedNode,
        WorkflowMessage::DeleteSelectedEdge,
        WorkflowMessage::DeleteNodeById("node".to_string()),
        WorkflowMessage::DeleteEdgeById("edge".to_string()),
        WorkflowMessage::JumpToNode("node".to_string()),
        WorkflowMessage::Undo,
        WorkflowMessage::Redo,
        WorkflowMessage::DismissError,
    ];

    for message in messages {
        let cloned = message.clone();
        assert!(!format!("{cloned:?}").is_empty());
    }
}

#[test]
fn workflow_save_target_clone_and_debug_preserve_payloads() {
    let local = WorkflowSaveTarget::LocalUuid("uuid".to_string());
    let file = WorkflowSaveTarget::FilePath("/tmp/workflow.yml".to_string());

    assert_eq!(format!("{:?}", local.clone()), "LocalUuid(\"uuid\")");
    assert_eq!(format!("{:?}", file.clone()), "FilePath(\"/tmp/workflow.yml\")");
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("mod_tests"));
}

#[test]
fn update_dispatches_to_app_node_document_and_canvas_handlers() {
    let mut app = app_with_workflow();

    update(&mut app, WorkflowMessage::SavedAppSearchChanged("abc".to_string()));
    assert_eq!(app.workflow_state.saved_app_search_query, "abc");

    update(&mut app, WorkflowMessage::OpenSavedAppRunPrompt("workflow-1".to_string()));
    assert_eq!(
        app.workflow_state.saved_app_run_prompt.as_ref().map(|p| p.uuid.as_str()),
        Some("workflow-1")
    );

    update(&mut app, WorkflowMessage::ToggleQuickInsertPanel);
    assert!(app.workflow_state.quick_insert_panel_open);

    update(&mut app, WorkflowMessage::OpenVariablePanel(WorkflowVariablePanelKind::System));
    assert_eq!(app.workflow_state.variable_panel, Some(WorkflowVariablePanelKind::System));

    update(&mut app, WorkflowMessage::SelectNode("start".to_string()));
    assert_eq!(app.workflow_state.selected_node_id.as_deref(), Some("start"));
}

#[test]
fn update_dispatches_run_preview_retry_and_cancel_messages() {
    let mut app = app_with_workflow();

    app.workflow_state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hello".to_string(),
        Default::default(),
    );
    update(&mut app, WorkflowMessage::CancelSavedAppRun);
    assert_eq!(app.workflow_state.running_saved_app_uuid, None);
    assert_eq!(
        app.workflow_state.saved_app_run_preview.as_ref().map(|preview| preview.status),
        Some(crate::apps::workflow::state::WorkflowRunStatus::Canceled)
    );

    app.workflow_state.saved_app_run_preview =
        Some(crate::apps::workflow::state::WorkflowRunPreview {
            uuid: "workflow-1".to_string(),
            run_id: Some("run-1".to_string()),
            status: crate::apps::workflow::state::WorkflowRunStatus::Failed,
            nodes: vec![],
            text_output: "boom".to_string(),
            error: Some(crate::apps::workflow::state::WorkflowRunErrorPreview {
                status: 500,
                code: "workflow_failed".to_string(),
                message: "boom".to_string(),
            }),
            is_finished: true,
        });
    app.workflow_state.running_saved_app_query = "hello".to_string();

    update(&mut app, WorkflowMessage::RetrySavedAppRunPreview);
    assert_eq!(app.workflow_state.running_saved_app_uuid.as_deref(), Some("workflow-1"));
    let preview = app.workflow_state.saved_app_run_preview.as_ref().expect("preview should reset");
    assert_eq!(preview.status, crate::apps::workflow::state::WorkflowRunStatus::Running);
    assert!(preview.nodes.is_empty());
    assert!(preview.error.is_none());
    assert!(preview.text_output.is_empty());
}

#[test]
fn apply_loaded_syncs_top_tab_and_suggested_position_accounts_for_viewport() {
    let mut app = App::new().0;
    app.window_size = (1200.0, 900.0);
    let loaded = load_document_from_text(
        None,
        "app:\n  name: Loaded\nworkflow:\n  graph:\n    nodes: []\n    edges: []\n".to_string(),
    )
    .expect("workflow should load");

    apply_loaded(&mut app, loaded);
    assert_eq!(app.screen, crate::app::Screen::WorkflowTool);
    assert_eq!(app.active_tab_id.as_deref(), Some(crate::apps::workflow::WORKFLOW_TOOL_TAB_ID));
    assert_eq!(app.workflow_state.title(), "Loaded");

    app.workflow_state.pan = Vector::new(20.0, 40.0);
    app.workflow_state.zoom = 2.0;
    app.workflow_state.document.nodes =
        vec![create_node_from_type("answer", "a".to_string(), Point::ORIGIN, 1.0).unwrap()];
    let position = suggested_new_node_position(&app);
    assert_eq!(position, Point::new(330.0, 233.0));
}

#[test]
fn helper_functions_cover_save_export_and_record_mapping() {
    let mut empty_app = App::new().0;
    let _ = load_saved_apps_task();
    let _ = open_saved_app_task("uuid".to_string());
    let _ = delete_saved_app_task("uuid".to_string());
    let _ = open_file();
    let _ = save_active_app(&mut empty_app, false);

    let mut app = app_with_workflow();
    assert_eq!(suggested_export_file_name(" Sales/Bot ", "svg"), "Sales_Bot.svg");
    assert_eq!(suggested_export_file_name("!!!", "png"), "workflow_app.png");

    let entry = app.workflow_state.active_entry_snapshot().expect("active entry");
    let _ = save_entry(entry.clone(), false);
    let _ = save_entry(entry, true);

    let _ = export_svg(&mut empty_app);
    let _ = export_svg(&mut app);
    let _ = export_png(&mut app);
    let _ = export_jpeg(&mut app);
    export_finished(&mut app, Ok(()));
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已导出工作流图片"));
    export_finished(&mut app, Err("bad export".to_string()));
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("bad export"));

    #[cfg(not(target_arch = "wasm32"))]
    {
        let summary = saved_app_summary_from_record(vw_gateway_client::WorkflowRecordSummary {
            uuid: "u".to_string(),
            name: "n".to_string(),
            description: "d".to_string(),
            created_at_ms: 1,
            updated_at_ms: 2,
        });
        assert_eq!(summary.uuid, "u");
        assert_eq!(summary.name, "n");
        assert_eq!(summary.description, "d");
        assert_eq!(summary.created_at_ms, 1);
        assert_eq!(summary.updated_at_ms, 2);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn svg_renderer_rejects_invalid_documents() {
    assert!(render_workflow_svg_to_png("not an svg document").is_none());
}

#[test]
fn suggested_node_position_clamps_zero_zoom_and_offsets_by_node_count() {
    let mut app = App::new().0;
    app.window_size = (100.0, 80.0);
    app.workflow_state.zoom = 0.0;
    app.workflow_state.pan = Vector::new(0.0, 0.0);

    let position = suggested_new_node_position(&app);
    assert!(position.x.is_finite());
    assert!(position.y.is_finite());
    assert_eq!(position, Point::new(520_000.0, 400_000.0));

    app.workflow_state.zoom = 1.0;
    app.workflow_state.document.nodes = (0..7)
        .map(|index| {
            create_node_from_type("answer", format!("node-{index}"), Point::ORIGIN, index as f32)
                .expect("answer node should be available")
        })
        .collect();
    assert_eq!(suggested_new_node_position(&app), Point::new(80.0, 68.0));
}
