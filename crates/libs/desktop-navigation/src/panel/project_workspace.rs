//! Project activation owns tab and preview bookkeeping; runtime resources stay in the host.
use super::tabs::State;
use crate::components::top_bar::types::AppTab;
use crate::ui_types::Screen;
impl State {
    pub fn begin_project(&mut self, path: &str) -> (Option<String>, bool) {
        let path = path.to_string();
        // 如果 Apps 中间页标签存在，则移除
        if let Some(pos) = self.open_tabs.iter().position(|t| t.id == "apps") {
            self.open_tabs.remove(pos);
        }
        // 创建或切换到项目标签页
        let (tab_id, title) = desktop_workspace::projects::project_tab(&path);
        if !self.open_tabs.iter().any(|t| t.id == tab_id) {
            self.open_tabs.push(AppTab {
                id: tab_id.clone(),
                title,
                screen: Screen::Project,
                project_path: Some(path.clone()),
            });
        }
        self.active_tab_id = Some(tab_id);

        let previous_project_path = self.project_path.clone();
        let project_changed = previous_project_path.as_deref() != Some(path.as_str());
        self.switch_preview_project(Some(path));
        self.preview_tab_menu_path = None;
        self.preview_tab_menu_pos = None;
        (previous_project_path, project_changed)
    }

    pub fn switch_preview_project(&mut self, project: Option<String>) {
        if self.project_path == project {
            return;
        }
        let tabs = std::mem::take(&mut self.preview_tabs);
        let active = self.active_preview_path.take();
        if let Some(previous) = self.project_path.take() {
            self.project_preview_tabs.insert(previous.clone(), tabs);
            self.project_preview_active_path.insert(previous, active);
        } else {
            self.standalone_preview_tabs = tabs;
            if active.is_some() {
                self.standalone_preview_active_path = active;
            }
        }
        if let Some(path) = &project {
            self.preview_tabs = self.project_preview_tabs.remove(path).unwrap_or_default();
            self.active_preview_path = self.project_preview_active_path.remove(path).flatten();
        } else {
            self.preview_tabs = std::mem::take(&mut self.standalone_preview_tabs);
        }
        self.project_path = project;
        self.preview_tab_menu_path = None;
        self.preview_tab_menu_pos = None;
    }
}
