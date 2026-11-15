//! 项目工作区布局模块，负责侧栏、主区域、右侧面板和拖拽提示的组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::{container, mouse_area, row};
use iced::{Element, Length};

use crate::app::components::{file_tree, preview_panel};
use crate::app::{App, Message, message};

use super::super::handles::HResizeHandle;
use super::super::styles::content_panel_style;
use super::chat::chat_area;

/// 执行本模块的界面辅助逻辑。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
/// - `spacing`: 当前视图构建所需的状态、配置或消息。
/// - `content_pad`: 当前视图构建所需的状态、配置或消息。
/// - `chat_content_pad`: 当前视图构建所需的状态、配置或消息。
/// - `corner_radius`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn main_area(
    app: &App,
    spacing: f32,
    content_pad: f32,
    chat_content_pad: f32,
    corner_radius: f32,
) -> Element<'_, Message> {
    let chat_panel = container(chat_area(app, spacing, corner_radius, chat_content_pad))
        .width(Length::Fill)
        .height(Length::Fill);

    let diff_panel_padding = if app.git_diff_fullscreen || app.git_diff_half_fullscreen {
        iced::Padding { top: content_pad, right: 10.0, bottom: 0.0, left: 10.0 }
    } else {
        iced::Padding::from(content_pad)
    };

    let diff_panel = container(preview_panel::view(app))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(content_panel_style)
        .padding(diff_panel_padding)
        .clip(true)
        .width(Length::Fill)
        .height(Length::Fill);

    if app.chat_panel_fullscreen {
        return chat_panel.into();
    }

    if app.chat_panel_half_fullscreen {
        let mut r = row![chat_panel].spacing(spacing);

        if app.file_manager.show_file_manager {
            let file_manager_divider = mouse_area(HResizeHandle)
                .on_press(Message::View(message::ViewMessage::FileManagerDragStarted));
            let file_manager_panel = container(file_tree::view_file_manager(app))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(content_panel_style)
                .padding(content_pad)
                .clip(true)
                .width(Length::Fixed(app.file_manager_width))
                .height(Length::Fill);
            r = r.push(file_manager_divider).push(file_manager_panel);
        }

        return r.width(Length::Fill).into();
    }

    if app.git_diff_fullscreen {
        return diff_panel.into();
    }

    if app.git_diff_half_fullscreen {
        let mut r = row![diff_panel].spacing(spacing);

        if app.file_manager.show_file_manager {
            let file_manager_divider = mouse_area(HResizeHandle)
                .on_press(Message::View(message::ViewMessage::FileManagerDragStarted));
            let file_manager_panel = container(file_tree::view_file_manager(app))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(content_panel_style)
                .padding(content_pad)
                .clip(true)
                .width(Length::Fixed(app.file_manager_width))
                .height(Length::Fill);
            r = r.push(file_manager_divider).push(file_manager_panel);
        }

        return r.width(Length::Fill).into();
    }

    let mut r: iced::widget::Row<'_, Message> = if app.show_diff {
        let left_portion = (app.split_ratio * 1000.0) as u16;
        let right_portion = 1000u16.saturating_sub(left_portion.max(1));
        let divider = mouse_area(HResizeHandle)
            .on_press(Message::View(message::ViewMessage::SplitDragStarted));
        let diff_panel = diff_panel.width(Length::FillPortion(right_portion.max(1)));
        row![chat_panel.width(Length::FillPortion(left_portion.max(1))), divider, diff_panel]
            .spacing(spacing)
    } else {
        row![chat_panel].spacing(spacing)
    };

    if app.file_manager.show_file_manager {
        let file_manager_divider = mouse_area(HResizeHandle)
            .on_press(Message::View(message::ViewMessage::FileManagerDragStarted));
        let file_manager_panel = container(file_tree::view_file_manager(app))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(content_panel_style)
            .padding(content_pad)
            .clip(true)
            .width(Length::Fixed(app.file_manager_width))
            .height(Length::Fill);
        r = r.push(file_manager_divider).push(file_manager_panel);
    }

    let content: Element<'_, Message> = r.width(Length::Fill).into();
    content
}
#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;
