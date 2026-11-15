use crate::{Shell, TerminalTheme};
use super::{TerminalMessage, Effect};
#[cfg(not(target_arch = "wasm32"))]
use crate::build_palette;
type TerminalState = crate::TerminalState<Shell>;
// 终端偏好更新、字号输入合并与异步持久化。

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
pub struct ConfigPersistence {
    pending: serde_json::Map<String, serde_json::Value>,
    generation: u64,
    saving: bool,
    font_size_preview: Option<f32>,
    font_size_generation: u64,
}

impl ConfigPersistence {
    pub fn displayed_font_size(&self, applied: f32) -> f32 {
        self.font_size_preview.unwrap_or(applied)
    }

    fn preview_font_size(&mut self, size: f32) -> u64 {
        self.font_size_preview = Some(size);
        self.font_size_generation = self.font_size_generation.wrapping_add(1);
        self.font_size_generation
    }

    fn take_font_size_preview(&mut self, generation: Option<u64>) -> Option<f32> {
        if generation.is_some_and(|value| value != self.font_size_generation) {
            return None;
        }
        self.font_size_preview.take()
    }

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

pub(super) fn update(
    terminal: &mut TerminalState,
    persistence: &mut super::config::ConfigPersistence,
    theme: &iced::Theme,
    effects: &mut Vec<Effect>,
    mut message: TerminalMessage,
) -> Task<TerminalMessage> {
    match message {
        TerminalMessage::FontSizePreview(size) => {
            // 拖动期间仅更新滑块；ChangeFont 会重排 PTY 历史并复制网格，不能逐像素执行。
            let generation = persistence.preview_font_size(size);
            return super::after(effects,
                std::time::Duration::from_millis(200),
                TerminalMessage::FontSizePreviewCommitDue(generation),
            );
        }
        TerminalMessage::FontSizePreviewCommit | TerminalMessage::FontSizePreviewCommitDue(_) => {
            let generation = match message {
                TerminalMessage::FontSizePreviewCommitDue(value) => Some(value),
                _ => None,
            };
            let Some(size) = persistence.take_font_size_preview(generation)
            else {
                return Task::none();
            };
            if size == terminal.font_size {
                return Task::none();
            }
            message = TerminalMessage::FontSizeChanged(size);
        }
        TerminalMessage::FontSizeChanged(_) => {
            persistence.font_size_preview = None;
        }
        TerminalMessage::ConfigSaveDue(generation) => {
            let Some(patch) = persistence.begin(generation) else {
                return Task::none();
            };
            effects.push(Effect::SaveConfig(patch));
            return Task::none();
        }
        TerminalMessage::ConfigSaved(result) => {
            let persistence = &mut *persistence;
            persistence.saving = false;
            let next = Task::done(TerminalMessage::ConfigSaveDue(persistence.generation));
            return match result {
                Ok(()) => next,
                Err(error) => {
                    Task::batch([next, { effects.push(Effect::Error(format!("终端配置保存失败：{error}"))); Task::none() }])
                }
            };
        }
        _ => {}
    }
    if let Some(update) = apply_update(&mut *terminal, theme, message) {
        #[cfg(not(target_arch = "wasm32"))]
        match &update {
            TerminalConfigUpdate::Shell(_) => effects.push(Effect::SettingEdited("terminal_shell")),
            TerminalConfigUpdate::FontFamily(_) => {
                effects.push(Effect::SettingEdited("terminal_font_family"))
            }
            TerminalConfigUpdate::FontSize(_) => {
                effects.push(Effect::SettingEdited("terminal_font_size"))
            }
            TerminalConfigUpdate::Theme(_) => {}
        }
        let generation = persistence.enqueue(update);
        return super::after(effects,
            std::time::Duration::from_millis(200),
            TerminalMessage::ConfigSaveDue(generation),
        );
    }
    Task::none()
}


#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
