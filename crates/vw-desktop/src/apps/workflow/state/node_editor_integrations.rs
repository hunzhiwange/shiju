//! # Workflow 集成节点编辑
//!
//! 该模块处理 tool、agent、llm、answer、code 等节点的集成字段编辑与提交。

use super::*;

impl WorkflowState {
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

    pub fn set_node_editor_code_language(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code { language, code_editor, .. } = draft {
                let normalized = normalize_code_language(&value);
                let previous_template = default_code_template(language);
                let current_code = code_editor.text();
                if current_code.trim().is_empty() || current_code == previous_template {
                    *code_editor =
                        text_editor::Content::with_text(default_code_template(&normalized));
                }
                *language = normalized;
                true
            } else {
                false
            }
        });
    }

    pub fn add_node_editor_code_input_variable(&mut self) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code { inputs, .. } = draft {
                inputs.push(WorkflowCodeVariableDraft {
                    variable: String::new(),
                    value_type: "string".to_string(),
                    selector: Vec::new(),
                });
                true
            } else {
                false
            }
        });
    }

    pub fn remove_node_editor_code_input_variable(&mut self, index: usize) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code { inputs, .. } = draft {
                if index < inputs.len() {
                    inputs.remove(index);
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_code_input_variable_name(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code { inputs, .. } = draft {
                if let Some(input) = inputs.get_mut(index) {
                    input.variable = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_code_input_variable_selector(
        &mut self,
        index: usize,
        selector_key: String,
        value_type: String,
    ) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code { inputs, .. } = draft {
                if let Some(input) = inputs.get_mut(index) {
                    input.selector = parse_code_selector_key(&selector_key);
                    input.value_type = value_type;
                    if input.variable.trim().is_empty() {
                        input.variable = input.selector.last().cloned().unwrap_or_default();
                    }
                    return true;
                }
            }
            false
        });
    }

    pub fn add_node_editor_code_output_variable(&mut self) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code {
                outputs,
                error_strategy,
                default_value_editor,
                ..
            } = draft
            {
                let previous_outputs = outputs.clone();
                outputs.push(WorkflowCodeOutputDraft {
                    key: next_code_output_name(outputs),
                    value_type: "string".to_string(),
                });
                maybe_sync_code_default_value_editor(
                    &previous_outputs,
                    outputs,
                    error_strategy,
                    default_value_editor,
                    false,
                );
                true
            } else {
                false
            }
        });
    }

    pub fn remove_node_editor_code_output_variable(&mut self, index: usize) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code {
                outputs,
                error_strategy,
                default_value_editor,
                ..
            } = draft
            {
                if index < outputs.len() {
                    let previous_outputs = outputs.clone();
                    outputs.remove(index);
                    maybe_sync_code_default_value_editor(
                        &previous_outputs,
                        outputs,
                        error_strategy,
                        default_value_editor,
                        false,
                    );
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_code_output_name(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code {
                outputs,
                error_strategy,
                default_value_editor,
                ..
            } = draft
            {
                let previous_outputs = outputs.clone();
                if index < outputs.len() {
                    outputs[index].key = value;
                    maybe_sync_code_default_value_editor(
                        &previous_outputs,
                        outputs,
                        error_strategy,
                        default_value_editor,
                        false,
                    );
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_code_output_type(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code {
                outputs,
                error_strategy,
                default_value_editor,
                ..
            } = draft
            {
                let previous_outputs = outputs.clone();
                if index < outputs.len() {
                    outputs[index].value_type = value;
                    maybe_sync_code_default_value_editor(
                        &previous_outputs,
                        outputs,
                        error_strategy,
                        default_value_editor,
                        true,
                    );
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_code_retry_enabled(&mut self, value: bool) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code { retry_config, .. } = draft {
                retry_config.enabled = value;
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_code_retry_max_retries(&mut self, value: u8) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code { retry_config, .. } = draft {
                retry_config.max_retries = value.clamp(1, 10);
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_code_retry_interval(&mut self, value: u16) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code { retry_config, .. } = draft {
                retry_config.retry_interval = value.clamp(100, 5000);
                true
            } else {
                false
            }
        });
    }

    pub fn set_node_editor_code_error_strategy(&mut self, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code {
                outputs,
                error_strategy,
                default_value_editor,
                ..
            } = draft
            {
                let normalized = normalize_code_error_strategy(&value);
                let previous_strategy = error_strategy.clone();
                let force_refresh =
                    normalized == "default-value" && previous_strategy != "default-value";
                *error_strategy = normalized;
                maybe_sync_code_default_value_editor(
                    outputs,
                    outputs,
                    error_strategy,
                    default_value_editor,
                    force_refresh,
                );
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_code_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code { code_editor, .. } = draft {
                code_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn node_editor_code_default_value_action(&mut self, action: text_editor::Action) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Code { default_value_editor, .. } = draft {
                default_value_editor.perform(action);
                true
            } else {
                false
            }
        });
    }

    pub fn submit_node_editor(&mut self) -> Result<(), String> {
        if let Some(editor) = self.node_editor.as_mut() {
            refresh_node_editor_validation(editor);
            if editor.validation.has_errors() {
                return Err("请先修正节点表单中的错误字段，再保存。".to_string());
            }
        }

        let Some(editor) = self.node_editor.as_ref() else {
            return Ok(());
        };

        let mode = editor.mode.clone();
        let block_type = editor.block_type.clone();
        let title = if editor.title.trim().is_empty() {
            pretty_block_type(&block_type)
        } else {
            editor.title.trim().to_string()
        };
        let description = editor.description.trim().to_string();
        let position = editor.position;
        let raw_data_yaml = if editor.visual_draft.is_some() {
            apply_visual_draft_to_yaml(
                &editor.block_type,
                &editor.raw_data_editor.text(),
                editor.visual_draft.as_ref(),
            )?
        } else {
            editor.raw_data_editor.text()
        };

        match mode {
            WorkflowNodeEditorMode::Create => {
                self.ensure_start_node_available(&block_type)?;
                let node_id = generate_node_id(&block_type);
                let next_z =
                    self.document.nodes.iter().map(|node| node.z_index).fold(0.0_f32, f32::max)
                        + 1.0;
                let base_node =
                    create_node_from_type(&block_type, node_id.clone(), position, next_z)?;
                let new_node =
                    rebuild_node_from_parts(&base_node, &title, &description, &raw_data_yaml)?;

                self.push_undo_snapshot();
                self.document.nodes.push(new_node);
                self.selected_node_id = Some(node_id);
                self.selected_edge_id = None;
                self.node_editor = None;
                self.refresh_dirty_state();
                self.status_message =
                    Some(format!("已新增 {} 节点", pretty_block_type(&block_type)));
                self.sync_selection_flags();
            }
            WorkflowNodeEditorMode::Edit(node_id) => {
                let index = self
                    .document
                    .nodes
                    .iter()
                    .position(|node| node.id == node_id)
                    .ok_or_else(|| "目标节点不存在".to_string())?;
                let existing = self.document.nodes[index].clone();
                let updated =
                    rebuild_node_from_parts(&existing, &title, &description, &raw_data_yaml)?;

                self.push_undo_snapshot();
                self.document.nodes[index] = updated;
                let removed_edges = self.prune_invalid_edges_for_node_handles(&node_id);

                self.selected_node_id = Some(node_id.clone());
                if removed_edges > 0 {
                    self.status_message = Some(format!(
                        "已更新 {} 节点，并移除 {} 条失效连线",
                        pretty_block_type(&block_type),
                        removed_edges,
                    ));
                } else {
                    self.status_message =
                        Some(format!("已更新 {} 节点", pretty_block_type(&block_type)));
                }
                self.node_editor = None;
                self.refresh_dirty_state();
                self.sync_selection_flags();
            }
        }

        Ok(())
    }
}

fn normalize_code_language(value: &str) -> String {
    if value.trim().eq_ignore_ascii_case("javascript") {
        "javascript".to_string()
    } else {
        "python3".to_string()
    }
}

fn normalize_code_error_strategy(value: &str) -> String {
    match value.trim() {
        "default-value" => "default-value".to_string(),
        "fail-branch" => "fail-branch".to_string(),
        _ => "none".to_string(),
    }
}

fn default_code_template(language: &str) -> &'static str {
    match language {
        "javascript" => "function main() {\n  return {}\n}\n",
        _ => "def main():\n    return {}\n",
    }
}

fn parse_code_selector_key(selector_key: &str) -> Vec<String> {
    selector_key
        .split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect()
}

fn next_code_output_name(outputs: &[WorkflowCodeOutputDraft]) -> String {
    let mut index = 1_usize;
    loop {
        let candidate = if index == 1 { "result".to_string() } else { format!("result_{index}") };
        if !outputs.iter().any(|output| output.key == candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn maybe_sync_code_default_value_editor(
    previous_outputs: &[WorkflowCodeOutputDraft],
    outputs: &[WorkflowCodeOutputDraft],
    error_strategy: &str,
    default_value_editor: &mut text_editor::Content,
    force: bool,
) {
    if error_strategy != "default-value" {
        return;
    }

    let current_text = default_value_editor.text();
    let previous_generated =
        value_yaml_for_editor(&default_code_default_value_value(previous_outputs));
    if force || current_text.trim().is_empty() || current_text.trim() == previous_generated.trim() {
        *default_value_editor = text_editor::Content::with_text(&value_yaml_for_editor(
            &default_code_default_value_value(outputs),
        ));
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
