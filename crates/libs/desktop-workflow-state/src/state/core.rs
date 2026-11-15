//! # Workflow 核心状态操作
//!
//! 该模块提供核心状态读写、选择管理、本地化反馈和浮层开关等通用逻辑。

use super::*;

impl WorkflowState {
    pub fn display_title(&self, i18n: desktop_i18n::I18n) -> String {
        if self.active_app_id.is_none() || self.source_name.trim().is_empty() {
            i18n.__("工作流").to_string()
        } else {
            self.title().to_string()
        }
    }

    pub fn title(&self) -> &str {
        if self.active_app_id.is_none() {
            return "工作流";
        }

        if self.source_name.trim().is_empty() { "工作流" } else { self.source_name.as_str() }
    }

    pub fn active_app(&self) -> Option<&WorkflowAppEntry> {
        let active_id = self.active_app_id.as_deref()?;
        self.apps.iter().find(|app| app.id == active_id)
    }

    pub fn has_apps(&self) -> bool {
        !self.apps.is_empty()
    }

    pub fn has_start_node(&self) -> bool {
        self.document.nodes.iter().any(|node| node.block_type == "start")
    }

    pub fn active_meta(&self) -> Option<&WorkflowAppMeta> {
        self.active_app().map(|app| &app.meta)
    }

    pub fn selected_node(&self) -> Option<&WorkflowNode> {
        self.selected_node_id.as_deref().and_then(|id| self.document.node(id))
    }

    pub fn selected_edge(&self) -> Option<&WorkflowEdge> {
        self.selected_edge_id.as_deref().and_then(|id| self.document.edge(id))
    }

    pub fn environment_variable(&self, id: &str) -> Option<&WorkflowEnvironmentVariable> {
        self.environment_variables.iter().find(|variable| variable.id == id)
    }

    pub fn conversation_variable(&self, id: &str) -> Option<&WorkflowConversationVariable> {
        self.conversation_variables.iter().find(|variable| variable.id == id)
    }

    pub fn set_error(&mut self, error: impl Into<desktop_i18n::workflow::Feedback>) {
        let error = error.into();
        self.error_message = Some(error.to_string());
        self.localized_error = Some(error);
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn select_node(&mut self, id: String) {
        self.selected_node_id = Some(id);
        self.selected_edge_id = None;
        self.context_menu = None;
        self.sync_selection_flags();
    }

    pub fn select_edge(&mut self, id: String) {
        self.selected_edge_id = Some(id);
        self.selected_node_id = None;
        self.context_menu = None;
        self.sync_selection_flags();
    }

    pub fn clear_selection(&mut self) {
        self.selected_node_id = None;
        self.selected_edge_id = None;
        self.context_menu = None;
        self.sync_selection_flags();
    }

    pub fn open_create_editor(&mut self) {
        self.node_editor = None;
        self.variable_editor = None;
        self.variable_panel = None;
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.app_editor = Some(WorkflowAppEditorDraft {
            mode: WorkflowAppEditorMode::Create,
            name: self.next_app_name(),
            description: String::new(),
            icon: "🤖".to_string(),
            use_icon_as_answer_icon: false,
            max_active_requests_input: "0".to_string(),
        });
    }

    pub fn open_edit_editor(&mut self, id: Option<&str>) {
        let target_id = id.map(|value| value.to_string()).or_else(|| self.active_app_id.clone());
        let Some(target_id) = target_id else {
            return;
        };

        if let Some(app) = self.apps.iter().find(|app| app.id == target_id) {
            self.node_editor = None;
            self.variable_editor = None;
            self.variable_panel = None;
            self.context_menu = None;
            self.quick_insert_panel_open = false;
            self.action_menu_open = false;
            self.zoom_menu_open = false;
            self.app_editor = Some(WorkflowAppEditorDraft {
                mode: WorkflowAppEditorMode::Edit(target_id),
                name: app.meta.name.clone(),
                description: app.meta.description.clone(),
                icon: app.meta.icon.clone(),
                use_icon_as_answer_icon: app.meta.use_icon_as_answer_icon,
                max_active_requests_input: app.meta.max_active_requests.to_string(),
            });
        }
    }

    pub fn close_editor(&mut self) {
        self.app_editor = None;
    }

    pub fn toggle_action_menu(&mut self) {
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.zoom_menu_open = false;
        self.action_menu_open = !self.action_menu_open;
    }

    pub fn toggle_zoom_menu(&mut self) {
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.action_menu_open = false;
        self.zoom_menu_open = !self.zoom_menu_open;
    }

    pub fn close_floating_panels(&mut self) {
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.quick_insert_panel_open = false;
    }

    pub fn toggle_quick_insert_panel(&mut self) {
        self.context_menu = None;
        self.action_menu_open = false;
        self.zoom_menu_open = false;
        self.quick_insert_panel_open = !self.quick_insert_panel_open;
    }

    pub fn close_quick_insert_panel(&mut self) {
        self.quick_insert_panel_open = false;
    }
}

impl WorkflowState {
    pub fn set_status(&mut self, message: desktop_i18n::format::Message) {
        self.status_message = Some(
            message
                .render(desktop_i18n::I18n::new(Default::default()))
                .expect("valid workflow status template"),
        );
        self.localized_status = Some(message);
    }

    pub fn status_text(&self, i18n: desktop_i18n::I18n) -> Option<String> {
        let raw = self.status_message.as_ref()?;
        if let Some(message) = &self.localized_status {
            if message.render(desktop_i18n::I18n::new(Default::default())).as_ref().ok()
                == Some(raw)
            {
                return Some(message.render(i18n).expect("valid workflow status template"));
            }
        }
        Some(raw.clone())
    }
}

impl WorkflowState {
    pub fn error_text(&self, i18n: desktop_i18n::I18n) -> Option<String> {
        let raw = self.error_message.as_ref()?;
        if let Some(message) = &self.localized_error {
            if message.to_string() == *raw {
                return Some(message.render(i18n));
            }
        }
        Some(raw.clone())
    }
}
