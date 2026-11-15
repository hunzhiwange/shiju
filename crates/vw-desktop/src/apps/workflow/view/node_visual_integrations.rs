//! Desktop mapping for workflow domain views.
use super::*;
pub(super) fn build_knowledge_visual_section<'a>(
    i18n: crate::app::i18n::I18n,
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
    desktop_workflow_ui::view::node_visual_integrations::build_knowledge_visual_section(
        i18n,
        validation,
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
    )
    .map(Message::WorkflowTool)
}
pub(super) fn build_tool_visual_section<'a>(
    i18n: crate::app::i18n::I18n,
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
    desktop_workflow_ui::view::node_visual_integrations::build_tool_visual_section(
        i18n,
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
    )
    .map(Message::WorkflowTool)
}
pub(super) fn build_agent_visual_section<'a>(
    i18n: crate::app::i18n::I18n,
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
    desktop_workflow_ui::view::node_visual_integrations::build_agent_visual_section(
        i18n,
        validation,
        strategy_provider_name,
        strategy_name,
        strategy_label,
        plugin_unique_identifier,
        output_schema_editor,
        parameters_editor,
        memory_enabled,
        memory_window_size_input,
        memory_prompt_editor,
    )
    .map(Message::WorkflowTool)
}
pub(super) fn build_code_visual_section<'a>(
    i18n: crate::app::i18n::I18n,
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
    desktop_workflow_ui::view::node_visual_integrations::build_code_visual_section(
        i18n,
        state,
        editor,
        validation,
        language,
        inputs,
        code_editor,
        outputs,
        retry_config,
        error_strategy,
        default_value_editor,
    )
    .map(Message::WorkflowTool)
}
