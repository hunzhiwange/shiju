//! File manager presentation and refresh lifecycle; the host performs Git and index effects.
#[derive(Debug, Default)]
pub struct FileManagerState {
    pub file_manager_show_changes: bool,
    pub file_manager_refresh_frame: usize,
    pub file_manager_changes_refreshing: bool,
    pub file_manager_file_tree_refreshing: bool,
    pub show_file_manager: bool,
}
pub struct Refresh {
    pub schedule_tick: bool,
}
impl FileManagerState {
    pub fn select_changes(&mut self, changes: bool) {
        self.file_manager_show_changes = changes;
        self.show_file_manager = true;
    }
    pub fn begin_changes_refresh(&mut self, git_loading: bool) -> Option<Refresh> {
        if git_loading || self.file_manager_changes_refreshing { return None; }
        self.file_manager_changes_refreshing = true;
        Some(Refresh { schedule_tick: !self.file_manager_file_tree_refreshing })
    }
    pub fn begin_tree_refresh(&mut self) -> Option<Refresh> {
        if self.file_manager_file_tree_refreshing { return None; }
        self.file_manager_file_tree_refreshing = true;
        Some(Refresh { schedule_tick: !self.file_manager_changes_refreshing })
    }
    pub fn tick(&mut self, git_commit_in_progress: bool) -> bool {
        if !self.file_manager_changes_refreshing && !self.file_manager_file_tree_refreshing && !git_commit_in_progress { return false; }
        self.file_manager_refresh_frame = self.file_manager_refresh_frame.wrapping_add(1);
        true
    }
    pub fn complete_tree_refresh(&mut self) -> bool {
        if !self.file_manager_file_tree_refreshing { return false; }
        self.file_manager_file_tree_refreshing = false;
        true
    }
}

impl FileManagerState {
    pub fn apply_preferences(&mut self, cfg: &serde_json::Value) {
        self.show_file_manager = cfg
            .get("show_file_manager")
            .and_then(|v| v.as_bool())
            .unwrap_or(self.show_file_manager);
        self.file_manager_show_changes = cfg
            .get("file_manager_show_changes")
            .and_then(|v| v.as_bool())
            .unwrap_or(self.file_manager_show_changes);
    }
}

impl FileManagerState {
    pub fn from_config(cfg: &serde_json::Value) -> Self {
        Self {
            file_manager_show_changes: cfg.get("file_manager_show_changes").and_then(serde_json::Value::as_bool).unwrap_or(true),
            show_file_manager: cfg.get("show_file_manager").and_then(serde_json::Value::as_bool).unwrap_or(true),
            ..Default::default()
        }
    }
}
