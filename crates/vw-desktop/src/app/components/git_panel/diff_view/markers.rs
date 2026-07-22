//! Git diff 局部渲染辅助。
//!
//! 本模块负责 diff 行、行号、选区、上下文菜单和配色的局部组合。

use iced::alignment::Horizontal;
/// 重新导出 use iced::widget::{Column, Space, container, row, text}，让上层模块通过稳定路径访问。
use iced::widget::{Column, Space, container, row, text};
/// 重新导出 use iced::{Background, Border, Color, Element, Length}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Color, Element, Length};

/// 重新导出 use crate::app::Message，让上层模块通过稳定路径访问。
use crate::app::Message;

/// 重新导出 use super::{DIFF_LINE_NUMBER_WIDTH, DIFF_MARKER_WIDTH}，让上层模块通过稳定路径访问。
use super::{DIFF_LINE_NUMBER_WIDTH, DIFF_MARKER_WIDTH};

/// LineMarkerKind 描述 markers 模块支持的离散状态。
///
/// 新增变体时需要同步检查显式分支，避免未知状态被静默吞掉。
#[derive(Clone, Copy)]
pub enum LineMarkerKind {
    None,
    Add,
    Delete,
}

/// LineNumberTone 描述 markers 模块支持的离散状态。
///
/// 新增变体时需要同步检查显式分支，避免未知状态被静默吞掉。
#[derive(Debug, Clone, Copy)]
pub enum LineNumberTone {
    Neutral,
    Add,
    Delete,
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
pub(super) fn is_dark_theme(theme: &iced::Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
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
pub(super) fn mix_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::from_rgba(
        a.r * (1.0 - t) + b.r * t,
        a.g * (1.0 - t) + b.g * t,
        a.b * (1.0 - t) + b.b * t,
        a.a * (1.0 - t) + b.a * t,
    )
}

/// 处理 emphasized marker alpha 对应的局部职责。
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
pub(super) fn emphasized_marker_alpha(
    kind: LineMarkerKind,
    emphasized: bool,
    is_dark: bool,
) -> f32 {
    match kind {
        // LineMarkerKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LineMarkerKind::None if emphasized => {
            if is_dark {
                0.34
            } else {
                0.28
            }
        }
        LineMarkerKind::None => 0.0,
        _ if emphasized => {
            if is_dark {
                0.82
            } else {
                0.76
            }
        }
        _ if is_dark => 0.66,
        _ => 0.58,
    }
}

/// 处理 vertical add bar 对应的局部职责。
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
fn vertical_add_bar<'a>(color: Color, alpha: f32) -> Element<'a, Message> {
    let bar = container(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fixed(4.0))
        .height(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            background: Some(Background::Color(color.scale_alpha(alpha))),
            // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
            ..Default::default()
        });

    container(bar)
        .width(Length::Fixed(DIFF_MARKER_WIDTH))
        .height(Length::Fill)
        .align_x(Horizontal::Left)
        .into()
}

/// 处理 delete dot pattern 对应的局部职责。
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
fn delete_dot_pattern<'a>(color: Color, alpha: f32) -> Element<'a, Message> {
    let dot_row = |row_alpha: f32| {
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fixed(4.0))
            .height(Length::Fixed(1.0))
            .style(move |_| iced::widget::container::Style {
                // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                background: Some(Background::Color(color.scale_alpha(row_alpha))),
                // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
                ..Default::default()
            })
    };

    let pattern = (0..10).fold(Column::new().spacing(1), |col, idx| {
        let fade = 1.0 - (idx as f32 * 0.07);
        col.push(dot_row(alpha * fade.max(0.35)))
    });

    container(pattern)
        .width(Length::Fixed(DIFF_MARKER_WIDTH))
        .height(Length::Fill)
        .align_x(Horizontal::Left)
        .center_y(Length::Fill)
        .into()
}

/// 处理 line marker cell emphasis 对应的局部职责。
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
pub fn line_marker_cell_emphasis<'a>(
    kind: LineMarkerKind,
    // emphasized 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    emphasized: bool,
) -> Element<'a, Message> {
    let base = match kind {
        // LineMarkerKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LineMarkerKind::Add => Color::from_rgb8(0x2F, 0x81, 0x4F),
        // LineMarkerKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LineMarkerKind::Delete => Color::from_rgb8(0xC9, 0x3C, 0x37),
        // LineMarkerKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LineMarkerKind::None => Color::from_rgb8(0x6B, 0x72, 0x80),
    };
    let highlight = Color::from_rgba8(255, 186, 102, 1.0);
    let color = if emphasized {
        match kind {
            // LineMarkerKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            LineMarkerKind::None => highlight,
            _ => mix_color(base, highlight, 0.40),
        }
    } else {
        base
    };

    let alpha = emphasized_marker_alpha(kind, emphasized, true);

    match kind {
        // LineMarkerKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LineMarkerKind::Add => vertical_add_bar(color, alpha),
        // LineMarkerKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LineMarkerKind::Delete => delete_dot_pattern(color, alpha),
        // LineMarkerKind 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LineMarkerKind::None => container(Space::new().width(Length::Fixed(DIFF_MARKER_WIDTH)))
            .width(Length::Fixed(DIFF_MARKER_WIDTH))
            .height(Length::Fill)
            .into(),
    }
}

/// 处理 line number divider 对应的局部职责。
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
fn line_number_divider<'a>(tone: LineNumberTone) -> Element<'a, Message> {
    let _ = tone;
    container(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fixed(4.0))
        .height(Length::Fill)
        .style(|theme: &iced::Theme| {
            let ext = theme.extended_palette();
            let is_dark = is_dark_theme(theme);
            let divider = if is_dark {
                ext.background.strong.color.scale_alpha(0.18)
            } else {
                ext.background.strong.color.scale_alpha(0.10)
            };
            let shadow_1 = Color::BLACK.scale_alpha(if is_dark { 0.06 } else { 0.025 });
            let shadow_2 = Color::BLACK.scale_alpha(if is_dark { 0.035 } else { 0.015 });
            let shadow_3 = Color::BLACK.scale_alpha(if is_dark { 0.018 } else { 0.008 });

            iced::widget::container::Style {
                // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                background: Some(Background::Gradient(
                    iced::gradient::Linear::new(iced::Degrees(90.0))
                        .add_stop(0.0, divider)
                        .add_stop(0.18, divider)
                        .add_stop(0.18, shadow_1)
                        .add_stop(0.44, shadow_2)
                        .add_stop(0.72, shadow_3)
                        .add_stop(1.0, Color::TRANSPARENT)
                        .into(),
                )),
                ..Default::default()
            }
        })
        .into()
}

/// 处理 line number right padding 对应的局部职责。
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
pub(super) fn line_number_right_padding(tone: LineNumberTone) -> f32 {
    match tone {
        // LineNumberTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LineNumberTone::Neutral => 6.0,
        // LineNumberTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LineNumberTone::Add | LineNumberTone::Delete => 7.0,
    }
}

/// 根据主题与语义状态计算 line number text color。
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
pub(super) fn line_number_text_color(theme: &iced::Theme, tone: LineNumberTone) -> Color {
    let ext = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    match tone {
        // LineNumberTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        LineNumberTone::Neutral => {
            if is_dark {
                mix_color(theme.palette().text, ext.secondary.base.text, 0.42).scale_alpha(0.88)
            } else {
                mix_color(ext.secondary.base.text, theme.palette().text, 0.22).scale_alpha(0.82)
            }
        }
        LineNumberTone::Add => {
            if is_dark {
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::from_rgb8(0x3F, 0xD9, 0x7B).scale_alpha(0.96)
            } else {
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::from_rgb8(0x1A, 0x7F, 0x37).scale_alpha(0.88)
            }
        }
        LineNumberTone::Delete => {
            if is_dark {
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::from_rgb8(0xFF, 0x7B, 0x72).scale_alpha(0.92)
            } else {
                // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Color::from_rgb8(0xCF, 0x22, 0x1E).scale_alpha(0.88)
            }
        }
    }
}

/// 处理 line number cell 对应的局部职责。
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
pub fn line_number_cell<'a>(number: String) -> Element<'a, Message> {
    line_number_cell_with_tone(number, LineNumberTone::Neutral)
}

/// 处理 line number cell with tone offset 对应的局部职责。
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
pub fn line_number_cell_with_tone_offset<'a>(
    number: String,
    // tone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tone: LineNumberTone,
    // extra_right_padding 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    extra_right_padding: f32,
) -> Element<'a, Message> {
    build_line_number_cell(number, tone, extra_right_padding)
}

/// 处理 line number cell with tone 对应的局部职责。
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
pub fn line_number_cell_with_tone<'a>(
    number: String,
    // tone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tone: LineNumberTone,
) -> Element<'a, Message> {
    build_line_number_cell(number, tone, 0.0)
}

/// 构建 line number cell 对应的 Iced 界面片段或中间数据。
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
fn build_line_number_cell<'a>(
    number: String,
    // tone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tone: LineNumberTone,
    // extra_right_padding 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    extra_right_padding: f32,
) -> Element<'a, Message> {
    let value = container(
        text(number)
            .size(13)
            .line_height(iced::widget::text::LineHeight::Relative(1.0))
            .font(iced::Font::with_name("JetBrains Mono"))
            .style(move |theme: &iced::Theme| iced::widget::text::Style {
                // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                color: Some(line_number_text_color(theme, tone)),
            }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(iced::Padding {
        // top 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        top: 0.0,
        // right 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        right: line_number_right_padding(tone) + extra_right_padding,
        // bottom 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        bottom: 0.0,
        // left 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        left: 5.0,
    })
    .align_x(Horizontal::Right)
    .center_y(Length::Fill);

    let divider = container(line_number_divider(tone)).height(Length::Fill);

    container(row![value, divider].width(Length::Fill).height(Length::Fill))
        .width(Length::Fixed(DIFF_LINE_NUMBER_WIDTH))
        .into()
}

/// 处理 empty line number cell 对应的局部职责。
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
pub fn empty_line_number_cell<'a>() -> Element<'a, Message> {
    let spacer = container(Space::new().width(Length::Fill).height(Length::Shrink))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 5.0 })
        .center_y(Length::Fill);

    let divider = container(line_number_divider(LineNumberTone::Neutral)).height(Length::Fill);

    container(row![spacer, divider].width(Length::Fill).height(Length::Fill))
        .width(Length::Fixed(DIFF_LINE_NUMBER_WIDTH))
        .into()
}
