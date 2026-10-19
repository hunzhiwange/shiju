//! 共享节点字段组件测试，覆盖草稿转换、编辑事件与暗黑主题构建路径。

use super::node_editor_fields::*;
use super::*;
use crate::apps::workflow::state::{
    WorkflowKeyValueDraft, WorkflowModelDraft, WorkflowNamedValueDraft, WorkflowNodeFieldEdit,
    WorkflowVariableSelectorDraft,
};

#[test]
fn node_editor_fields_emit_stable_edits_and_normalize_selector_input() {
    let selector = WorkflowVariableSelectorDraft::from_input(" start . query ");
    assert_eq!(selector.path, vec!["start", "query"]);
    assert_eq!(selector.to_input(), "start.query");

    assert_eq!(
        WorkflowNodeFieldEdit::VariableSelectorChanged("start.query".to_string()),
        WorkflowNodeFieldEdit::VariableSelectorChanged("start.query".to_string())
    );
    assert_ne!(
        WorkflowNodeFieldEdit::NamedValueNameChanged(0, "answer".to_string()),
        WorkflowNodeFieldEdit::NamedValueNameChanged(1, "answer".to_string())
    );
}

#[test]
fn node_editor_fields_build_all_shared_controls() {
    let selector = WorkflowVariableSelectorDraft::from_input("start.query");
    let named = vec![WorkflowNamedValueDraft {
        id: "output-1".to_string(),
        name: "answer".to_string(),
        value_type: "string".to_string(),
        selector: selector.clone(),
    }];
    let pairs = vec![WorkflowKeyValueDraft {
        key: "Authorization".to_string(),
        value: "Bearer {{#env.token#}}".to_string(),
    }];
    let model = WorkflowModelDraft {
        provider: "langgenius/openai/openai".to_string(),
        name: "gpt-4o-mini".to_string(),
        mode: "chat".to_string(),
    };
    let retry = WorkflowNodeRetryDraft { enabled: true, max_retries: 3, retry_interval: 500 };

    let _ = build_variable_selector_field("输入变量", &selector, |_| {
        Message::WorkflowTool(WorkflowMessage::CloseNodeEditor)
    });
    let _ =
        build_named_value_list(&named, |_| Message::WorkflowTool(WorkflowMessage::CloseNodeEditor));
    let _ =
        build_key_value_list(&pairs, |_| Message::WorkflowTool(WorkflowMessage::CloseNodeEditor));
    let _ = build_model_fields(&model, |_| Message::WorkflowTool(WorkflowMessage::CloseNodeEditor));
    let _ = build_retry_fields(retry, |_| Message::WorkflowTool(WorkflowMessage::CloseNodeEditor));

    for theme in [Theme::Light, Theme::Dark] {
        assert!(node_editor_field_card_style(&theme).border.width > 0.0);
    }
}
