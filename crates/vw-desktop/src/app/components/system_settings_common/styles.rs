//! 系统设置页面复用的通用控件、样式与辅助能力。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::theme::is_dark_theme;
use iced::widget::{button, checkbox, pick_list, text_editor, text_input, toggler};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

/// 构建或处理 `rounded_action_btn_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn rounded_action_btn_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let bg = match status {
        button::Status::Hovered => Some(
            if is_dark {
                palette.background.weak.color.scale_alpha(0.9)
            } else {
                Color::WHITE.scale_alpha(0.88)
            }
            .into(),
        ),
        button::Status::Pressed => Some(
            if is_dark {
                palette.background.strong.color.scale_alpha(0.94)
            } else {
                palette.background.weak.color.scale_alpha(0.92)
            }
            .into(),
        ),
        button::Status::Disabled => Some(palette.background.weak.color.scale_alpha(0.50).into()),
        button::Status::Active => Some(
            if is_dark {
                palette.background.base.color.scale_alpha(0.52)
            } else {
                Color::WHITE.scale_alpha(0.56)
            }
            .into(),
        ),
    };

    button::Style {
        background: bg,
        text_color: if status == button::Status::Disabled {
            theme.palette().text.scale_alpha(0.55)
        } else {
            theme.palette().text
        },
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.86)
            } else {
                Color::from_rgba8(15, 23, 42, 0.08)
            },
        },
        ..Default::default()
    }
}

/// 构建或处理 `round_icon_btn_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn round_icon_btn_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let bg = match status {
        button::Status::Hovered => Some(
            if is_dark {
                palette.background.weak.color.scale_alpha(0.92)
            } else {
                Color::WHITE.scale_alpha(0.94)
            }
            .into(),
        ),
        button::Status::Pressed => Some(
            if is_dark {
                palette.background.strong.color.scale_alpha(0.96)
            } else {
                palette.background.weak.color.scale_alpha(0.96)
            }
            .into(),
        ),
        _ => Some(
            if is_dark {
                palette.background.base.color.scale_alpha(0.58)
            } else {
                Color::WHITE.scale_alpha(0.76)
            }
            .into(),
        ),
    };

    button::Style {
        background: bg,
        text_color: theme.palette().text,
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.86)
            } else {
                Color::from_rgba8(15, 23, 42, 0.08)
            },
        },
        ..Default::default()
    }
}

/// 构建或处理 `danger_action_btn_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn danger_action_btn_style(theme: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Active => (Color::from_rgb8(196, 43, 40), Color::WHITE),
        button::Status::Hovered => (Color::from_rgb8(174, 34, 31), Color::WHITE),
        button::Status::Pressed => (Color::from_rgb8(145, 27, 25), Color::WHITE),
        button::Status::Disabled => {
            (theme.extended_palette().background.weak.color, theme.palette().text.scale_alpha(0.55))
        }
    };

    button::Style {
        background: Some(background.into()),
        text_color,
        border: Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// 构建或处理 `primary_action_btn_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn primary_action_btn_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let (background, text_color) = match status {
        button::Status::Active => (palette.primary.base.color, palette.primary.base.text),
        button::Status::Hovered => (palette.primary.strong.color, palette.primary.strong.text),
        button::Status::Pressed => (palette.primary.weak.color, palette.primary.weak.text),
        button::Status::Disabled => {
            (palette.background.weak.color, theme.palette().text.scale_alpha(0.55))
        }
    };

    button::Style {
        background: Some(background.into()),
        text_color,
        border: Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// 构建或处理 `settings_muted_text_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_muted_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(if is_dark_theme(theme) {
            theme.palette().text.scale_alpha(0.72)
        } else {
            theme.palette().text.scale_alpha(0.62)
        }),
    }
}

/// 构建或处理 `settings_panel_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            palette.background.base.color.scale_alpha(0.92)
        } else {
            Color::from_rgba8(255, 255, 255, 0.88)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.9)
            } else {
                Color::from_rgba8(15, 23, 42, 0.08)
            },
            radius: 18.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.07 } else { 0.03 }),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 7.0,
        },
        snap: false,
        ..Default::default()
    }
}

/// 构建或处理 `settings_modal_backdrop_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_modal_backdrop_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark_theme(theme) {
            Color::from_rgba8(2, 6, 23, 0.70)
        } else {
            Color::from_rgba8(15, 23, 42, 0.30)
        })),
        ..Default::default()
    }
}

/// 构建或处理 `settings_modal_card_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_modal_card_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            palette.background.base.color.scale_alpha(0.98)
        } else {
            Color::from_rgba8(255, 255, 255, 0.97)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.96)
            } else {
                theme.palette().primary.scale_alpha(0.10)
            },
            radius: 24.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.34 } else { 0.14 }),
            offset: Vector::new(0.0, 24.0),
            blur_radius: 52.0,
        },
        snap: false,
        ..Default::default()
    }
}

/// 构建或处理 `settings_text_input_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let is_focused = matches!(status, text_input::Status::Focused { .. });
    let is_hovered = matches!(status, text_input::Status::Hovered)
        || matches!(status, text_input::Status::Focused { is_hovered: true });
    let is_disabled = matches!(status, text_input::Status::Disabled);

    let background = if is_disabled {
        extended.background.base.color.scale_alpha(if is_dark { 0.40 } else { 0.60 })
    } else if is_focused {
        if is_dark {
            extended.background.weak.color.scale_alpha(0.96)
        } else {
            Color::WHITE.scale_alpha(0.98)
        }
    } else if is_hovered {
        if is_dark {
            extended.background.base.color.scale_alpha(0.96)
        } else {
            Color::WHITE.scale_alpha(0.94)
        }
    } else if is_dark {
        extended.background.base.color.scale_alpha(0.84)
    } else {
        Color::WHITE.scale_alpha(0.90)
    };

    let border_color = if is_focused {
        palette.primary.scale_alpha(0.84)
    } else if is_hovered {
        extended.background.strong.color.scale_alpha(0.92)
    } else {
        let idle_border = if is_dark {
            extended.background.strong.color.scale_alpha(0.82)
        } else {
            Color::from_rgba8(15, 23, 42, 0.10)
        };
        if is_disabled { idle_border.scale_alpha(0.55) } else { idle_border }
    };

    text_input::Style {
        background: Background::Color(background),
        border: Border { width: 1.0, color: border_color, radius: 14.0.into() },
        icon: palette.text.scale_alpha(0.65),
        placeholder: palette.text.scale_alpha(0.50),
        value: if is_disabled { palette.text.scale_alpha(0.55) } else { palette.text },
        selection: palette.primary.scale_alpha(0.20),
    }
}

/// 使用与单行输入相同的背景、边框和文字状态。
pub fn settings_text_editor_style(
    theme: &Theme,
    status: text_editor::Status,
) -> text_editor::Style {
    let input = settings_text_input_style(
        theme,
        match status {
            text_editor::Status::Active => text_input::Status::Active,
            text_editor::Status::Hovered => text_input::Status::Hovered,
            text_editor::Status::Focused { is_hovered } => {
                text_input::Status::Focused { is_hovered }
            }
            text_editor::Status::Disabled => text_input::Status::Disabled,
        },
    );
    text_editor::Style {
        background: input.background,
        border: input.border,
        placeholder: input.placeholder,
        value: input.value,
        selection: input.selection,
    }
}

/// Iced 的只读编辑器跳过事件更新，绘制时仍可能提供 Active。
/// 使用与绑定编辑回调相同的 enabled 值确保禁用状态可见。
pub fn settings_text_editor_style_for(
    enabled: bool,
) -> impl Fn(&Theme, text_editor::Status) -> text_editor::Style {
    move |theme, status| {
        settings_text_editor_style(
            theme,
            if enabled { status } else { text_editor::Status::Disabled },
        )
    }
}

/// 构建或处理 `settings_pick_list_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_pick_list_style(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let is_active = matches!(status, pick_list::Status::Active);
    let is_hovered = matches!(status, pick_list::Status::Hovered)
        || matches!(status, pick_list::Status::Opened { is_hovered: true });
    let is_open = matches!(status, pick_list::Status::Opened { .. });

    let background = if is_open {
        if is_dark {
            extended.background.weak.color.scale_alpha(0.94)
        } else {
            Color::WHITE.scale_alpha(0.98)
        }
    } else if is_hovered {
        if is_dark {
            extended.background.base.color.scale_alpha(0.96)
        } else {
            Color::WHITE.scale_alpha(0.94)
        }
    } else if is_active {
        if is_dark {
            extended.background.base.color.scale_alpha(0.84)
        } else {
            Color::WHITE.scale_alpha(0.90)
        }
    } else {
        extended.background.base.color.into()
    };

    let border_color = if is_open {
        palette.primary.scale_alpha(0.82)
    } else if is_hovered {
        extended.background.strong.color.scale_alpha(0.92)
    } else if is_dark {
        extended.background.strong.color.scale_alpha(0.82)
    } else {
        Color::from_rgba8(15, 23, 42, 0.10)
    };

    pick_list::Style {
        text_color: palette.text,
        placeholder_color: palette.text.scale_alpha(0.50),
        handle_color: if is_open {
            palette.primary.scale_alpha(0.92)
        } else {
            palette.text.scale_alpha(0.68)
        },
        background: Background::Color(background),
        border: Border { width: 1.0, color: border_color, radius: 14.0.into() },
    }
}

/// 构建或处理 `settings_pick_list_menu_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_pick_list_menu_style(theme: &Theme) -> iced::overlay::menu::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::overlay::menu::Style {
        background: Background::Color(if is_dark {
            extended.background.base.color.scale_alpha(0.98)
        } else {
            Color::from_rgba8(255, 255, 255, 0.98)
        }),
        border: Border {
            width: 1.0,
            color: if is_dark {
                extended.background.strong.color.scale_alpha(0.94)
            } else {
                Color::from_rgba8(15, 23, 42, 0.10)
            },
            radius: 16.0.into(),
        },
        text_color: palette.text,
        selected_text_color: palette.text,
        selected_background: Background::Color(if is_dark {
            palette.primary.scale_alpha(0.24)
        } else {
            palette.primary.scale_alpha(0.10)
        }),
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.30 } else { 0.12 }),
            offset: Vector::new(0.0, 10.0),
            blur_radius: 22.0,
        },
    }
}

/// 构建或处理 `settings_checkbox_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_checkbox_style(theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let (is_checked, is_hovered, is_disabled) = match status {
        checkbox::Status::Active { is_checked } => (is_checked, false, false),
        checkbox::Status::Hovered { is_checked } => (is_checked, true, false),
        checkbox::Status::Disabled { is_checked } => (is_checked, false, true),
    };

    let background = if is_checked {
        if is_hovered {
            palette.primary.scale_alpha(0.94)
        } else if is_disabled {
            palette.primary.scale_alpha(0.36)
        } else {
            palette.primary.scale_alpha(0.86)
        }
    } else if is_disabled {
        extended.background.base.color.scale_alpha(if is_dark { 0.36 } else { 0.58 })
    } else if is_hovered {
        if is_dark {
            extended.background.base.color.scale_alpha(0.96)
        } else {
            Color::WHITE.scale_alpha(0.94)
        }
    } else if is_dark {
        extended.background.base.color.scale_alpha(0.82)
    } else {
        Color::WHITE.scale_alpha(0.90)
    };

    checkbox::Style {
        background: Background::Color(background),
        icon_color: if is_checked { Color::WHITE } else { Color::TRANSPARENT },
        border: Border {
            width: 1.0,
            color: if is_checked {
                palette.primary.scale_alpha(if is_disabled { 0.36 } else { 0.84 })
            } else if !is_disabled {
                palette.text.scale_alpha(if is_hovered { 0.66 } else { 0.50 })
            } else if is_dark {
                extended.background.strong.color.scale_alpha(0.82)
            } else {
                Color::from_rgba8(15, 23, 42, 0.10)
            },
            radius: 8.0.into(),
        },
        text_color: Some(if is_disabled {
            palette.text.scale_alpha(0.48)
        } else {
            palette.text.scale_alpha(0.94)
        }),
    }
}

/// 构建或处理 `settings_segment_button_style` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn settings_segment_button_style(
    theme: &Theme,
    status: button::Status,
    is_active: bool,
) -> button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    let background = if is_active {
        Some(Background::Color(if is_dark {
            theme.palette().primary.scale_alpha(0.24)
        } else {
            theme.palette().primary.scale_alpha(0.10)
        }))
    } else {
        match status {
            button::Status::Hovered => Some(Background::Color(if is_dark {
                palette.background.weak.color.scale_alpha(0.70)
            } else {
                Color::WHITE.scale_alpha(0.82)
            })),
            button::Status::Pressed => Some(Background::Color(if is_dark {
                palette.background.strong.color.scale_alpha(0.86)
            } else {
                palette.background.weak.color.scale_alpha(0.92)
            })),
            _ => Some(Background::Color(if is_dark {
                palette.background.base.color.scale_alpha(0.40)
            } else {
                Color::WHITE.scale_alpha(0.52)
            })),
        }
    };

    button::Style {
        background,
        text_color: if is_active {
            theme.palette().primary.scale_alpha(0.96)
        } else {
            theme.palette().text.scale_alpha(0.92)
        },
        border: Border {
            width: 1.0,
            color: if is_active {
                theme.palette().primary.scale_alpha(0.28)
            } else if is_dark {
                palette.background.strong.color.scale_alpha(0.80)
            } else {
                Color::from_rgba8(15, 23, 42, 0.08)
            },
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

/// 常规设置开关的共享样式。
pub(super) fn settings_toggle_style(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let mut style = toggler::default(theme, status);
    // 保持开关滑块稳定可见，悬停时只改变轨道颜色。
    style.foreground = Color::WHITE.into();
    let palette = theme.extended_palette();
    match status {
        toggler::Status::Hovered { is_toggled: true } => {
            style.background = palette.primary.strong.color.into();
        }
        toggler::Status::Hovered { is_toggled: false } => {
            style.background = palette.background.strongest.color.into();
        }
        toggler::Status::Disabled { .. } => {
            style.foreground = Color::WHITE.scale_alpha(0.8).into();
        }
        toggler::Status::Active { .. } => {}
    }
    style
}
