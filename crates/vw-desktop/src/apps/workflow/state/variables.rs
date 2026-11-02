//! # Workflow 变量状态操作
//!
//! 该模块处理环境变量和会话变量面板、编辑器、提交与删除逻辑。

use super::*;

impl WorkflowState {
    pub fn open_variable_panel(&mut self, kind: WorkflowVariablePanelKind) {
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.variable_panel = Some(kind);
    }

    pub fn close_variable_panel(&mut self) {
        self.variable_panel = None;
    }

    pub fn open_create_environment_variable_editor(&mut self) {
        self.context_menu = None;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.variable_panel = Some(WorkflowVariablePanelKind::Environment);
        self.variable_editor = Some(WorkflowVariableEditorDraft {
            mode: WorkflowVariableEditorMode::CreateEnvironment,
            name: self.next_environment_variable_name(),
            description: String::new(),
            value_type: "string".to_string(),
            raw_value_editor: text_editor::Content::with_text(""),
        });
    }

    pub fn open_edit_environment_variable_editor(
        &mut self,
        id: &str,
    ) -> Result<(), crate::app::i18n::workflow::Feedback> {
        let variable = self
            .environment_variable(id)
            .cloned()
            .ok_or_else(|| crate::app::i18n::workflow::Feedback::new("环境变量不存在", vec![]))?;

        self.context_menu = None;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.variable_panel = Some(WorkflowVariablePanelKind::Environment);
        self.variable_editor = Some(WorkflowVariableEditorDraft {
            mode: WorkflowVariableEditorMode::EditEnvironment(variable.id.clone()),
            name: variable.name.clone(),
            description: variable.description.clone(),
            value_type: variable.value_type.clone(),
            raw_value_editor: text_editor::Content::with_text(&value_yaml_for_editor(
                &variable.value,
            )),
        });
        Ok(())
    }

    pub fn open_create_conversation_variable_editor(&mut self) {
        self.context_menu = None;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.variable_panel = Some(WorkflowVariablePanelKind::Conversation);
        self.variable_editor = Some(WorkflowVariableEditorDraft {
            mode: WorkflowVariableEditorMode::CreateConversation,
            name: self.next_conversation_variable_name(),
            description: String::new(),
            value_type: "string".to_string(),
            raw_value_editor: text_editor::Content::with_text(""),
        });
    }

    pub fn open_edit_conversation_variable_editor(
        &mut self,
        id: &str,
    ) -> Result<(), crate::app::i18n::workflow::Feedback> {
        let variable = self
            .conversation_variable(id)
            .cloned()
            .ok_or_else(|| crate::app::i18n::workflow::Feedback::new("会话变量不存在", vec![]))?;

        self.context_menu = None;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.variable_panel = Some(WorkflowVariablePanelKind::Conversation);
        self.variable_editor = Some(WorkflowVariableEditorDraft {
            mode: WorkflowVariableEditorMode::EditConversation(variable.id.clone()),
            name: variable.name.clone(),
            description: variable.description.clone(),
            value_type: variable.value_type.clone(),
            raw_value_editor: text_editor::Content::with_text(&value_yaml_for_editor(
                &variable.value,
            )),
        });
        Ok(())
    }

    pub fn close_variable_editor(&mut self) {
        self.variable_editor = None;
    }

    pub fn set_variable_editor_name(&mut self, value: String) {
        if let Some(editor) = self.variable_editor.as_mut() {
            editor.name = value;
        }
    }

    pub fn set_variable_editor_description(&mut self, value: String) {
        if let Some(editor) = self.variable_editor.as_mut() {
            editor.description = value;
        }
    }

    pub fn set_variable_editor_type(&mut self, value: String) {
        if let Some(editor) = self.variable_editor.as_mut() {
            editor.value_type = value;
        }
    }

    pub fn variable_editor_action(&mut self, action: text_editor::Action) {
        if let Some(editor) = self.variable_editor.as_mut() {
            editor.raw_value_editor.perform(action);
        }
    }

    pub fn submit_variable_editor(&mut self) -> Result<(), crate::app::i18n::workflow::Feedback> {
        let Some(editor) = self.variable_editor.as_ref() else {
            return Ok(());
        };

        let mode = editor.mode.clone();
        let value_type_input = editor.value_type.clone();
        let name = editor.name.trim().to_string();
        if name.is_empty() {
            return Err(crate::app::i18n::workflow::Feedback::new("变量名称不能为空", vec![]));
        }

        let description = editor.description.trim().to_string();
        let raw_value = parse_yaml_editor_value(&editor.raw_value_editor.text())?;

        match mode {
            WorkflowVariableEditorMode::CreateEnvironment => {
                let value_type = normalize_environment_value_type(&value_type_input)?;
                validate_environment_value(&value_type, &raw_value)?;
                ensure_unique_variable_name(&self.environment_variables, &name, None, "环境变量")?;

                self.push_undo_snapshot();
                self.environment_variables.push(WorkflowEnvironmentVariable {
                    id: generate_variable_id("env"),
                    name: name.clone(),
                    value_type,
                    value: raw_value,
                    description,
                    raw_variable: Value::Null,
                });
                self.variable_panel = Some(WorkflowVariablePanelKind::Environment);
                self.variable_editor = None;
                self.refresh_dirty_state();
                self.set_status(crate::app::i18n::format::Message::new(
                    "已新增环境变量 %s",
                    vec![name.to_string().into()],
                ));
            }
            WorkflowVariableEditorMode::EditEnvironment(id) => {
                let value_type = normalize_environment_value_type(&value_type_input)?;
                validate_environment_value(&value_type, &raw_value)?;
                ensure_unique_variable_name(
                    &self.environment_variables,
                    &name,
                    Some(&id),
                    "环境变量",
                )?;

                let index = self
                    .environment_variables
                    .iter()
                    .position(|variable| variable.id == id)
                    .ok_or_else(|| {
                        crate::app::i18n::workflow::Feedback::new("环境变量不存在", vec![])
                    })?;
                let raw_variable = self.environment_variables[index].raw_variable.clone();

                self.push_undo_snapshot();
                self.environment_variables[index] = WorkflowEnvironmentVariable {
                    id,
                    name: name.clone(),
                    value_type,
                    value: raw_value,
                    description,
                    raw_variable,
                };
                self.variable_panel = Some(WorkflowVariablePanelKind::Environment);
                self.variable_editor = None;
                self.refresh_dirty_state();
                self.set_status(crate::app::i18n::format::Message::new(
                    "已更新环境变量 %s",
                    vec![name.to_string().into()],
                ));
            }
            WorkflowVariableEditorMode::CreateConversation => {
                let value_type = normalize_conversation_value_type(&value_type_input)?;
                ensure_unique_variable_name(&self.conversation_variables, &name, None, "会话变量")?;

                self.push_undo_snapshot();
                self.conversation_variables.push(WorkflowConversationVariable {
                    id: generate_variable_id("conversation"),
                    name: name.clone(),
                    value_type,
                    value: raw_value,
                    description,
                    raw_variable: Value::Null,
                });
                self.variable_panel = Some(WorkflowVariablePanelKind::Conversation);
                self.variable_editor = None;
                self.refresh_dirty_state();
                self.set_status(crate::app::i18n::format::Message::new(
                    "已新增会话变量 %s",
                    vec![name.to_string().into()],
                ));
            }
            WorkflowVariableEditorMode::EditConversation(id) => {
                let value_type = normalize_conversation_value_type(&value_type_input)?;
                ensure_unique_variable_name(
                    &self.conversation_variables,
                    &name,
                    Some(&id),
                    "会话变量",
                )?;

                let index = self
                    .conversation_variables
                    .iter()
                    .position(|variable| variable.id == id)
                    .ok_or_else(|| {
                        crate::app::i18n::workflow::Feedback::new("会话变量不存在", vec![])
                    })?;
                let raw_variable = self.conversation_variables[index].raw_variable.clone();

                self.push_undo_snapshot();
                self.conversation_variables[index] = WorkflowConversationVariable {
                    id,
                    name: name.clone(),
                    value_type,
                    value: raw_value,
                    description,
                    raw_variable,
                };
                self.variable_panel = Some(WorkflowVariablePanelKind::Conversation);
                self.variable_editor = None;
                self.refresh_dirty_state();
                self.set_status(crate::app::i18n::format::Message::new(
                    "已更新会话变量 %s",
                    vec![name.to_string().into()],
                ));
            }
        }

        Ok(())
    }

    pub fn delete_environment_variable(&mut self, id: &str) -> bool {
        let Some(index) = self.environment_variables.iter().position(|variable| variable.id == id)
        else {
            return false;
        };

        self.push_undo_snapshot();

        let removed = self.environment_variables.remove(index);
        if self.variable_editor.as_ref().is_some_and(|editor| {
            matches!(&editor.mode, WorkflowVariableEditorMode::EditEnvironment(edit_id) if edit_id == id)
        }) {
            self.variable_editor = None;
        }

        self.variable_panel = Some(WorkflowVariablePanelKind::Environment);
        self.refresh_dirty_state();
        self.set_status(crate::app::i18n::format::Message::new(
            "已删除环境变量 %s",
            vec![removed.name.to_string().into()],
        ));
        true
    }

    pub fn delete_conversation_variable(&mut self, id: &str) -> bool {
        let Some(index) = self.conversation_variables.iter().position(|variable| variable.id == id)
        else {
            return false;
        };

        self.push_undo_snapshot();

        let removed = self.conversation_variables.remove(index);
        if self.variable_editor.as_ref().is_some_and(|editor| {
            matches!(&editor.mode, WorkflowVariableEditorMode::EditConversation(edit_id) if edit_id == id)
        }) {
            self.variable_editor = None;
        }

        self.variable_panel = Some(WorkflowVariablePanelKind::Conversation);
        self.refresh_dirty_state();
        self.set_status(crate::app::i18n::format::Message::new(
            "已删除会话变量 %s",
            vec![removed.name.to_string().into()],
        ));
        true
    }

    fn next_environment_variable_name(&self) -> String {
        let mut index = 1;
        loop {
            let candidate = format!("env_{}", index);
            if !self.environment_variables.iter().any(|variable| variable.name == candidate) {
                return candidate;
            }
            index += 1;
        }
    }

    fn next_conversation_variable_name(&self) -> String {
        let mut index = 1;
        loop {
            let candidate = format!("conversation_{}", index);
            if !self.conversation_variables.iter().any(|variable| variable.name == candidate) {
                return candidate;
            }
            index += 1;
        }
    }
}

#[cfg(test)]
#[path = "variables_tests.rs"]
mod variables_tests;
