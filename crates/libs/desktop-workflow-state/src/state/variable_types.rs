//! 工作流变量面板与变量编辑草稿。

use iced::widget::text_editor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowVariablePanelKind {
    System,
    Environment,
    Conversation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowVariableEditorMode {
    CreateEnvironment,
    EditEnvironment(String),
    CreateConversation,
    EditConversation(String),
}

#[derive(Debug)]
pub struct WorkflowVariableEditorDraft {
    pub mode: WorkflowVariableEditorMode,
    pub name: String,
    pub description: String,
    pub value_type: String,
    pub raw_value_editor: text_editor::Content,
}
