#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("node_tests"));
}

use super::*;
use crate::apps::workflow::state::{
    WorkflowCanvasContextMenuTarget, WorkflowNodeEditorMode, WorkflowNodeEditorTab,
    WorkflowNodeVisualDraft,
};
use iced::Point;
use iced::widget::text_editor;
use std::sync::Arc;

fn editor_paste(text: &str) -> text_editor::Action {
    text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(text.to_string())))
}

fn editor_scroll() -> text_editor::Action {
    text_editor::Action::Scroll { lines: 1 }
}

fn app_with_workflow() -> App {
    let mut app = App::new().0;
    app.window_size = (1280.0, 860.0);
    let loaded = load_document_from_text(
        None,
        r#"
app:
  name: 节点消息测试
workflow:
  graph:
    nodes:
      - id: start
        position:
          x: 0
          y: 0
        data:
          title: Start
          type: start
          variables: []
      - id: answer
        position:
          x: 360
          y: 0
        data:
          title: Answer
          type: answer
    edges:
      - id: start-answer
        source: start
        sourceHandle: source
        target: answer
        targetHandle: target
"#
        .to_string(),
    )
    .expect("workflow should load");
    apply_loaded(&mut app, loaded);
    app
}

fn node_message(app: &mut App, message: WorkflowMessage) {
    assert!(super::node::handle(app, message).is_some());
}

fn open_create_editor(app: &mut App, block_type: &str) {
    node_message(
        app,
        WorkflowMessage::OpenCreateNodeEditorAt(block_type.to_string(), Point::new(12.0, 34.0)),
    );
    assert_eq!(
        app.workflow_state.node_editor.as_ref().map(|editor| editor.block_type.as_str()),
        Some(block_type)
    );
}

#[test]
fn quick_insert_create_edit_and_submit_messages_update_node_state() {
    let mut app = app_with_workflow();

    node_message(&mut app, WorkflowMessage::ToggleQuickInsertPanel);
    assert!(app.workflow_state.quick_insert_panel_open);

    let node_count = app.workflow_state.document.nodes.len();
    node_message(&mut app, WorkflowMessage::InsertSuggestedNode("answer".to_string()));
    assert_eq!(app.workflow_state.document.nodes.len(), node_count + 1);
    node_message(&mut app, WorkflowMessage::InsertSuggestedNode("start".to_string()));
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("开始节点只能有一个"));

    node_message(&mut app, WorkflowMessage::OpenCreateNodeEditor("llm".to_string()));
    assert_eq!(
        app.workflow_state.node_editor.as_ref().map(|editor| editor.block_type.as_str()),
        Some("llm")
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorTabSelected(WorkflowNodeEditorTab::AdvancedDsl),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorTitleChanged("  草稿标题  ".to_string()));
    node_message(&mut app, WorkflowMessage::NodeEditorDescriptionChanged("描述".to_string()));
    node_message(&mut app, WorkflowMessage::NodeEditorDescriptionAction(editor_paste(" plus")));
    node_message(&mut app, WorkflowMessage::NodeEditorShowRawDataEditorChanged(true));
    let editor = app.workflow_state.node_editor.as_ref().expect("node editor");
    assert_eq!(editor.active_tab, WorkflowNodeEditorTab::AdvancedDsl);
    assert_eq!(editor.title, "  草稿标题  ");
    assert!(editor.description.contains(" plus"));
    assert!(editor.show_raw_data_editor);

    node_message(&mut app, WorkflowMessage::CloseNodeEditor);
    assert!(app.workflow_state.node_editor.is_none());

    node_message(&mut app, WorkflowMessage::OpenEditNodeEditor(Some("answer".to_string())));
    node_message(&mut app, WorkflowMessage::NodeEditorTitleChanged("新回复".to_string()));
    node_message(&mut app, WorkflowMessage::SubmitNodeEditor);
    assert_eq!(
        app.workflow_state.document.node("answer").map(|node| node.title.as_str()),
        Some("Answer")
    );

    node_message(&mut app, WorkflowMessage::OpenEditNodeEditor(Some("missing".to_string())));
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("目标节点不存在"));
}

#[test]
fn start_variable_editor_messages_cover_nested_editor_paths() {
    let mut app = App::new().0;
    open_create_editor(&mut app, "start");

    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableHovered(Some(0)));
    node_message(&mut app, WorkflowMessage::NodeEditorStartAddVariable);
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorLabelChanged("附件".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorNameChanged("files".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorTypeChanged("file-list".to_string()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableEditorRequiredChanged(true));
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableEditorHiddenChanged(true));
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorMaxLengthChanged("2".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorToggleFileType("custom".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorExtensionsChanged(".pdf, docx".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorUploadMethodChanged("remote_url".to_string()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFile);
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableEditorOpenDefaultFileUrlInput);
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorDefaultFileUrlChanged(
            "https://example.test/a.pdf".to_string(),
        ),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableEditorSubmitDefaultFileUrl);
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFileFinished(Ok(Some(
            "/tmp/a.pdf".to_string(),
        ))),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFileFinished(Ok(None)),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFileFinished(Err(
            "pick failed".to_string()
        )),
    );
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("pick failed"));
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableEditorRemoveDefaultFile(0));
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableEditorCloseDefaultFileUrlInput);
    node_message(&mut app, WorkflowMessage::NodeEditorStartSubmitVariableEditor);
    assert!(app.workflow_state.error_message.is_some());

    node_message(&mut app, WorkflowMessage::NodeEditorStartCloseVariableEditor);
    node_message(&mut app, WorkflowMessage::NodeEditorStartAddVariable);
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorLabelChanged("选项".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorNameChanged("choice".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorTypeChanged("select".to_string()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableEditorAddOption);
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorOptionChanged(0, "A".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorDefaultChanged("A".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorDefaultAction(editor_scroll()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableEditorRemoveOption(99));
    node_message(&mut app, WorkflowMessage::NodeEditorStartSubmitVariableEditor);

    let editor = app.workflow_state.node_editor.as_ref().expect("node editor");
    let WorkflowNodeVisualDraft::Start { variables } =
        editor.visual_draft.as_ref().expect("start draft")
    else {
        panic!("expected start draft");
    };
    assert!(variables.iter().any(|variable| variable.variable == "choice"));

    node_message(&mut app, WorkflowMessage::NodeEditorStartSelectVariable(0));
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableLabelChanged(0, "问题".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableNameChanged(0, "question".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableTypeChanged(0, "paragraph".to_string()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableRequiredChanged(0, true));
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableDefaultChanged(0, "hello".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariablePlaceholderChanged(0, "输入".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableHintChanged(0, "提示".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableMaxLengthChanged(0, "128".to_string()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorStartRemoveVariable(99));
}

#[test]
fn downstream_and_context_node_messages_report_success_and_errors() {
    let mut app = app_with_workflow();

    node_message(&mut app, WorkflowMessage::OpenDownstreamNodePicker("start".to_string()));
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("右键菜单已关闭"));

    app.workflow_state.open_context_menu(
        WorkflowCanvasContextMenuTarget::Node("start".to_string()),
        Point::new(10.0, 20.0),
        Point::new(30.0, 40.0),
    );
    node_message(&mut app, WorkflowMessage::OpenDownstreamNodePicker("start".to_string()));
    assert!(matches!(
        app.workflow_state.context_menu.as_ref().map(|menu| &menu.target),
        Some(WorkflowCanvasContextMenuTarget::NodeInsert(id)) if id == "start"
    ));

    let node_count = app.workflow_state.document.nodes.len();
    node_message(
        &mut app,
        WorkflowMessage::InsertDownstreamNode("start".to_string(), "answer".to_string()),
    );
    assert_eq!(app.workflow_state.document.nodes.len(), node_count + 1);

    node_message(
        &mut app,
        WorkflowMessage::InsertDownstreamNodeFromHandle(
            "start".to_string(),
            "missing".to_string(),
            "answer".to_string(),
        ),
    );
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("右键菜单已关闭"));

    app.workflow_state.open_context_menu(
        WorkflowCanvasContextMenuTarget::NodeInsert("start".to_string()),
        Point::new(1.0, 2.0),
        Point::new(3.0, 4.0),
    );
    node_message(&mut app, WorkflowMessage::CreateContextNode("answer".to_string()));
    assert!(app.workflow_state.status_message.as_deref().unwrap_or_default().contains("已新增"));
}

#[test]
fn downstream_from_valid_handle_inserts_node_and_records_source_handle() {
    let mut app = app_with_workflow();
    let node_count = app.workflow_state.document.nodes.len();
    let edge_count = app.workflow_state.document.edges.len();

    node_message(
        &mut app,
        WorkflowMessage::InsertDownstreamNodeFromHandle(
            "start".to_string(),
            "source".to_string(),
            "answer".to_string(),
        ),
    );

    assert_eq!(app.workflow_state.document.nodes.len(), node_count + 1);
    assert_eq!(app.workflow_state.document.edges.len(), edge_count + 1);
    assert!(
        app.workflow_state.document.edges.iter().any(|edge| {
            edge.source == "start" && edge.source_handle.as_deref() == Some("source")
        })
    );
}

#[test]
fn branch_knowledge_tool_agent_llm_answer_and_code_messages_update_visual_drafts() {
    let mut app = App::new().0;

    open_create_editor(&mut app, "if-else");
    node_message(&mut app, WorkflowMessage::NodeEditorIfElseAddCase);
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorIfElseCaseLogicalOperatorChanged(0, "or".to_string()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorIfElseAddCondition(0));
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorIfElseConditionSelectorChanged(0, 0, "start.query".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorIfElseConditionOperatorChanged(0, 0, "contains".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorIfElseConditionValueChanged(0, 0, "hello".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorIfElseConditionVarTypeChanged(0, 0, "string".to_string()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorIfElseRemoveCondition(0, 99));

    open_create_editor(&mut app, "knowledge-retrieval");
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorKnowledgeQuerySelectorChanged("start.query".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorKnowledgeQueryAttachmentSelectorChanged(
            "start.files".to_string(),
        ),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorKnowledgeDatasetIdsChanged("ds1, ds2".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorKnowledgeRetrievalModeChanged("single".to_string()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorKnowledgeTopKChanged("8".to_string()));
    node_message(&mut app, WorkflowMessage::NodeEditorKnowledgeScoreThresholdEnabledChanged(true));
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorKnowledgeScoreThresholdChanged("0.7".to_string()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorKnowledgeRerankingEnabledChanged(true));
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorKnowledgeSingleModelProviderChanged("openai".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorKnowledgeSingleModelNameChanged("embed".to_string()),
    );
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorKnowledgeSingleModelModeChanged("embedding".to_string()),
    );

    open_create_editor(&mut app, "tool");
    for message in [
        WorkflowMessage::NodeEditorToolProviderIdChanged("provider".to_string()),
        WorkflowMessage::NodeEditorToolProviderTypeChanged("builtin".to_string()),
        WorkflowMessage::NodeEditorToolProviderNameChanged("Provider".to_string()),
        WorkflowMessage::NodeEditorToolNameChanged("search".to_string()),
        WorkflowMessage::NodeEditorToolLabelChanged("Search".to_string()),
        WorkflowMessage::NodeEditorToolDescriptionChanged("desc".to_string()),
        WorkflowMessage::NodeEditorToolCredentialIdChanged("cred".to_string()),
        WorkflowMessage::NodeEditorToolPluginUniqueIdentifierChanged("plugin".to_string()),
        WorkflowMessage::NodeEditorToolParametersAction(editor_scroll()),
        WorkflowMessage::NodeEditorToolConfigurationsAction(editor_scroll()),
    ] {
        node_message(&mut app, message);
    }

    open_create_editor(&mut app, "agent");
    for message in [
        WorkflowMessage::NodeEditorAgentStrategyProviderChanged("provider".to_string()),
        WorkflowMessage::NodeEditorAgentStrategyNameChanged("strategy".to_string()),
        WorkflowMessage::NodeEditorAgentStrategyLabelChanged("Strategy".to_string()),
        WorkflowMessage::NodeEditorAgentPluginUniqueIdentifierChanged("plugin".to_string()),
        WorkflowMessage::NodeEditorAgentOutputSchemaAction(editor_scroll()),
        WorkflowMessage::NodeEditorAgentParametersAction(editor_scroll()),
        WorkflowMessage::NodeEditorAgentMemoryEnabledChanged(true),
        WorkflowMessage::NodeEditorAgentMemoryWindowSizeChanged("6".to_string()),
        WorkflowMessage::NodeEditorAgentMemoryPromptAction(editor_scroll()),
    ] {
        node_message(&mut app, message);
    }

    open_create_editor(&mut app, "llm");
    for message in [
        WorkflowMessage::NodeEditorLlmProviderChanged("openai".to_string()),
        WorkflowMessage::NodeEditorLlmModelNameChanged("gpt".to_string()),
        WorkflowMessage::NodeEditorLlmModelModeChanged("chat".to_string()),
        WorkflowMessage::NodeEditorLlmEnableThinkingChanged(true),
        WorkflowMessage::NodeEditorLlmContextEnabledChanged(true),
        WorkflowMessage::NodeEditorLlmContextSelectorChanged("start.query".to_string()),
        WorkflowMessage::NodeEditorLlmSystemPromptAction(editor_scroll()),
        WorkflowMessage::NodeEditorLlmUserPromptAction(editor_scroll()),
        WorkflowMessage::NodeEditorLlmVisionEnabledChanged(true),
    ] {
        node_message(&mut app, message);
    }

    open_create_editor(&mut app, "answer");
    node_message(&mut app, WorkflowMessage::NodeEditorAnswerAction(editor_scroll()));

    open_create_editor(&mut app, "code");
    for message in [
        WorkflowMessage::NodeEditorCodeLanguageChanged("javascript".to_string()),
        WorkflowMessage::NodeEditorCodeAddInputVariable,
        WorkflowMessage::NodeEditorCodeInputVariableNameChanged(0, "query".to_string()),
        WorkflowMessage::NodeEditorCodeInputVariableSelectorChanged(
            0,
            "start.query".to_string(),
            "string".to_string(),
        ),
        WorkflowMessage::NodeEditorCodeAddOutputVariable,
        WorkflowMessage::NodeEditorCodeOutputNameChanged(0, "result".to_string()),
        WorkflowMessage::NodeEditorCodeOutputTypeChanged(0, "number".to_string()),
        WorkflowMessage::NodeEditorCodeRetryEnabledChanged(true),
        WorkflowMessage::NodeEditorCodeRetryMaxRetriesChanged(5),
        WorkflowMessage::NodeEditorCodeRetryIntervalChanged(1200),
        WorkflowMessage::NodeEditorCodeErrorStrategyChanged("default-value".to_string()),
        WorkflowMessage::NodeEditorCodeAction(editor_scroll()),
        WorkflowMessage::NodeEditorCodeDefaultValueAction(editor_scroll()),
        WorkflowMessage::NodeEditorDataAction(editor_scroll()),
    ] {
        node_message(&mut app, message);
    }
    node_message(&mut app, WorkflowMessage::NodeEditorCodeRemoveInputVariable(99));
    node_message(&mut app, WorkflowMessage::NodeEditorCodeRemoveOutputVariable(99));

    let editor = app.workflow_state.node_editor.as_ref().expect("code editor");
    assert!(matches!(editor.visual_draft, Some(WorkflowNodeVisualDraft::Code { .. })));
}

#[test]
fn start_variable_empty_default_file_url_is_ignored() {
    let mut app = App::new().0;
    open_create_editor(&mut app, "start");

    node_message(&mut app, WorkflowMessage::NodeEditorStartAddVariable);
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorTypeChanged("file".to_string()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableEditorOpenDefaultFileUrlInput);
    node_message(
        &mut app,
        WorkflowMessage::NodeEditorStartVariableEditorDefaultFileUrlChanged("   ".to_string()),
    );
    node_message(&mut app, WorkflowMessage::NodeEditorStartVariableEditorSubmitDefaultFileUrl);

    assert!(app.workflow_state.error_message.is_none());
}

#[test]
fn submit_node_editor_success_and_error_paths_are_reported() {
    let mut app = App::new().0;
    open_create_editor(&mut app, "answer");
    node_message(&mut app, WorkflowMessage::NodeEditorTitleChanged("回复节点".to_string()));
    node_message(&mut app, WorkflowMessage::SubmitNodeEditor);
    assert_eq!(app.workflow_state.document.nodes.len(), 1);
    assert!(app.workflow_state.node_editor.is_none());

    open_create_editor(&mut app, "tool");
    node_message(&mut app, WorkflowMessage::SubmitNodeEditor);
    assert_eq!(
        app.workflow_state.error_message.as_deref(),
        Some("请先修正节点表单中的错误字段，再保存。")
    );

    let ignored = super::node::handle(&mut app, WorkflowMessage::SelectNode("x".to_string()));
    assert!(ignored.is_none());
}

#[test]
fn selected_node_and_context_create_errors_are_reported() {
    let mut app = app_with_workflow();

    app.workflow_state.selected_node_id = Some("answer".to_string());
    node_message(&mut app, WorkflowMessage::OpenEditNodeEditor(None));
    let editor = app.workflow_state.node_editor.as_ref().expect("node editor");
    assert!(matches!(&editor.mode, WorkflowNodeEditorMode::Edit(node_id) if node_id == "answer"));

    node_message(&mut app, WorkflowMessage::CloseNodeEditor);
    app.workflow_state.selected_node_id = Some("missing".to_string());
    node_message(&mut app, WorkflowMessage::OpenEditNodeEditor(None));
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("目标节点不存在"));

    node_message(&mut app, WorkflowMessage::CreateContextNode("answer".to_string()));
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("目标节点不存在"));
}

#[test]
fn second_start_node_editor_is_rejected_without_opening_editor() {
    let mut app = app_with_workflow();
    app.workflow_state.error_message = None;

    node_message(&mut app, WorkflowMessage::OpenCreateNodeEditor("start".to_string()));

    assert!(app.workflow_state.node_editor.is_none());
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("开始节点只能有一个"));
}

#[test]
fn node_messages_without_matching_state_are_safe_noops() {
    let mut app = App::new().0;

    for message in [
        WorkflowMessage::NodeEditorTabSelected(WorkflowNodeEditorTab::Visual),
        WorkflowMessage::NodeEditorTitleChanged("ignored".to_string()),
        WorkflowMessage::NodeEditorDescriptionChanged("ignored".to_string()),
        WorkflowMessage::NodeEditorDescriptionAction(editor_scroll()),
        WorkflowMessage::NodeEditorStartVariableHovered(Some(3)),
        WorkflowMessage::NodeEditorStartRemoveVariable(3),
        WorkflowMessage::NodeEditorStartCloseVariableEditor,
        WorkflowMessage::NodeEditorStartSubmitVariableEditor,
        WorkflowMessage::NodeEditorStartVariableEditorLabelChanged("ignored".to_string()),
        WorkflowMessage::NodeEditorStartVariableEditorDefaultAction(editor_scroll()),
        WorkflowMessage::NodeEditorStartVariableLabelChanged(0, "ignored".to_string()),
        WorkflowMessage::NodeEditorIfElseAddCase,
        WorkflowMessage::NodeEditorKnowledgeTopKChanged("3".to_string()),
        WorkflowMessage::NodeEditorToolNameChanged("ignored".to_string()),
        WorkflowMessage::NodeEditorAgentMemoryEnabledChanged(true),
        WorkflowMessage::NodeEditorLlmEnableThinkingChanged(true),
        WorkflowMessage::NodeEditorAnswerAction(editor_scroll()),
        WorkflowMessage::NodeEditorCodeRemoveInputVariable(0),
        WorkflowMessage::NodeEditorCodeRemoveOutputVariable(0),
        WorkflowMessage::NodeEditorDataAction(editor_scroll()),
        WorkflowMessage::SubmitNodeEditor,
    ] {
        node_message(&mut app, message);
    }

    assert!(app.workflow_state.node_editor.is_none());
    assert!(app.workflow_state.error_message.is_none());

    let ignored = super::node::handle(&mut app, WorkflowMessage::ToggleSavedAppActions("x".into()));
    assert!(ignored.is_none());
}
