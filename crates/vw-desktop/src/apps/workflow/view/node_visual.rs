//! Compose panels that still belong to the desktop workflow shell.
use super::*;
pub(super) fn build_node_visual_section<'a>(i18n: crate::app::i18n::I18n, state: &'a WorkflowState, editor: &'a state::WorkflowNodeEditorDraft) -> Option<Element<'a, Message>> {
    let validation = &editor.validation;
    let supplemental = match editor.visual_draft.as_ref()? {
        WorkflowNodeVisualDraft::Start { variables } => {
            Some(build_start_visual_section(i18n, state, variables, validation))
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
            i18n,
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
            i18n,
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
            i18n,
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
        _ => None,
    };
    // Local panels return workflow messages; shell-owned panels are already mapped.
    if supplemental.is_some() { return supplemental; }
    desktop_workflow_ui::view::node_visual::build_node_visual_section(i18n, editor).map(|view| view.map(Message::WorkflowTool))
}
#[cfg(test)]
#[path = "node_visual_tests.rs"]
mod node_visual_tests;
