//! 处理应用视图级别的主题或布局消息。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::ViewMessage;
use crate::app::config::SystemSettingsSection;
use crate::app::theme_catalog::{
    config_value_for_theme, find_custom_theme_id, theme_store_from_home,
};
use crate::app::{App, Message, TerminalTheme};
use iced::Task;
use iced::Theme;
use iced_code_editor::theme;

fn sync_active_custom_theme_id(app: &mut App) {
    if !matches!(app.app_theme, Theme::Custom(_)) {
        app.active_custom_theme_id = None;
        crate::app::theme_catalog::clear_active_theme_skin(app);
        return;
    }
    if app.active_custom_theme_id.is_none() {
        if let Some(store) = theme_store_from_home() {
            app.active_custom_theme_id = find_custom_theme_id(&app.app_theme, &store);
        }
    }
    crate::app::theme_catalog::sync_active_theme_skin(app);
}

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::AppThemeSelected(theme) => {
            app.app_theme = theme.clone();
            sync_active_custom_theme_id(app);
            if app.editor_follow_system_theme {
                let editor_theme = app.effective_editor_theme();
                for tab in app.preview_tabs.iter_mut() {
                    tab.editor.set_theme(editor_theme.clone());
                }
                app.git_copy_modal_code_editor.set_theme(theme::from_iced_theme(&editor_theme));
            }

            if app.terminal.theme == TerminalTheme::System {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    app.terminal.apply_app_theme(&app.app_theme);
                }
            }

            let app_theme =
                config_value_for_theme(&app.app_theme, app.active_custom_theme_id.as_deref());
            app.save_system_settings_fields(
                SystemSettingsSection::Theme,
                serde_json::Map::from_iter([("app_theme".into(), app_theme.into())]),
            )
        }
        _ => Task::none(),
    }
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
