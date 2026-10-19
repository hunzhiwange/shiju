//! 工作流节点可视化视图模块，根据节点类型分派到对应的配置面板渲染函数。

use super::node_editor_fields::{
    build_model_fields, build_named_value_list, build_variable_selector_field,
};
use super::state::{
    WorkflowClassifierClassDraft, WorkflowModelDraft, WorkflowNodeEditorValidation,
    WorkflowNodeFieldEdit, WorkflowParameterDraft, WorkflowStructuredNodeEdit,
    WorkflowVariableSelectorDraft,
};
use super::*;
use iced::widget::{column, row};

/// 构建 node visual section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_node_visual_section<'a>(
    state: &'a WorkflowState,
    editor: &'a super::state::WorkflowNodeEditorDraft,
) -> Option<Element<'a, Message>> {
    let validation = &editor.validation;

    match editor.visual_draft.as_ref()? {
        WorkflowNodeVisualDraft::Start { variables } => {
            Some(build_start_visual_section(state, variables, validation))
        }
        WorkflowNodeVisualDraft::Llm {
            provider,
            model_name,
            model_mode,
            enable_thinking,
            context_enabled,
            context_selector_input,
            system_prompt_editor,
            user_prompt_editor,
            vision_enabled,
        } => Some(
            column![
                row![
                    build_editor_field_validated(
                        "模型 Provider",
                        workflow_text_input(
                            "例如：langgenius/tongyi/tongyi",
                            provider,
                            |value| {
                                Message::WorkflowTool(
                                    WorkflowMessage::NodeEditorLlmProviderChanged(value),
                                )
                            }
                        ),
                        validation.first_error_for("llm.provider"),
                    ),
                    build_editor_field_validated(
                        "模型名称",
                        workflow_text_input("例如：qwen-turbo-2025-07-15", model_name, |value| {
                            Message::WorkflowTool(WorkflowMessage::NodeEditorLlmModelNameChanged(
                                value,
                            ))
                        }),
                        validation.first_error_for("llm.model_name"),
                    ),
                ]
                .spacing(12),
                row![
                    build_editor_field_validated(
                        "模型模式",
                        workflow_text_input("例如：chat", model_mode, |value| {
                            Message::WorkflowTool(WorkflowMessage::NodeEditorLlmModelModeChanged(
                                value,
                            ))
                        }),
                        validation.first_error_for("llm.model_mode"),
                    ),
                    build_editor_field(
                        "推理开关",
                        row![
                            toggler(*enable_thinking).on_toggle(|value| Message::WorkflowTool(
                                WorkflowMessage::NodeEditorLlmEnableThinkingChanged(value),
                            )),
                            text("completion_params.enable_thinking")
                                .size(12)
                                .style(settings_muted_text_style),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .into(),
                    ),
                ]
                .spacing(12),
                build_editor_field_validated(
                    "上下文变量",
                    column![
                        row![
                            toggler(*context_enabled).on_toggle(|value| Message::WorkflowTool(
                                WorkflowMessage::NodeEditorLlmContextEnabledChanged(value),
                            )),
                            text("启用上下文拼接").size(12).style(settings_muted_text_style),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        workflow_text_input(
                            "用逗号分隔，例如：node_id.text, sys.query",
                            context_selector_input,
                            |value| {
                                Message::WorkflowTool(
                                    WorkflowMessage::NodeEditorLlmContextSelectorChanged(value),
                                )
                            },
                        ),
                    ]
                    .spacing(8)
                    .into(),
                    validation.first_error_for("llm.context_selector"),
                ),
                build_editor_field(
                    "SYSTEM Prompt",
                    build_embedded_text_editor(
                        system_prompt_editor,
                        "输入 system prompt",
                        WorkflowMessage::NodeEditorLlmSystemPromptAction,
                        180.0,
                    ),
                ),
                build_editor_field(
                    "USER Prompt",
                    build_embedded_text_editor(
                        user_prompt_editor,
                        "输入 user prompt",
                        WorkflowMessage::NodeEditorLlmUserPromptAction,
                        120.0,
                    ),
                ),
                build_editor_field(
                    "视觉输入",
                    row![
                        toggler(*vision_enabled).on_toggle(|value| Message::WorkflowTool(
                            WorkflowMessage::NodeEditorLlmVisionEnabledChanged(value),
                        )),
                        text("vision.enabled").size(12).style(settings_muted_text_style),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into(),
                ),
            ]
            .spacing(12)
            .into(),
        ),
        WorkflowNodeVisualDraft::Answer { answer_editor } => Some(build_editor_field_validated(
            "回复内容",
            build_embedded_text_editor(
                answer_editor,
                "输入回复文本或模版内容",
                WorkflowMessage::NodeEditorAnswerAction,
                220.0,
            ),
            validation.first_error_for("answer.text"),
        )),
        WorkflowNodeVisualDraft::End { outputs } => Some(
            column![
                text("输出变量").size(13),
                build_named_value_list(outputs, |edit| Message::WorkflowTool(
                    WorkflowMessage::NodeEditorEndFieldEdit(edit)
                )),
            ]
            .spacing(8)
            .into(),
        ),
        WorkflowNodeVisualDraft::TemplateTransform { variables, template_editor } => Some(
            column![
                text("模板变量").size(13),
                build_named_value_list(variables, |edit| Message::WorkflowTool(
                    WorkflowMessage::NodeEditorTemplateFieldEdit(edit)
                )),
                build_editor_field_validated(
                    "Jinja 模板",
                    build_embedded_text_editor(
                        template_editor,
                        "输入 Jinja 模板",
                        WorkflowMessage::NodeEditorTemplateAction,
                        220.0,
                    ),
                    validation.first_error_for("template.template"),
                ),
            ]
            .spacing(10)
            .into(),
        ),
        WorkflowNodeVisualDraft::DocumentExtractor { variable_selector } => {
            Some(build_variable_selector_field("文件变量", variable_selector, |edit| match edit
            {
                WorkflowNodeFieldEdit::VariableSelectorChanged(value) => {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorDocumentSelectorChanged(value))
                }
                _ => Message::WorkflowTool(WorkflowMessage::NodeEditorDocumentSelectorChanged(
                    String::new(),
                )),
            }))
        }
        WorkflowNodeVisualDraft::QuestionClassifier {
            query,
            model,
            classes,
            instruction_editor,
        } => Some(build_question_classifier_section(
            validation,
            query,
            model,
            classes,
            instruction_editor,
        )),
        WorkflowNodeVisualDraft::ParameterExtractor {
            query,
            model,
            parameters,
            reasoning_mode,
            instruction_editor,
        } => Some(build_parameter_extractor_section(
            validation,
            query,
            model,
            parameters,
            reasoning_mode,
            instruction_editor,
        )),
        WorkflowNodeVisualDraft::IfElse { cases } => {
            Some(build_if_else_visual_section(cases, validation))
        }
        WorkflowNodeVisualDraft::KnowledgeRetrieval {
            query_selector_input,
            query_attachment_selector_input,
            dataset_ids_input,
            retrieval_mode,
            top_k_input,
            score_threshold_enabled,
            score_threshold_input,
            reranking_enable,
            single_model_provider,
            single_model_name,
            single_model_mode,
        } => Some(build_knowledge_visual_section(
            validation,
            query_selector_input,
            query_attachment_selector_input,
            dataset_ids_input,
            retrieval_mode,
            top_k_input,
            *score_threshold_enabled,
            score_threshold_input,
            *reranking_enable,
            single_model_provider,
            single_model_name,
            single_model_mode,
        )),
        WorkflowNodeVisualDraft::Tool {
            provider_id,
            provider_type,
            provider_name,
            tool_name,
            tool_label,
            tool_description,
            credential_id,
            plugin_unique_identifier,
            tool_parameters_editor,
            tool_configurations_editor,
        } => Some(build_tool_visual_section(
            validation,
            provider_id,
            provider_type,
            provider_name,
            tool_name,
            tool_label,
            tool_description,
            credential_id,
            plugin_unique_identifier,
            tool_parameters_editor,
            tool_configurations_editor,
        )),
        WorkflowNodeVisualDraft::Agent {
            strategy_provider_name,
            strategy_name,
            strategy_label,
            plugin_unique_identifier,
            output_schema_editor,
            parameters_editor,
            memory_enabled,
            memory_window_size_input,
            memory_prompt_editor,
        } => Some(build_agent_visual_section(
            validation,
            strategy_provider_name,
            strategy_name,
            strategy_label,
            plugin_unique_identifier,
            output_schema_editor,
            parameters_editor,
            *memory_enabled,
            memory_window_size_input,
            memory_prompt_editor,
        )),
        WorkflowNodeVisualDraft::Code {
            language,
            inputs,
            code_editor,
            outputs,
            retry_config,
            error_strategy,
            default_value_editor,
        } => Some(build_code_visual_section(
            state,
            editor,
            validation,
            language,
            inputs,
            code_editor,
            outputs,
            *retry_config,
            error_strategy,
            default_value_editor,
        )),
    }
}

fn structured_message(edit: WorkflowStructuredNodeEdit) -> Message {
    Message::WorkflowTool(WorkflowMessage::NodeEditorStructuredFieldEdit(edit))
}

fn build_structured_model_header<'a>(
    query: &'a WorkflowVariableSelectorDraft,
    model: &'a WorkflowModelDraft,
) -> Element<'a, Message> {
    column![
        build_variable_selector_field("输入变量", query, |edit| match edit {
            WorkflowNodeFieldEdit::VariableSelectorChanged(value) => {
                structured_message(WorkflowStructuredNodeEdit::Query(value))
            }
            _ => structured_message(WorkflowStructuredNodeEdit::Query(String::new())),
        }),
        build_model_fields(model, |edit| match edit {
            WorkflowNodeFieldEdit::ModelProviderChanged(value) => {
                structured_message(WorkflowStructuredNodeEdit::ModelProvider(value))
            }
            WorkflowNodeFieldEdit::ModelNameChanged(value) => {
                structured_message(WorkflowStructuredNodeEdit::ModelName(value))
            }
            WorkflowNodeFieldEdit::ModelModeChanged(value) => {
                structured_message(WorkflowStructuredNodeEdit::ModelMode(value))
            }
            _ => structured_message(WorkflowStructuredNodeEdit::ModelMode(String::new())),
        }),
    ]
    .spacing(10)
    .into()
}

fn build_question_classifier_section<'a>(
    validation: &'a WorkflowNodeEditorValidation,
    query: &'a WorkflowVariableSelectorDraft,
    model: &'a WorkflowModelDraft,
    classes: &'a [WorkflowClassifierClassDraft],
    instruction_editor: &'a text_editor::Content,
) -> Element<'a, Message> {
    let mut list = column![text("分类").size(13)].spacing(8);
    for (index, class) in classes.iter().enumerate() {
        list = list.push(
            container(
                row![
                    text_input("分类 ID", &class.id).on_input(move |value| Message::WorkflowTool(
                        WorkflowMessage::NodeEditorClassifierClassIdChanged(index, value)
                    )),
                    text_input("分类名称", &class.name).on_input(move |value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorClassifierClassNameChanged(index, value),
                        )
                    }),
                    button(text("删除")).on_press(Message::WorkflowTool(
                        WorkflowMessage::NodeEditorClassifierRemoveClass(index),
                    )),
                ]
                .spacing(8),
            )
            .padding(10)
            .style(super::node_editor_fields::node_editor_field_card_style),
        );
    }
    column![
        build_structured_model_header(query, model),
        list.push(
            button(text("新增分类"))
                .on_press(Message::WorkflowTool(WorkflowMessage::NodeEditorClassifierAddClass))
        ),
        build_editor_field_validated(
            "分类指令",
            build_embedded_text_editor(
                instruction_editor,
                "输入分类指令",
                |action| WorkflowMessage::NodeEditorStructuredFieldEdit(
                    WorkflowStructuredNodeEdit::Instruction(action)
                ),
                140.0,
            ),
            validation.first_error_for("classifier.instruction"),
        ),
    ]
    .spacing(10)
    .into()
}

fn build_parameter_extractor_section<'a>(
    _validation: &'a WorkflowNodeEditorValidation,
    query: &'a WorkflowVariableSelectorDraft,
    model: &'a WorkflowModelDraft,
    parameters: &'a [WorkflowParameterDraft],
    reasoning_mode: &'a str,
    instruction_editor: &'a text_editor::Content,
) -> Element<'a, Message> {
    let mut list = column![text("参数").size(13)].spacing(8);
    for (index, parameter) in parameters.iter().enumerate() {
        list = list.push(
            container(
                column![
                    row![
                        text_input("参数名", &parameter.name).on_input(move |value| {
                            structured_message(WorkflowStructuredNodeEdit::ParameterName(
                                index, value,
                            ))
                        }),
                        text_input("类型", &parameter.value_type).on_input(move |value| {
                            structured_message(WorkflowStructuredNodeEdit::ParameterType(
                                index, value,
                            ))
                        }),
                        button(text("删除")).on_press(Message::WorkflowTool(
                            WorkflowMessage::NodeEditorExtractorRemoveParameter(index),
                        )),
                    ]
                    .spacing(8),
                    text_input("参数说明", &parameter.description).on_input(move |value| {
                        structured_message(WorkflowStructuredNodeEdit::ParameterDescription(
                            index, value,
                        ))
                    }),
                    row![
                        toggler(parameter.required).on_toggle(move |value| {
                            structured_message(WorkflowStructuredNodeEdit::ParameterRequired(
                                index, value,
                            ))
                        }),
                        text("必填").size(12),
                    ]
                    .spacing(8),
                ]
                .spacing(8),
            )
            .padding(10)
            .style(super::node_editor_fields::node_editor_field_card_style),
        );
    }
    column![
        build_structured_model_header(query, model),
        workflow_text_input("推理模式", reasoning_mode, |value| structured_message(
            WorkflowStructuredNodeEdit::ReasoningMode(value)
        )),
        list.push(
            button(text("新增参数"))
                .on_press(Message::WorkflowTool(WorkflowMessage::NodeEditorExtractorAddParameter))
        ),
        build_editor_field(
            "提取指令",
            build_embedded_text_editor(
                instruction_editor,
                "输入提取指令",
                |action| WorkflowMessage::NodeEditorStructuredFieldEdit(
                    WorkflowStructuredNodeEdit::Instruction(action)
                ),
                140.0,
            ),
        ),
    ]
    .spacing(10)
    .into()
}

#[cfg(test)]
#[path = "node_visual_tests.rs"]
mod node_visual_tests;
