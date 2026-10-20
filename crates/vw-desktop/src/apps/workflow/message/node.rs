//! 工作流节点消息处理，负责节点属性、连接和运行配置更新。

use super::*;
use crate::app::components::text_editor_context_menu::{
    focus_editor_task, paste_action, paste_task, selection_copy_task, selection_cut_task,
    selection_delete_task,
};

/// 构建或更新 handle 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub(super) fn handle(app: &mut App, message: WorkflowMessage) -> Option<Task<Message>> {
    Some(match message {
        WorkflowMessage::ToggleQuickInsertPanel => {
            app.workflow_state.toggle_quick_insert_panel();
            Task::none()
        }
        WorkflowMessage::InsertSuggestedNode(block_type) => {
            let position = suggested_new_node_position(app);
            if let Err(error) = app.workflow_state.insert_node_immediately(&block_type, position) {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::OpenCreateNodeEditor(block_type) => {
            let position = suggested_new_node_position(app);
            if let Err(error) = app.workflow_state.open_create_node_editor(&block_type, position) {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::OpenCreateNodeEditorAt(block_type, position) => {
            if let Err(error) = app.workflow_state.open_create_node_editor(&block_type, position) {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::CreateContextNode(block_type) => {
            if let Err(error) = app.workflow_state.create_context_node(&block_type) {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::OpenDownstreamNodePicker(node_id) => {
            if let Err(error) = app.workflow_state.open_downstream_node_picker(&node_id) {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::OpenEditNodeEditor(id) => {
            if let Err(error) = app.workflow_state.open_edit_node_editor(id.as_deref()) {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::CloseNodeEditor => {
            app.workflow_state.close_node_editor();
            Task::none()
        }
        WorkflowMessage::NodeEditorTabSelected(tab) => {
            app.workflow_state.set_node_editor_active_tab(tab);
            Task::none()
        }
        WorkflowMessage::NodeEditorTitleChanged(value) => {
            app.workflow_state.set_node_editor_title(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorDescriptionChanged(value) => {
            app.workflow_state.set_node_editor_description(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorDescriptionAction(action) => {
            app.workflow_state.node_editor_description_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorScrollCaptured => Task::none(),
        WorkflowMessage::NodeEditorStartAddVariable => {
            app.workflow_state.open_node_editor_start_variable_create();
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableHovered(index) => {
            app.workflow_state.set_node_editor_start_variable_hovered(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartRemoveVariable(index) => {
            app.workflow_state.remove_node_editor_start_variable(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartSelectVariable(index) => {
            app.workflow_state.open_node_editor_start_variable_edit(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartCloseVariableEditor => {
            app.workflow_state.close_node_editor_start_variable_editor();
            Task::none()
        }
        WorkflowMessage::NodeEditorStartSubmitVariableEditor => {
            if let Err(error) = app.workflow_state.submit_node_editor_start_variable_editor() {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorLabelChanged(value) => {
            app.workflow_state.set_node_editor_start_variable_editor_label(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorNameChanged(value) => {
            app.workflow_state.set_node_editor_start_variable_editor_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorTypeChanged(value) => {
            app.workflow_state.set_node_editor_start_variable_editor_type(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorRequiredChanged(value) => {
            app.workflow_state.set_node_editor_start_variable_editor_required(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorHiddenChanged(value) => {
            app.workflow_state.set_node_editor_start_variable_editor_hidden(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorDefaultChanged(value) => {
            app.workflow_state.set_node_editor_start_variable_editor_default(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorDefaultAction(action) => {
            app.workflow_state.node_editor_start_variable_editor_default_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorMaxLengthChanged(value) => {
            app.workflow_state.set_node_editor_start_variable_editor_max_length(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorAddOption => {
            app.workflow_state.add_node_editor_start_variable_editor_option();
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorRemoveOption(index) => {
            app.workflow_state.remove_node_editor_start_variable_editor_option(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorOptionChanged(index, value) => {
            app.workflow_state.set_node_editor_start_variable_editor_option(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorToggleFileType(value) => {
            app.workflow_state.toggle_node_editor_start_variable_editor_file_type(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorExtensionsChanged(value) => {
            app.workflow_state.set_node_editor_start_variable_editor_extensions(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorUploadMethodChanged(value) => {
            app.workflow_state.set_node_editor_start_variable_editor_upload_method(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFile => {
            #[cfg(target_arch = "wasm32")]
            {
                Task::perform(
                    async { Err("Web 平台暂不支持选择默认文件".to_string()) },
                    |res| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFileFinished(
                                res,
                            ),
                        )
                    },
                )
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new().pick_file().await;
                        Ok(file.map(|handle| handle.path().to_string_lossy().to_string()))
                    },
                    |res| {
                        Message::WorkflowTool(
                            WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFileFinished(
                                res,
                            ),
                        )
                    },
                )
            }
        }
        WorkflowMessage::NodeEditorStartVariableEditorPickDefaultFileFinished(result) => {
            match result {
                Ok(Some(path)) => {
                    if let Err(error) = app
                        .workflow_state
                        .set_node_editor_start_variable_editor_default_file_path(path)
                    {
                        app.workflow_state.set_error(error);
                    }
                }
                Ok(None) => {}
                Err(error) => app.workflow_state.set_error(error),
            }
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorRemoveDefaultFile(index) => {
            app.workflow_state.remove_node_editor_start_variable_editor_default_file(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorOpenDefaultFileUrlInput => {
            app.workflow_state.open_node_editor_start_variable_editor_default_file_url_input();
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorCloseDefaultFileUrlInput => {
            app.workflow_state.close_node_editor_start_variable_editor_default_file_url_input();
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorDefaultFileUrlChanged(value) => {
            app.workflow_state.set_node_editor_start_variable_editor_default_file_url_input(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableEditorSubmitDefaultFileUrl => {
            if let Err(error) =
                app.workflow_state.submit_node_editor_start_variable_editor_default_file_url()
            {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableLabelChanged(index, value) => {
            app.workflow_state.set_node_editor_start_variable_label(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableNameChanged(index, value) => {
            app.workflow_state.set_node_editor_start_variable_name(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableTypeChanged(index, value) => {
            app.workflow_state.set_node_editor_start_variable_type(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableRequiredChanged(index, value) => {
            app.workflow_state.set_node_editor_start_variable_required(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableDefaultChanged(index, value) => {
            app.workflow_state.set_node_editor_start_variable_default(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariablePlaceholderChanged(index, value) => {
            app.workflow_state.set_node_editor_start_variable_placeholder(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableHintChanged(index, value) => {
            app.workflow_state.set_node_editor_start_variable_hint(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStartVariableMaxLengthChanged(index, value) => {
            app.workflow_state.set_node_editor_start_variable_max_length(index, value);
            Task::none()
        }
        WorkflowMessage::InsertDownstreamNode(source_node_id, block_type) => {
            if let Err(error) =
                app.workflow_state.insert_downstream_node(&source_node_id, &block_type)
            {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::InsertDownstreamNodeFromHandle(
            source_node_id,
            source_handle_id,
            block_type,
        ) => {
            if let Err(error) = app.workflow_state.insert_downstream_node_from_handle(
                &source_node_id,
                &source_handle_id,
                &block_type,
            ) {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::NodeEditorShowRawDataEditorChanged(value) => {
            app.workflow_state.set_node_editor_show_raw_data_editor(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseAddCase => {
            app.workflow_state.add_node_editor_if_else_case();
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseCaseLogicalOperatorChanged(index, value) => {
            app.workflow_state.set_node_editor_if_else_case_logical_operator(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseAddCondition(case_index) => {
            app.workflow_state.add_node_editor_if_else_condition(case_index);
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseRemoveCondition(case_index, condition_index) => {
            app.workflow_state.remove_node_editor_if_else_condition(case_index, condition_index);
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseConditionSelectorChanged(
            case_index,
            condition_index,
            value,
        ) => {
            app.workflow_state.set_node_editor_if_else_condition_selector(
                case_index,
                condition_index,
                value,
            );
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseConditionOperatorChanged(
            case_index,
            condition_index,
            value,
        ) => {
            app.workflow_state.set_node_editor_if_else_condition_operator(
                case_index,
                condition_index,
                value,
            );
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseConditionValueChanged(
            case_index,
            condition_index,
            value,
        ) => {
            app.workflow_state.set_node_editor_if_else_condition_value(
                case_index,
                condition_index,
                value,
            );
            Task::none()
        }
        WorkflowMessage::NodeEditorIfElseConditionVarTypeChanged(
            case_index,
            condition_index,
            value,
        ) => {
            app.workflow_state.set_node_editor_if_else_condition_var_type(
                case_index,
                condition_index,
                value,
            );
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeQuerySelectorChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_query_selector(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeQueryAttachmentSelectorChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_query_attachment_selector(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeDatasetIdsChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_dataset_ids(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeRetrievalModeChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_retrieval_mode(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeTopKChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_top_k(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeScoreThresholdEnabledChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_score_threshold_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeScoreThresholdChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_score_threshold(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeRerankingEnabledChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_reranking_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeRerankingModelAction(action) => {
            app.workflow_state.node_editor_knowledge_reranking_model_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeMetadataFilteringModeChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_metadata_filtering_mode(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeMetadataModelConfigAction(action) => {
            app.workflow_state.node_editor_knowledge_metadata_model_config_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeSingleModelProviderChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_single_model_provider(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeSingleModelNameChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_single_model_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorKnowledgeSingleModelModeChanged(value) => {
            app.workflow_state.set_node_editor_knowledge_single_model_mode(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolProviderIdChanged(value) => {
            app.workflow_state.set_node_editor_tool_provider_id(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolProviderTypeChanged(value) => {
            app.workflow_state.set_node_editor_tool_provider_type(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolProviderNameChanged(value) => {
            app.workflow_state.set_node_editor_tool_provider_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolNameChanged(value) => {
            app.workflow_state.set_node_editor_tool_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolLabelChanged(value) => {
            app.workflow_state.set_node_editor_tool_label(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolDescriptionChanged(value) => {
            app.workflow_state.set_node_editor_tool_description(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolCredentialIdChanged(value) => {
            app.workflow_state.set_node_editor_tool_credential_id(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolPluginUniqueIdentifierChanged(value) => {
            app.workflow_state.set_node_editor_tool_plugin_unique_identifier(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolParametersAction(action) => {
            app.workflow_state.node_editor_tool_parameters_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorToolConfigurationsAction(action) => {
            app.workflow_state.node_editor_tool_configurations_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentStrategyProviderChanged(value) => {
            app.workflow_state.set_node_editor_agent_strategy_provider(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentStrategyNameChanged(value) => {
            app.workflow_state.set_node_editor_agent_strategy_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentStrategyLabelChanged(value) => {
            app.workflow_state.set_node_editor_agent_strategy_label(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentPluginUniqueIdentifierChanged(value) => {
            app.workflow_state.set_node_editor_agent_plugin_unique_identifier(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentOutputSchemaAction(action) => {
            app.workflow_state.node_editor_agent_output_schema_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentParametersAction(action) => {
            app.workflow_state.node_editor_agent_parameters_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentMemoryEnabledChanged(value) => {
            app.workflow_state.set_node_editor_agent_memory_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentMemoryWindowSizeChanged(value) => {
            app.workflow_state.set_node_editor_agent_memory_window_size(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAgentMemoryPromptAction(action) => {
            app.workflow_state.node_editor_agent_memory_prompt_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmProviderChanged(value) => {
            app.workflow_state.set_node_editor_llm_provider(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmModelNameChanged(value) => {
            app.workflow_state.set_node_editor_llm_model_name(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmModelModeChanged(value) => {
            app.workflow_state.set_node_editor_llm_model_mode(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmEnableThinkingChanged(value) => {
            app.workflow_state.set_node_editor_llm_enable_thinking(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmCompletionParamsAction(action) => {
            app.workflow_state.node_editor_llm_completion_params_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmStructuredOutputAction(action) => {
            app.workflow_state.node_editor_llm_structured_output_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmMemoryAction(action) => {
            app.workflow_state.node_editor_llm_memory_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmContextEnabledChanged(value) => {
            app.workflow_state.set_node_editor_llm_context_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmContextSelectorChanged(value) => {
            app.workflow_state.set_node_editor_llm_context_selector_input(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmSystemPromptAction(action) => {
            app.workflow_state.node_editor_llm_system_prompt_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmUserPromptAction(action) => {
            app.workflow_state.node_editor_llm_user_prompt_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmVisionEnabledChanged(value) => {
            app.workflow_state.set_node_editor_llm_vision_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorLlmVisionSelectorChanged(value) => {
            app.workflow_state.set_node_editor_llm_vision_selector(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorAnswerAction(action) => {
            app.workflow_state.node_editor_answer_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorEndFieldEdit(edit) => {
            app.workflow_state.edit_node_editor_end_field(edit);
            Task::none()
        }
        WorkflowMessage::NodeEditorTemplateFieldEdit(edit) => {
            app.workflow_state.edit_node_editor_template_field(edit);
            Task::none()
        }
        WorkflowMessage::NodeEditorTemplateAction(action) => {
            app.workflow_state.node_editor_template_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorDocumentSelectorChanged(value) => {
            app.workflow_state.set_node_editor_document_selector(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorStructuredFieldEdit(edit) => {
            app.workflow_state.edit_node_editor_structured_field(edit);
            Task::none()
        }
        WorkflowMessage::NodeEditorClassifierAddClass => {
            app.workflow_state.add_node_editor_classifier_class();
            Task::none()
        }
        WorkflowMessage::NodeEditorClassifierRemoveClass(index) => {
            app.workflow_state.remove_node_editor_classifier_class(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorClassifierClassIdChanged(index, value) => {
            app.workflow_state.set_node_editor_classifier_class_id(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorClassifierClassNameChanged(index, value) => {
            app.workflow_state.set_node_editor_classifier_class_name(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorExtractorAddParameter => {
            app.workflow_state.add_node_editor_extractor_parameter();
            Task::none()
        }
        WorkflowMessage::NodeEditorExtractorRemoveParameter(index) => {
            app.workflow_state.remove_node_editor_extractor_parameter(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorHttpRequestEdit(edit) => {
            app.workflow_state.edit_node_editor_http_request(edit);
            Task::none()
        }
        WorkflowMessage::NodeEditorDataNodeEdit(edit) => {
            app.workflow_state.edit_node_editor_data_node(edit);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeLanguageChanged(value) => {
            app.workflow_state.set_node_editor_code_language(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeAddInputVariable => {
            app.workflow_state.add_node_editor_code_input_variable();
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeRemoveInputVariable(index) => {
            app.workflow_state.remove_node_editor_code_input_variable(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeInputVariableNameChanged(index, value) => {
            app.workflow_state.set_node_editor_code_input_variable_name(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeInputVariableSelectorChanged(
            index,
            selector,
            value_type,
        ) => {
            app.workflow_state
                .set_node_editor_code_input_variable_selector(index, selector, value_type);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeAddOutputVariable => {
            app.workflow_state.add_node_editor_code_output_variable();
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeRemoveOutputVariable(index) => {
            app.workflow_state.remove_node_editor_code_output_variable(index);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeOutputNameChanged(index, value) => {
            app.workflow_state.set_node_editor_code_output_name(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeOutputTypeChanged(index, value) => {
            app.workflow_state.set_node_editor_code_output_type(index, value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeRetryEnabledChanged(value) => {
            app.workflow_state.set_node_editor_code_retry_enabled(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeRetryMaxRetriesChanged(value) => {
            app.workflow_state.set_node_editor_code_retry_max_retries(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeRetryIntervalChanged(value) => {
            app.workflow_state.set_node_editor_code_retry_interval(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeErrorStrategyChanged(value) => {
            app.workflow_state.set_node_editor_code_error_strategy(value);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeAction(action) => {
            app.workflow_state.node_editor_code_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorCodeDefaultValueAction(action) => {
            app.workflow_state.node_editor_code_default_value_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorDataAction(action) => {
            close_node_editor_dsl_context_menu(app);
            app.workflow_state.node_editor_action(action);
            Task::none()
        }
        WorkflowMessage::NodeEditorDslOpenContextMenu { x, y } => {
            app.workflow_state.node_editor_dsl_context_menu_open = true;
            app.workflow_state.node_editor_dsl_context_menu_pos = Some((x, y));
            Task::none()
        }
        WorkflowMessage::NodeEditorDslCloseContextMenu => {
            close_node_editor_dsl_context_menu(app);
            focus_editor_task(&node_editor_dsl_editor_id())
        }
        WorkflowMessage::NodeEditorDslUndo => {
            close_node_editor_dsl_context_menu(app);
            app.workflow_state.undo_node_editor_dsl();
            focus_editor_task(&node_editor_dsl_editor_id())
        }
        WorkflowMessage::NodeEditorDslRedo => {
            close_node_editor_dsl_context_menu(app);
            app.workflow_state.redo_node_editor_dsl();
            focus_editor_task(&node_editor_dsl_editor_id())
        }
        WorkflowMessage::NodeEditorDslContextMenuCopy => {
            close_node_editor_dsl_context_menu(app);
            app.workflow_state
                .node_editor
                .as_ref()
                .map(|editor| {
                    selection_copy_task(&editor.raw_data_editor, &node_editor_dsl_editor_id()).1
                })
                .unwrap_or_else(Task::none)
        }
        WorkflowMessage::NodeEditorDslContextMenuCut => {
            close_node_editor_dsl_context_menu(app);
            let previous = app
                .workflow_state
                .node_editor
                .as_ref()
                .map(|editor| editor.raw_data_editor.text())
                .unwrap_or_default();
            let task = app
                .workflow_state
                .node_editor
                .as_mut()
                .map(|editor| {
                    selection_cut_task(&mut editor.raw_data_editor, &node_editor_dsl_editor_id()).1
                })
                .unwrap_or_else(Task::none);
            app.workflow_state.finish_node_editor_raw_edit(previous);
            task
        }
        WorkflowMessage::NodeEditorDslContextMenuPaste => {
            close_node_editor_dsl_context_menu(app);
            paste_task(&node_editor_dsl_editor_id(), |content| {
                Message::WorkflowTool(WorkflowMessage::NodeEditorDataAction(paste_action(content)))
            })
        }
        WorkflowMessage::NodeEditorDslContextMenuDelete => {
            close_node_editor_dsl_context_menu(app);
            let previous = app
                .workflow_state
                .node_editor
                .as_ref()
                .map(|editor| editor.raw_data_editor.text())
                .unwrap_or_default();
            let task = app
                .workflow_state
                .node_editor
                .as_mut()
                .map(|editor| {
                    selection_delete_task(&mut editor.raw_data_editor, &node_editor_dsl_editor_id())
                        .1
                })
                .unwrap_or_else(Task::none);
            app.workflow_state.finish_node_editor_raw_edit(previous);
            task
        }
        WorkflowMessage::SubmitNodeEditor => {
            if let Err(error) = app.workflow_state.submit_node_editor() {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        _ => return None,
    })
}

fn node_editor_dsl_editor_id() -> iced::widget::Id {
    iced::widget::Id::new(crate::apps::workflow::NODE_EDITOR_DSL_EDITOR_ID)
}

fn close_node_editor_dsl_context_menu(app: &mut App) {
    app.workflow_state.node_editor_dsl_context_menu_open = false;
    app.workflow_state.node_editor_dsl_context_menu_pos = None;
}
