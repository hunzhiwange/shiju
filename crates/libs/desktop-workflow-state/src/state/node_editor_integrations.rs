//! # Workflow 集成节点编辑
//!
//! 该模块处理集成字段编辑，数据节点、代码节点及提交由子模块分别负责。

use super::*;

mod code;
mod data;
mod submit;

impl WorkflowState {
    pub fn edit_node_editor_http_request(&mut self, edit: WorkflowHttpRequestEdit) {
        self.update_node_visual_draft(|draft| {
            let WorkflowNodeVisualDraft::HttpRequest {
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
            } = draft
            else {
                return false;
            };
            match edit {
                WorkflowHttpRequestEdit::Method(v) => *method = v,
                WorkflowHttpRequestEdit::Url(v) => *url = v,
                WorkflowHttpRequestEdit::AuthType(v) => *auth_type = v,
                WorkflowHttpRequestEdit::AuthUsername(v) => *auth_username = v,
                WorkflowHttpRequestEdit::AuthPassword(v) => *auth_password = v,
                WorkflowHttpRequestEdit::AuthToken(v) => *auth_token = v,
                WorkflowHttpRequestEdit::Headers(a) => headers_editor.perform(a),
                WorkflowHttpRequestEdit::Params(a) => params_editor.perform(a),
                WorkflowHttpRequestEdit::BodyType(v) => *body_type = v,
                WorkflowHttpRequestEdit::Body(a) => body_editor.perform(a),
                WorkflowHttpRequestEdit::TimeoutConnect(v) => *timeout_connect = v,
                WorkflowHttpRequestEdit::TimeoutRead(v) => *timeout_read = v,
                WorkflowHttpRequestEdit::TimeoutWrite(v) => *timeout_write = v,
                WorkflowHttpRequestEdit::SslVerify(v) => *ssl_verify = v,
                WorkflowHttpRequestEdit::RetryEnabled(v) => retry_config.enabled = v,
                WorkflowHttpRequestEdit::RetryMaxRetries(v) => retry_config.max_retries = v,
                WorkflowHttpRequestEdit::RetryInterval(v) => retry_config.retry_interval = v,
                WorkflowHttpRequestEdit::ErrorStrategy(v) => *error_strategy = v,
            }
            true
        });
    }

    pub fn set_node_editor_tool_provider_id(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Tool { provider_id, .. } = draft {
                *provider_id = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_tool_provider_type(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Tool { provider_type, .. } = draft {
                *provider_type = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_tool_provider_name(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Tool { provider_name, .. } = draft {
                *provider_name = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_tool_name(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Tool { tool_name, .. } = draft {
                *tool_name = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_tool_label(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Tool { tool_label, .. } = draft {
                *tool_label = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_tool_description(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Tool { tool_description, .. } = draft {
                *tool_description = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_tool_credential_id(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Tool { credential_id, .. } = draft {
                *credential_id = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_tool_plugin_unique_identifier(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Tool { plugin_unique_identifier, .. } = draft {
                *plugin_unique_identifier = value;
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_tool_parameters_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Tool { tool_parameters_editor, .. } = draft {
                tool_parameters_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_tool_configurations_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Tool { tool_configurations_editor, .. } = draft {
                tool_configurations_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_agent_strategy_provider(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Agent { strategy_provider_name, .. } = draft {
                *strategy_provider_name = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_agent_strategy_name(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Agent { strategy_name, .. } = draft {
                *strategy_name = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_agent_strategy_label(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Agent { strategy_label, .. } = draft {
                *strategy_label = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_agent_plugin_unique_identifier(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Agent { plugin_unique_identifier, .. } = draft {
                *plugin_unique_identifier = value;
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_agent_output_schema_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Agent { output_schema_editor, .. } = draft {
                output_schema_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_agent_parameters_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Agent { parameters_editor, .. } = draft {
                parameters_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_agent_memory_enabled(&mut self, value: bool) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Agent { memory_enabled, .. } = draft {
                *memory_enabled = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_agent_memory_window_size(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Agent { memory_window_size_input, .. } = draft {
                *memory_window_size_input = value;
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_agent_memory_prompt_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Agent { memory_prompt_editor, .. } = draft {
                memory_prompt_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_llm_provider(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { provider, .. } = draft {
                *provider = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_llm_model_name(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { model_name, .. } = draft {
                *model_name = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_llm_model_mode(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { model_mode, .. } = draft {
                *model_mode = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_llm_enable_thinking(&mut self, value: bool) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { enable_thinking, .. } = draft {
                *enable_thinking = value;
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_llm_completion_params_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { completion_params_editor, .. } = draft {
                completion_params_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_llm_structured_output_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { structured_output_editor, .. } = draft {
                structured_output_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_llm_memory_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { memory_editor, .. } = draft {
                memory_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_llm_context_enabled(&mut self, value: bool) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { context_enabled, .. } = draft {
                *context_enabled = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_llm_context_selector_input(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { context_selector_input, .. } = draft {
                *context_selector_input = value;
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_llm_system_prompt_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { system_prompt_editor, .. } = draft {
                system_prompt_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_llm_user_prompt_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { user_prompt_editor, .. } = draft {
                user_prompt_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_llm_vision_enabled(&mut self, value: bool) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { vision_enabled, .. } = draft {
                *vision_enabled = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_llm_vision_selector(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Llm { vision_selector_input, .. } = draft {
                *vision_selector_input = value;
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_answer_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Answer { answer_editor } = draft {
                answer_editor.perform(action);
                true
            } else {
                false
            }
        });
    }
}


impl WorkflowState {
    pub fn edit_node_editor_end_field(&mut self, edit: WorkflowNodeFieldEdit) {
        self.update_node_visual_draft(|draft| match draft {
            WorkflowNodeVisualDraft::End { outputs } => apply_named_value_edit(outputs, edit),
            _ => false,
        });
    }

    pub fn edit_node_editor_template_field(&mut self, edit: WorkflowNodeFieldEdit) {
        self.update_node_visual_draft(|draft| match draft {
            WorkflowNodeVisualDraft::TemplateTransform { variables, .. } => {
                apply_named_value_edit(variables, edit)
            }
            _ => false,
        });
    }

    pub fn node_editor_template_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| match draft {
            WorkflowNodeVisualDraft::TemplateTransform { template_editor, .. } => {
                template_editor.perform(action);
                true
            }
            _ => false,
        });
    }

    pub fn set_node_editor_document_selector(&mut self, value: String) {
        self.update_node_visual_draft(|draft| match draft {
            WorkflowNodeVisualDraft::DocumentExtractor { variable_selector } => {
                *variable_selector = WorkflowVariableSelectorDraft::from_input(&value);
                true
            }
            _ => false,
        });
    }
}

fn apply_named_value_edit(
    values: &mut Vec<WorkflowNamedValueDraft>,
    edit: WorkflowNodeFieldEdit,
) -> bool {
    match edit {
        WorkflowNodeFieldEdit::NamedValueAdded => {
            values.push(WorkflowNamedValueDraft {
                value_type: "string".to_string(),
                ..WorkflowNamedValueDraft::default()
            });
            true
        }
        WorkflowNodeFieldEdit::NamedValueRemoved(index) if index < values.len() => {
            values.remove(index);
            true
        }
        WorkflowNodeFieldEdit::NamedValueNameChanged(index, value) => {
            values.get_mut(index).map(|item| item.name = value).is_some()
        }
        WorkflowNodeFieldEdit::NamedValueTypeChanged(index, value) => {
            values.get_mut(index).map(|item| item.value_type = value).is_some()
        }
        WorkflowNodeFieldEdit::NamedValueSelectorChanged(index, value) => values
            .get_mut(index)
            .map(|item| item.selector = WorkflowVariableSelectorDraft::from_input(&value))
            .is_some(),
        _ => false,
    }
}
