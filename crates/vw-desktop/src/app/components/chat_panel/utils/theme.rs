//! 聊天面板通用辅助函数。
//!
//! 本模块提供状态、路径、文本、主题、时间或菜单相关的小型工具，供聊天面板视图复用。

use iced::widget::scrollable::{Direction, Scrollbar};
/// 重新导出 use iced::widget::svg::{self, Svg}，让上层模块通过稳定路径访问。
use iced::widget::svg::{self, Svg};
/// 重新导出 use iced::widget::{button, text}，让上层模块通过稳定路径访问。
use iced::widget::{button, text};
/// 重新导出 use iced::{Alignment, Background, Border, Color, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

/// 重新导出 use crate::app::Message，让上层模块通过稳定路径访问。
use crate::app::Message;
/// 重新导出 use crate::app::assets::{self, Icon}，让上层模块通过稳定路径访问。
use crate::app::assets::{self, Icon};

/// 根据主题与语义状态计算 muted icon color。
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
pub fn muted_icon_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        theme.palette().text.scale_alpha(0.75)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(0.85)
    }
}

/// 根据主题与语义状态计算 mix color。
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
pub fn mix_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::from_rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// 处理 icon svg 对应的局部职责。
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
pub fn icon_svg(icon: Icon) -> Svg<'static> {
    // Svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Svg::new(assets::get_icon(icon)).width(Length::Fixed(14.0)).height(Length::Fixed(14.0))
}

/// 处理 chat scroll direction 对应的局部职责。
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
pub fn chat_scroll_direction() -> Direction {
    // Direction 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))
}

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
pub fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

/// 根据主题与语义状态计算 chat secondary text color。
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
pub fn chat_secondary_text_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        theme.palette().text.scale_alpha(0.92)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(0.90)
    }
}

/// 根据主题与语义状态计算 chat secondary muted text color。
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
pub fn chat_secondary_muted_text_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        theme.palette().text.scale_alpha(0.72)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(0.72)
    }
}

/// 根据主题与语义状态计算 chat secondary subtle text color。
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
pub fn chat_secondary_subtle_text_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        theme.palette().text.scale_alpha(0.56)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(0.58)
    }
}

/// 构建 icon button style 控件，并绑定既有消息或样式。
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
pub fn icon_button_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let is_dark = is_dark_theme(theme);
    let idle_bg = if is_dark {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(24, 25, 29, 0.88)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(247, 248, 250, 1.0)
    };
    let hover_bg = if is_dark {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(31, 33, 38, 0.94)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(240, 243, 246, 1.0)
    };
    let pressed_bg = if is_dark {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(36, 38, 44, 0.98)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(232, 236, 241, 1.0)
    };
    let idle_border = if is_dark {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(45, 48, 54, 0.9)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(226, 231, 237, 1.0)
    };
    let active_border = if is_dark {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(63, 67, 75, 0.94)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(210, 216, 224, 1.0)
    };
    let (background, border_color) = match status {
        // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        iced::widget::button::Status::Pressed => (pressed_bg, active_border),
        // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        iced::widget::button::Status::Hovered => (hover_bg, active_border),
        _ => (idle_bg, idle_border),
    };
    iced::widget::button::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(background)),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 1.0, color: border_color, radius: 999.0.into() },
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: chat_secondary_text_color(theme),
        ..Default::default()
    }
}

/// 构建 eye icon button style 控件，并绑定既有消息或样式。
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
pub fn eye_icon_button_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: chat_secondary_text_color(theme),
        ..Default::default()
    }
}

/// 根据主题与状态计算 eye icon svg style。
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
pub fn eye_icon_svg_style(theme: &Theme, _status: svg::Status) -> svg::Style {
    // svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    svg::Style {
        // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        color: Some(if is_dark_theme(theme) {
            theme.palette().text.scale_alpha(0.92)
        } else {
            theme.extended_palette().secondary.base.text.scale_alpha(0.90)
        }),
    }
}

/// 根据主题与状态计算 simplified block style。
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
pub fn simplified_block_style(theme: &Theme) -> iced::widget::container::Style {
    let _ = theme;
    iced::widget::container::Style {
        background: None,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

/// 根据主题与状态计算 simplified code block style。
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
pub fn simplified_code_block_style(theme: &Theme) -> iced::widget::container::Style {
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(if is_dark {
            // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Color::from_rgba8(24, 25, 29, 0.90)
        } else {
            // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Color::from_rgba8(247, 248, 250, 1.0)
        })),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border {
            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            width: 1.0,
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: if is_dark {
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::from_rgba8(45, 48, 54, 0.90)
            } else {
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::from_rgba8(226, 231, 237, 1.0)
            },
            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

/// 构建 pill button style 控件，并绑定既有消息或样式。
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
pub fn pill_button_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = button::secondary(theme, status);
    style.border.radius = 999.0.into();
    style
}

/// 构建 file button style 控件，并绑定既有消息或样式。
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
pub fn file_button_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = button::secondary(theme, status);
    style.border.radius = 10.0.into();
    style
}

/// 构建 weak file button style 控件，并绑定既有消息或样式。
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
pub fn weak_file_button_style(
    theme: &Theme,
    // status 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let ext = theme.extended_palette();
    let bg = match status {
        // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        iced::widget::button::Status::Pressed => {
            Some(ext.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.22
            } else {
                0.16
            }))
        }
        iced::widget::button::Status::Hovered => {
            Some(ext.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.28
            } else {
                0.35
            }))
        }
        _ => None,
    };
    iced::widget::button::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: bg.map(Background::Color),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 10.0.into() },
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: chat_secondary_muted_text_color(theme),
        ..Default::default()
    }
}

/// 处理 additions pill 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn additions_pill<'a>(count: usize) -> Element<'a, Message> {
    let label = format!("+{}", count);
    iced::widget::container(text(label).size(11))
        .padding([2, 8])
        .style(|theme: &Theme| {
            let ext = theme.extended_palette();
            let fg = ext.success.base.color;
            let bg = fg.scale_alpha(0.16);
            iced::widget::container::Style {
                // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                background: Some(Background::Color(bg)),
                // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
                // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                text_color: Some(fg),
                ..Default::default()
            }
        })
        .into()
}

/// 处理 deletions pill 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn deletions_pill<'a>(count: usize) -> Element<'a, Message> {
    let label = format!("-{}", count);
    iced::widget::container(text(label).size(11))
        .padding([2, 8])
        .style(|theme: &Theme| {
            let ext = theme.extended_palette();
            let fg = ext.danger.base.color;
            let bg = fg.scale_alpha(0.14);
            iced::widget::container::Style {
                // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                background: Some(Background::Color(bg)),
                // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
                // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                text_color: Some(fg),
                ..Default::default()
            }
        })
        .into()
}

/// 处理 change pills 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn change_pills<'a>(adds: usize, dels: usize) -> Element<'a, Message> {
    // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    iced::widget::row![additions_pill(adds), deletions_pill(dels)]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
}
