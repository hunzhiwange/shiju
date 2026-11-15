//! 已保存应用列表、打开与删除状态。

use super::*;

impl WorkflowState {
    pub fn begin_saved_apps_load(&mut self) {
        self.saved_apps_loading = true;
        self.saved_apps_error = None;
        self.saved_app_actions_menu_uuid = None;
    }

    pub fn finish_saved_apps_load(&mut self, result: Result<Vec<WorkflowSavedAppSummary>, String>) {
        self.saved_apps_loading = false;
        self.saved_apps_loaded = true;

        match result {
            Ok(apps) => {
                self.saved_apps = apps;
                self.saved_apps_error = None;
            }
            Err(error) => {
                self.saved_apps_error = Some(error);
            }
        }
    }

    pub fn begin_saved_app_open(&mut self, uuid: String) {
        self.opening_saved_app_uuid = Some(uuid);
        self.saved_apps_error = None;
        self.saved_app_actions_menu_uuid = None;
        self.close_floating_panels();
    }

    pub fn finish_saved_app_open(&mut self) {
        self.opening_saved_app_uuid = None;
    }

    pub fn begin_saved_app_delete(&mut self, uuid: String) {
        self.deleting_saved_app_uuid = Some(uuid);
        self.confirm_delete_saved_app_uuid = None;
        self.saved_app_actions_menu_uuid = None;
        self.saved_apps_error = None;
    }

    pub fn finish_saved_app_delete(&mut self) {
        self.deleting_saved_app_uuid = None;
    }

    pub fn open_saved_app_delete_confirm(&mut self, uuid: String) {
        self.confirm_delete_saved_app_uuid = Some(uuid);
        self.saved_app_actions_menu_uuid = None;
        self.saved_apps_error = None;
    }

    pub fn close_saved_app_delete_confirm(&mut self) {
        self.confirm_delete_saved_app_uuid = None;
    }

    pub fn remove_saved_app(&mut self, uuid: &str) -> bool {
        self.saved_apps.retain(|app| app.uuid != uuid);
        self.apps.retain(|app| app.local_uuid.as_deref() != Some(uuid));
        if self.confirm_delete_saved_app_uuid.as_deref() == Some(uuid) {
            self.confirm_delete_saved_app_uuid = None;
        }

        if self.saved_app_actions_menu_uuid.as_deref() == Some(uuid) {
            self.saved_app_actions_menu_uuid = None;
        }

        if self.copied_saved_app_uuid.as_deref() == Some(uuid) {
            self.copied_saved_app_uuid = None;
        }

        let removed_active = self.local_uuid.as_deref() == Some(uuid);
        if removed_active {
            self.active_app_id = None;
            self.active_is_dirty = false;
            self.source_name.clear();
            self.local_uuid = None;
            self.source_path = None;
            self.document = WorkflowDocument::default();
            self.environment_variables.clear();
            self.conversation_variables.clear();
            self.selected_node_id = None;
            self.selected_edge_id = None;
            self.connection_draft = None;
            self.undo_stack.clear();
            self.redo_stack.clear();
            self.saved_snapshot = None;
        }

        removed_active
    }

    pub fn show_saved_apps(&mut self) {
        self.persist_active_snapshot();
        self.active_app_id = None;
        self.close_floating_panels();
        self.context_menu = None;
        self.quick_insert_panel_open = false;
        self.zoom_menu_open = false;
        self.app_editor = None;
        self.node_editor = None;
        self.variable_panel = None;
        self.variable_editor = None;
        self.error_message = None;
        self.saved_app_actions_menu_uuid = None;
    }

    pub fn set_saved_app_search_query(&mut self, query: String) {
        self.saved_app_search_query = query;
        self.saved_app_actions_menu_uuid = None;
    }

    pub fn toggle_saved_app_actions(&mut self, uuid: String) {
        if self.saved_app_actions_menu_uuid.as_deref() == Some(uuid.as_str()) {
            self.saved_app_actions_menu_uuid = None;
        } else {
            self.saved_app_actions_menu_uuid = Some(uuid);
        }
    }

    pub fn close_saved_app_actions(&mut self) {
        self.saved_app_actions_menu_uuid = None;
    }

    pub fn mark_saved_app_uuid_copied(&mut self, uuid: String) {
        self.copied_saved_app_uuid = Some(uuid);
        self.saved_app_actions_menu_uuid = None;
    }

    pub fn clear_saved_app_uuid_copied(&mut self, uuid: &str) {
        if self.copied_saved_app_uuid.as_deref() == Some(uuid) {
            self.copied_saved_app_uuid = None;
        }
    }

}
