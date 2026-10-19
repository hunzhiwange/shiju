//! 工作流视图样式模块，集中定义画布、面板、弹窗和校验提示的容器样式。

use super::*;

/// 提供 floating panel style 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn floating_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        border: Border { width: 1.0, color: palette.background.weak.color, radius: 12.0.into() },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(0.12),
            offset: Vector::new(0.0, 10.0),
            blur_radius: 22.0,
        },
        ..Default::default()
    }
}

/// 提供 root style 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn root_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark_theme(theme) {
            palette.background.base.color.scale_alpha(0.98)
        } else {
            Color::from_rgba8(244, 247, 252, 1.0)
        })),
        ..Default::default()
    }
}

/// 提供 canvas panel style 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn canvas_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            palette.background.base.color.scale_alpha(0.94)
        } else {
            Color::from_rgba8(251, 253, 255, 0.98)
        })),
        border: Border {
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.82)
            } else {
                Color::from_rgba8(15, 23, 42, 0.08)
            },
            width: 1.0,
            radius: 24.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.10 } else { 0.05 }),
            offset: Vector::new(0.0, 10.0),
            blur_radius: 24.0,
        },
        ..Default::default()
    }
}

/// 提供 inspector style 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn inspector_style(theme: &Theme) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    style.background = Some(Background::Color(if is_dark_theme(theme) {
        theme.extended_palette().background.base.color.scale_alpha(0.88)
    } else {
        Color::from_rgba8(255, 255, 255, 0.92)
    }));
    style.border.radius = 20.0.into();
    style
}

/// 提供 value card style 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn value_card_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            palette.background.base.color.scale_alpha(0.62)
        } else {
            Color::WHITE.scale_alpha(0.90)
        })),
        border: Border {
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.72)
            } else {
                Color::from_rgba8(15, 23, 42, 0.06)
            },
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.04 } else { 0.02 }),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..Default::default()
    }
}

/// 提供 modal backdrop style 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn modal_backdrop_style(theme: &Theme) -> iced::widget::container::Style {
    settings_modal_backdrop_style(theme)
}

/// 提供 modal card style 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn modal_card_style(theme: &Theme) -> iced::widget::container::Style {
    settings_modal_card_style(theme)
}

/// 节点编辑器作为覆盖画布的右侧抽屉展示，不使用模态圆角与遮罩。
pub(super) fn node_editor_drawer_style(theme: &Theme) -> iced::widget::container::Style {
    let mut style = settings_modal_card_style(theme);
    style.border.radius = 0.0.into();
    style.shadow = Shadow {
        color: Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.28 } else { 0.16 }),
        offset: Vector::new(-8.0, 0.0),
        blur_radius: 24.0,
    };
    style
}

/// 提供 validation summary style 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub(super) fn validation_summary_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(palette.danger.weak.color.scale_alpha(0.20))),
        border: Border {
            color: palette.danger.base.color.scale_alpha(0.45),
            width: 1.0,
            radius: 16.0.into(),
        },
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "styles_tests.rs"]
mod styles_tests;
