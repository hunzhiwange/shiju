//! # Workflow 开始节点编辑
//!
//! 该模块处理节点编辑器基础行为，以及开始节点变量子编辑器的打开、修改、提交与删除。

use super::*;

const NODE_EDITOR_DSL_HISTORY_LIMIT: usize = 100;

fn build_start_variable_editor_draft(
    mode: WorkflowStartVariableEditorMode,
    variable: WorkflowStartVariableDraft,
) -> WorkflowStartVariableEditorDraft {
    WorkflowStartVariableEditorDraft {
        default_value_editor: text_editor::Content::with_text(&variable.default_value),
        default_file_url_input: variable.default_value.clone(),
        mode,
        show_default_file_url_input: false,
        variable,
    }
}

impl WorkflowState {
    pub(super) fn append_node_editor_start_variable_editor_default_file(
        &mut self,
        value: String,
    ) -> Result<(), String> {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            return Ok(());
        }

        let mut result = Ok(());
        self.update_node_editor_start_variable_editor(|editor| {
            match editor.variable.input_type.as_str() {
                "file-list" => {
                    let max_count = usize::from(normalized_start_variable_file_list_max_length(
                        &editor.variable.max_length_input,
                    ));
                    if editor.variable.default_file_values.len() >= max_count {
                        result = Err(format!("默认文件最多只能添加 {} 个", max_count));
                        return;
                    }
                    editor.variable.default_file_values.push(trimmed.clone());
                    normalize_start_variable_draft(&mut editor.variable);
                    editor.default_value_editor =
                        text_editor::Content::with_text(&editor.variable.default_value);
                    editor.default_file_url_input.clear();
                    editor.show_default_file_url_input = false;
                }
                "file" => {
                    editor.variable.default_file_values = vec![trimmed.clone()];
                    normalize_start_variable_draft(&mut editor.variable);
                    editor.default_value_editor =
                        text_editor::Content::with_text(&editor.variable.default_value);
                    editor.default_file_url_input = editor.variable.default_value.clone();
                    editor.show_default_file_url_input = false;
                }
                _ => {
                    editor.variable.default_value = trimmed.clone();
                }
            }
        });
        result
    }

    pub fn set_node_editor_start_variable_hovered(&mut self, index: Option<usize>) {
        if let Some(editor) = self.node_editor.as_mut() {
            editor.hovered_start_variable_index = index;
        }
    }

    pub fn set_node_editor_title(&mut self, value: String) {
        if let Some(editor) = self.node_editor.as_mut() {
            editor.title = value;
            clear_node_editor_validation(editor);
        }
    }

    pub fn set_node_editor_description(&mut self, value: String) {
        if let Some(editor) = self.node_editor.as_mut() {
            editor.description = value;
            editor.description_editor = text_editor::Content::with_text(&editor.description);
            clear_node_editor_validation(editor);
        }
    }

    pub fn node_editor_description_action(&mut self, action: text_editor::Action) {
        if let Some(editor) = self.node_editor.as_mut() {
            editor.description_editor.perform(action);
            editor.description = editor.description_editor.text();
            clear_node_editor_validation(editor);
        }
    }

    pub fn delete_edge_by_id(&mut self, edge_id: &str) -> bool {
        self.selected_edge_id = Some(edge_id.to_string());
        self.selected_node_id = None;
        self.context_menu = None;
        self.delete_selected_edge()
    }

    pub fn delete_node_by_id(&mut self, node_id: &str) -> bool {
        self.selected_node_id = Some(node_id.to_string());
        self.selected_edge_id = None;
        self.context_menu = None;
        self.delete_selected_node()
    }

    pub fn focus_node(&mut self, id: &str, window_size: (f32, f32)) -> Result<(), String> {
        let node = self.document.node(id).cloned().ok_or_else(|| "目标节点不存在".to_string())?;

        let usable_width = (window_size.0 - 380.0).max(320.0);
        let usable_height = (window_size.1 - 220.0).max(260.0);
        let screen_center = Vector::new(usable_width / 2.0 + 24.0, usable_height / 2.0 + 24.0);
        let world_center = Point::new(
            node.position.x + node.size.width / 2.0,
            node.position.y + node.size.height / 2.0,
        );

        self.pan =
            screen_center - Vector::new(world_center.x * self.zoom, world_center.y * self.zoom);
        self.selected_node_id = Some(node.id.clone());
        self.selected_edge_id = None;
        self.node_editor = None;
        self.context_menu = None;
        self.connection_draft = None;
        self.status_message = Some(format!("已定位到节点 {}", node.title));
        self.sync_selection_flags();

        Ok(())
    }

    pub fn node_editor_action(&mut self, action: text_editor::Action) {
        if let Some(editor) = self.node_editor.as_mut() {
            let previous = editor.raw_data_editor.text();
            let records_history = action.is_edit();
            editor.raw_data_editor.perform(action);
            let changed = records_history && editor.raw_data_editor.text() != previous;
            refresh_node_editor_from_raw(editor);
            if changed {
                push_node_editor_dsl_history(&mut self.node_editor_dsl_undo_stack, previous);
                self.node_editor_dsl_redo_stack.clear();
            }
        }
    }

    pub fn finish_node_editor_raw_edit(&mut self, previous: String) {
        if let Some(editor) = self.node_editor.as_mut() {
            let changed = editor.raw_data_editor.text() != previous;
            refresh_node_editor_from_raw(editor);
            if changed {
                push_node_editor_dsl_history(&mut self.node_editor_dsl_undo_stack, previous);
                self.node_editor_dsl_redo_stack.clear();
            }
        }
    }

    pub fn undo_node_editor_dsl(&mut self) {
        let Some(previous) = self.node_editor_dsl_undo_stack.pop() else {
            return;
        };
        if let Some(editor) = self.node_editor.as_mut() {
            let current = editor.raw_data_editor.text();
            push_node_editor_dsl_history(&mut self.node_editor_dsl_redo_stack, current);
            editor.raw_data_editor = text_editor::Content::with_text(&previous);
            refresh_node_editor_from_raw(editor);
        }
    }

    pub fn redo_node_editor_dsl(&mut self) {
        let Some(next) = self.node_editor_dsl_redo_stack.pop() else {
            return;
        };
        if let Some(editor) = self.node_editor.as_mut() {
            let current = editor.raw_data_editor.text();
            push_node_editor_dsl_history(&mut self.node_editor_dsl_undo_stack, current);
            editor.raw_data_editor = text_editor::Content::with_text(&next);
            refresh_node_editor_from_raw(editor);
        }
    }

    pub fn set_node_editor_show_raw_data_editor(&mut self, value: bool) {
        if let Some(editor) = self.node_editor.as_mut() {
            if value {
                let _ = sync_node_editor_raw_from_visual(editor);
            }
            editor.show_raw_data_editor = value;
            clear_node_editor_validation(editor);
        }
    }

    pub(super) fn update_node_visual_draft<F>(&mut self, update: F)
    where
        F: FnOnce(&mut WorkflowNodeVisualDraft) -> bool,
    {
        if let Some(editor) = self.node_editor.as_mut() {
            let changed = editor.visual_draft.as_mut().map(update).unwrap_or(false);
            if changed {
                let _ = sync_node_editor_raw_from_visual(editor);
                clear_node_editor_validation(editor);
            }
        }
    }

    pub fn add_node_editor_start_variable(&mut self) {
        self.open_node_editor_start_variable_create();
    }

    pub fn open_node_editor_start_variable_create(&mut self) {
        if let Some(editor) = self.node_editor.as_mut()
            && matches!(editor.visual_draft.as_ref(), Some(WorkflowNodeVisualDraft::Start { .. }))
        {
            editor.start_variable_editor = Some(build_start_variable_editor_draft(
                WorkflowStartVariableEditorMode::Create,
                default_start_variable_draft(),
            ));
        }
    }

    pub fn open_node_editor_start_variable_edit(&mut self, index: usize) {
        if let Some(editor) = self.node_editor.as_mut()
            && let Some(WorkflowNodeVisualDraft::Start { variables }) = editor.visual_draft.as_ref()
            && let Some(variable) = variables.get(index)
        {
            editor.hovered_start_variable_index = Some(index);
            editor.start_variable_focus_index = Some(index);
            editor.start_variable_editor = Some(build_start_variable_editor_draft(
                WorkflowStartVariableEditorMode::Edit(index),
                variable.clone(),
            ));
        }
    }

    pub fn close_node_editor_start_variable_editor(&mut self) {
        if let Some(editor) = self.node_editor.as_mut() {
            editor.start_variable_editor = None;
        }
    }

    fn update_node_editor_start_variable_editor<F>(&mut self, update: F)
    where
        F: FnOnce(&mut WorkflowStartVariableEditorDraft),
    {
        if let Some(editor) = self.node_editor.as_mut()
            && let Some(start_variable_editor) = editor.start_variable_editor.as_mut()
        {
            update(start_variable_editor);
        }
    }

    pub fn set_node_editor_start_variable_editor_label(&mut self, value: String) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.variable.label = value;
        });
    }

    pub fn set_node_editor_start_variable_editor_name(&mut self, value: String) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.variable.variable = value;
        });
    }

    pub fn set_node_editor_start_variable_editor_type(&mut self, value: String) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.variable.input_type = value;
            normalize_start_variable_draft(&mut editor.variable);
            editor.default_value_editor =
                text_editor::Content::with_text(&editor.variable.default_value);
            editor.default_file_url_input = editor.variable.default_value.clone();
            editor.show_default_file_url_input = false;
        });
    }

    pub fn set_node_editor_start_variable_editor_required(&mut self, value: bool) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.variable.required = value;
            if value {
                editor.variable.hidden = false;
            }
        });
    }

    pub fn set_node_editor_start_variable_editor_hidden(&mut self, value: bool) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.variable.hidden = value;
            if value {
                editor.variable.required = false;
            }
        });
    }

    pub fn set_node_editor_start_variable_editor_default(&mut self, value: String) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.variable.default_value = value;
            editor.default_value_editor =
                text_editor::Content::with_text(&editor.variable.default_value);
            editor.default_file_url_input = editor.variable.default_value.clone();
        });
    }

    pub fn node_editor_start_variable_editor_default_action(
        &mut self,
        action: text_editor::Action,
    ) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.default_value_editor.perform(action);
            editor.variable.default_value = editor.default_value_editor.text();
        });
    }

    pub fn set_node_editor_start_variable_editor_max_length(&mut self, value: String) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.variable.max_length_input = value;
            normalize_start_variable_draft(&mut editor.variable);
        });
    }

    pub fn add_node_editor_start_variable_editor_option(&mut self) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.variable.options.push(String::new());
        });
    }

    pub fn remove_node_editor_start_variable_editor_option(&mut self, index: usize) {
        self.update_node_editor_start_variable_editor(|editor| {
            if index < editor.variable.options.len() {
                editor.variable.options.remove(index);
            }
        });
    }

    pub fn set_node_editor_start_variable_editor_option(&mut self, index: usize, value: String) {
        self.update_node_editor_start_variable_editor(|editor| {
            if let Some(option) = editor.variable.options.get_mut(index) {
                *option = value;
            }
        });
    }

    pub fn toggle_node_editor_start_variable_editor_file_type(&mut self, value: String) {
        self.update_node_editor_start_variable_editor(|editor| {
            let file_types = &mut editor.variable.allowed_file_types;

            if value == "custom" {
                if file_types.iter().any(|item| item == &value) {
                    file_types.retain(|item| item != &value);
                    editor.variable.allowed_file_extensions.clear();
                    editor.variable.allowed_file_extensions_input.clear();
                } else {
                    file_types.clear();
                    file_types.push(value);
                }
            } else {
                file_types.retain(|item| item != "custom");
                if file_types.iter().any(|item| item == &value) {
                    file_types.retain(|item| item != &value);
                } else {
                    file_types.push(value);
                }
            }

            normalize_start_variable_draft(&mut editor.variable);
        });
    }

    pub fn set_node_editor_start_variable_editor_extensions(&mut self, value: String) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.variable.allowed_file_extensions_input = value;
            normalize_start_variable_draft(&mut editor.variable);
        });
    }

    pub fn set_node_editor_start_variable_editor_upload_method(&mut self, value: String) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.variable.allowed_file_upload_methods = match value.as_str() {
                "local_file" => vec!["local_file".to_string()],
                "remote_url" => vec!["remote_url".to_string()],
                _ => default_start_variable_allowed_upload_methods(),
            };
            if !editor.variable.allowed_file_upload_methods.iter().any(|item| item == "remote_url")
            {
                editor.show_default_file_url_input = false;
            }
        });
    }

    pub fn open_node_editor_start_variable_editor_default_file_url_input(&mut self) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.default_file_url_input = if editor.variable.input_type == "file" {
                editor.variable.default_value.clone()
            } else {
                String::new()
            };
            editor.show_default_file_url_input = true;
        });
    }

    pub fn close_node_editor_start_variable_editor_default_file_url_input(&mut self) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.show_default_file_url_input = false;
        });
    }

    pub fn set_node_editor_start_variable_editor_default_file_url_input(&mut self, value: String) {
        self.update_node_editor_start_variable_editor(|editor| {
            editor.default_file_url_input = value;
        });
    }

    pub fn submit_node_editor_start_variable_editor_default_file_url(
        &mut self,
    ) -> Result<(), String> {
        let value = self
            .node_editor
            .as_ref()
            .and_then(|editor| editor.start_variable_editor.as_ref())
            .map(|editor| editor.default_file_url_input.clone())
            .unwrap_or_default();
        self.append_node_editor_start_variable_editor_default_file(value)
    }

    pub fn set_node_editor_start_variable_editor_default_file_path(
        &mut self,
        path: String,
    ) -> Result<(), String> {
        self.append_node_editor_start_variable_editor_default_file(path)
    }

    pub fn remove_node_editor_start_variable_editor_default_file(&mut self, index: usize) {
        self.update_node_editor_start_variable_editor(|editor| {
            if editor.variable.default_file_values.get(index).is_some() {
                editor.variable.default_file_values.remove(index);
                normalize_start_variable_draft(&mut editor.variable);
                editor.default_value_editor =
                    text_editor::Content::with_text(&editor.variable.default_value);
                if editor.variable.input_type == "file" {
                    editor.default_file_url_input = editor.variable.default_value.clone();
                }
            }
        });
    }

    pub fn submit_node_editor_start_variable_editor(&mut self) -> Result<(), String> {
        let Some(editor) = self.node_editor.as_mut() else {
            return Ok(());
        };
        let Some(start_variable_editor) = editor.start_variable_editor.take() else {
            return Ok(());
        };

        let WorkflowStartVariableEditorDraft { mode, mut variable, .. } = start_variable_editor;
        normalize_start_variable_draft(&mut variable);

        let Some(WorkflowNodeVisualDraft::Start { variables }) = editor.visual_draft.as_mut()
        else {
            editor.start_variable_editor = Some(build_start_variable_editor_draft(mode, variable));
            return Err("当前节点不支持开始变量编辑".to_string());
        };

        if let Err(error) = validate_start_variable_editor_draft(&variable, variables, mode) {
            editor.start_variable_editor = Some(build_start_variable_editor_draft(mode, variable));
            return Err(error);
        }

        if let Err(error) = merge_start_variable_value(&variable) {
            editor.start_variable_editor = Some(build_start_variable_editor_draft(mode, variable));
            return Err(error);
        }

        match mode {
            WorkflowStartVariableEditorMode::Create => {
                variables.push(variable);
                editor.start_variable_focus_index = Some(variables.len().saturating_sub(1));
            }
            WorkflowStartVariableEditorMode::Edit(index) => {
                let Some(slot) = variables.get_mut(index) else {
                    editor.start_variable_editor =
                        Some(build_start_variable_editor_draft(mode, variable));
                    return Err("目标变量不存在".to_string());
                };
                *slot = variable;
                editor.start_variable_focus_index = Some(index);
            }
        }

        let _ = sync_node_editor_raw_from_visual(editor);
        clamp_node_editor_start_variable_focus(editor);
        clear_node_editor_validation(editor);
        Ok(())
    }

    pub fn remove_node_editor_start_variable(&mut self, index: usize) {
        if let Some(editor) = self.node_editor.as_mut() {
            let mut changed = false;

            if let Some(WorkflowNodeVisualDraft::Start { variables }) = editor.visual_draft.as_mut()
            {
                if index < variables.len() {
                    variables.remove(index);
                    editor.start_variable_focus_index = match editor.start_variable_focus_index {
                        Some(_) if variables.is_empty() => None,
                        Some(current) if current > index => Some(current - 1),
                        Some(current) if current == index => {
                            Some(index.min(variables.len().saturating_sub(1)))
                        }
                        focus => focus,
                    };
                    changed = true;
                }
            }

            if changed {
                let _ = sync_node_editor_raw_from_visual(editor);
                clamp_node_editor_start_variable_focus(editor);
                clear_node_editor_validation(editor);
            }
        }
    }

    pub fn set_node_editor_start_variable_focus(&mut self, index: usize) {
        if let Some(editor) = self.node_editor.as_mut()
            && let Some(WorkflowNodeVisualDraft::Start { variables }) = editor.visual_draft.as_ref()
            && index < variables.len()
        {
            editor.start_variable_focus_index = Some(index);
        }
    }

    pub fn set_node_editor_start_variable_label(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Start { variables } = draft {
                if let Some(variable) = variables.get_mut(index) {
                    variable.label = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_start_variable_name(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Start { variables } = draft {
                if let Some(variable) = variables.get_mut(index) {
                    variable.variable = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_start_variable_type(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Start { variables } = draft {
                if let Some(variable) = variables.get_mut(index) {
                    variable.input_type = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_start_variable_required(&mut self, index: usize, value: bool) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Start { variables } = draft {
                if let Some(variable) = variables.get_mut(index) {
                    variable.required = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_start_variable_default(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Start { variables } = draft {
                if let Some(variable) = variables.get_mut(index) {
                    variable.default_value = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_start_variable_placeholder(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Start { variables } = draft {
                if let Some(variable) = variables.get_mut(index) {
                    variable.placeholder = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_start_variable_hint(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Start { variables } = draft {
                if let Some(variable) = variables.get_mut(index) {
                    variable.hint = value;
                    return true;
                }
            }
            false
        });
    }

    pub fn set_node_editor_start_variable_max_length(&mut self, index: usize, value: String) {
        self.update_node_visual_draft(|draft| {
            if let WorkflowNodeVisualDraft::Start { variables } = draft {
                if let Some(variable) = variables.get_mut(index) {
                    variable.max_length_input = value;
                    return true;
                }
            }
            false
        });
    }
}

fn refresh_node_editor_from_raw(editor: &mut WorkflowNodeEditorDraft) {
    if let Ok(visual_draft) =
        build_node_visual_draft(&editor.block_type, &editor.raw_data_editor.text())
    {
        editor.visual_draft = visual_draft;
        clamp_node_editor_start_variable_focus(editor);
    }
    clear_node_editor_validation(editor);
}

fn push_node_editor_dsl_history(history: &mut Vec<String>, value: String) {
    history.push(value);
    if history.len() > NODE_EDITOR_DSL_HISTORY_LIMIT {
        history.remove(0);
    }
}

#[cfg(test)]
#[path = "node_editor_start_tests.rs"]
mod node_editor_start_tests;
