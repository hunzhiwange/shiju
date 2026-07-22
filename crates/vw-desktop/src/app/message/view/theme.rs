//! 处理应用视图级别的主题或布局消息。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::ViewMessage;
#[cfg(not(test))]
use crate::app::update_system_settings_config;
use crate::app::{App, Message, TerminalTheme};
use iced::Task;
use iced_code_editor::theme;

#[cfg(not(test))]
fn persist_app_theme(app_theme: String) {
    update_system_settings_config(|system| {
        system.app_theme = app_theme;
    });
}

#[cfg(test)]
fn persist_app_theme(_app_theme: String) {}

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::AppThemeSelected(theme) => {
            app.app_theme = theme.clone();
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

            let app_theme = app.app_theme.to_string();
            persist_app_theme(app_theme);
            Task::none()
        }
        _ => Task::none(),
    }
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
