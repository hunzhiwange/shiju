//! 预览面板视图组件。
//!
//! 本模块负责预览内容、菜单、面包屑、LSP 标识或浮层宿主的局部构建。

use crate::app::assets::Icon;
/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};
/// 重新导出 use iced::widget::{Space, button, column, container, row, scrollable, stack, text}，让上层模块通过稳定路径访问。
use iced::widget::{Space, button, column, container, row, scrollable, stack, text};
/// 重新导出 use iced::{Background, Border, Color, Element, Length, Vector}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Color, Element, Length, Vector};

/// 重新导出 use super::super::styles::{file_icon_for, menu_button_style, small_icon_svg}，让上层模块通过稳定路径访问。
use super::super::styles::{file_icon_for, menu_button_style, small_icon_svg};

/// 构建 menu ui 对应的 Iced 界面片段或中间数据。
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
pub fn build_menu_ui(app: &App) -> Element<'_, Message> {
    let context_menu = build_context_menu(app);
    let nav_popup = build_nav_popup(app);
    stack![context_menu, nav_popup].into()
}

/// 构建 context menu 对应的 Iced 界面片段或中间数据。
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
fn build_context_menu(app: &App) -> Element<'_, Message> {
    if !app.show_preview_context_menu {
        return container(Space::new()).into();
    }

    let Some((path, _start_line, _start_col, _end_line, _end_col)) = &app.preview_context_target
    else {
        return container(Space::new()).into();
    };

    let Some(_pos) = &app.preview_context_menu_pos else {
        return container(Space::new()).into();
    };

    let tab_opt = app.preview_tabs.iter().find(|t| t.path == *path);
    let items: Element<'_, Message> = if tab_opt.is_some() {
        column![
            button(text("复制选择"))
                .style(menu_button_style)
                .on_press(Message::Preview(message::PreviewMessage::ContextMenuCopy))
                .width(Length::Fill),
            button(text("剪切"))
                .style(menu_button_style)
                .on_press(Message::Preview(message::PreviewMessage::ContextMenuCut))
                .width(Length::Fill),
            button(text("粘贴"))
                .style(menu_button_style)
                .on_press(Message::Preview(message::PreviewMessage::ContextMenuPaste))
                .width(Length::Fill),
            button(text("删除"))
                .style(menu_button_style)
                .on_press(Message::Preview(message::PreviewMessage::ContextMenuDelete))
                .width(Length::Fill),
            button(text("插入选择到chat"))
                .style(menu_button_style)
                .on_press(Message::Chat(message::ChatMessage::InsertSelected))
                .width(Length::Fill),
            button(text("插入选区位置信息"))
                .style(menu_button_style)
                .on_press(Message::Chat(message::ChatMessage::InsertSelectionPositions))
                .width(Length::Fill),
        ]
        .spacing(6)
        .into()
    } else {
        // Space 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Space::new().into()
    };

    container(items)
        .width(Length::Fixed(188.0))
        .padding([6, 8])
        .style(context_menu_container_style)
        .into()
}

pub(super) fn context_menu_container_style(theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgba(
            theme.palette().background.r,
            theme.palette().background.g,
            theme.palette().background.b,
            0.98,
        ))),
        border: Border {
            width: 1.0,
            color: Color::from_rgb8(0xCC, 0xCC, 0xCC),
            radius: iced::border::Radius::from(8.0),
        },
        ..Default::default()
    }
}

/// 构建 nav popup 对应的 Iced 界面片段或中间数据。
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
fn build_nav_popup(app: &App) -> Element<'_, Message> {
    let Some((parent_path, _x, _y, items)) = &app.preview_nav_popup else {
        return container(Space::new()).into();
    };

    let list = column(items.iter().map(|(name, is_dir)| {
        let icon = nav_popup_item_icon(name, *is_dir);

        button(
            row![container(small_icon_svg(icon)).width(Length::Fixed(16.0)), text(name).size(13)]
                .spacing(6)
                .align_y(iced::Alignment::Center),
        )
        .on_press(nav_popup_item_message(parent_path, name, *is_dir))
        .style(nav_popup_item_button_style)
        .width(Length::Fill)
        .padding([4, 8])
        .into()
    }))
    .spacing(0);

    let popup = container(scrollable(list).height(Length::Fixed(300.0)))
        .width(Length::Fixed(240.0))
        .padding(4)
        .style(nav_popup_container_style);

    popup.into()
}

pub(super) fn nav_popup_item_icon(name: &str, is_dir: bool) -> Icon {
    if is_dir { Icon::FolderOpen } else { file_icon_for(name) }
}

pub(super) fn nav_popup_item_message(parent_path: &str, name: &str, is_dir: bool) -> Message {
    let full_path = std::path::Path::new(parent_path).join(name).to_string_lossy().to_string();

    Message::Batch(vec![
        Message::Preview(message::PreviewMessage::CloseNavPopup),
        if is_dir {
            Message::Project(crate::app::message::ProjectMessage::ToggleTreeDir(full_path))
        } else {
            Message::Preview(message::PreviewMessage::Open(full_path))
        },
    ])
}

pub(super) fn nav_popup_item_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let p = theme.palette().primary;
    let hover_bg = Color::from_rgba(p.r, p.g, p.b, 0.10);
    let pressed_bg = Color::from_rgba(p.r, p.g, p.b, 0.18);
    let bg = match status {
        iced::widget::button::Status::Hovered => Some(Background::Color(hover_bg)),
        iced::widget::button::Status::Pressed => Some(Background::Color(pressed_bg)),
        _ => None,
    };
    iced::widget::button::Style {
        background: bg,
        text_color: theme.palette().text,
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 4.0.into() },
        ..Default::default()
    }
}

pub(super) fn nav_popup_container_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        border: Border { width: 1.0, color: palette.primary.weak.color, radius: 8.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK,
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}
