//! 工作流节点可视化分派测试，覆盖各节点类型草稿到视图段落的构建行为。

use super::*;
use iced::{Point, widget::text_editor};

use crate::apps::workflow::state::{
    WorkflowClassifierClassDraft, WorkflowCodeOutputDraft, WorkflowCodeVariableDraft,
    WorkflowIfElseCaseDraft, WorkflowModelDraft, WorkflowNamedValueDraft, WorkflowNodeEditorDraft,
    WorkflowNodeEditorValidation, WorkflowNodeRetryDraft, WorkflowNodeValidationError,
    WorkflowParameterDraft, WorkflowVariableSelectorDraft,
};

fn editor_with_visual_draft(
    block_type: &str,
    visual_draft: Option<WorkflowNodeVisualDraft>,
) -> WorkflowNodeEditorDraft {
    WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Edit("node_1".to_string()),
        active_tab: WorkflowNodeEditorTab::Visual,
        block_type: block_type.to_string(),
        title: "测试节点".to_string(),
        description: String::new(),
        description_editor: text_editor::Content::with_text(""),
        position: Point::new(0.0, 0.0),
        visual_draft,
        validation: WorkflowNodeEditorValidation::default(),
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::with_text("{}"),
        hovered_start_variable_index: None,
        start_variable_focus_index: None,
        start_variable_editor: None,
    }
}

fn assert_visual_section_builds(block_type: &str, visual_draft: WorkflowNodeVisualDraft) {
    let state = WorkflowState::default();
    let editor = editor_with_visual_draft(block_type, Some(visual_draft));

    let section = build_node_visual_section(&state, &editor);

    assert!(section.is_some());
}

#[test]
fn node_visual_section_returns_none_without_visual_draft() {
    let state = WorkflowState::default();
    let editor = editor_with_visual_draft("custom", None);

    let section = build_node_visual_section(&state, &editor);

    assert!(section.is_none());
}

#[test]
fn node_visual_section_builds_start_visual() {
    assert_visual_section_builds("start", WorkflowNodeVisualDraft::Start { variables: Vec::new() });
}

#[test]
fn output_and_text_nodes_visual_sections_build() {
    let named = WorkflowNamedValueDraft {
        name: "result".to_string(),
        value_type: "string".to_string(),
        selector: WorkflowVariableSelectorDraft::from_input("start.query"),
        ..WorkflowNamedValueDraft::default()
    };
    assert_visual_section_builds(
        "end",
        WorkflowNodeVisualDraft::End { outputs: vec![named.clone()] },
    );
    assert_visual_section_builds(
        "template-transform",
        WorkflowNodeVisualDraft::TemplateTransform {
            variables: vec![named],
            template_editor: text_editor::Content::with_text("{{ result }}"),
        },
    );
    assert_visual_section_builds(
        "document-extractor",
        WorkflowNodeVisualDraft::DocumentExtractor {
            variable_selector: WorkflowVariableSelectorDraft::from_input("start.file"),
        },
    );
}

#[test]
fn classifier_and_parameter_extractor_visual_sections_build() {
    let model = WorkflowModelDraft {
        provider: "openai".to_string(),
        name: "gpt".to_string(),
        mode: "chat".to_string(),
    };
    assert_visual_section_builds(
        "question-classifier",
        WorkflowNodeVisualDraft::QuestionClassifier {
            query: WorkflowVariableSelectorDraft::from_input("start.query"),
            model: model.clone(),
            classes: vec![WorkflowClassifierClassDraft {
                id: "support".to_string(),
                name: "售后".to_string(),
                ..WorkflowClassifierClassDraft::default()
            }],
            instruction_editor: text_editor::Content::with_text("分类"),
        },
    );
    assert_visual_section_builds(
        "parameter-extractor",
        WorkflowNodeVisualDraft::ParameterExtractor {
            query: WorkflowVariableSelectorDraft::from_input("start.query"),
            model,
            parameters: vec![WorkflowParameterDraft {
                name: "city".to_string(),
                value_type: "string".to_string(),
                description: "城市".to_string(),
                required: true,
                ..WorkflowParameterDraft::default()
            }],
            reasoning_mode: "prompt".to_string(),
            instruction_editor: text_editor::Content::with_text("提取"),
        },
    );
}

#[test]
fn node_visual_section_builds_llm_visual() {
    assert_visual_section_builds(
        "llm",
        WorkflowNodeVisualDraft::Llm {
            provider: "langgenius/openai/openai".to_string(),
            model_name: "gpt-4.1".to_string(),
            model_mode: "chat".to_string(),
            enable_thinking: true,
            context_enabled: true,
            context_selector_input: "start.query".to_string(),
            system_prompt_editor: text_editor::Content::with_text("You are helpful."),
            user_prompt_editor: text_editor::Content::with_text("{{query}}"),
            vision_enabled: true,
        },
    );
}

#[test]
fn node_visual_section_builds_with_validation_errors() {
    let state = WorkflowState::default();
    let mut llm_editor = editor_with_visual_draft(
        "llm",
        Some(WorkflowNodeVisualDraft::Llm {
            provider: String::new(),
            model_name: String::new(),
            model_mode: String::new(),
            enable_thinking: false,
            context_enabled: true,
            context_selector_input: "bad".to_string(),
            system_prompt_editor: text_editor::Content::with_text(""),
            user_prompt_editor: text_editor::Content::with_text(""),
            vision_enabled: false,
        }),
    );
    llm_editor.validation.field_errors = vec![
        WorkflowNodeValidationError {
            path: "llm.provider".to_string(),
            message: "provider required".to_string(),
        },
        WorkflowNodeValidationError {
            path: "llm.model_name".to_string(),
            message: "model required".to_string(),
        },
        WorkflowNodeValidationError {
            path: "llm.model_mode".to_string(),
            message: "mode required".to_string(),
        },
        WorkflowNodeValidationError {
            path: "llm.context_selector".to_string(),
            message: "selector invalid".to_string(),
        },
    ];

    assert!(build_node_visual_section(&state, &llm_editor).is_some());

    let mut answer_editor = editor_with_visual_draft(
        "answer",
        Some(WorkflowNodeVisualDraft::Answer {
            answer_editor: text_editor::Content::with_text(""),
        }),
    );
    answer_editor.validation.field_errors.push(WorkflowNodeValidationError {
        path: "answer.text".to_string(),
        message: "answer required".to_string(),
    });

    assert!(build_node_visual_section(&state, &answer_editor).is_some());
}

#[test]
fn node_visual_section_builds_answer_visual() {
    assert_visual_section_builds(
        "answer",
        WorkflowNodeVisualDraft::Answer {
            answer_editor: text_editor::Content::with_text("answer"),
        },
    );
}

#[test]
fn node_visual_section_dispatches_by_visual_draft_instead_of_block_type() {
    assert_visual_section_builds(
        "answer",
        WorkflowNodeVisualDraft::Tool {
            provider_id: "provider_1".to_string(),
            provider_type: "builtin".to_string(),
            provider_name: "search".to_string(),
            tool_name: "web_search".to_string(),
            tool_label: "Web Search".to_string(),
            tool_description: "search the web".to_string(),
            credential_id: "credential_1".to_string(),
            plugin_unique_identifier: "plugin.search".to_string(),
            tool_parameters_editor: text_editor::Content::with_text("{}"),
            tool_configurations_editor: text_editor::Content::with_text("{}"),
        },
    );
}

#[test]
fn node_visual_section_builds_if_else_visual() {
    assert_visual_section_builds(
        "if-else",
        WorkflowNodeVisualDraft::IfElse {
            cases: vec![WorkflowIfElseCaseDraft {
                raw_case: serde_yaml::Value::Null,
                case_id: "case_1".to_string(),
                logical_operator: "and".to_string(),
                conditions: Vec::new(),
            }],
        },
    );
}

#[test]
fn node_visual_section_builds_empty_if_else_visual() {
    assert_visual_section_builds("if-else", WorkflowNodeVisualDraft::IfElse { cases: Vec::new() });
}

#[test]
fn node_visual_section_builds_knowledge_retrieval_visual() {
    assert_visual_section_builds(
        "knowledge-retrieval",
        WorkflowNodeVisualDraft::KnowledgeRetrieval {
            query_selector_input: "start.query".to_string(),
            query_attachment_selector_input: "start.files".to_string(),
            dataset_ids_input: "dataset_1,dataset_2".to_string(),
            retrieval_mode: "multiple".to_string(),
            top_k_input: "3".to_string(),
            score_threshold_enabled: true,
            score_threshold_input: "0.5".to_string(),
            reranking_enable: true,
            single_model_provider: "langgenius/openai/openai".to_string(),
            single_model_name: "text-embedding-3-large".to_string(),
            single_model_mode: "embedding".to_string(),
        },
    );
}

#[test]
fn node_visual_section_builds_tool_visual() {
    assert_visual_section_builds(
        "tool",
        WorkflowNodeVisualDraft::Tool {
            provider_id: "provider_1".to_string(),
            provider_type: "builtin".to_string(),
            provider_name: "search".to_string(),
            tool_name: "web_search".to_string(),
            tool_label: "Web Search".to_string(),
            tool_description: "search the web".to_string(),
            credential_id: "credential_1".to_string(),
            plugin_unique_identifier: "plugin.search".to_string(),
            tool_parameters_editor: text_editor::Content::with_text("{}"),
            tool_configurations_editor: text_editor::Content::with_text("{}"),
        },
    );
}

#[test]
fn node_visual_section_builds_agent_visual() {
    assert_visual_section_builds(
        "agent",
        WorkflowNodeVisualDraft::Agent {
            strategy_provider_name: "langgenius/agent/agent".to_string(),
            strategy_name: "function_calling".to_string(),
            strategy_label: "Function Calling".to_string(),
            plugin_unique_identifier: "plugin.agent".to_string(),
            output_schema_editor: text_editor::Content::with_text("{}"),
            parameters_editor: text_editor::Content::with_text("{}"),
            memory_enabled: true,
            memory_window_size_input: "5".to_string(),
            memory_prompt_editor: text_editor::Content::with_text("memory"),
        },
    );
}

#[test]
fn node_visual_section_builds_code_visual() {
    assert_visual_section_builds(
        "code",
        WorkflowNodeVisualDraft::Code {
            language: "python3".to_string(),
            inputs: vec![WorkflowCodeVariableDraft {
                variable: "query".to_string(),
                value_type: "string".to_string(),
                selector: vec!["start".to_string(), "query".to_string()],
            }],
            code_editor: text_editor::Content::with_text("def main(query):\n    return query\n"),
            outputs: vec![WorkflowCodeOutputDraft {
                key: "result".to_string(),
                value_type: "string".to_string(),
            }],
            retry_config: WorkflowNodeRetryDraft {
                enabled: true,
                max_retries: 2,
                retry_interval: 1000,
            },
            error_strategy: "fail-branch".to_string(),
            default_value_editor: text_editor::Content::with_text("[]"),
        },
    );
}
