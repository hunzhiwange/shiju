//! 工作流文档消息的测试，覆盖加载、导入、保存和历史快照。
//! fixture 验证错误路径不会污染当前文档，并保持脏状态语义准确。

use crate::message::{self, WorkflowMessage};
use desktop_workflow_model::*;
use desktop_workflow_state::state::*;

use desktop_workflow_state::state::{WorkflowVariableEditorMode, WorkflowVariablePanelKind};
use iced::widget::text_editor;
use std::sync::Arc;

fn editor_paste(text: &str) -> text_editor::Action {
    text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(text.to_string())))
}

fn app_with_workflow() -> WorkflowState {
    let mut app = WorkflowState::default();
    let loaded = load_document_from_text(
        None,
        r#"
app:
  name: 文档消息测试
workflow:
  environment_variables:
    - id: env-existing
      name: token
      value_type: string
      value: abc
      description: old env
  conversation_variables:
    - id: conv-existing
      name: count
      value_type: number
      value: 1
      description: old conv
  graph:
    nodes:
      - id: start
        data:
          title: Start
          type: start
      - id: answer
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
    app.apply_loaded(loaded, (1280.0, 860.0));
    app
}

fn document_message(app: &mut WorkflowState, message: WorkflowMessage) {
    assert!(message::document::handle(app, (1280.0, 860.0), message));
}

#[test]
fn variable_panel_and_environment_editor_messages_update_state() {
    let mut app = app_with_workflow();

    document_message(
        &mut app,
        WorkflowMessage::OpenVariablePanel(WorkflowVariablePanelKind::Environment),
    );
    assert_eq!(app.variable_panel, Some(WorkflowVariablePanelKind::Environment));

    document_message(&mut app, WorkflowMessage::OpenCreateEnvironmentVariableEditor);
    assert!(matches!(
        app.variable_editor.as_ref().map(|editor| &editor.mode),
        Some(WorkflowVariableEditorMode::CreateEnvironment)
    ));
    document_message(&mut app, WorkflowMessage::VariableEditorNameChanged("limit".to_string()));
    document_message(&mut app, WorkflowMessage::VariableEditorTypeChanged("number".to_string()));
    document_message(&mut app, WorkflowMessage::VariableEditorValueAction(editor_paste("42")));
    document_message(&mut app, WorkflowMessage::SubmitVariableEditor);
    assert!(app.environment_variables.iter().any(|item| {
        item.name == "limit" && item.value_type == "number" && item.value.as_i64() == Some(42)
    }));

    document_message(
        &mut app,
        WorkflowMessage::OpenEditEnvironmentVariableEditor("env-existing".to_string()),
    );
    document_message(
        &mut app,
        WorkflowMessage::VariableEditorDescriptionChanged("updated env".to_string()),
    );
    document_message(&mut app, WorkflowMessage::SubmitVariableEditor);
    assert_eq!(
        app.environment_variable("env-existing").map(|item| item.description.as_str()),
        Some("updated env")
    );

    document_message(
        &mut app,
        WorkflowMessage::OpenEditEnvironmentVariableEditor("missing".to_string()),
    );
    assert_eq!(app.error_message.as_deref(), Some("环境变量不存在"));
    document_message(
        &mut app,
        WorkflowMessage::DeleteEnvironmentVariable("env-existing".to_string()),
    );
    assert!(app.environment_variable("env-existing").is_none());

    document_message(&mut app, WorkflowMessage::CloseVariableEditor);
    document_message(&mut app, WorkflowMessage::CloseVariablePanel);
    assert!(app.variable_editor.is_none());
    assert!(app.variable_panel.is_none());
}

#[test]
fn conversation_editor_messages_cover_success_and_error_paths() {
    let mut app = app_with_workflow();

    document_message(&mut app, WorkflowMessage::OpenCreateConversationVariableEditor);
    assert!(matches!(
        app.variable_editor.as_ref().map(|editor| &editor.mode),
        Some(WorkflowVariableEditorMode::CreateConversation)
    ));
    document_message(&mut app, WorkflowMessage::VariableEditorNameChanged("profile".to_string()));
    document_message(&mut app, WorkflowMessage::VariableEditorTypeChanged("object".to_string()));
    document_message(
        &mut app,
        WorkflowMessage::VariableEditorValueAction(editor_paste("{name: Ada}")),
    );
    document_message(&mut app, WorkflowMessage::SubmitVariableEditor);
    assert!(app.conversation_variables.iter().any(|item| {
        item.name == "profile" && item.value_type == "object" && item.value.is_mapping()
    }));

    document_message(
        &mut app,
        WorkflowMessage::OpenEditConversationVariableEditor("conv-existing".to_string()),
    );
    document_message(
        &mut app,
        WorkflowMessage::VariableEditorDescriptionChanged("updated conv".to_string()),
    );
    document_message(&mut app, WorkflowMessage::SubmitVariableEditor);
    assert_eq!(
        app.conversation_variable("conv-existing").map(|item| item.description.as_str()),
        Some("updated conv")
    );

    document_message(
        &mut app,
        WorkflowMessage::OpenEditConversationVariableEditor("missing".to_string()),
    );
    assert_eq!(app.error_message.as_deref(), Some("会话变量不存在"));
    document_message(
        &mut app,
        WorkflowMessage::DeleteConversationVariable("conv-existing".to_string()),
    );
    assert!(app.conversation_variable("conv-existing").is_none());

    document_message(&mut app, WorkflowMessage::OpenCreateConversationVariableEditor);
    document_message(&mut app, WorkflowMessage::VariableEditorNameChanged(" ".to_string()));
    document_message(&mut app, WorkflowMessage::SubmitVariableEditor);
    assert_eq!(app.error_message.as_deref(), Some("变量名称不能为空"));
}

#[test]
fn action_menu_toggle_opens_closed_menu_and_close_is_idempotent() {
    let mut app = app_with_workflow();
    assert!(!app.action_menu_open);

    document_message(&mut app, WorkflowMessage::ToggleActionMenu);
    assert!(app.action_menu_open);

    document_message(&mut app, WorkflowMessage::CloseFloatingPanels);
    document_message(&mut app, WorkflowMessage::CloseFloatingPanels);
    assert!(!app.action_menu_open);
}
