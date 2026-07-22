//! 管理终端相关消息、配置和标签页状态更新。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::TerminalMessage;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::terminal::build_palette;
use crate::app::{
    App, Message, Shell, TerminalState, TerminalTheme, update_system_settings_config,
};
use iced::Task;

enum TerminalConfigUpdate {
    Shell(&'static str),
    Theme(&'static str),
    FontFamily(String),
    FontSize(f32),
}

fn apply_update(
    terminal: &mut TerminalState,
    _app_theme: &iced::Theme,
    message: TerminalMessage,
) -> Option<TerminalConfigUpdate> {
    match message {
        TerminalMessage::ShellSelected(shell) => {
            terminal.shell = shell;
            let key = match shell {
                Shell::Bash => "bash",
                Shell::Zsh => "zsh",
            };
            Some(TerminalConfigUpdate::Shell(key))
        }
        TerminalMessage::ThemeSelected(theme) => {
            terminal.theme = theme;

            #[cfg(not(target_arch = "wasm32"))]
            {
                let palette = build_palette(_app_theme);
                for tab in &mut terminal.tabs {
                    let _ =
                        tab.term.handle(iced_term::Command::ChangeTheme(Box::new(palette.clone())));
                }
            }

            let key = match theme {
                TerminalTheme::System => "system",
            };
            Some(TerminalConfigUpdate::Theme(key))
        }
        TerminalMessage::FontSelected(family) => {
            terminal.set_font_family(family.clone());
            Some(TerminalConfigUpdate::FontFamily(family))
        }
        TerminalMessage::FontSizeChanged(size) => {
            terminal.set_font_size(size);
            Some(TerminalConfigUpdate::FontSize(size))
        }
        _ => None,
    }
}

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: TerminalMessage) -> Task<Message> {
    if let Some(config_update) = apply_update(&mut app.terminal, &app.app_theme, message) {
        match config_update {
            TerminalConfigUpdate::Shell(shell_key) => {
                update_system_settings_config(|system| {
                    system.terminal_shell = shell_key.to_string();
                });
            }
            TerminalConfigUpdate::Theme(theme_key) => {
                update_system_settings_config(|system| {
                    system.terminal_theme = theme_key.to_string();
                });
            }
            TerminalConfigUpdate::FontFamily(family) => {
                update_system_settings_config(|system| {
                    system.terminal_font_family = family;
                });
            }
            TerminalConfigUpdate::FontSize(size) => {
                update_system_settings_config(|system| {
                    system.terminal_font_size = size;
                });
            }
        }
    }

    Task::none()
}
#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
