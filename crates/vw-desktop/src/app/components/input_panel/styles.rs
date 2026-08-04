//! 输入面板样式定义模块
//!
//! 本模块提供输入面板组件的各种视觉样式函数，用于定义按钮、容器、编辑器等 UI 元素的外观。
//! 所有样式函数均基于 iced 框架的主题系统，支持响应式主题切换和状态变化。

use iced::widget::{button, container, text_editor};
use iced::{Background, Border, Color, Theme};

use crate::app::theme_catalog::ChromeStyleOverrides;

pub const BOTTOM_BAR_ICON_BUTTON_SIZE: f32 = 24.0;
pub const BOTTOM_BAR_ICON_SIZE: f32 = 12.0;
pub const BOTTOM_BAR_LARGE_ICON_SIZE: f32 = 14.0;
pub const BOTTOM_BAR_CHEVRON_ICON_SIZE: f32 = 11.0;
pub const BOTTOM_BAR_LABEL_SIZE: f32 = 12.0;

fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

/// 快捷消息条中的紧凑按钮样式。
pub fn quick_message_button_style(
    theme: &Theme,
    status: button::Status,
    is_action: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let palette = theme.extended_palette();
    let neutral = if is_dark {
        palette.background.weak.color
    } else {
        Color::from_rgba8(246, 248, 251, 1.0)
    };
    let action = if is_dark {
        palette.primary.weak.color.scale_alpha(0.72)
    } else {
        palette.primary.weak.color.scale_alpha(0.58)
    };
    let base = if is_action { action } else { neutral };
    let background = match status {
        button::Status::Hovered => base.scale_alpha(if is_dark { 0.98 } else { 0.94 }),
        button::Status::Pressed => {
            if is_action {
                palette.primary.base.color.scale_alpha(if is_dark { 0.55 } else { 0.35 })
            } else {
                palette.background.strong.color.scale_alpha(if is_dark { 0.82 } else { 0.48 })
            }
        }
        _ => base.scale_alpha(if is_dark { 0.82 } else { 0.78 }),
    };

    button::Style {
        background: Some(Background::Color(background)),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: if is_action {
                palette.primary.base.color.scale_alpha(if is_dark { 0.58 } else { 0.38 })
            } else {
                palette.background.strong.color.scale_alpha(if is_dark { 0.88 } else { 0.52 })
            },
        },
        text_color: theme.palette().text.scale_alpha(if is_dark { 0.94 } else { 0.88 }),
        ..Default::default()
    }
}

/// 快捷消息分组入口样式。
pub fn quick_message_group_button_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let palette = theme.extended_palette();
    let emphasized = active || matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: emphasized
            .then(|| Background::Color(palette.background.weak.color.scale_alpha(0.82))),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if emphasized { 0.78 } else { 0.5 }),
        },
        text_color: theme.palette().text.scale_alpha(0.9),
        ..Default::default()
    }
}

/// 识矩技能菜单中的单选行样式。
pub fn shiju_skill_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let palette = theme.extended_palette();
    let pointed = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = if selected {
        Some(Background::Color(
            theme.palette().primary.scale_alpha(if pointed {
                if is_dark { 0.20 } else { 0.14 }
            } else if is_dark {
                0.16
            } else {
                0.10
            }),
        ))
    } else if pointed {
        Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark { 0.92 } else { 0.78 }),
        ))
    } else {
        None
    };
    let outlined = selected || pointed;

    button::Style {
        background,
        border: Border {
            radius: 7.0.into(),
            width: if outlined { 1.0 } else { 0.0 },
            color: if selected {
                theme.palette().primary.scale_alpha(if is_dark { 0.46 } else { 0.28 })
            } else if pointed {
                palette.background.strong.color.scale_alpha(if is_dark { 0.80 } else { 0.55 })
            } else {
                Color::TRANSPARENT
            },
        },
        text_color: theme.palette().text.scale_alpha(if is_dark { 0.92 } else { 0.88 }),
        ..Default::default()
    }
}

/// 识矩版本差异导航按钮样式。
pub fn shiju_upgrade_button_style(
    theme: &Theme,
    status: button::Status,
    enabled: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let palette = theme.extended_palette();
    let pointed =
        enabled && matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = enabled.then(|| {
        Background::Color(palette.primary.weak.color.scale_alpha(if pointed {
            if is_dark { 0.82 } else { 0.62 }
        } else if is_dark {
            0.66
        } else {
            0.48
        }))
    });

    button::Style {
        background,
        border: Border {
            radius: 7.0.into(),
            width: enabled.then_some(1.0).unwrap_or_default(),
            color: if enabled {
                palette.primary.base.color.scale_alpha(if is_dark { 0.58 } else { 0.38 })
            } else {
                Color::TRANSPARENT
            },
        },
        text_color: theme
            .palette()
            .text
            .scale_alpha(if enabled { 0.94 } else if is_dark { 0.36 } else { 0.42 }),
        ..Default::default()
    }
}

/// 快捷消息悬停菜单表面。
pub fn quick_message_menu_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.86),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.2),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

/// 创建新会话动作标记。
pub fn quick_message_action_badge_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.primary.base.color),
        background: Some(Background::Color(palette.primary.weak.color.scale_alpha(0.48))),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// 生成工具提示的深色样式
///
/// 返回一个半透明深色背景的容器样式，适用于工具提示浮层。
/// 样式包含圆角边框、白色文字和柔和阴影效果。
///
/// # 参数
///
/// * `_theme` - iced 主题引用（当前未使用，预留用于未来主题适配）
///
/// # 返回值
///
/// 返回配置好的 `container::Style` 实例，包含：
/// - 深灰色半透明背景（RGBA: 24, 24, 24, 0.96）
/// - 白色文字
/// - 8px 圆角无边框
/// - 黑色柔和阴影（模糊半径 18px，向下偏移 6px）
pub fn tooltip_dark_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba8(12, 13, 15, 0.97))),
        text_color: Some(Color::WHITE),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: Color::from_rgba8(255, 255, 255, 0.08),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.32),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 20.0,
        },
        ..Default::default()
    }
}

/// 生成方形图标按钮样式
///
/// 根据按钮状态和启用状态返回相应的样式配置。
/// 悬停时显示弱背景色，按下时显示强背景色，支持禁用状态视觉反馈。
///
/// # 参数
///
/// * `theme` - iced 主题引用，用于获取调色板颜色
/// * `status` - 按钮当前状态（Active、Hovered、Pressed、Disabled）
/// * `enabled` - 按钮是否启用，影响文字颜色
///
/// # 返回值
///
/// 返回配置好的 `button::Style` 实例：
/// - 悬停状态：弱背景色（35% 透明度）
/// - 按下状态：强背景色（45% 透明度）
/// - 其他状态：透明背景
/// - 6px 圆角，无边框
/// - 启用时使用主题文字颜色，禁用时使用弱背景文字颜色
pub fn square_icon_button_style(
    theme: &Theme,
    status: button::Status,
    enabled: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let idle_bg = if is_dark {
        Color::from_rgba8(24, 25, 29, 0.92)
    } else {
        Color::from_rgba8(247, 248, 250, 1.0)
    };
    let hover_bg = if is_dark {
        Color::from_rgba8(31, 33, 38, 0.96)
    } else {
        Color::from_rgba8(241, 243, 246, 1.0)
    };
    let pressed_bg = if is_dark {
        Color::from_rgba8(36, 38, 44, 0.98)
    } else {
        Color::from_rgba8(232, 236, 241, 1.0)
    };
    let idle_border = if is_dark {
        Color::from_rgba8(45, 48, 54, 0.95)
    } else {
        Color::from_rgba8(226, 231, 237, 1.0)
    };
    let active_border = if is_dark {
        Color::from_rgba8(63, 67, 75, 0.95)
    } else {
        Color::from_rgba8(210, 216, 224, 1.0)
    };
    let (background, border_color) = match status {
        button::Status::Pressed => (pressed_bg, active_border),
        button::Status::Hovered => (hover_bg, active_border),
        _ => (idle_bg, idle_border),
    };
    button::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 10.0.into(), width: 1.0, color: border_color },
        text_color: if enabled {
            theme.palette().text.scale_alpha(if is_dark { 0.92 } else { 0.88 })
        } else {
            theme.palette().text.scale_alpha(if is_dark { 0.36 } else { 0.42 })
        },
        ..Default::default()
    }
}

/// 生成无边框圆形图标按钮样式
///
/// 提供与发送按钮一致的圆形轮廓，但保持中性背景，不使用描边。
pub fn round_icon_button_style(
    theme: &Theme,
    status: button::Status,
    enabled: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let idle_bg = if is_dark {
        Color::from_rgba8(24, 25, 29, 0.92)
    } else {
        Color::from_rgba8(247, 248, 250, 1.0)
    };
    let hover_bg = if is_dark {
        Color::from_rgba8(31, 33, 38, 0.96)
    } else {
        Color::from_rgba8(241, 243, 246, 1.0)
    };
    let pressed_bg = if is_dark {
        Color::from_rgba8(36, 38, 44, 0.98)
    } else {
        Color::from_rgba8(232, 236, 241, 1.0)
    };
    let background = match status {
        button::Status::Pressed => pressed_bg,
        button::Status::Hovered => hover_bg,
        _ => idle_bg,
    };

    button::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 999.0.into(), width: 0.0, color: Color::TRANSPARENT },
        text_color: if enabled {
            theme.palette().text.scale_alpha(if is_dark { 0.92 } else { 0.88 })
        } else {
            theme.palette().text.scale_alpha(if is_dark { 0.36 } else { 0.42 })
        },
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

pub fn selector_label_font() -> iced::Font {
    iced::Font { weight: iced::font::Weight::Medium, ..Default::default() }
}

pub fn selector_text_color(theme: &Theme, highlighted: bool) -> Color {
    if highlighted {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.97 } else { 0.95 })
    } else {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.92 } else { 0.88 })
    }
}

pub fn selector_chevron_color(theme: &Theme, highlighted: bool) -> Color {
    if highlighted {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.72 } else { 0.68 })
    } else {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.58 } else { 0.62 })
    }
}

pub fn selector_pill_button_style(
    theme: &Theme,
    status: button::Status,
    highlighted: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let idle_bg = if highlighted {
        if is_dark {
            Color::from_rgba8(28, 30, 35, 0.97)
        } else {
            Color::from_rgba8(244, 246, 249, 1.0)
        }
    } else if is_dark {
        Color::from_rgba8(24, 25, 29, 0.92)
    } else {
        Color::from_rgba8(248, 249, 251, 1.0)
    };
    let hover_bg = if highlighted {
        if is_dark {
            Color::from_rgba8(34, 36, 42, 0.98)
        } else {
            Color::from_rgba8(238, 241, 245, 1.0)
        }
    } else if is_dark {
        Color::from_rgba8(31, 33, 38, 0.96)
    } else {
        Color::from_rgba8(241, 243, 246, 1.0)
    };
    let pressed_bg = if highlighted {
        if is_dark {
            Color::from_rgba8(38, 40, 46, 1.0)
        } else {
            Color::from_rgba8(232, 236, 241, 1.0)
        }
    } else if is_dark {
        Color::from_rgba8(36, 38, 44, 0.98)
    } else {
        Color::from_rgba8(233, 236, 241, 1.0)
    };
    let idle_border = if highlighted {
        if is_dark {
            Color::from_rgba8(70, 74, 83, 0.96)
        } else {
            Color::from_rgba8(206, 213, 223, 1.0)
        }
    } else if is_dark {
        Color::from_rgba8(45, 48, 54, 0.92)
    } else {
        Color::from_rgba8(226, 231, 237, 1.0)
    };
    let active_border = if highlighted {
        if is_dark {
            Color::from_rgba8(84, 88, 98, 0.98)
        } else {
            Color::from_rgba8(194, 202, 214, 1.0)
        }
    } else if is_dark {
        Color::from_rgba8(63, 67, 75, 0.95)
    } else {
        Color::from_rgba8(210, 216, 224, 1.0)
    };
    let (background, border_color) = match status {
        button::Status::Pressed => (pressed_bg, active_border),
        button::Status::Hovered => (hover_bg, active_border),
        _ => (idle_bg, idle_border),
    };

    button::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 12.0.into(), width: 1.0, color: border_color },
        text_color: selector_text_color(theme, highlighted),
        shadow: if highlighted {
            iced::Shadow {
                color: Color::BLACK.scale_alpha(if is_dark { 0.10 } else { 0.035 }),
                offset: iced::Vector::new(0.0, 6.0),
                blur_radius: 16.0,
            }
        } else {
            iced::Shadow::default()
        },
        ..Default::default()
    }
}

pub fn selectable_list_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let hovered = matches!(status, button::Status::Hovered);
    let pressed = matches!(status, button::Status::Pressed);
    let neutral_hover = if is_dark {
        Color::from_rgba8(31, 33, 38, 0.94)
    } else {
        Color::from_rgba8(241, 243, 246, 1.0)
    };
    let selected_bg = Color::from_rgba(
        theme.palette().primary.r,
        theme.palette().primary.g,
        theme.palette().primary.b,
        if is_dark { 0.15 } else { 0.10 },
    );
    let selected_hover_bg = Color::from_rgba(
        theme.palette().primary.r,
        theme.palette().primary.g,
        theme.palette().primary.b,
        if is_dark { 0.19 } else { 0.13 },
    );
    let background = if pressed {
        Some(Background::Color(if selected { selected_hover_bg } else { neutral_hover }))
    } else if hovered {
        Some(Background::Color(if selected { selected_hover_bg } else { neutral_hover }))
    } else if selected {
        Some(Background::Color(selected_bg))
    } else {
        None
    };
    let border_color = if selected {
        theme.palette().primary.scale_alpha(if is_dark { 0.42 } else { 0.24 })
    } else if hovered || pressed {
        if is_dark {
            Color::from_rgba8(58, 61, 69, 0.92)
        } else {
            Color::from_rgba8(218, 223, 230, 1.0)
        }
    } else {
        Color::TRANSPARENT
    };

    button::Style {
        background,
        border: Border {
            radius: 8.0.into(),
            width: if selected || hovered || pressed { 1.0 } else { 0.0 },
            color: border_color,
        },
        text_color: if selected {
            selector_text_color(theme, true)
        } else {
            theme.palette().text.scale_alpha(if is_dark { 0.90 } else { 0.88 })
        },
        ..Default::default()
    }
}

/// 生成弹出框容器样式
///
/// 返回适用于弹出菜单、下拉面板等浮动组件的容器样式。
/// 样式包含高不透明度背景和细边框，与主题颜色协调。
///
/// # 参数
///
/// * `theme` - iced 主题引用，用于获取调色板颜色
///
/// # 返回值
///
/// 返回配置好的 `container::Style` 实例：
/// - 背景：主题背景色（98% 不透明度）
/// - 1px 强背景色边框
/// - 8px 圆角
pub fn popover_style(theme: &Theme) -> container::Style {
    let is_dark = is_dark_theme(theme);
    let background = if is_dark {
        Color::from_rgba8(20, 21, 24, 0.985)
    } else {
        Color::from_rgba8(252, 252, 253, 0.985)
    };
    let border = if is_dark {
        Color::from_rgba8(44, 47, 53, 0.96)
    } else {
        Color::from_rgba8(226, 231, 237, 1.0)
    };
    container::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 14.0.into(), width: 1.0, color: border },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.22 } else { 0.08 }),
            offset: iced::Vector::new(0.0, 10.0),
            blur_radius: 28.0,
        },
        ..Default::default()
    }
}

/// 生成输入卡片容器样式
///
/// 返回输入面板卡片的样式，支持文件拖拽悬停状态的视觉反馈。
/// 拖拽悬停时显示主题色边框和淡色背景。
///
/// # 参数
///
/// * `theme` - iced 主题引用，用于获取调色板颜色
/// * `drop_hovered` - 是否处于文件拖拽悬停状态
/// * `chrome` - 可选的活动主题控件皮肤覆盖
///
/// # 返回值
///
/// 返回配置好的 `container::Style` 实例：
/// - 普通状态：无背景，强背景色边框
/// - 拖拽悬停状态：主色边框，主色淡背景（8% 透明度）
/// - 14px 圆角，无阴影
pub fn input_card_style(
    theme: &Theme,
    drop_hovered: bool,
    chrome: Option<&ChromeStyleOverrides>,
) -> container::Style {
    let is_dark = is_dark_theme(theme);
    let default_bg = crate::app::theme_catalog::input_surface_color(
        theme,
        if is_dark { 0.96 } else { 1.0 },
    );
    let chrome_bg = chrome.and_then(|c| c.input_bg.or(c.panel_bg));
    let base_bg = chrome_bg.unwrap_or(default_bg);
    let palette = theme.extended_palette();
    let default_border = if is_dark {
        palette.background.strong.color.scale_alpha(0.98)
    } else {
        palette.background.strong.color.scale_alpha(0.70)
    };
    let chrome_border = chrome.and_then(|c| c.input_border.or(c.panel_border));
    let border_color = if drop_hovered {
        theme.palette().primary.scale_alpha(0.82)
    } else {
        chrome_border.unwrap_or(default_border)
    };
    let background = if drop_hovered {
        Color::from_rgba(
            base_bg.r * 0.94 + theme.palette().primary.r * 0.06,
            base_bg.g * 0.94 + theme.palette().primary.g * 0.06,
            base_bg.b * 0.94 + theme.palette().primary.b * 0.06,
            base_bg.a,
        )
    } else {
        base_bg
    };
    let radius = chrome
        .and_then(|c| c.radius_px)
        .unwrap_or(20.0);
    container::Style {
        background: Some(Background::Color(background)),
        border: Border { width: 1.0, color: border_color, radius: radius.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.18 } else { 0.05 }),
            offset: iced::Vector::new(0.0, 12.0),
            blur_radius: 32.0,
        },
        ..Default::default()
    }
}

pub fn manual_context_card_style(theme: &Theme) -> container::Style {
    let is_dark = is_dark_theme(theme);
    let primary = theme.palette().primary;
    let background = if is_dark {
        Color::from_rgba(
            primary.r * 0.20 + 18.0 / 255.0 * 0.80,
            primary.g * 0.20 + 19.0 / 255.0 * 0.80,
            primary.b * 0.20 + 22.0 / 255.0 * 0.80,
            0.96,
        )
    } else {
        Color::from_rgba(
            primary.r * 0.08 + 250.0 / 255.0 * 0.92,
            primary.g * 0.08 + 251.0 / 255.0 * 0.92,
            primary.b * 0.08 + 253.0 / 255.0 * 0.92,
            1.0,
        )
    };
    let border = if is_dark { primary.scale_alpha(0.34) } else { primary.scale_alpha(0.24) };
    container::Style {
        background: Some(Background::Color(background)),
        border: Border { width: 1.0, color: border, radius: 12.0.into() },
        ..Default::default()
    }
}

pub fn manual_context_card_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 12.0.into() },
        text_color: selector_text_color(theme, true),
        ..Default::default()
    };

    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        style.background =
            Some(Background::Color(theme.palette().primary.scale_alpha(if is_dark_theme(theme) {
                0.10
            } else {
                0.06
            })));
    }

    style
}

/// 生成文本编辑器样式
///
/// 返回主输入编辑器的样式配置，支持请求中状态的视觉反馈。
/// 根据主题亮度自动调整占位符文字颜色。
///
/// # 参数
///
/// * `theme` - iced 主题引用，用于获取调色板颜色
/// * `_status` - 编辑器状态（当前未使用）
/// * `requesting` - 是否正在发送请求，影响文字颜色
/// * `chrome` - 可选的活动主题控件皮肤覆盖
///
/// # 返回值
///
/// 返回配置好的 `text_editor::Style` 实例：
/// - 透明背景，无边框
/// - 普通状态：主题文字颜色
/// - 请求中状态：次要色文字颜色
/// - 选中文本：主色背景（30% 透明度）
/// - 占位符：根据主题亮度自动调整（深色主题 60% 透明度，浅色主题 80% 透明度）
pub fn editor_style(
    theme: &Theme,
    _status: text_editor::Status,
    requesting: bool,
    chrome: Option<&ChromeStyleOverrides>,
) -> text_editor::Style {
    let p = theme.extended_palette();
    let palette = theme.palette();
    let is_dark = is_dark_theme(theme);
    let chrome_text = chrome.and_then(|c| c.input_text);
    let value = if let Some(text) = chrome_text {
        if requesting {
            text.scale_alpha(0.92)
        } else {
            text
        }
    } else if requesting {
        palette.text.scale_alpha(0.92)
    } else {
        palette.text.scale_alpha(if is_dark { 0.96 } else { 0.94 })
    };
    let placeholder = if is_dark {
        chrome_text
            .unwrap_or(palette.text)
            .scale_alpha(0.48)
    } else {
        p.secondary.base.text.scale_alpha(0.56)
    };
    text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        value,
        selection: theme.palette().primary.scale_alpha(if is_dark { 0.26 } else { 0.18 }),
        placeholder,
    }
}

/// 生成子任务文本编辑器样式
///
/// 返回子任务编辑器的样式配置，与主编辑器相比增加了边框。
/// 根据主题亮度自动调整占位符文字颜色。
///
/// # 参数
///
/// * `theme` - iced 主题引用，用于获取调色板颜色
/// * `_status` - 编辑器状态（当前未使用）
///
/// # 返回值
///
/// 返回配置好的 `text_editor::Style` 实例：
/// - 透明背景
/// - 1px 强背景色边框，8px 圆角
/// - 主题文字颜色
/// - 选中文本：主色背景（30% 透明度）
/// - 占位符：根据主题亮度自动调整（深色主题 60% 透明度，浅色主题 80% 透明度）
pub fn subtask_editor_style(theme: &Theme, _status: text_editor::Status) -> text_editor::Style {
    let p = theme.extended_palette();
    let palette = theme.palette();
    let is_dark = is_dark_theme(theme);
    let placeholder = if is_dark {
        palette.text.scale_alpha(0.48)
    } else {
        p.secondary.base.text.scale_alpha(0.56)
    };
    text_editor::Style {
        background: Background::Color(if is_dark {
            Color::from_rgba8(22, 23, 27, 0.92)
        } else {
            Color::from_rgba8(248, 249, 251, 1.0)
        }),
        border: Border {
            width: 1.0,
            color: if is_dark {
                Color::from_rgba8(43, 46, 52, 0.96)
            } else {
                Color::from_rgba8(226, 231, 237, 1.0)
            },
            radius: 10.0.into(),
        },
        value: theme.palette().text.scale_alpha(if is_dark { 0.94 } else { 0.92 }),
        selection: theme.palette().primary.scale_alpha(if is_dark { 0.26 } else { 0.18 }),
        placeholder,
    }
}
