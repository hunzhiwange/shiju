//! 工作流节点消息处理，负责节点属性、连接和运行配置更新。

use super::*;
use desktop_workflow_state::state::WorkflowState;
use iced::Task;

/// 构建或更新 handle 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn handle(
    state: &mut WorkflowState,
    window_size: (f32, f32),
    message: WorkflowMessage,
) -> Option<Task<WorkflowMessage>> {
    if let Some(task) = start_inputs::handle(state, message.clone()) {
        return Some(task);
    }
    Some(match message {
        WorkflowMessage::ToggleQuickInsertPanel => {
            state.toggle_quick_insert_panel();
            Task::none()
        }
        WorkflowMessage::InsertSuggestedNode(block_type) => {
            let position = suggested_new_node_position(state, window_size);
            if let Err(error) = state.insert_node_immediately(&block_type, position) {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::OpenCreateNodeEditor(block_type) => {
            let position = suggested_new_node_position(state, window_size);
            if let Err(error) = state.open_create_node_editor(&block_type, position) {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::OpenCreateNodeEditorAt(block_type, position) => {
            if let Err(error) = state.open_create_node_editor(&block_type, position) {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::CreateContextNode(block_type) => {
            if let Err(error) = state.create_context_node(&block_type) {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::OpenDownstreamNodePicker(node_id) => {
            if let Err(error) = state.open_downstream_node_picker(&node_id) {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::OpenEditNodeEditor(id) => {
            if let Err(error) = state.open_edit_node_editor(id.as_deref()) {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::CloseNodeEditor => {
            state.close_node_editor();
            Task::none()
        }
        WorkflowMessage::NodeEditorTabSelected(tab) => {
            state.set_node_editor_active_tab(tab);
            Task::none()
        }
        WorkflowMessage::NodeEditorTitleChanged(value) => {
            state.set_node_editor_title(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorDescriptionChanged(value) => {
            state.set_node_editor_description(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorDescriptionAction(action) => {
            state.node_editor_description_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorScrollCaptured => Task::none(),
        WorkflowMessage::InsertDownstreamNode(source_node_id, block_type) => {
            if let Err(error) = state.insert_downstream_node(&source_node_id, &block_type) {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::InsertDownstreamNodeFromHandle(
            source_node_id,
            source_handle_id,
            block_type,
        ) => {
            if let Err(error) = state.insert_downstream_node_from_handle(
                &source_node_id,
                &source_handle_id,
                &block_type,
            ) {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::NodeEditorShowRawDataEditorChanged(value) => {
            state.set_node_editor_show_raw_data_editor(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseAddCase => {
            state.add_node_editor_if_else_case();
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseCaseLogicalOperatorChanged(index, value) => {
            state.set_node_editor_if_else_case_logical_operator(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseAddCondition(case_index) => {
            state.add_node_editor_if_else_condition(case_index);
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseRemoveCondition(case_index, condition_index) => {
            state.remove_node_editor_if_else_condition(case_index, condition_index);
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseConditionSelectorChanged(
            case_index,
            condition_index,
            value,
        ) => {
            state.set_node_editor_if_else_condition_selector(case_index, condition_index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseConditionOperatorChanged(
            case_index,
            condition_index,
            value,
        ) => {
            state.set_node_editor_if_else_condition_operator(case_index, condition_index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseConditionValueChanged(
            case_index,
            condition_index,
            value,
        ) => {
            state.set_node_editor_if_else_condition_value(case_index, condition_index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseConditionVarTypeChanged(
            case_index,
            condition_index,
            value,
        ) => {
            state.set_node_editor_if_else_condition_var_type(case_index, condition_index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeQuerySelectorChanged(value) => {
            state.set_node_editor_knowledge_query_selector(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeQueryAttachmentSelectorChanged(value) => {
            state.set_node_editor_knowledge_query_attachment_selector(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeDatasetIdsChanged(value) => {
            state.set_node_editor_knowledge_dataset_ids(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeRetrievalModeChanged(value) => {
            state.set_node_editor_knowledge_retrieval_mode(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeTopKChanged(value) => {
            state.set_node_editor_knowledge_top_k(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeScoreThresholdEnabledChanged(value) => {
            state.set_node_editor_knowledge_score_threshold_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeScoreThresholdChanged(value) => {
            state.set_node_editor_knowledge_score_threshold(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeRerankingEnabledChanged(value) => {
            state.set_node_editor_knowledge_reranking_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeRerankingModelAction(action) => {
            state.node_editor_knowledge_reranking_model_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeMetadataFilteringModeChanged(value) => {
            state.set_node_editor_knowledge_metadata_filtering_mode(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeMetadataModelConfigAction(action) => {
            state.node_editor_knowledge_metadata_model_config_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeSingleModelProviderChanged(value) => {
            state.set_node_editor_knowledge_single_model_provider(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeSingleModelNameChanged(value) => {
            state.set_node_editor_knowledge_single_model_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeSingleModelModeChanged(value) => {
            state.set_node_editor_knowledge_single_model_mode(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolProviderIdChanged(value) => {
            state.set_node_editor_tool_provider_id(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolProviderTypeChanged(value) => {
            state.set_node_editor_tool_provider_type(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolProviderNameChanged(value) => {
            state.set_node_editor_tool_provider_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolNameChanged(value) => {
            state.set_node_editor_tool_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolLabelChanged(value) => {
            state.set_node_editor_tool_label(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolDescriptionChanged(value) => {
            state.set_node_editor_tool_description(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolCredentialIdChanged(value) => {
            state.set_node_editor_tool_credential_id(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolPluginUniqueIdentifierChanged(value) => {
            state.set_node_editor_tool_plugin_unique_identifier(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolParametersAction(action) => {
            state.node_editor_tool_parameters_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolConfigurationsAction(action) => {
            state.node_editor_tool_configurations_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentStrategyProviderChanged(value) => {
            state.set_node_editor_agent_strategy_provider(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentStrategyNameChanged(value) => {
            state.set_node_editor_agent_strategy_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentStrategyLabelChanged(value) => {
            state.set_node_editor_agent_strategy_label(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentPluginUniqueIdentifierChanged(value) => {
            state.set_node_editor_agent_plugin_unique_identifier(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentOutputSchemaAction(action) => {
            state.node_editor_agent_output_schema_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentParametersAction(action) => {
            state.node_editor_agent_parameters_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentMemoryEnabledChanged(value) => {
            state.set_node_editor_agent_memory_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentMemoryWindowSizeChanged(value) => {
            state.set_node_editor_agent_memory_window_size(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentMemoryPromptAction(action) => {
            state.node_editor_agent_memory_prompt_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmProviderChanged(value) => {
            state.set_node_editor_llm_provider(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmModelNameChanged(value) => {
            state.set_node_editor_llm_model_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmModelModeChanged(value) => {
            state.set_node_editor_llm_model_mode(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmEnableThinkingChanged(value) => {
            state.set_node_editor_llm_enable_thinking(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmCompletionParamsAction(action) => {
            state.node_editor_llm_completion_params_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmStructuredOutputAction(action) => {
            state.node_editor_llm_structured_output_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmMemoryAction(action) => {
            state.node_editor_llm_memory_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmContextEnabledChanged(value) => {
            state.set_node_editor_llm_context_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmContextSelectorChanged(value) => {
            state.set_node_editor_llm_context_selector_input(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmSystemPromptAction(action) => {
            state.node_editor_llm_system_prompt_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmUserPromptAction(action) => {
            state.node_editor_llm_user_prompt_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmVisionEnabledChanged(value) => {
            state.set_node_editor_llm_vision_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmVisionSelectorChanged(value) => {
            state.set_node_editor_llm_vision_selector(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAnswerAction(action) => {
            state.node_editor_answer_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorEndFieldEdit(edit) => {
            state.edit_node_editor_end_field(edit);
            Task::none()
        }
        WorkflowMessage::NodeEditorTemplateFieldEdit(edit) => {
            state.edit_node_editor_template_field(edit);
            Task::none()
        }
        WorkflowMessage::NodeEditorTemplateAction(action) => {
            state.node_editor_template_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorDocumentSelectorChanged(value) => {
            state.set_node_editor_document_selector(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStructuredFieldEdit(edit) => {
            state.edit_node_editor_structured_field(edit);
            Task::none()
        }
        WorkflowMessage::NodeEditorClassifierAddClass => {
            state.add_node_editor_classifier_class();
            Task::none()
        }
        WorkflowMessage::NodeEditorClassifierRemoveClass(index) => {
            state.remove_node_editor_classifier_class(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorClassifierClassIdChanged(index, value) => {
            state.set_node_editor_classifier_class_id(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorClassifierClassNameChanged(index, value) => {
            state.set_node_editor_classifier_class_name(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorExtractorAddParameter => {
            state.add_node_editor_extractor_parameter();
            Task::none()
        }
        WorkflowMessage::NodeEditorExtractorRemoveParameter(index) => {
            state.remove_node_editor_extractor_parameter(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorHttpRequestEdit(edit) => {
            state.edit_node_editor_http_request(edit);
            Task::none()
        }
        WorkflowMessage::NodeEditorDataNodeEdit(edit) => {
            state.edit_node_editor_data_node(edit);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeLanguageChanged(value) => {
            state.set_node_editor_code_language(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeAddInputVariable => {
            state.add_node_editor_code_input_variable();
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeRemoveInputVariable(index) => {
            state.remove_node_editor_code_input_variable(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeInputVariableNameChanged(index, value) => {
            state.set_node_editor_code_input_variable_name(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeInputVariableSelectorChanged(
            index,
            selector,
            value_type,
        ) => {
            state.set_node_editor_code_input_variable_selector(index, selector, value_type);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeAddOutputVariable => {
            state.add_node_editor_code_output_variable();
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeRemoveOutputVariable(index) => {
            state.remove_node_editor_code_output_variable(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeOutputNameChanged(index, value) => {
            state.set_node_editor_code_output_name(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeOutputTypeChanged(index, value) => {
            state.set_node_editor_code_output_type(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeRetryEnabledChanged(value) => {
            state.set_node_editor_code_retry_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeRetryMaxRetriesChanged(value) => {
            state.set_node_editor_code_retry_max_retries(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeRetryIntervalChanged(value) => {
            state.set_node_editor_code_retry_interval(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeErrorStrategyChanged(value) => {
            state.set_node_editor_code_error_strategy(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeAction(action) => {
            state.node_editor_code_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeDefaultValueAction(action) => {
            state.node_editor_code_default_value_action(action);
            Task::none()
        }
        WorkflowMessage::SubmitNodeEditor => {
            if let Err(error) = state.submit_node_editor() {
                state.set_error(error);
            }
            Task::none()
        }
        _ => return dsl::handle(state, message),
    })
}

fn suggested_new_node_position(state: &WorkflowState, window_size: (f32, f32)) -> Point {
    let zoom = state.zoom.max(0.0001);
    let center_screen = Point::new(window_size.0 * 0.52, window_size.1 * 0.5);
    let offset = (state.document.nodes.len() % 6) as f32 * 28.0;
    Point::new(
        (center_screen.x - state.pan.x) / zoom + offset,
        (center_screen.y - state.pan.y) / zoom + offset,
    )
}

mod start_inputs;

mod dsl;
