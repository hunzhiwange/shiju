//! 工作流运行请求、输入草稿与预览状态。

use iced::widget::text_editor;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkflowRunSubscriptionRequest {
    pub uuid: String,
    pub query: String,
    pub inputs: BTreeMap<String, serde_json::Value>,
    pub user: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WorkflowRunPromptDraft {
    pub uuid: String,
    pub query: String,
    pub loading: bool,
    pub inputs: Vec<WorkflowRunInputDraft>,
    pub error: Option<desktop_i18n::workflow::Feedback>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WorkflowRunInputDraft {
    pub name: String,
    pub label: String,
    pub input_type: String,
    pub required: bool,
    pub hidden: bool,
    pub options: Vec<String>,
    pub value: String,
    pub placeholder: String,
    pub hint: String,
    pub max_length_input: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRunStatus {
    Running,
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunPreview {
    pub uuid: String,
    pub run_id: Option<String>,
    pub status: WorkflowRunStatus,
    pub nodes: Vec<WorkflowRunNodePreview>,
    pub text_output: String,
    pub error: Option<WorkflowRunErrorPreview>,
    pub is_finished: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunNodePreview {
    pub id: String,
    pub title: String,
    pub status: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunErrorPreview {
    pub status: u32,
    pub code: String,
    pub message: String,
}

/// 运行预览中“查看详细”弹窗的状态：保存标题与可滚动、可选中复制的完整内容。
#[derive(Debug, Default)]
pub struct WorkflowRunDetailDialog {
    pub title: String,
    pub title_source: Option<&'static str>,
    pub content: text_editor::Content,
}
