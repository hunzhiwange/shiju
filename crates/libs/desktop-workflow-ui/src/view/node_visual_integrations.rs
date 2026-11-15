//! 工作流集成节点视图模块，负责知识库、工具、智能体和代码节点的配置面板。

use super::*;
use desktop_workflow_model::workflow_system_variables;
use state::{WorkflowCodeVariableDraft, WorkflowCodeOutputDraft};
use iced::widget::slider;
use desktop_ui_controls::system_settings_common::settings_toggle;
use iced::widget::{column, row};

#[cfg(test)]
#[path = "node_visual_integrations_tests.rs"]
mod tests;

/// 构建 knowledge visual section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub fn build_knowledge_visual_section<'a>(
    i18n: desktop_i18n::I18n,
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
                i18n.__("知识库 ID"),
                workflow_text_input(
                    i18n.__("逗号分隔，例如：kb_orders, kb_faq"),
                    dataset_ids_input,
                    |value| {

                            WorkflowMessage::NodeEditorKnowledgeDatasetIdsChanged(value)
                    }
                ),
                validation.localized_error_for(i18n, "knowledge.dataset_ids"),
            ),
            build_editor_field_validated(
                i18n.__("检索模式"),
                workflow_text_input(i18n.__("single 或 multiple"), retrieval_mode, |value| {
                    WorkflowMessage::NodeEditorKnowledgeRetrievalModeChanged(
                        value,
                    )
                }),
                validation.localized_error_for(i18n, "knowledge.retrieval_mode"),
            ),
        ]
        .spacing(12),
        row![
            build_editor_field_validated(
                i18n.__("查询变量选择器"),
                workflow_text_input(i18n.__("例如：sys.query"), query_selector_input, |value| {
                    WorkflowMessage::NodeEditorKnowledgeQuerySelectorChanged(
                        value,
                    )
                }),
                validation.localized_error_for(i18n, "knowledge.query_selector"),
            ),
            build_editor_field(
                i18n.__("附件变量选择器"),
                workflow_text_input(
                    i18n.__("例如：start.files"),
                    query_attachment_selector_input,
                    |value| {

                            WorkflowMessage::NodeEditorKnowledgeQueryAttachmentSelectorChanged(
                                value,
                            )
                    }
                ),
            ),
        ]
        .spacing(12),
        row![
            build_editor_field_validated(
                "top_k",
                workflow_text_input(i18n.__("例如：5"), top_k_input, |value| {
                    WorkflowMessage::NodeEditorKnowledgeTopKChanged(value)
                }),
                validation.localized_error_for(i18n, "knowledge.multiple.top_k"),
            ),
            build_editor_field(
                "Reranking",
                row![
                    settings_toggle(reranking_enable).on_toggle(|value|
                        WorkflowMessage::NodeEditorKnowledgeRerankingEnabledChanged(value)),
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
                i18n.__("输入 reranking_model YAML map"),
                WorkflowMessage::NodeEditorKnowledgeRerankingModelAction,
                120.0,
            ),
        ),
        build_editor_field(
            "Metadata Filtering Mode",
            workflow_text_input(
                "disabled / automatic / manual",
                metadata_filtering_mode,
                |value|
                    WorkflowMessage::NodeEditorKnowledgeMetadataFilteringModeChanged(value),
            ),
        ),
        build_editor_field(
            "Metadata Model Config YAML",
            build_embedded_text_editor(
                metadata_model_config_editor,
                i18n.__("输入 metadata_model_config YAML map"),
                WorkflowMessage::NodeEditorKnowledgeMetadataModelConfigAction,
                120.0,
            ),
        ),
        build_editor_field_validated(
            "Score Threshold",
            column![
                row![
                    settings_toggle(score_threshold_enabled).on_toggle(|value| {

                            WorkflowMessage::NodeEditorKnowledgeScoreThresholdEnabledChanged(value)
                    }),
                    text(i18n.__("启用 score_threshold")).size(12).style(settings_muted_text_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                workflow_text_input(
                    i18n.__("0 到 1，例如：0.5"),
                    score_threshold_input,
                    |value| {

                            WorkflowMessage::NodeEditorKnowledgeScoreThresholdChanged(value)
                    }
                ),
            ]
            .spacing(8)
            .into(),
            validation.localized_error_for(i18n, "knowledge.multiple.score_threshold"),
        ),
        row![
            build_editor_field_validated(
                "Single Provider",
                workflow_text_input(
                    i18n.__("例如：langgenius/openai/openai"),
                    single_model_provider,
                    |value| {

                            WorkflowMessage::NodeEditorKnowledgeSingleModelProviderChanged(value)
                    }
                ),
                validation.localized_error_for(i18n, "knowledge.single.provider"),
            ),
            build_editor_field_validated(
                "Single Model",
                workflow_text_input(i18n.__("例如：gpt-4o-mini"), single_model_name, |value| {

                        WorkflowMessage::NodeEditorKnowledgeSingleModelNameChanged(value)
                }),
                validation.localized_error_for(i18n, "knowledge.single.model_name"),
            ),
        ]
        .spacing(12),
        build_editor_field_validated(
            "Single Mode",
            workflow_text_input(i18n.__("例如：chat"), single_model_mode, |value| {
                WorkflowMessage::NodeEditorKnowledgeSingleModelModeChanged(
                    value,
                )
            }),
            validation.localized_error_for(i18n, "knowledge.single.model_mode"),
        ),
    ]
    .spacing(12)
    .into()
}

/// 构建 tool visual section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub fn build_tool_visual_section<'a>(
    i18n: desktop_i18n::I18n,
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
                workflow_text_input(i18n.__("例如：google"), provider_id, |value| {
                    WorkflowMessage::NodeEditorToolProviderIdChanged(value)
                }),
                validation.localized_error_for(i18n, "tool.provider_id"),
            ),
            build_editor_field_validated(
                "Provider Type",
                workflow_text_input(i18n.__("例如：builtin / api"), provider_type, |value| {
                    WorkflowMessage::NodeEditorToolProviderTypeChanged(value)
                }),
                validation.localized_error_for(i18n, "tool.provider_type"),
            ),
        ]
        .spacing(12),
        row![
            build_editor_field_validated(
                "Provider Name",
                workflow_text_input(i18n.__("例如：google/google"), provider_name, |value| {
                    WorkflowMessage::NodeEditorToolProviderNameChanged(value)
                }),
                validation.localized_error_for(i18n, "tool.provider_name"),
            ),
            build_editor_field_validated(
                "Tool Name",
                workflow_text_input(i18n.__("例如：search"), tool_name, |value| {
                    WorkflowMessage::NodeEditorToolNameChanged(value)
                }),
                validation.localized_error_for(i18n, "tool.tool_name"),
            ),
        ]
        .spacing(12),
        row![
            build_editor_field(
                "Tool Label",
                workflow_text_input(i18n.__("面向用户展示的名称"), tool_label, |value| {
                    WorkflowMessage::NodeEditorToolLabelChanged(value)
                }),
            ),
            build_editor_field(
                "Credential ID",
                workflow_text_input(i18n.__("可选，例如：cred-123"), credential_id, |value| {
                    WorkflowMessage::NodeEditorToolCredentialIdChanged(value)
                }),
            ),
        ]
        .spacing(12),
        build_editor_field(
            "Tool Description",
            workflow_text_input(i18n.__("工具说明"), tool_description, |value| {
                WorkflowMessage::NodeEditorToolDescriptionChanged(value)
            }),
        ),
        build_editor_field(
            "Plugin Unique Identifier",
            workflow_text_input(
                i18n.__("可选插件唯一标识"),
                plugin_unique_identifier,
                |value| {

                        WorkflowMessage::NodeEditorToolPluginUniqueIdentifierChanged(value)
                }
            ),
        ),
        build_editor_field_validated(
            "Tool Parameters YAML",
            build_embedded_text_editor(
                tool_parameters_editor,
                i18n.__("输入 tool_parameters YAML map"),
                WorkflowMessage::NodeEditorToolParametersAction,
                150.0,
            ),
            validation.localized_error_for(i18n, "tool.tool_parameters"),
        ),
        build_editor_field_validated(
            "Tool Configurations YAML",
            build_embedded_text_editor(
                tool_configurations_editor,
                i18n.__("输入 tool_configurations YAML map"),
                WorkflowMessage::NodeEditorToolConfigurationsAction,
                150.0,
            ),
            validation.localized_error_for(i18n, "tool.tool_configurations"),
        ),
    ]
    .spacing(12)
    .into()
}

/// 构建 agent visual section 对应的界面元素。
///
/// 参数由当前工作流状态或编辑草稿提供；返回值是可直接嵌入 iced 视图树的元素。
pub fn build_agent_visual_section<'a>(
    i18n: desktop_i18n::I18n,
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
                i18n.__("策略 Provider"),
                workflow_text_input(
                    i18n.__("例如：langgenius/openai/openai"),
                    strategy_provider_name,
                    |value| {

                            WorkflowMessage::NodeEditorAgentStrategyProviderChanged(value)
                    }
                ),
                validation.localized_error_for(i18n, "agent.strategy_provider"),
            ),
            build_editor_field_validated(
                i18n.__("策略名称"),
                workflow_text_input(i18n.__("例如：function_call"), strategy_name, |value| {
                    WorkflowMessage::NodeEditorAgentStrategyNameChanged(
                        value,
                    )
                }),
                validation.localized_error_for(i18n, "agent.strategy_name"),
            ),
        ]
        .spacing(12),
        row![
            build_editor_field_validated(
                i18n.__("策略显示名称"),
                workflow_text_input(i18n.__("例如：Function Call"), strategy_label, |value| {
                    WorkflowMessage::NodeEditorAgentStrategyLabelChanged(
                        value,
                    )
                }),
                validation.localized_error_for(i18n, "agent.strategy_label"),
            ),
            build_editor_field(
                "Plugin Unique Identifier",
                workflow_text_input(
                    i18n.__("可选插件唯一标识"),
                    plugin_unique_identifier,
                    |value| {

                            WorkflowMessage::NodeEditorAgentPluginUniqueIdentifierChanged(value)
                    }
                ),
            ),
        ]
        .spacing(12),
        build_editor_field_validated(
            i18n.__("输出结构 YAML"),
            build_embedded_text_editor(
                output_schema_editor,
                i18n.__("输入 output_schema YAML map"),
                WorkflowMessage::NodeEditorAgentOutputSchemaAction,
                150.0,
            ),
            validation.localized_error_for(i18n, "agent.output_schema"),
        ),
        build_editor_field_validated(
            i18n.__("Agent 参数 YAML"),
            build_embedded_text_editor(
                parameters_editor,
                i18n.__("输入 agent_parameters YAML map"),
                WorkflowMessage::NodeEditorAgentParametersAction,
                150.0,
            ),
            validation.localized_error_for(i18n, "agent.parameters"),
        ),
        build_editor_field_validated(
            "Memory",
            column![
                row![
                    settings_toggle(memory_enabled).on_toggle(|value|
                        WorkflowMessage::NodeEditorAgentMemoryEnabledChanged(value)),
                    text(i18n.__("启用 memory.window")).size(12).style(settings_muted_text_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                workflow_text_input(
                    i18n.__("window size，例如：3"),
                    memory_window_size_input,
                    |value| {

                            WorkflowMessage::NodeEditorAgentMemoryWindowSizeChanged(value)
                    }
                ),
            ]
            .spacing(8)
            .into(),
            validation.localized_error_for(i18n, "agent.memory.window_size"),
        ),
        build_editor_field(
            "Memory Prompt",
            build_embedded_text_editor(
                memory_prompt_editor,
                i18n.__("输入 memory.query_prompt_template"),
                WorkflowMessage::NodeEditorAgentMemoryPromptAction,
                140.0,
            ),
        ),
    ]
    .spacing(12)
    .into()
}

mod code_view;
pub use code_view::build_code_visual_section;

mod code_variables;
use code_variables::*;
