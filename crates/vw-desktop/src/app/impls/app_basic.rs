//! 实现 App 上的基础状态辅助方法。
//! 本模块集中维护展开文件、焦点、提示和轻量 UI 状态，避免主结构体实现过度膨胀。

use iced::Theme;
use std::time::Duration;

use super::state::ExternalOpenApp;
use super::{App, Screen};
use crate::app::message::NotificationMessage;
use crate::app::state::ToastKind;

impl App {
    pub(crate) fn is_chat_fullscreen(&self) -> bool {
        self.navigation_mode.workspace_mode.chat_fullscreen(self.chat_interaction.chat_panel_fullscreen, self.navigation_workspace.work_chat_fullscreen)
    }

    pub(crate) fn is_chat_half_fullscreen(&self) -> bool {
        self.navigation_mode.workspace_mode.chat_fullscreen(self.chat_interaction.chat_panel_half_fullscreen, self.navigation_workspace.work_chat_half_fullscreen)
    }

    fn normalize_file_search_query(query: &str) -> String {
        desktop_session::message::chat::input::file_search::normalize_file_search_query(query)
    }

    /// 模块内可见函数，执行 is_diff_file_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn is_diff_file_expanded(&self, path: &str) -> bool {
        desktop_git_ui::message_git::diff::is_diff_file_expanded(&self.git.expanded_files_set, path)
    }

    /// 模块内可见函数，执行 toggle_diff_file_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    #[allow(dead_code)]
    pub(crate) fn toggle_diff_file_expanded(&mut self, path: String) {
        desktop_git_ui::message_git::diff::toggle_diff_file_expanded(&mut self.git.expanded_files, &mut self.git.expanded_files_set, path)
    }

    /// 模块内可见函数，执行 ensure_diff_file_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn ensure_diff_file_expanded(&mut self, path: String) {
        desktop_git_ui::message_git::diff::ensure_diff_file_expanded(&mut self.git.expanded_files_set, &mut self.git.expanded_files, path)
    }

    /// 模块内可见函数，执行 replace_expanded_files 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    #[allow(dead_code)]
    pub(crate) fn replace_expanded_files(&mut self, files: Vec<String>) {
        desktop_git_ui::message_git::diff::replace_expanded_files(&mut self.git.expanded_files, &mut self.git.expanded_files_set, files)
    }

    /// 模块内可见函数，执行 clear_expanded_files 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn clear_expanded_files(&mut self) {
        desktop_git_ui::message_git::diff::clear_expanded_files(&mut self.git.expanded_files, &mut self.git.expanded_files_set)
    }

    /// 模块内可见函数，执行 set_single_expanded_file 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn set_single_expanded_file(&mut self, path: String) {
        desktop_git_ui::message_git::diff::set_single_expanded_file(&mut self.git.expanded_files, &mut self.git.expanded_files_set, path)
    }

    /// 模块内可见函数，执行 is_file_tree_dir_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn is_file_tree_dir_expanded(&self, path: &str) -> bool {
        self.workspace_state.file_tree_expanded_set.contains(path)
    }

    /// 模块内可见函数，执行 toggle_file_tree_dir_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn toggle_file_tree_dir_expanded(&mut self, path: String) {
        desktop_file_tree::state::toggle_file_tree_dir_expanded(&mut self.workspace_state.file_tree_expanded, &mut self.workspace_state.file_tree_expanded_set, path)
    }

    /// 模块内可见函数，执行 ensure_file_tree_dir_expanded 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn ensure_file_tree_dir_expanded(&mut self, path: String) {
        desktop_file_tree::state::ensure_file_tree_dir_expanded(&mut self.workspace_state.file_tree_expanded, &mut self.workspace_state.file_tree_expanded_set, path)
    }

    /// 公开函数，执行 title 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn title(&self) -> String {
        desktop_navigation::window_lifecycle::title(self.navigation_tabs.screen, self.general_settings.desktop_locale)
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
        self.workspace_state.current_file_index(self.navigation_tabs.project_path.as_deref())
    }

    /// 模块内可见函数，执行 current_file_tree_model 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn current_file_tree_model(
        &self,
    ) -> Option<&crate::app::components::file_tree::model::FileTreeNode> {
        self.navigation_tabs.project_path.as_ref().and_then(|path| self.workspace_state.file_tree_model_cache.get(path))
    }

    /// 模块内可见函数，执行 refresh_search_panel_file_cache 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn refresh_search_panel_file_cache(&mut self) {
        self.workspace_state.refresh_search_panel(
            self.navigation_tabs.project_path.as_deref(),
            Self::normalize_file_search_query(&self.search.search_text),
        );
    }

    /// 模块内可见函数，执行 cached_search_panel_file_results 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn cached_search_panel_file_results(&self) -> &[String] {
        &self.workspace_state.search_panel_file_cache_results
    }

    /// 模块内可见函数，执行 refresh_file_search_cache 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn refresh_file_search_cache(&mut self) {
        let mut state = std::mem::take(&mut self.input_suggestions);
        state.refresh_file_cache(self.current_file_index(), self.navigation_tabs.project_path.as_deref(), self.workspace_state.file_index_revision);
        self.input_suggestions = state;
    }

    /// 模块内可见函数，执行 cached_file_search_entries 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn cached_file_search_entries(
        &self,
    ) -> &[crate::app::message::chat::input::FileSearchResult] {
        &self.input_suggestions.file_search_cache_entries
    }

    /// 模块内可见函数，执行 set_file_index 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn set_file_index(&mut self, path: &str, files: Vec<String>) {
        self.workspace_state.set_file_index(path, files);
        self.refresh_search_panel_file_cache();
        self.refresh_file_search_cache();
    }

    /// 一次发布网关文件列表和目录树；后台项目结果仅更新自己的缓存。
    pub(crate) fn set_file_tree_snapshot(
        &mut self,
        path: &str,
        snapshot: crate::app::message::project::helpers::FileTreeIndexSnapshot,
    ) {
        self.workspace_state.set_file_tree(path, snapshot.files, snapshot.tree);
        if self.navigation_tabs.project_path.as_deref() == Some(path) {
            self.refresh_search_panel_file_cache();
            self.refresh_file_search_cache();
        }
    }

    /// 模块内可见函数，执行 has_file_index 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn has_file_index(&self, path: &str) -> bool {
        self.workspace_state.file_index_cache.contains_key(path)
    }

    /// 模块内可见函数，执行 can_open_external 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub(crate) fn can_open_external(&self, target: ExternalOpenApp) -> bool {
        desktop_navigation::external_apps_update::can_open_external(
            &self.external_navigation.open_external_exists,
            target,
        )
    }

    /// 公开函数，执行 push_notification 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn push_notification(&mut self, message: String) {
        desktop_navigation::components::notification::push_notification(
            &mut self.notification_state.notifications, &mut self.notification_state.next_notification_id, message,
        );
    }

    pub(crate) fn set_localized_input_error(
        &mut self,
        field: crate::app::i18n::input::ErrorField,
        source: &str,
        arguments: Vec<crate::app::i18n::format::Argument>,
    ) {
        let feedback = crate::app::i18n::input::Feedback::new(source, arguments);
        let target = match field {
            crate::app::i18n::input::ErrorField::General => &mut self.input_feedback.error_message,
            crate::app::i18n::input::ErrorField::WorktreeDelete => {
                &mut self.workspace_state.new_session_delete_error
            }
            crate::app::i18n::input::ErrorField::WorktreeReset => {
                &mut self.workspace_state.new_session_reset_error
            }
        };
        *target = Some(feedback.fallback.clone());
        self.input_feedback.localized_input_errors.insert(field, feedback);
    }
    pub(crate) fn render_input_error(
        &self,
        field: crate::app::i18n::input::ErrorField,
        raw: &str,
    ) -> String {
        self.input_feedback.render_error(
            field,
            crate::app::i18n::I18n::new(self.general_settings.desktop_locale),
            raw,
        )
    }

    pub fn push_localized_notification(
        &mut self,
        source: &str,
        arguments: Vec<crate::app::i18n::format::Argument>,
    ) {
        desktop_navigation::components::notification::push_localized_notification(
            &mut self.notification_state.notifications, &mut self.notification_state.next_notification_id, source, arguments,
        );
    }

    /// 公开函数，执行 show_toast 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn show_toast(
        &mut self,
        kind: ToastKind,
        message: impl Into<String>,
    ) -> iced::Task<crate::app::Message> {
        let id = desktop_ui_controls::toast::show_toast(
            &mut self.toast_state.active_toast, &mut self.toast_state.next_toast_id, kind, message.into(),
        );
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
        let (fallback, message) = desktop_ui_controls::toast::localized_toast_message(source, arguments);
        let task = self.show_toast(kind, fallback);
        if let Some(toast) = &mut self.toast_state.active_toast { toast.localized_message = Some(message); }
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
        self.appearance.app_theme.clone()
    }

    pub fn theme_for_window(&self, window: iced::window::Id) -> Theme {
        desktop_navigation::window_lifecycle::theme_for_surface(
            &self.appearance.app_theme,
            self.task_pet_window_id == Some(window),
        )
    }

    /// Clear the native Mini surface without making menus or form controls transparent.
    pub fn window_style(_: &Self, theme: &Theme) -> iced::theme::Style {
        desktop_navigation::window_lifecycle::window_style(theme)
    }

    /// 公开函数，执行 effective_editor_theme 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn effective_editor_theme(&self) -> Theme {
        self.appearance.effective_editor_theme().clone()
    }

    /// 公开函数，执行 effective_editor_theme_ref 对应的应用流程。
    /// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
    pub fn effective_editor_theme_ref(&self) -> &Theme {
        self.appearance.effective_editor_theme()
    }
}

#[cfg(test)]
#[path = "app_basic_tests.rs"]
mod app_basic_tests;
