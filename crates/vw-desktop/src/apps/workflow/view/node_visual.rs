//! 工作流节点可视化视图模块，根据节点类型分派到对应的配置面板渲染函数。

use super::node_editor_fields::{
    build_model_fields, build_named_value_list, build_variable_selector_field,
};
use super::state::{
    WorkflowClassifierClassDraft, WorkflowLoopConditionDraft, WorkflowLoopVariableDraft,
    WorkflowModelDraft, WorkflowNodeEditorValidation, WorkflowNodeFieldEdit,
    WorkflowParameterDraft, WorkflowStructuredNodeEdit, WorkflowVariableSelectorDraft,
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
            completion_params_editor,
            structured_output_editor,
            memory_editor,
            context_enabled,
            context_selector_input,
            system_prompt_editor,
            user_prompt_editor,
            vision_enabled,
            vision_selector_input,
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
                build_editor_field(
                    "Completion Params YAML",
                    build_embedded_text_editor(
                        completion_params_editor,
                        "输入 completion_params YAML map",
                        WorkflowMessage::NodeEditorLlmCompletionParamsAction,
                        120.0,
                    ),
                ),
                build_editor_field(
                    "Structured Output YAML",
                    build_embedded_text_editor(
                        structured_output_editor,
                        "输入 structured_output YAML map",
                        WorkflowMessage::NodeEditorLlmStructuredOutputAction,
                        120.0,
                    ),
                ),
                build_editor_field(
                    "Memory YAML",
                    build_embedded_text_editor(
                        memory_editor,
                        "输入 memory YAML map",
                        WorkflowMessage::NodeEditorLlmMemoryAction,
                        120.0,
                    ),
                ),
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
                    column![
                        row![
                            toggler(*vision_enabled).on_toggle(|value| Message::WorkflowTool(
                                WorkflowMessage::NodeEditorLlmVisionEnabledChanged(value),
                            )),
                            text("vision.enabled").size(12).style(settings_muted_text_style),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        workflow_text_input(
                            "例如：start.image",
                            vision_selector_input,
                            |value| Message::WorkflowTool(
                                WorkflowMessage::NodeEditorLlmVisionSelectorChanged(value),
                            ),
                        ),
                    ]
                    .spacing(8)
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
            reranking_model_editor,
            metadata_filtering_mode,
            metadata_model_config_editor,
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
            reranking_model_editor,
            metadata_filtering_mode,
            metadata_model_config_editor,
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
        WorkflowNodeVisualDraft::HttpRequest {
            method,
            url,
            auth_type,
            auth_username,
            auth_password,
            auth_token,
            headers_editor,
            params_editor,
            body_type,
            body_editor,
            timeout_connect,
            timeout_read,
            timeout_write,
            ssl_verify,
            retry_config,
            error_strategy,
        } => Some(build_http_request_section(
            validation,
            method,
            url,
            auth_type,
            auth_username,
            auth_password,
            auth_token,
            headers_editor,
            params_editor,
            body_type,
            body_editor,
            timeout_connect,
            timeout_read,
            timeout_write,
            *ssl_verify,
            *retry_config,
            error_strategy,
        )),
        WorkflowNodeVisualDraft::VariableAssigner { items } => {
            Some(build_variable_assigner_section(items))
        }
        WorkflowNodeVisualDraft::VariableAggregator { variables, output_type } => {
            Some(build_variable_aggregator_section(variables, output_type))
        }
        WorkflowNodeVisualDraft::ListOperator {
            variable,
            conditions,
            filter_enabled,
            order_enabled,
            order_key,
            order_direction,
            limit_enabled,
            limit_size,
        } => Some(build_list_operator_section(
            variable,
            conditions,
            *filter_enabled,
            *order_enabled,
            order_key,
            order_direction,
            *limit_enabled,
            limit_size,
        )),
        WorkflowNodeVisualDraft::Iteration {
            iterator_selector,
            output_selector,
            output_type,
            is_parallel,
            parallel_nums,
            error_handle_mode,
        } => Some(build_iteration_section(
            iterator_selector,
            output_selector,
            output_type,
            *is_parallel,
            parallel_nums,
            error_handle_mode,
        )),
        WorkflowNodeVisualDraft::Loop {
            variables,
            break_conditions,
            logical_operator,
            max_loop_count,
        } => {
            Some(build_loop_section(variables, break_conditions, logical_operator, max_loop_count))
        }
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

fn data_node_message(edit: WorkflowDataNodeEdit) -> Message {
    Message::WorkflowTool(WorkflowMessage::NodeEditorDataNodeEdit(edit))
}

fn build_iteration_section<'a>(
    iterator_selector: &'a str,
    output_selector: &'a str,
    output_type: &'a str,
    is_parallel: bool,
    parallel_nums: &'a str,
    error_handle_mode: &'a str,
) -> Element<'a, Message> {
    column![
        build_editor_field(
            "迭代输入",
            workflow_text_input("例如：start.items", iterator_selector, |v| data_node_message(
                WorkflowDataNodeEdit::IterationIterator(v)
            ))
        ),
        build_editor_field(
            "迭代输出",
            workflow_text_input("例如：iteration.item", output_selector, |v| data_node_message(
                WorkflowDataNodeEdit::IterationOutput(v)
            ))
        ),
        row![
            build_editor_field(
                "输出类型",
                workflow_text_input("array[string]", output_type, |v| data_node_message(
                    WorkflowDataNodeEdit::IterationOutputType(v)
                ))
            ),
            build_editor_field(
                "错误处理",
                workflow_text_input("terminated", error_handle_mode, |v| data_node_message(
                    WorkflowDataNodeEdit::IterationErrorMode(v)
                ))
            ),
        ]
        .spacing(12),
        build_editor_field(
            "并行执行",
            row![
                toggler(is_parallel)
                    .on_toggle(|v| data_node_message(WorkflowDataNodeEdit::IterationParallel(v))),
                workflow_text_input("并行数量", parallel_nums, |v| data_node_message(
                    WorkflowDataNodeEdit::IterationParallelNums(v)
                )),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        ),
    ]
    .spacing(10)
    .into()
}

fn build_loop_section<'a>(
    variables: &'a [WorkflowLoopVariableDraft],
    conditions: &'a [WorkflowLoopConditionDraft],
    logical_operator: &'a str,
    max_loop_count: &'a str,
) -> Element<'a, Message> {
    let mut variable_list = column![text("循环变量").size(13)].spacing(8);
    for (index, item) in variables.iter().enumerate() {
        variable_list = variable_list.push(
            row![
                workflow_text_input("ID", &item.id, move |v| data_node_message(
                    WorkflowDataNodeEdit::LoopVariableId(index, v)
                )),
                workflow_text_input("名称", &item.label, move |v| data_node_message(
                    WorkflowDataNodeEdit::LoopVariableLabel(index, v)
                )),
                workflow_text_input("类型", &item.value_type, move |v| data_node_message(
                    WorkflowDataNodeEdit::LoopVariableType(index, v)
                )),
                workflow_text_input("初始值", &item.initial_value_input, move |v| {
                    data_node_message(WorkflowDataNodeEdit::LoopVariableValue(index, v))
                }),
                button(text("删除"))
                    .style(danger_action_btn_style)
                    .padding([8, 10])
                    .on_press(data_node_message(WorkflowDataNodeEdit::LoopVariableRemove(index))),
            ]
            .spacing(8),
        );
    }
    let mut condition_list = column![text("终止条件").size(13)].spacing(8);
    for (index, item) in conditions.iter().enumerate() {
        condition_list = condition_list.push(
            row![
                workflow_text_input("变量选择器", &item.selector, move |v| data_node_message(
                    WorkflowDataNodeEdit::LoopConditionSelector(index, v)
                )),
                workflow_text_input("操作符", &item.comparison_operator, move |v| {
                    data_node_message(WorkflowDataNodeEdit::LoopConditionOperator(index, v))
                }),
                workflow_text_input("比较值", &item.value_input, move |v| data_node_message(
                    WorkflowDataNodeEdit::LoopConditionValue(index, v)
                )),
                button(text("删除"))
                    .style(danger_action_btn_style)
                    .padding([8, 10])
                    .on_press(data_node_message(WorkflowDataNodeEdit::LoopConditionRemove(index))),
            ]
            .spacing(8),
        );
    }
    column![
        variable_list.push(
            button(text("新增循环变量"))
                .style(rounded_action_btn_style)
                .padding([8, 12])
                .on_press(data_node_message(WorkflowDataNodeEdit::LoopVariableAdd))
        ),
        condition_list.push(
            button(text("新增终止条件"))
                .style(rounded_action_btn_style)
                .padding([8, 12])
                .on_press(data_node_message(WorkflowDataNodeEdit::LoopConditionAdd))
        ),
        row![
            build_editor_field(
                "条件关系",
                workflow_text_input("and / or", logical_operator, |v| data_node_message(
                    WorkflowDataNodeEdit::LoopLogicalOperator(v)
                ))
            ),
            build_editor_field(
                "最大循环次数",
                workflow_text_input("10", max_loop_count, |v| data_node_message(
                    WorkflowDataNodeEdit::LoopMaxCount(v)
                ))
            ),
        ]
        .spacing(12),
    ]
    .spacing(12)
    .into()
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
                    workflow_text_input("分类 ID", &class.id, move |value| Message::WorkflowTool(
                        WorkflowMessage::NodeEditorClassifierClassIdChanged(index, value)
                    )),
                    workflow_text_input("分类名称", &class.name, move |value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorClassifierClassNameChanged(index, value),
                        )
                    }),
                    button(text("删除")).style(danger_action_btn_style).padding([8, 10]).on_press(
                        Message::WorkflowTool(WorkflowMessage::NodeEditorClassifierRemoveClass(
                            index
                        ),)
                    ),
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
                .style(rounded_action_btn_style)
                .padding([8, 12])
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
                        workflow_text_input("参数名", &parameter.name, move |value| {
                            structured_message(WorkflowStructuredNodeEdit::ParameterName(
                                index, value,
                            ))
                        }),
                        workflow_text_input("类型", &parameter.value_type, move |value| {
                            structured_message(WorkflowStructuredNodeEdit::ParameterType(
                                index, value,
                            ))
                        }),
                        button(text("删除"))
                            .style(danger_action_btn_style)
                            .padding([8, 10])
                            .on_press(Message::WorkflowTool(
                                WorkflowMessage::NodeEditorExtractorRemoveParameter(index),
                            )),
                    ]
                    .spacing(8),
                    workflow_text_input("参数说明", &parameter.description, move |value| {
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
                .style(rounded_action_btn_style)
                .padding([8, 12])
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
