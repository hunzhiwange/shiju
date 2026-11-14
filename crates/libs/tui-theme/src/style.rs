//! TUI 各展示面共享的语义颜色与控件样式。
//! 颜色选择尊重终端背景与既有调色板，避免硬编码造成浅色主题不可读。

use crate::color::blend;
use crate::color::is_light;
use crate::terminal_palette::StdoutColorLevel;
use crate::terminal_palette::best_color;
use crate::terminal_palette::default_bg;
use crate::terminal_palette::default_fg;
use crate::terminal_palette::effective_stdout_color_level;
use crate::terminal_palette::rgb_color;
use crate::terminal_palette::stdout_color_level;
use ratatui::style::Color;
use ratatui::style::Style;

const LIGHT_BG_ACCENT_RGB: (u8, u8, u8) = (0, 95, 135);

#[derive(Clone, Copy)]
pub enum StatusTone {
    Success,
    Attention,
    Failure,
}

/// Semantic status colors that preserve the terminal's configured palette.
/// 保留终端配置调色板的语义状态颜色。
pub fn status_style(tone: StatusTone) -> Style {
    status_style_for(tone, default_bg(), effective_stdout_color_level())
}

fn status_style_for(
    tone: StatusTone,
    terminal_bg: Option<(u8, u8, u8)>,
    color_level: StdoutColorLevel,
) -> Style {
    let light = terminal_bg.is_some_and(is_light);
    let color = match (tone, color_level) {
        (_, StdoutColorLevel::Unknown) => Color::Reset,
        (StatusTone::Success, _) => Color::Green,
        (StatusTone::Failure, _) => Color::Red,
        // Yellow can disappear on light themes; use it only with a known dark background.
        // 黄色在浅色主题上可能不可见，只在确认暗色背景时使用。
        (StatusTone::Attention, _) if light || terminal_bg.is_none() => Color::Reset,
        (StatusTone::Attention, _) => Color::Yellow,
    };
    Style::default().fg(color).bold()
}
// Decorative table rules should remain visible without competing with cell content.
// 表格装饰线应可见但不能抢过单元格内容的视觉层级。
const TABLE_SEPARATOR_FG_ALPHA: f32 = 0.20;

pub fn user_message_style() -> Style {
    user_message_style_for(default_bg())
}

pub fn proposed_plan_style() -> Style {
    proposed_plan_style_for(default_bg())
}

/// Returns a low-contrast rule style for separators within markdown tables.
/// 返回 Markdown 表格分隔线使用的低对比度样式。
pub fn table_separator_style() -> Style {
    table_separator_style_for(default_fg(), default_bg(), stdout_color_level())
}

/// Returns the shared accent style for active or selected TUI controls.
/// 返回活动或选中控件共享的强调样式。
pub fn accent_style() -> Style {
    accent_style_for(default_bg())
}

/// Returns the style for a user-authored message using the provided terminal background.
/// 根据给定终端背景返回用户消息样式，确保文本对比度。
pub fn user_message_style_for(terminal_bg: Option<(u8, u8, u8)>) -> Style {
    match terminal_bg {
        Some(bg) => Style::default().bg(user_message_bg(bg)),
        None => Style::default(),
    }
}

pub fn proposed_plan_style_for(terminal_bg: Option<(u8, u8, u8)>) -> Style {
    match terminal_bg {
        Some(bg) => Style::default().bg(proposed_plan_bg(bg)),
        None => Style::default(),
    }
}

/// Returns the shared accent style for the provided terminal background.
/// 根据给定终端背景返回共享强调色样式。
// 其余轻量辅助函数统一复用这些语义入口，避免各组件自行选择不一致颜色。
pub fn accent_style_for(terminal_bg: Option<(u8, u8, u8)>) -> Style {
    if terminal_bg.is_some_and(is_light) {
        Style::default().fg(best_color(LIGHT_BG_ACCENT_RGB)).bold()
    } else {
        Style::default().fg(Color::Cyan).bold()
    }
}

fn table_separator_style_for(
    terminal_fg: Option<(u8, u8, u8)>,
    terminal_bg: Option<(u8, u8, u8)>,
    color_level: StdoutColorLevel,
) -> Style {
    let (Some(fg), Some(bg)) = (terminal_fg, terminal_bg) else {
        return Style::default().dim();
    };
    let separator_rgb = blend(fg, bg, TABLE_SEPARATOR_FG_ALPHA);
    match color_level {
        StdoutColorLevel::TrueColor => Style::default().fg(rgb_color(separator_rgb)),
        StdoutColorLevel::Ansi256 => Style::default().fg(best_color(separator_rgb)),
        StdoutColorLevel::Ansi16 | StdoutColorLevel::Unknown => Style::default().dim(),
    }
}

#[allow(clippy::disallowed_methods)]
pub fn user_message_bg(terminal_bg: (u8, u8, u8)) -> Color {
    best_color(user_message_bg_rgb(terminal_bg))
}

pub fn user_message_bg_rgb(terminal_bg: (u8, u8, u8)) -> (u8, u8, u8) {
    let (top, alpha) =
        if is_light(terminal_bg) { ((0, 0, 0), 0.04) } else { ((255, 255, 255), 0.12) };
    blend(top, terminal_bg, alpha)
}

#[allow(clippy::disallowed_methods)]
pub fn proposed_plan_bg(terminal_bg: (u8, u8, u8)) -> Color {
    user_message_bg(terminal_bg)
}

#[cfg(test)]
#[path = "style_tests.rs"]
mod tests;
