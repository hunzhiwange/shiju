use iced::{Background, Border, Color, Theme, Vector};
use desktop_ui_controls::system_settings_common::rounded_action_btn_style;
use desktop_ui_controls::system_settings_common::settings_panel_style;
/// 处理 is dark theme 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `true` 表示当前输入满足该辅助函数描述的条件。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

/// 根据主题与状态计算 panel style。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn panel_style(theme: &Theme) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    let is_dark = is_dark_theme(theme);
    style.border.radius = 22.0.into();
    style.background = Some(Background::Color(if is_dark {
        theme.extended_palette().background.base.color.scale_alpha(0.96)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::WHITE.scale_alpha(0.95)
    }));
    style.shadow = iced::Shadow {
        // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        color: Color::BLACK.scale_alpha(if is_dark { 0.16 } else { 0.08 }),
        // offset 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        offset: Vector::new(0.0, 16.0),
        // blur_radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        blur_radius: 28.0,
    };
    style
}

/// 根据主题与状态计算 accent badge style。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn accent_badge_style(theme: &Theme, accent: Color) -> iced::widget::container::Style {
    // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    iced::widget::container::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(if is_dark_theme(theme) {
            accent.scale_alpha(0.18)
        } else {
            accent.scale_alpha(0.12)
        })),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border {
            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            width: 1.0,
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: accent.scale_alpha(if is_dark_theme(theme) { 0.34 } else { 0.18 }),
            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

/// 根据主题与状态计算 item style。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn item_style(theme: &Theme, accent: Color) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(if is_dark {
            palette.background.weak.color.scale_alpha(0.26)
        } else {
            // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Color::WHITE.scale_alpha(0.82)
        })),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border {
            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            width: 1.0,
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: if is_dark {
                accent.scale_alpha(0.22)
            } else {
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::from_rgba8(15, 23, 42, 0.06)
            },
            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            radius: 16.0.into(),
        },
        // shadow 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shadow: iced::Shadow {
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: Color::BLACK.scale_alpha(if is_dark { 0.07 } else { 0.025 }),
            // offset 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            offset: Vector::new(0.0, 8.0),
            // blur_radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

/// 根据主题与状态计算 compact action btn style。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn compact_action_btn_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    style.border.radius = 999.0.into();
    style
}

/// 根据主题与状态计算 copy action btn style。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn copy_action_btn_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
    // copied 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    copied: bool,
) -> iced::widget::button::Style {
    let mut style = compact_action_btn_style(theme, status);
    if copied {
        let success = theme.extended_palette().success.base.color;
        style.text_color = success;
        style.border.color = success.scale_alpha(if is_dark_theme(theme) { 0.42 } else { 0.24 });
    }
    style
}

/// 根据主题与状态计算 delete action btn style。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(crate) fn delete_action_btn_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = compact_action_btn_style(theme, status);
    let danger = if is_dark_theme(theme) {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xF5, 0xA3, 0xA3, 0.96)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xB4, 0x23, 0x18, 0.94)
    };
    style.text_color = danger;
    style.border.color = danger.scale_alpha(if is_dark_theme(theme) { 0.32 } else { 0.18 });
    style
}

pub(crate) fn notification_copy_label(copied: bool) -> &'static str {
    if copied { "✓" } else { "复制" }
}

pub(crate) fn notification_text_editor_style(theme: &Theme) -> iced::widget::text_editor::Style {
    let value = theme.palette().text;
    iced::widget::text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        value,
        selection: if is_dark_theme(theme) {
            Color::from_rgba8(0x8B, 0x93, 0x9C, 0.34)
        } else {
            Color::from_rgba8(0xD9, 0xDE, 0xE5, 0.92)
        },
        placeholder: value.scale_alpha(0.7),
    }
}
