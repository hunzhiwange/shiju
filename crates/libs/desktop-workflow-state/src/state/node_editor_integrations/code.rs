//! Code node fields and generated defaults.

use super::*;

impl WorkflowState {
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
