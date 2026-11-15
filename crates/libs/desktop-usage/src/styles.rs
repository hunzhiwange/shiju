//! 用量视图样式集合，统一维护暗色主题下的文本、卡片和交互控件外观。

use iced::{Background, Border, Color, Theme, Vector};

/// 构建或更新 card style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn card_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(p.background.base.color)),
        border: Border { width: 1.0, color: p.background.strong.color, radius: 12.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.10),
            offset: Vector::new(0.0, 10.0),
            blur_radius: 30.0,
        },
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "styles_tests.rs"]
mod styles_tests;
