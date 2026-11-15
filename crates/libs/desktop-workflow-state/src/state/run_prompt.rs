//! 工作流运行参数草稿与文档输入投影。

use super::*;

impl WorkflowState {
    pub fn open_saved_app_run_prompt(&mut self, uuid: String) {
        // 回填上次运行的 query，输入项的值在参数加载完成后再按名称覆盖。
        let query =
            self.last_run_drafts.get(&uuid).map(|draft| draft.query.clone()).unwrap_or_default();
        self.saved_app_run_prompt = Some(WorkflowRunPromptDraft {
            uuid,
            query,
            loading: true,
            inputs: Vec::new(),
            error: None,
        });
        self.saved_app_actions_menu_uuid = None;
        self.saved_apps_error = None;
    }

    pub fn close_saved_app_run_prompt(&mut self) {
        self.saved_app_run_prompt = None;
    }

    pub fn set_saved_app_run_query(&mut self, query: String) {
        if let Some(prompt) = self.saved_app_run_prompt.as_mut() {
            prompt.query = query;
            prompt.error = None;
        }
    }

    pub fn finish_saved_app_run_prompt_load(
        &mut self,
        uuid: &str,
        result: Result<Vec<WorkflowRunInputDraft>, String>,
    ) {
        let Some(prompt) = self.saved_app_run_prompt.as_mut() else {
            return;
        };
        if prompt.uuid != uuid {
            return;
        }

        prompt.loading = false;
        match result {
            Ok(mut inputs) => {
                // 结构以文档为准，但用上次运行缓存的同名输入值回填，方便重复测试。
                if let Some(cached) = self.last_run_drafts.get(uuid) {
                    for input in inputs.iter_mut() {
                        if let Some(remembered) = cached.inputs.iter().find(|cached_input| {
                            cached_input.name == input.name
                                && cached_input.input_type == input.input_type
                        }) {
                            input.value = remembered.value.clone();
                        }
                    }
                }
                prompt.inputs = inputs;
                prompt.error = None;
            }
            Err(error) => {
                prompt.inputs.clear();
                prompt.error = Some(error.into());
            }
        }
    }

    pub fn set_saved_app_run_input_value(&mut self, index: usize, value: String) {
        if let Some(prompt) = self.saved_app_run_prompt.as_mut() {
            if let Some(input) = prompt.inputs.get_mut(index) {
                input.value = value;
            }
            prompt.error = None;
        }
    }

    pub fn toggle_saved_app_run_input_value(&mut self, index: usize, value: bool) {
        if let Some(prompt) = self.saved_app_run_prompt.as_mut() {
            if let Some(input) = prompt.inputs.get_mut(index) {
                input.value = value.to_string();
            }
            prompt.error = None;
        }
    }

    pub fn set_saved_app_run_prompt_error(
        &mut self,
        error: impl Into<desktop_i18n::workflow::Feedback>,
    ) {
        if let Some(prompt) = self.saved_app_run_prompt.as_mut() {
            prompt.error = Some(error.into());
        }
    }

    /// 缓存本次运行填写的参数草稿，供下次打开同一应用的运行弹窗时回填。
    pub fn remember_saved_app_run_draft(&mut self, prompt: WorkflowRunPromptDraft) {
        let mut draft = prompt;
        draft.loading = false;
        draft.error = None;
        self.last_run_drafts.insert(draft.uuid.clone(), draft);
    }

}

#[cfg(not(target_arch = "wasm32"))]
pub fn workflow_run_inputs_from_document(
    document: &WorkflowDocument,
) -> Vec<WorkflowRunInputDraft> {
    document
        .nodes
        .iter()
        .filter(|node| node.block_type == "start")
        .flat_map(workflow_run_inputs_from_start_node)
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn workflow_run_inputs_from_start_node(node: &WorkflowNode) -> Vec<WorkflowRunInputDraft> {
    let Some(variables) = node
        .raw_node
        .as_mapping()
        .and_then(|node| node.get(&Value::String("data".to_string())))
        .and_then(Value::as_mapping)
        .and_then(|data| data.get(&Value::String("variables".to_string())))
        .and_then(Value::as_sequence)
    else {
        return Vec::new();
    };

    variables
        .iter()
        .map(super::start_variable_drafts::build_start_variable_draft)
        .filter(|variable| !variable.variable.trim().is_empty())
        .map(workflow_run_input_from_start_variable)
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn workflow_run_input_from_start_variable(
    variable: WorkflowStartVariableDraft,
) -> WorkflowRunInputDraft {
    let value = if matches!(variable.input_type.as_str(), "file" | "file-list") {
        variable.default_file_values.join("\n")
    } else {
        variable.default_value.clone()
    };

    WorkflowRunInputDraft {
        name: variable.variable.trim().to_string(),
        label: if variable.label.trim().is_empty() {
            variable.variable.trim().to_string()
        } else {
            variable.label.trim().to_string()
        },
        input_type: variable.input_type,
        required: variable.required,
        hidden: variable.hidden,
        options: variable.options,
        value,
        placeholder: variable.placeholder,
        hint: variable.hint,
        max_length_input: variable.max_length_input,
    }
}
