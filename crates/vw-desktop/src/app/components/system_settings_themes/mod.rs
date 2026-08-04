//! 系统设置「主题」页：画廊列表与编辑器。

pub mod editor;
pub mod gallery;
pub mod preview;

#[cfg(test)]
#[path = "gallery_tests.rs"]
mod gallery_tests;

use crate::app::state::ThemesSettingsMode;
use crate::app::{App, Message};
use iced::Element;

/// 按模式渲染主题页（画廊或编辑器）。
pub fn view(app: &App) -> Element<'_, Message> {
    match app.themes_settings.mode {
        ThemesSettingsMode::List => gallery::view(app),
        ThemesSettingsMode::Edit => editor::view(app),
    }
}
