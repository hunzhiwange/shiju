//! Git diff 局部渲染辅助。
//!
//! 本模块负责 diff 行、行号、选区、上下文菜单和配色的局部组合。

use iced::widget::{button, column, container, text};
/// 重新导出 use iced::{Background, Border, Color, Element, Length}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Color, Element, Length};

/// 重新导出 use desktop_ui_controls::overlays::PointBelowOverlay，让上层模块通过稳定路径访问。
use desktop_ui_controls::overlays::PointBelowOverlay;
/// 重新导出 use desktop_ui_controls::right_click_area::RightClickArea，让上层模块通过稳定路径访问。
use desktop_ui_controls::right_click_area::RightClickArea;
use super::GitDiffContextMenuState;
use super::DiffMessage as Message;

pub fn is_dark_theme(theme: &iced::Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub fn diff_selection_menu_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let ext = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::button::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: match status {
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::button::Status::Pressed => {
                Some(Background::Color(ext.background.strong.color.scale_alpha(if is_dark {
                    0.34
                } else {
                    0.18
                })))
            }
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::button::Status::Hovered => {
                Some(Background::Color(ext.background.weak.color.scale_alpha(if is_dark {
                    0.3
                } else {
                    0.72
                })))
            }
            _ => None,
        },
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 5.0.into() },
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: theme.palette().text,
        ..Default::default()
    }
}

pub fn diff_selection_menu_container_style(
    theme: &iced::Theme,
) -> iced::widget::container::Style {
    let ext = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(if is_dark {
            ext.background.base.color.scale_alpha(0.96)
        } else {
            ext.background.base.color.scale_alpha(0.99)
        })),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border {
            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            width: 1.0,
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: ext.background.strong.color.scale_alpha(if is_dark { 0.52 } else { 0.74 }),
            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            radius: 10.0.into(),
        },
        // shadow 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shadow: iced::Shadow {
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: Color::BLACK.scale_alpha(if is_dark { 0.20 } else { 0.08 }),
            // offset 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            offset: iced::Vector::new(0.0, 4.0),
            // blur_radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            blur_radius: 10.0,
        },
        ..Default::default()
    }
}

pub fn open_diff_context_menu_message(
    file: &str,
    line: usize,
    is_old: bool,
    text: &str,
    pos: iced::Point,
) -> Message {
    Message::OpenDiffContextMenu {
        file: file.to_string(),
        line,
        is_old,
        text: text.to_string(),
        x: pos.x,
        y: pos.y,
    }
}

pub fn diff_context_menu_matches(
    menu: &GitDiffContextMenuState,
    file: &str,
    line: usize,
    is_old: bool,
) -> bool {
    menu.file == file && menu.line == line && menu.is_old == is_old
}

/// 处理 diff selection menu 对应的局部职责。
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
pub fn diff_selection_menu<'a>(
    locale: config_ui::DesktopLocale,
) -> Element<'a, Message> {
    let menu_item = |label: &'static str, message: Message| {
        button(
            container(text(label).size(12))
                .width(Length::Fill)
                .padding([5, 9])
                .align_x(iced::alignment::Horizontal::Left),
        )
        .width(Length::Fill)
        .padding(0)
        .style(diff_selection_menu_button_style)
        .on_press(message)
    };

    container(
        column![
            menu_item(
                desktop_i18n::I18n::new(locale).__("添加评论"),
                Message::OpenDiffCommentDraft
            ),
            menu_item(
                desktop_i18n::I18n::new(locale).__("选择行"),
                Message::SelectDiffContextStageLines
            ),
            menu_item(
                desktop_i18n::I18n::new(locale).__("取消行"),
                Message::ClearDiffContextStageLines
            ),
            menu_item(
                desktop_i18n::I18n::new(locale).__("丢弃更改"),
                Message::DiscardDiffSelection
            ),
            menu_item(
                desktop_i18n::I18n::new(locale).__("复制"),
                Message::CopyDiffSelection
            ),
            menu_item(
                desktop_i18n::I18n::new(locale).__("添加到会话"),
                Message::InsertDiffSelectionToChat
            ),
        ]
        .spacing(0)
        .width(Length::Fixed(138.0)),
    )
    .padding([3, 3])
    .style(diff_selection_menu_container_style)
    .into()
}

/// 处理 wrap diff row with context menu 对应的局部职责。
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
pub fn wrap_diff_row_with_context_menu<'a, Outer: Clone + 'a>(
    locale: config_ui::DesktopLocale, menu: Option<&GitDiffContextMenuState>,
    // file_key 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    file_key: &str,
    // event_line 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    event_line: usize,
    // event_is_old 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    event_is_old: bool,
    // row_text 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    row_text: String,
    // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    content: Element<'a, Outer>,
    map: fn(Message) -> Outer,
) -> Element<'a, Outer> {
    let file_for_click = file_key.to_string();
    let file_for_overlay = file_key.to_string();
    let row_text_for_click = row_text.clone();
    let right_click = Element::new(
        RightClickArea::new(
            content,
            // Box 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Box::new(move |pos| {
                map(open_diff_context_menu_message(
                    &file_for_click,
                    event_line,
                    event_is_old,
                    &row_text_for_click,
                    pos,
                ))
            }),
        )
        .preserve_on_right_click(),
    );

    let is_open = menu.is_some_and(|menu| {
        diff_context_menu_matches(menu, &file_for_overlay, event_line, event_is_old)
    });

    if let Some(menu) = menu
        .filter(|menu| diff_context_menu_matches(menu, &file_for_overlay, event_line, event_is_old))
    {
        // PointBelowOverlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        PointBelowOverlay::new(right_click, diff_selection_menu(locale).map(map))
            .show(is_open)
            .anchor(iced::Point::new(menu.x, menu.y))
            .gap(2.0)
            .on_close(map(Message::CloseDiffContextMenu))
            .capture_outside_click(false)
            .into()
    } else {
        right_click
    }
}
