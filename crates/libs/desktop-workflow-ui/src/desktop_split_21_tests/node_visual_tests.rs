//! 工作流节点可视化分派测试，覆盖各节点类型草稿到视图段落的构建行为。

use crate::view::node_visual::build_node_visual_section;
use desktop_workflow_state::state::*;
use iced::{Point, widget::text_editor};

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
    let editor = editor_with_visual_draft(block_type, Some(visual_draft));

    let section = build_node_visual_section(desktop_i18n::I18n::new(Default::default()), &editor);

    assert!(section.is_some());
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
