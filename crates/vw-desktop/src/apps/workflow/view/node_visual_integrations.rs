//! 工作流集成节点视图模块，负责知识库、工具、智能体和代码节点的配置面板。

use super::*;
use iced::widget::{column, row};

#[cfg(test)]
#[path = "node_visual_integrations_tests.rs"]
mod tests;

/// 构建 knowledge visual section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_knowledge_visual_section<'a>(
    validation: &'a super::state::WorkflowNodeEditorValidation,
    query_selector_input: &'a str,
    query_attachment_selector_input: &'a str,
    dataset_ids_input: &'a str,
    retrieval_mode: &'a str,
    top_k_input: &'a str,
    score_threshold_enabled: bool,
    score_threshold_input: &'a str,
    reranking_enable: bool,
    reranking_model_editor: &'a text_editor::Content,
    metadata_filtering_mode: &'a str,
    metadata_model_config_editor: &'a text_editor::Content,
    single_model_provider: &'a str,
    single_model_name: &'a str,
    single_model_mode: &'a str,
) -> Element<'a, Message> {
    column![
        row![
            build_editor_field_validated(
                "知识库 ID",
                workflow_text_input(
                    "逗号分隔，例如：kb_orders, kb_faq",
                    dataset_ids_input,
                    |value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorKnowledgeDatasetIdsChanged(value),
                        )
                    }
                ),
                validation.first_error_for("knowledge.dataset_ids"),
            ),
            build_editor_field_validated(
                "检索模式",
                workflow_text_input("single 或 multiple", retrieval_mode, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorKnowledgeRetrievalModeChanged(
                        value,
                    ))
                }),
                validation.first_error_for("knowledge.retrieval_mode"),
            ),
        ]
        .spacing(12),
        row![
            build_editor_field_validated(
                "查询变量选择器",
                workflow_text_input("例如：sys.query", query_selector_input, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorKnowledgeQuerySelectorChanged(
                        value,
                    ))
                }),
                validation.first_error_for("knowledge.query_selector"),
            ),
            build_editor_field(
                "附件变量选择器",
                workflow_text_input(
                    "例如：start.files",
                    query_attachment_selector_input,
                    |value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorKnowledgeQueryAttachmentSelectorChanged(
                                value,
                            ),
                        )
                    }
                ),
            ),
        ]
        .spacing(12),
        row![
            build_editor_field_validated(
                "top_k",
                workflow_text_input("例如：5", top_k_input, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorKnowledgeTopKChanged(value))
                }),
                validation.first_error_for("knowledge.multiple.top_k"),
            ),
            build_editor_field(
                "Reranking",
                row![
                    toggler(reranking_enable).on_toggle(|value| Message::WorkflowTool(
                        WorkflowMessage::NodeEditorKnowledgeRerankingEnabledChanged(value),
                    )),
                    text("multiple_retrieval_config.reranking_enable")
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            ),
        ]
        .spacing(12),
        build_editor_field(
            "Reranking Model YAML",
            build_embedded_text_editor(
                reranking_model_editor,
                "输入 reranking_model YAML map",
                WorkflowMessage::NodeEditorKnowledgeRerankingModelAction,
                120.0,
            ),
        ),
        build_editor_field(
            "Metadata Filtering Mode",
            workflow_text_input(
                "disabled / automatic / manual",
                metadata_filtering_mode,
                |value| Message::WorkflowTool(
                    WorkflowMessage::NodeEditorKnowledgeMetadataFilteringModeChanged(value),
                ),
            ),
        ),
        build_editor_field(
            "Metadata Model Config YAML",
            build_embedded_text_editor(
                metadata_model_config_editor,
                "输入 metadata_model_config YAML map",
                WorkflowMessage::NodeEditorKnowledgeMetadataModelConfigAction,
                120.0,
            ),
        ),
        build_editor_field_validated(
            "Score Threshold",
            column![
                row![
                    toggler(score_threshold_enabled).on_toggle(|value| Message::WorkflowTool(
                        WorkflowMessage::NodeEditorKnowledgeScoreThresholdEnabledChanged(value),
                    )),
                    text("启用 score_threshold").size(12).style(settings_muted_text_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                workflow_text_input("0 到 1，例如：0.5", score_threshold_input, |value| {
                    Message::WorkflowTool(
                        WorkflowMessage::NodeEditorKnowledgeScoreThresholdChanged(value),
                    )
                }),
            ]
            .spacing(8)
            .into(),
            validation.first_error_for("knowledge.multiple.score_threshold"),
        ),
        row![
            build_editor_field_validated(
                "Single Provider",
                workflow_text_input(
                    "例如：langgenius/openai/openai",
                    single_model_provider,
                    |value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorKnowledgeSingleModelProviderChanged(value),
                        )
                    }
                ),
                validation.first_error_for("knowledge.single.provider"),
            ),
            build_editor_field_validated(
                "Single Model",
                workflow_text_input("例如：gpt-4o-mini", single_model_name, |value| {
                    Message::WorkflowTool(
                        WorkflowMessage::NodeEditorKnowledgeSingleModelNameChanged(value),
                    )
                }),
                validation.first_error_for("knowledge.single.model_name"),
            ),
        ]
        .spacing(12),
        build_editor_field_validated(
            "Single Mode",
            workflow_text_input("例如：chat", single_model_mode, |value| {
                Message::WorkflowTool(WorkflowMessage::NodeEditorKnowledgeSingleModelModeChanged(
                    value,
                ))
            }),
            validation.first_error_for("knowledge.single.model_mode"),
        ),
    ]
    .spacing(12)
    .into()
}

/// 构建 tool visual section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_tool_visual_section<'a>(
    validation: &'a super::state::WorkflowNodeEditorValidation,
    provider_id: &'a str,
    provider_type: &'a str,
    provider_name: &'a str,
    tool_name: &'a str,
    tool_label: &'a str,
    tool_description: &'a str,
    credential_id: &'a str,
    plugin_unique_identifier: &'a str,
    tool_parameters_editor: &'a text_editor::Content,
    tool_configurations_editor: &'a text_editor::Content,
) -> Element<'a, Message> {
    column![
        row![
            build_editor_field_validated(
                "Provider ID",
                workflow_text_input("例如：google", provider_id, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorToolProviderIdChanged(value))
                }),
                validation.first_error_for("tool.provider_id"),
            ),
            build_editor_field_validated(
                "Provider Type",
                workflow_text_input("例如：builtin / api", provider_type, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorToolProviderTypeChanged(value))
                }),
                validation.first_error_for("tool.provider_type"),
            ),
        ]
        .spacing(12),
        row![
            build_editor_field_validated(
                "Provider Name",
                workflow_text_input("例如：google/google", provider_name, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorToolProviderNameChanged(value))
                }),
                validation.first_error_for("tool.provider_name"),
            ),
            build_editor_field_validated(
                "Tool Name",
                workflow_text_input("例如：search", tool_name, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorToolNameChanged(value))
                }),
                validation.first_error_for("tool.tool_name"),
            ),
        ]
        .spacing(12),
        row![
            build_editor_field(
                "Tool Label",
                workflow_text_input("面向用户展示的名称", tool_label, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorToolLabelChanged(value))
                }),
            ),
            build_editor_field(
                "Credential ID",
                workflow_text_input("可选，例如：cred-123", credential_id, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorToolCredentialIdChanged(value))
                }),
            ),
        ]
        .spacing(12),
        build_editor_field(
            "Tool Description",
            workflow_text_input("工具说明", tool_description, |value| {
                Message::WorkflowTool(WorkflowMessage::NodeEditorToolDescriptionChanged(value))
            }),
        ),
        build_editor_field(
            "Plugin Unique Identifier",
            workflow_text_input("可选插件唯一标识", plugin_unique_identifier, |value| {
                Message::WorkflowTool(WorkflowMessage::NodeEditorToolPluginUniqueIdentifierChanged(
                    value,
                ))
            }),
        ),
        build_editor_field_validated(
            "Tool Parameters YAML",
            build_embedded_text_editor(
                tool_parameters_editor,
                "输入 tool_parameters YAML map",
                WorkflowMessage::NodeEditorToolParametersAction,
                150.0,
            ),
            validation.first_error_for("tool.tool_parameters"),
        ),
        build_editor_field_validated(
            "Tool Configurations YAML",
            build_embedded_text_editor(
                tool_configurations_editor,
                "输入 tool_configurations YAML map",
                WorkflowMessage::NodeEditorToolConfigurationsAction,
                150.0,
            ),
            validation.first_error_for("tool.tool_configurations"),
        ),
    ]
    .spacing(12)
    .into()
}

/// 构建 agent visual section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_agent_visual_section<'a>(
    validation: &'a super::state::WorkflowNodeEditorValidation,
    strategy_provider_name: &'a str,
    strategy_name: &'a str,
    strategy_label: &'a str,
    plugin_unique_identifier: &'a str,
    output_schema_editor: &'a text_editor::Content,
    parameters_editor: &'a text_editor::Content,
    memory_enabled: bool,
    memory_window_size_input: &'a str,
    memory_prompt_editor: &'a text_editor::Content,
) -> Element<'a, Message> {
    column![
        row![
            build_editor_field_validated(
                "策略 Provider",
                workflow_text_input(
                    "例如：langgenius/openai/openai",
                    strategy_provider_name,
                    |value| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorAgentStrategyProviderChanged(value),
                        )
                    }
                ),
                validation.first_error_for("agent.strategy_provider"),
            ),
            build_editor_field_validated(
                "策略名称",
                workflow_text_input("例如：function_call", strategy_name, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorAgentStrategyNameChanged(
                        value,
                    ))
                }),
                validation.first_error_for("agent.strategy_name"),
            ),
        ]
        .spacing(12),
        row![
            build_editor_field_validated(
                "策略显示名称",
                workflow_text_input("例如：Function Call", strategy_label, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorAgentStrategyLabelChanged(
                        value,
                    ))
                }),
                validation.first_error_for("agent.strategy_label"),
            ),
            build_editor_field(
                "Plugin Unique Identifier",
                workflow_text_input("可选插件唯一标识", plugin_unique_identifier, |value| {
                    Message::WorkflowTool(
                        WorkflowMessage::NodeEditorAgentPluginUniqueIdentifierChanged(value),
                    )
                }),
            ),
        ]
        .spacing(12),
        build_editor_field_validated(
            "输出结构 YAML",
            build_embedded_text_editor(
                output_schema_editor,
                "输入 output_schema YAML map",
                WorkflowMessage::NodeEditorAgentOutputSchemaAction,
                150.0,
            ),
            validation.first_error_for("agent.output_schema"),
        ),
        build_editor_field_validated(
            "Agent 参数 YAML",
            build_embedded_text_editor(
                parameters_editor,
                "输入 agent_parameters YAML map",
                WorkflowMessage::NodeEditorAgentParametersAction,
                150.0,
            ),
            validation.first_error_for("agent.parameters"),
        ),
        build_editor_field_validated(
            "Memory",
            column![
                row![
                    toggler(memory_enabled).on_toggle(|value| Message::WorkflowTool(
                        WorkflowMessage::NodeEditorAgentMemoryEnabledChanged(value),
                    )),
                    text("启用 memory.window").size(12).style(settings_muted_text_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                workflow_text_input("window size，例如：3", memory_window_size_input, |value| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorAgentMemoryWindowSizeChanged(
                        value,
                    ))
                }),
            ]
            .spacing(8)
            .into(),
            validation.first_error_for("agent.memory.window_size"),
        ),
        build_editor_field(
            "Memory Prompt",
            build_embedded_text_editor(
                memory_prompt_editor,
                "输入 memory.query_prompt_template",
                WorkflowMessage::NodeEditorAgentMemoryPromptAction,
                140.0,
            ),
        ),
    ]
    .spacing(12)
    .into()
}

/// 构建 code visual section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub(super) fn build_code_visual_section<'a>(
    state: &'a WorkflowState,
    editor: &'a super::state::WorkflowNodeEditorDraft,
    validation: &'a super::state::WorkflowNodeEditorValidation,
    language: &'a str,
    inputs: &'a [WorkflowCodeVariableDraft],
    code_editor: &'a text_editor::Content,
    outputs: &'a [WorkflowCodeOutputDraft],
    retry_config: WorkflowNodeRetryDraft,
    error_strategy: &'a str,
    default_value_editor: &'a text_editor::Content,
) -> Element<'a, Message> {
    let language_options = code_language_options().to_vec();
    let selected_language = code_picker_option(language, &language_options);
    let variable_options = build_code_variable_reference_options(state, editor);
    let output_type_options = code_output_type_options().to_vec();
    let error_strategy_options = code_error_strategy_options().to_vec();
    let selected_error_strategy = code_picker_option(error_strategy, &error_strategy_options);

    let input_list: Element<'a, Message> = if inputs.is_empty() {
        text("当前没有输入变量，可添加已有流程变量或系统变量作为代码参数。")
            .size(12)
            .style(settings_muted_text_style)
            .into()
    } else {
        column(
            inputs
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    build_code_input_variable_row(index, input, &variable_options, validation)
                })
                .collect::<Vec<Element<'a, Message>>>(),
        )
        .spacing(8)
        .into()
    };

    let output_list: Element<'a, Message> = if outputs.is_empty() {
        text("至少新增一个输出变量，代码节点的返回值会按这里声明的类型导出。")
            .size(12)
            .style(settings_muted_text_style)
            .into()
    } else {
        column(
            outputs
                .iter()
                .enumerate()
                .map(|(index, output)| {
                    build_code_output_variable_row(index, output, &output_type_options, validation)
                })
                .collect::<Vec<Element<'a, Message>>>(),
        )
        .spacing(8)
        .into()
    };

    let retry_error = validation
        .first_error_for("code.retry.max_retries")
        .or_else(|| validation.first_error_for("code.retry.retry_interval"));
    let error_strategy_hint = match error_strategy {
        "default-value" => "节点发生异常时，使用下面的默认输出继续流转。",
        "fail-branch" => "节点发生异常时，会暴露失败分支，可在“下一步”里继续编排处理节点。",
        _ => "发生异常且未处理时，节点将停止运行。",
    };

    column![
        build_editor_field(
            "输入变量",
            column![
                input_list,
                button(text("新增输入变量").size(12))
                    .style(rounded_action_btn_style)
                    .padding([8, 12])
                    .on_press(Message::WorkflowTool(
                        WorkflowMessage::NodeEditorCodeAddInputVariable,
                    )),
            ]
            .spacing(8)
            .into(),
        ),
        build_editor_field_validated(
            "代码语言",
            pick_list(language_options, selected_language, |option| {
                Message::WorkflowTool(WorkflowMessage::NodeEditorCodeLanguageChanged(
                    option.value.to_string(),
                ))
            })
            .padding([10, 12])
            .text_size(13)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .width(Length::Fill)
            .into(),
            validation.first_error_for("code.language"),
        ),
        build_editor_field_validated(
            "代码内容",
            build_embedded_text_editor(
                code_editor,
                "输入代码体",
                WorkflowMessage::NodeEditorCodeAction,
                220.0,
            ),
            validation.first_error_for("code.body"),
        ),
        build_editor_field_validated(
            "输出变量",
            column![
                output_list,
                button(text("新增输出变量").size(12))
                    .style(rounded_action_btn_style)
                    .padding([8, 12])
                    .on_press(Message::WorkflowTool(
                        WorkflowMessage::NodeEditorCodeAddOutputVariable,
                    )),
            ]
            .spacing(8)
            .into(),
            validation.first_error_for("code.outputs"),
        ),
        build_editor_field_validated(
            "失败时重试",
            column![
                row![
                    text("启用失败重试").size(12),
                    Space::new().width(Length::Fill),
                    toggler(retry_config.enabled).on_toggle(|value| Message::WorkflowTool(
                        WorkflowMessage::NodeEditorCodeRetryEnabledChanged(value),
                    )),
                ]
                .align_y(Alignment::Center),
                {
                    let retry_detail: Element<'a, Message> = if retry_config.enabled {
                        column![
                            row![
                                text("最大重试次数")
                                    .size(12)
                                    .style(settings_muted_text_style)
                                    .width(Length::Fixed(88.0)),
                                slider(
                                    1.0..=10.0,
                                    retry_config.max_retries as f32,
                                    |value: f32| {
                                        Message::WorkflowTool(
                                            WorkflowMessage::NodeEditorCodeRetryMaxRetriesChanged(
                                                (value.round() as u8).clamp(1, 10),
                                            ),
                                        )
                                    }
                                )
                                .step(1.0_f32)
                                .width(Length::Fill),
                                container(text(retry_config.max_retries.to_string()).size(15))
                                    .padding([6, 10])
                                    .width(Length::Fixed(56.0))
                                    .style(value_card_style),
                                text("次").size(12).style(settings_muted_text_style),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                            row![
                                text("重试间隔")
                                    .size(12)
                                    .style(settings_muted_text_style)
                                    .width(Length::Fixed(88.0)),
                                slider(
                                    100.0..=5000.0,
                                    retry_config.retry_interval as f32,
                                    |value: f32| {
                                        Message::WorkflowTool(
                                            WorkflowMessage::NodeEditorCodeRetryIntervalChanged(
                                                ((value.round() as u16) / 100).max(1) * 100,
                                            ),
                                        )
                                    }
                                )
                                .step(100.0_f32)
                                .width(Length::Fill),
                                container(text(retry_config.retry_interval.to_string()).size(15))
                                    .padding([6, 10])
                                    .width(Length::Fixed(76.0))
                                    .style(value_card_style),
                                text("毫秒").size(12).style(settings_muted_text_style),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        ]
                        .spacing(10)
                        .into()
                    } else {
                        container(
                            text("关闭后，代码节点发生错误会直接进入异常处理逻辑。")
                                .size(12)
                                .style(settings_muted_text_style),
                        )
                        .padding([6, 0])
                        .into()
                    };
                    retry_detail
                },
            ]
            .spacing(10)
            .into(),
            retry_error,
        ),
        build_editor_field_validated(
            "异常处理",
            column![
                pick_list(error_strategy_options, selected_error_strategy, |option| {
                    Message::WorkflowTool(WorkflowMessage::NodeEditorCodeErrorStrategyChanged(
                        option.value.to_string(),
                    ))
                })
                .padding([10, 12])
                .text_size(13)
                .style(settings_pick_list_style)
                .menu_style(settings_pick_list_menu_style)
                .width(Length::Fill),
                text(error_strategy_hint).size(12).style(settings_muted_text_style),
            ]
            .spacing(8)
            .into(),
            validation.first_error_for("code.error_strategy"),
        ),
        if error_strategy == "default-value" {
            build_editor_field_validated(
                "默认输出 YAML",
                build_embedded_text_editor(
                    default_value_editor,
                    "- key: result\n  type: string\n  value: ''",
                    WorkflowMessage::NodeEditorCodeDefaultValueAction,
                    160.0,
                ),
                validation.first_error_for("code.default_value"),
            )
        } else {
            container(Space::new().width(1).height(1)).into()
        },
    ]
    .spacing(12)
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WorkflowCodePickerOption {
    value: &'static str,
    label: &'static str,
}

impl std::fmt::Display for WorkflowCodePickerOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowCodeVariableReferenceOption {
    selector_key: String,
    label: String,
    value_type: String,
}

impl std::fmt::Display for WorkflowCodeVariableReferenceOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} · {}", self.label, code_value_type_label(&self.value_type))
    }
}

fn build_code_input_variable_row<'a>(
    index: usize,
    input: &'a WorkflowCodeVariableDraft,
    options: &[WorkflowCodeVariableReferenceOption],
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    let variable_path = format!("code.inputs[{index}].variable");
    let selector_path = format!("code.inputs[{index}].selector");
    let value_type_path = format!("code.inputs[{index}].value_type");
    let selected = selected_code_variable_reference_option(input, options);
    let mut row_options = options.to_vec();
    if let Some(selected) = selected.clone()
        && !row_options.iter().any(|option| option == &selected)
    {
        row_options.push(selected);
    }

    let selector_control: Element<'a, Message> = if row_options.is_empty() {
        text("当前没有可选变量").size(12).style(settings_muted_text_style).into()
    } else {
        pick_list(row_options, selected, move |option| {
            Message::WorkflowTool(WorkflowMessage::NodeEditorCodeInputVariableSelectorChanged(
                index,
                option.selector_key.clone(),
                option.value_type.clone(),
            ))
        })
        .padding([10, 12])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::FillPortion(3))
        .into()
    };

    let mut content = column![
        row![
            selector_control,
            workflow_text_input("参数名", &input.variable, move |value| {
                Message::WorkflowTool(WorkflowMessage::NodeEditorCodeInputVariableNameChanged(
                    index, value,
                ))
            }),
            text(code_value_type_label(&input.value_type))
                .size(11)
                .style(settings_muted_text_style),
            button(text("删除").size(12)).style(danger_action_btn_style).padding([8, 10]).on_press(
                Message::WorkflowTool(WorkflowMessage::NodeEditorCodeRemoveInputVariable(index),)
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    if let Some(error) = validation.first_error_for(&variable_path) {
        content = content.push(build_inline_error(error));
    }
    if let Some(error) = validation.first_error_for(&selector_path) {
        content = content.push(build_inline_error(error));
    }
    if let Some(error) = validation.first_error_for(&value_type_path) {
        content = content.push(build_inline_error(error));
    }

    content.into()
}

fn build_code_output_variable_row<'a>(
    index: usize,
    output: &'a WorkflowCodeOutputDraft,
    type_options: &[WorkflowCodePickerOption],
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    let key_path = format!("code.outputs[{index}].key");
    let type_path = format!("code.outputs[{index}].type");
    let selected = code_picker_option(&output.value_type, type_options)
        .or_else(|| Some(WorkflowCodePickerOption { value: "string", label: "String" }));

    let mut content = column![
        row![
            workflow_text_input("输出变量名", &output.key, move |value| {
                Message::WorkflowTool(WorkflowMessage::NodeEditorCodeOutputNameChanged(
                    index, value,
                ))
            }),
            pick_list(type_options.to_vec(), selected, move |option| {
                Message::WorkflowTool(WorkflowMessage::NodeEditorCodeOutputTypeChanged(
                    index,
                    option.value.to_string(),
                ))
            })
            .padding([10, 12])
            .text_size(13)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .width(Length::Fixed(196.0)),
            button(text("删除").size(12)).style(danger_action_btn_style).padding([8, 10]).on_press(
                Message::WorkflowTool(WorkflowMessage::NodeEditorCodeRemoveOutputVariable(index),)
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    if let Some(error) = validation.first_error_for(&key_path) {
        content = content.push(build_inline_error(error));
    }
    if let Some(error) = validation.first_error_for(&type_path) {
        content = content.push(build_inline_error(error));
    }

    content.into()
}

fn code_language_options() -> [WorkflowCodePickerOption; 2] {
    [
        WorkflowCodePickerOption { value: "python3", label: "PYTHON3" },
        WorkflowCodePickerOption { value: "javascript", label: "JAVASCRIPT" },
    ]
}

fn code_output_type_options() -> [WorkflowCodePickerOption; 8] {
    [
        WorkflowCodePickerOption { value: "string", label: "String" },
        WorkflowCodePickerOption { value: "number", label: "Number" },
        WorkflowCodePickerOption { value: "boolean", label: "Boolean" },
        WorkflowCodePickerOption { value: "array[number]", label: "Array[Number]" },
        WorkflowCodePickerOption { value: "array[string]", label: "Array[String]" },
        WorkflowCodePickerOption { value: "array[boolean]", label: "Array[Boolean]" },
        WorkflowCodePickerOption { value: "array[object]", label: "Array[Object]" },
        WorkflowCodePickerOption { value: "object", label: "Object" },
    ]
}

fn code_error_strategy_options() -> [WorkflowCodePickerOption; 3] {
    [
        WorkflowCodePickerOption { value: "none", label: "无" },
        WorkflowCodePickerOption { value: "default-value", label: "默认值" },
        WorkflowCodePickerOption { value: "fail-branch", label: "异常分支" },
    ]
}

fn code_picker_option(
    value: &str,
    options: &[WorkflowCodePickerOption],
) -> Option<WorkflowCodePickerOption> {
    options.iter().copied().find(|option| option.value == value)
}

fn code_value_type_label(value_type: &str) -> &'static str {
    match value_type {
        "number" => "Number",
        "boolean" => "Boolean",
        "object" => "Object",
        "array[number]" => "Array[Number]",
        "array[string]" => "Array[String]",
        "array[boolean]" => "Array[Boolean]",
        "array[object]" => "Array[Object]",
        "file" => "File",
        "array[file]" => "Array[File]",
        _ => "String",
    }
}

fn selected_code_variable_reference_option(
    input: &WorkflowCodeVariableDraft,
    options: &[WorkflowCodeVariableReferenceOption],
) -> Option<WorkflowCodeVariableReferenceOption> {
    let selector_key = input.selector.join(".");
    if selector_key.is_empty() {
        return None;
    }

    options.iter().find(|option| option.selector_key == selector_key).cloned().or_else(|| {
        Some(WorkflowCodeVariableReferenceOption {
            selector_key,
            label: if input.variable.trim().is_empty() {
                input.selector.join(".")
            } else {
                input.variable.clone()
            },
            value_type: input.value_type.clone(),
        })
    })
}

fn build_code_variable_reference_options(
    state: &WorkflowState,
    editor: &super::state::WorkflowNodeEditorDraft,
) -> Vec<WorkflowCodeVariableReferenceOption> {
    let current_node_id = match &editor.mode {
        super::state::WorkflowNodeEditorMode::Edit(node_id) => Some(node_id.as_str()),
        super::state::WorkflowNodeEditorMode::Create => None,
    };
    let restrict_to_upstream = current_node_id.is_some();
    let upstream_ids =
        current_node_id.map(|node_id| upstream_node_ids(state, node_id)).unwrap_or_default();
    let mut options = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for node in &state.document.nodes {
        if current_node_id == Some(node.id.as_str()) {
            continue;
        }
        if restrict_to_upstream && !upstream_ids.contains(node.id.as_str()) {
            continue;
        }

        if node.block_type == "start" {
            for (name, value_type) in start_node_variable_entries(node) {
                let selector_key = format!("{}.{}", node.id, name);
                if seen.insert(selector_key.clone()) {
                    options.push(WorkflowCodeVariableReferenceOption {
                        selector_key,
                        label: name,
                        value_type,
                    });
                }
            }
        }

        for (key, value_type) in node_output_entries(node) {
            let selector_key = format!("{}.{}", node.id, key);
            if seen.insert(selector_key.clone()) {
                options.push(WorkflowCodeVariableReferenceOption {
                    selector_key,
                    label: format!("{} · {}", node.title, key),
                    value_type,
                });
            }
        }
    }

    for variable in &state.environment_variables {
        let selector_key = format!("env.{}", variable.name);
        if seen.insert(selector_key.clone()) {
            options.push(WorkflowCodeVariableReferenceOption {
                selector_key,
                label: format!("env.{}", variable.name),
                value_type: variable.value_type.clone(),
            });
        }
    }

    for variable in &state.conversation_variables {
        let selector_key = format!("conversation.{}", variable.name);
        if seen.insert(selector_key.clone()) {
            options.push(WorkflowCodeVariableReferenceOption {
                selector_key,
                label: format!("conversation.{}", variable.name),
                value_type: variable.value_type.clone(),
            });
        }
    }

    if let Some(meta) = state.active_meta() {
        for variable in workflow_system_variables(meta) {
            let key = variable.name.trim_start_matches("sys.");
            let selector_key = format!("sys.{key}");
            if seen.insert(selector_key.clone()) {
                options.push(WorkflowCodeVariableReferenceOption {
                    selector_key,
                    label: variable.name.to_string(),
                    value_type: variable.value_type.to_string(),
                });
            }
        }
    }

    options
}

fn upstream_node_ids<'a>(
    state: &'a WorkflowState,
    node_id: &str,
) -> std::collections::HashSet<&'a str> {
    let mut visited = std::collections::HashSet::new();
    let mut frontier = vec![node_id];

    while let Some(target_id) = frontier.pop() {
        for edge in state.document.edges.iter().filter(|edge| edge.target == target_id) {
            if visited.insert(edge.source.as_str()) {
                frontier.push(edge.source.as_str());
            }
        }
    }

    visited
}

fn start_node_variable_entries(node: &WorkflowNode) -> Vec<(String, String)> {
    node_data_map(node)
        .and_then(|data| data.get(&serde_yaml::Value::String("variables".to_string())))
        .and_then(serde_yaml::Value::as_sequence)
        .map(|variables| {
            variables
                .iter()
                .filter_map(serde_yaml::Value::as_mapping)
                .filter_map(|item| {
                    let name = item
                        .get(&serde_yaml::Value::String("variable".to_string()))
                        .and_then(serde_yaml::Value::as_str)?
                        .trim()
                        .to_string();
                    if name.is_empty() {
                        return None;
                    }

                    let input_type = item
                        .get(&serde_yaml::Value::String("type".to_string()))
                        .and_then(serde_yaml::Value::as_str)
                        .unwrap_or("text-input");
                    Some((name, start_input_value_type(input_type).to_string()))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn node_output_entries(node: &WorkflowNode) -> Vec<(String, String)> {
    node_data_map(node)
        .and_then(|data| data.get(&serde_yaml::Value::String("outputs".to_string())))
        .and_then(serde_yaml::Value::as_mapping)
        .map(|outputs| {
            outputs
                .iter()
                .filter_map(|(key, value)| {
                    let key = key.as_str()?.trim().to_string();
                    if key.is_empty() {
                        return None;
                    }

                    let value_type = value
                        .as_mapping()
                        .and_then(|map| map.get(&serde_yaml::Value::String("type".to_string())))
                        .and_then(serde_yaml::Value::as_str)
                        .unwrap_or("string")
                        .to_string();
                    Some((key, value_type))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn node_data_map(node: &WorkflowNode) -> Option<&serde_yaml::Mapping> {
    node.raw_node
        .as_mapping()
        .and_then(|map| map.get(&serde_yaml::Value::String("data".to_string())))
        .and_then(serde_yaml::Value::as_mapping)
}

fn start_input_value_type(input_type: &str) -> &'static str {
    match input_type {
        "number" => "number",
        "checkbox" => "boolean",
        "file" => "file",
        "file-list" => "array[file]",
        _ => "string",
    }
}
