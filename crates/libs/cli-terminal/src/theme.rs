//! CLI/TUI 共享调色板。
//!
//! 这里集中定义终端界面使用的显式 RGB 颜色，避免依赖终端主题里的 ANSI 灰色，
//! 导致在暗色终端下出现低对比度文本。

use ratatui::style::Color;

pub(crate) const SCANLINE_DARK: Color = Color::Rgb(7, 16, 25);
pub(crate) const SCANLINE_LIGHT: Color = Color::Rgb(10, 20, 32);
pub(crate) const SURFACE_BASE: Color = Color::Rgb(8, 18, 28);
pub(crate) const SURFACE_ELEVATED: Color = Color::Rgb(13, 30, 43);

pub(crate) const TEXT_PRIMARY: Color = Color::Rgb(234, 240, 247);
pub const TEXT_MUTED: Color = Color::Rgb(185, 198, 214);
pub const TEXT_SUBTLE: Color = Color::Rgb(144, 160, 180);

pub const ACCENT_CYAN: Color = Color::Rgb(128, 223, 255);
pub const ACCENT_RED: Color = Color::Rgb(255, 126, 126);
pub const SUCCESS: Color = Color::Rgb(111, 219, 165);
pub const WARNING: Color = Color::Rgb(255, 208, 110);

pub(crate) const EXECUTION_DOT: Color = Color::Rgb(108, 127, 149);
pub const PENDING_BADGE_BG: Color = Color::Rgb(78, 96, 118);
pub const PENDING_BADGE_FG: Color = Color::Rgb(239, 244, 250);
pub(crate) const SCROLLBAR_TRACK: Color = Color::Rgb(80, 98, 120);
pub(crate) const SCROLLBAR_THUMB: Color = Color::Rgb(138, 228, 255);
