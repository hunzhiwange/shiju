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

        // 保存前一个项目的预览标签页与激活路径
        let previous_project_path = self.project_path.clone();
        let project_changed = previous_project_path.as_deref() != Some(path.as_str());
        if let Some(prev_path) = &previous_project_path {
            self.project_preview_tabs
                .insert(prev_path.clone(), std::mem::take(&mut self.preview_tabs));
            self.project_preview_active_path
                .insert(prev_path.clone(), self.active_preview_path.take());
        }
        self.project_path = Some(path.clone());
        (previous_project_path, project_changed)
    }
    pub fn restore_project_preview(&mut self, path: &str) {
        // 恢复当前项目的预览标签页与激活路径（若存在）
        let tabs = self.project_preview_tabs.remove(path).unwrap_or_default();
        let active = self.project_preview_active_path.remove(path).unwrap_or_default();
        self.preview_tabs = tabs;
        self.active_preview_path = active;
        self.preview_tab_menu_path = None;
        self.preview_tab_menu_pos = None;
    }
}
