//! 实现 App 上的基础状态辅助方法。
//! 本模块集中维护展开文件、焦点、提示和轻量 UI 状态，避免主结构体实现过度膨胀。

use iced::{Color, Theme};
use std::time::Duration;

use super::state::ExternalOpenApp;
use super::{App, Screen};
use crate::app::message::NotificationMessage;
use crate::app::state::{Toast, ToastKind, WorkspaceMode};

impl App {
    pub(crate) fn is_chat_fullscreen(&self) -> bool {
        match self.workspace_mode {
            WorkspaceMode::Code => self.chat_panel_fullscreen,
            WorkspaceMode::Work => self.work_chat_fullscreen,
        }
    }

    pub(crate) fn is_chat_half_fullscreen(&self) -> bool {
        match self.workspace_mode {
            WorkspaceMode::Code => self.chat_panel_half_fullscreen,
            WorkspaceMode::Work => self.work_chat_half_fullscreen,
        }
    }

    fn normalize_file_search_query(query: &str) -> String {
        query.trim().replace('\\', "/").to_ascii_lowercase()
    }

    /// 模块内可见函数，执行 is_diff_file_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn is_diff_file_expanded(&self, path: &str) -> bool {
        self.expanded_files_set.contains(path)
    }

    /// 模块内可见函数，执行 toggle_diff_file_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    #[allow(dead_code)]
    pub(crate) fn toggle_diff_file_expanded(&mut self, path: String) {
        if self.expanded_files_set.remove(&path) {
            if let Some(pos) = self.expanded_files.iter().position(|item| item == &path) {
                self.expanded_files.remove(pos);
            }
        } else {
            self.expanded_files_set.insert(path.clone());
            self.expanded_files.push(path);
        }
    }

    /// 模块内可见函数，执行 ensure_diff_file_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn ensure_diff_file_expanded(&mut self, path: String) {
        if self.expanded_files_set.insert(path.clone()) {
            self.expanded_files.push(path);
        }
    }

    /// 模块内可见函数，执行 replace_expanded_files 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    #[allow(dead_code)]
    pub(crate) fn replace_expanded_files(&mut self, files: Vec<String>) {
        self.expanded_files_set = files.iter().cloned().collect();
        self.expanded_files = files;
    }

    /// 模块内可见函数，执行 clear_expanded_files 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn clear_expanded_files(&mut self) {
        self.expanded_files.clear();
        self.expanded_files_set.clear();
    }

    /// 模块内可见函数，执行 set_single_expanded_file 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn set_single_expanded_file(&mut self, path: String) {
        self.expanded_files.clear();
        self.expanded_files_set.clear();
        self.expanded_files_set.insert(path.clone());
        self.expanded_files.push(path);
    }

    /// 模块内可见函数，执行 is_file_tree_dir_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn is_file_tree_dir_expanded(&self, path: &str) -> bool {
        self.file_tree_expanded_set.contains(path)
    }

    /// 模块内可见函数，执行 toggle_file_tree_dir_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn toggle_file_tree_dir_expanded(&mut self, path: String) {
        if self.file_tree_expanded_set.remove(&path) {
            if let Some(pos) = self.file_tree_expanded.iter().position(|item| item == &path) {
                self.file_tree_expanded.remove(pos);
            }
        } else {
            self.file_tree_expanded_set.insert(path.clone());
            self.file_tree_expanded.push(path);
        }
    }

    /// 模块内可见函数，执行 ensure_file_tree_dir_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn ensure_file_tree_dir_expanded(&mut self, path: String) {
        if self.file_tree_expanded_set.insert(path.clone()) {
            self.file_tree_expanded.push(path);
        }
    }

    /// 公开函数，执行 title 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn title(&self) -> String {
        match self.screen {
            Screen::Home => {
                crate::app::i18n::I18n::new(self.desktop_locale).__("识矩 - 项目").to_string()
            }
            Screen::Project => {
                crate::app::i18n::I18n::new(self.desktop_locale).__("识矩 - 项目").to_string()
            }
            Screen::Preview => {
                crate::app::i18n::I18n::new(self.desktop_locale).__("识矩 - 预览").to_string()
            }
            Screen::Apps => {
                crate::app::i18n::I18n::new(self.desktop_locale).__("识矩 - 应用").to_string()
            }
            Screen::Usage => {
                crate::app::i18n::I18n::new(self.desktop_locale).__("识矩 - 用量").to_string()
            }
            Screen::Knowledge => {
                crate::app::i18n::I18n::new(self.desktop_locale).__("识矩 - 知识库").to_string()
            }
            Screen::WorkflowTool => {
                crate::app::i18n::I18n::new(self.desktop_locale).__("识矩 - 工作流").to_string()
            }
            Screen::WorkflowManager => {
                crate::app::i18n::I18n::new(self.desktop_locale).__("识矩 - 任务编排").to_string()
            }
            Screen::TaskBoard => {
                crate::app::i18n::I18n::new(self.desktop_locale).__("识矩 - 任务看板").to_string()
            }
            #[cfg(not(target_arch = "wasm32"))]
            Screen::ShijuApps => {
                crate::app::i18n::I18n::new(self.desktop_locale).__("识矩 - 识矩应用").to_string()
            }
            #[cfg(not(target_arch = "wasm32"))]
            Screen::ShijuSkills => {
                crate::app::i18n::I18n::new(self.desktop_locale).__("识矩 - 识矩技能").to_string()
            }
            #[cfg(not(target_arch = "wasm32"))]
            Screen::ExpertHub => crate::app::i18n::I18n::new(self.desktop_locale)
                .__("识矩 - 专家·技能·连接器")
                .to_string(),
        }
    }

    pub fn title_for_window(&self, window: iced::window::Id) -> String {
        if self.task_pet_window_id == Some(window) {
            "VibeWindow Pet".to_string()
        } else {
            self.title()
        }
    }

    /// 模块内可见函数，执行 current_file_index 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn current_file_index(&self) -> &[String] {
        self.project_path
            .as_ref()
            .and_then(|path| self.file_index_cache.get(path).map(Vec::as_slice))
            .unwrap_or(&[])
    }

    /// 模块内可见函数，执行 current_file_tree_model 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn current_file_tree_model(
        &self,
    ) -> Option<&crate::app::components::file_tree::model::FileTreeNode> {
        self.project_path.as_ref().and_then(|path| self.file_tree_model_cache.get(path))
    }

    /// 模块内可见函数，执行 refresh_search_panel_file_cache 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn refresh_search_panel_file_cache(&mut self) {
        let query = Self::normalize_file_search_query(&self.search_text);
        let project_path = self.project_path.clone();

        if self.search_panel_file_cache_query == query
            && self.search_panel_file_cache_project_path == project_path
            && self.search_panel_file_cache_revision == self.file_index_revision
        {
            return;
        }

        let results = self
            .current_file_index()
            .iter()
            .filter(|path| path.to_ascii_lowercase().contains(&query))
            .take(8)
            .cloned()
            .collect::<Vec<_>>();

        self.search_panel_file_cache_query = query;
        self.search_panel_file_cache_project_path = project_path;
        self.search_panel_file_cache_revision = self.file_index_revision;
        self.search_panel_file_cache_results = results;
    }

    /// 模块内可见函数，执行 cached_search_panel_file_results 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn cached_search_panel_file_results(&self) -> &[String] {
        &self.search_panel_file_cache_results
    }

    /// 模块内可见函数，执行 refresh_file_search_cache 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn refresh_file_search_cache(&mut self) {
        let query = Self::normalize_file_search_query(&self.file_search_query);
        let project_path = self.project_path.clone();

        if self.file_search_cache_query == query
            && self.file_search_cache_project_path == project_path
            && self.file_search_cache_revision == self.file_index_revision
        {
            return;
        }

        let results = crate::app::message::chat::input::build_ranked_file_search_entries(
            self.current_file_index(),
            &query,
        );

        self.file_search_cache_query = query;
        self.file_search_cache_project_path = project_path;
        self.file_search_cache_revision = self.file_index_revision;
        self.file_search_cache_entries = results;
    }

    /// 模块内可见函数，执行 cached_file_search_entries 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn cached_file_search_entries(
        &self,
    ) -> &[crate::app::message::chat::input::FileSearchResult] {
        &self.file_search_cache_entries
    }

    /// 模块内可见函数，执行 set_file_index 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn set_file_index(&mut self, path: &str, files: Vec<String>) {
        let tree = crate::app::components::file_tree::model::build_file_tree_model(path, &files);
        self.file_index_cache.insert(path.to_string(), files);
        self.file_tree_model_cache.insert(path.to_string(), tree);
        self.file_index_revision = self.file_index_revision.wrapping_add(1);
        self.refresh_search_panel_file_cache();
        self.refresh_file_search_cache();
    }

    /// 一次发布网关文件列表和目录树；后台项目结果仅更新自己的缓存。
    pub(crate) fn set_file_tree_snapshot(
        &mut self,
        path: &str,
        snapshot: crate::app::message::project::helpers::FileTreeIndexSnapshot,
    ) {
        self.file_index_cache.insert(path.to_string(), snapshot.files);
        self.file_tree_model_cache.insert(path.to_string(), snapshot.tree);
        self.file_index_revision = self.file_index_revision.wrapping_add(1);
        if self.project_path.as_deref() == Some(path) {
            self.refresh_search_panel_file_cache();
            self.refresh_file_search_cache();
        }
    }

    /// 模块内可见函数，执行 has_file_index 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn has_file_index(&self, path: &str) -> bool {
        self.file_index_cache.contains_key(path)
    }

    /// 模块内可见函数，执行 can_open_external 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn can_open_external(&self, target: ExternalOpenApp) -> bool {
        self.open_external_exists.get(&target).copied().unwrap_or(false)
    }

    /// 公开函数，执行 push_notification 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn push_notification(&mut self, message: String) {
        let id = self.next_notification_id;
        self.next_notification_id += 1;
        self.notifications.push(crate::app::state::Notification {
            localized_message: None,
            id,
            message,
            created_at: {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    std::time::SystemTime::now()
                }
                #[cfg(target_arch = "wasm32")]
                {
                    web_time::SystemTime::now()
                }
            },
        });
    }

    pub(crate) fn set_localized_input_error(
        &mut self,
        field: crate::app::i18n::input::ErrorField,
        source: &str,
        arguments: Vec<crate::app::i18n::format::Argument>,
    ) {
        let feedback = crate::app::i18n::input::Feedback::new(source, arguments);
        let target = match field {
            crate::app::i18n::input::ErrorField::General => &mut self.error_message,
            crate::app::i18n::input::ErrorField::WorktreeDelete => {
                &mut self.new_session_delete_error
            }
            crate::app::i18n::input::ErrorField::WorktreeReset => &mut self.new_session_reset_error,
        };
        *target = Some(feedback.fallback.clone());
        self.localized_input_errors.insert(field, feedback);
    }
    pub(crate) fn render_input_error(
        &self,
        field: crate::app::i18n::input::ErrorField,
        raw: &str,
    ) -> String {
        self.localized_input_errors
            .get(&field)
            .map(|feedback| feedback.render(crate::app::i18n::I18n::new(self.desktop_locale), raw))
            .unwrap_or_else(|| raw.to_owned())
    }

    pub fn push_localized_notification(
        &mut self,
        source: &str,
        arguments: Vec<crate::app::i18n::format::Argument>,
    ) {
        let message = crate::app::i18n::format::Message::new(source, arguments);
        let fallback = message
            .render(crate::app::i18n::I18n::new(
                vw_config_types::ui::DesktopLocale::SimplifiedChinese,
            ))
            .expect("valid notification template");
        self.push_notification(fallback);
        self.notifications.last_mut().expect("notification was inserted").localized_message =
            Some(message);
    }

    /// 公开函数，执行 show_toast 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn show_toast(
        &mut self,
        kind: ToastKind,
        message: impl Into<String>,
    ) -> iced::Task<crate::app::Message> {
        let id = self.next_toast_id;
        self.next_toast_id += 1;
        self.active_toast =
            Some(Toast { localized_message: None, id, kind, message: message.into() });
        crate::app::message::after(
            Duration::from_millis(2200),
            crate::app::Message::Notification(NotificationMessage::HideToast(id)),
        )
    }

    /// 公开函数，执行 show_success_toast 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn show_localized_toast(
        &mut self,
        kind: ToastKind,
        source: &str,
        arguments: Vec<crate::app::i18n::format::Argument>,
    ) -> iced::Task<crate::app::Message> {
        let message = crate::app::i18n::format::Message::new(source, arguments);
        let fallback = message
            .render(crate::app::i18n::I18n::new(
                vw_config_types::ui::DesktopLocale::SimplifiedChinese,
            ))
            .expect("valid toast template");
        let task = self.show_toast(kind, fallback);
        if let Some(toast) = &mut self.active_toast {
            toast.localized_message = Some(message);
        }
        task
    }

    pub fn show_success_toast(
        &mut self,
        message: impl Into<String>,
    ) -> iced::Task<crate::app::Message> {
        self.show_toast(ToastKind::Success, message)
    }

    /// 公开函数，执行 show_error_toast 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn show_error_toast(
        &mut self,
        message: impl Into<String>,
    ) -> iced::Task<crate::app::Message> {
        self.show_toast(ToastKind::Error, message)
    }

    pub fn show_info_toast(
        &mut self,
        message: impl Into<String>,
    ) -> iced::Task<crate::app::Message> {
        self.show_toast(ToastKind::Info, message)
    }

    pub fn show_warning_toast(
        &mut self,
        message: impl Into<String>,
    ) -> iced::Task<crate::app::Message> {
        self.show_toast(ToastKind::Warning, message)
    }

    /// 公开函数，执行 theme 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn theme(&self) -> Theme {
        self.app_theme.clone()
    }

    pub fn theme_for_window(&self, window: iced::window::Id) -> Theme {
        if self.task_pet_window_id == Some(window) {
            let mut palette = self.app_theme.palette();
            palette.background = Color::TRANSPARENT;
            Theme::custom_with_fn("VibeWindow Pet", palette, |_| *self.app_theme.extended_palette())
        } else {
            self.theme()
        }
    }

    /// 公开函数，执行 effective_editor_theme 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn effective_editor_theme(&self) -> Theme {
        if self.editor_follow_system_theme {
            self.app_theme.clone()
        } else {
            self.editor_theme.clone()
        }
    }

    /// 公开函数，执行 effective_editor_theme_ref 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn effective_editor_theme_ref(&self) -> &Theme {
        if self.editor_follow_system_theme { &self.app_theme } else { &self.editor_theme }
    }
}

#[cfg(test)]
#[path = "app_basic_tests.rs"]
mod app_basic_tests;
