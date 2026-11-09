//! 状态动画辅助模块
//!
//! 提供运行中状态共用的动画帧与探索摘要数字翻转时长常量。

use std::fmt::Write as _;
use std::sync::LazyLock;

use iced::widget::{Container, Space, Svg, container, svg};
use iced::{Color, border};

/// 共用活动动画的刷新间隔，避免请求期间的动画刷新挤占 UI 更新。
pub const ACTIVITY_ANIMATION_INTERVAL_MS: u64 = 90;

/// 与文件管理器刷新按钮一致的帧动画序列。
pub const STATUS_SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// 探索摘要数字翻转动画持续时长（毫秒）。
pub const EXPLORE_SUMMARY_FLIP_DURATION_MS: u64 = 1_100;

/// 根据动画帧序号返回当前应显示的旋转字符。
pub fn spinner_frame(frame: usize) -> &'static str {
    STATUS_SPINNER_FRAMES[frame % STATUS_SPINNER_FRAMES.len()]
}

static SPINNER_ICONS: LazyLock<[svg::Handle; STATUS_SPINNER_FRAMES.len()]> = LazyLock::new(|| {
    STATUS_SPINNER_FRAMES.map(|frame| {
        let dots = frame.chars().next().expect("spinner frame has one character") as u32 - 0x2800;
        let mut source =
            String::from(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 7 10">"#);
        // Braille dots 1–3 form the left column and 4–6 form the right column.
        for (bit, (x, y)) in
            [(2, 2), (2, 5), (2, 8), (5, 2), (5, 5), (5, 8)].into_iter().enumerate()
        {
            if dots & (1 << bit) != 0 {
                write!(source, r#"<circle cx="{x}" cy="{y}" r="1"/>"#)
                    .expect("writing to a string succeeds");
            }
        }
        source.push_str("</svg>");
        svg::Handle::from_memory(source.into_bytes())
    })
});

/// 绘制运行动画，使点阵中心与行中心对齐，不受字体基线影响。
pub(crate) fn activity_spinner(frame: usize, size: f32) -> Svg<'static> {
    svg(SPINNER_ICONS[frame % SPINNER_ICONS.len()].clone()).width(size * 0.7).height(size)
}

pub(crate) fn session_spinner(frame: usize, size: f32, color: Color) -> Svg<'static> {
    activity_spinner(frame, size).style(move |_, _| svg::Style { color: Some(color) })
}

/// 绘制颜色随会话状态变化的圆点。
pub(crate) fn session_running_dot<Message: 'static>(color: Color) -> Container<'static, Message> {
    container(Space::new())
        .width(8)
        .height(8)
        .style(move |_| container::background(color).border(border::rounded(4)))
}
