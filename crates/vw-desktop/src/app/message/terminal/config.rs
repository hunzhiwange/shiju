//! 管理终端相关消息、配置和标签页状态更新。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::TerminalMessage;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::terminal::build_palette;
use crate::app::{App, Message, Shell, TerminalState, TerminalTheme};
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

/// 合并连续输入，并保证同一终端设置最多只有一个在途写入。
#[derive(Default)]
pub(crate) struct ConfigPersistence {
    pending: serde_json::Map<String, serde_json::Value>,
    generation: u64,
    saving: bool,
}

impl ConfigPersistence {
    fn enqueue(&mut self, update: TerminalConfigUpdate) -> u64 {
        let (key, value) = match update {
            TerminalConfigUpdate::Shell(value) => ("terminal_shell", serde_json::json!(value)),
            TerminalConfigUpdate::Theme(value) => ("terminal_theme", serde_json::json!(value)),
            TerminalConfigUpdate::FontFamily(value) => {
                ("terminal_font_family", serde_json::json!(value))
            }
            TerminalConfigUpdate::FontSize(value) => {
                ("terminal_font_size", serde_json::json!(value))
            }
        };
        self.pending.insert(key.into(), value);
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }

    fn begin(&mut self, generation: u64) -> Option<serde_json::Value> {
        if generation != self.generation || self.saving || self.pending.is_empty() {
            return None;
        }
        self.saving = true;
        Some(serde_json::Value::Object(std::mem::take(&mut self.pending)))
    }
}

pub fn update(app: &mut App, message: TerminalMessage) -> Task<Message> {
    match message {
        TerminalMessage::ConfigSaveDue(generation) => {
            let Some(patch) = app.terminal_config_persistence.begin(generation) else {
                return Task::none();
            };
            let client = crate::app::config::gateway_client();
            return Task::perform(
                async move { client?.desktop_system_settings_patch(&patch).await },
                |result| Message::Terminal(TerminalMessage::ConfigSaved(result)),
            );
        }
        TerminalMessage::ConfigSaved(result) => {
            let persistence = &mut app.terminal_config_persistence;
            persistence.saving = false;
            let next = Task::done(Message::Terminal(TerminalMessage::ConfigSaveDue(
                persistence.generation,
            )));
            return match result {
                Ok(()) => next,
                Err(error) => {
                    Task::batch([next, app.show_error_toast(format!("终端配置保存失败：{error}"))])
                }
            };
        }
        _ => {}
    }
    if let Some(update) = apply_update(&mut app.terminal, &app.app_theme, message) {
        let generation = app.terminal_config_persistence.enqueue(update);
        return crate::app::message::after(
            std::time::Duration::from_millis(200),
            Message::Terminal(TerminalMessage::ConfigSaveDue(generation)),
        );
    }
    Task::none()
}
#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
