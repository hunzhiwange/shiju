//! 项目工作区布局模块，负责侧栏、主区域、右侧面板和拖拽提示的组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::{container, mouse_area, row};
use iced::{Element, Length};

use super::super::handles::HResizeHandle;
use super::super::badge_styles::content_panel_style;

/// 执行本模块的界面辅助逻辑。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
#[derive(Clone, Copy)]
pub struct MainLayout {
    pub chat_panel_fullscreen: bool,
    pub chat_panel_half_fullscreen: bool,
    pub git_diff_fullscreen: bool,
    pub git_diff_half_fullscreen: bool,
    pub file_manager_width: f32,
    pub split_ratio: f32,
    pub show_diff: bool,
    pub show_file_manager: bool,
}
pub fn main_area<'a, Message: Clone + 'static>(state: MainLayout, chat: Element<'a, Message>, preview: Element<'a, Message>, mut files: impl FnMut() -> Element<'a, Message>, split_drag: Message, file_drag: Message, spacing: f32, content_pad: f32) -> Element<'a, Message> {
    let chat_panel = container(chat)
        .width(Length::Fill)
        .height(Length::Fill);

    let diff_panel_padding = if state.git_diff_fullscreen || state.git_diff_half_fullscreen {
        iced::Padding { top: content_pad, right: 10.0, bottom: 0.0, left: 10.0 }
    } else {
        iced::Padding::from(content_pad)
    };

    let diff_panel = container(preview)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(content_panel_style)
        .padding(diff_panel_padding)
        .clip(true)
        .width(Length::Fill)
        .height(Length::Fill);

    if state.chat_panel_fullscreen {
        return chat_panel.into();
    }

    if state.chat_panel_half_fullscreen {
        let mut r = row![chat_panel].spacing(spacing);

        if state.show_file_manager {
            let file_manager_divider = mouse_area(HResizeHandle)
                .on_press(file_drag.clone());
            let file_manager_panel = container(files())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(content_panel_style)
                .padding(content_pad)
                .clip(true)
                .width(Length::Fixed(state.file_manager_width))
                .height(Length::Fill);
            r = r.push(file_manager_divider).push(file_manager_panel);
        }

        return r.width(Length::Fill).into();
    }

    if state.git_diff_fullscreen {
        return diff_panel.into();
    }

    if state.git_diff_half_fullscreen {
        let mut r = row![diff_panel].spacing(spacing);

        if state.show_file_manager {
            let file_manager_divider = mouse_area(HResizeHandle)
                .on_press(file_drag.clone());
            let file_manager_panel = container(files())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(content_panel_style)
                .padding(content_pad)
                .clip(true)
                .width(Length::Fixed(state.file_manager_width))
                .height(Length::Fill);
            r = r.push(file_manager_divider).push(file_manager_panel);
        }

        return r.width(Length::Fill).into();
    }

    let mut r: iced::widget::Row<'_, Message> = if state.show_diff {
        let left_portion = (state.split_ratio * 1000.0) as u16;
        let right_portion = 1000u16.saturating_sub(left_portion.max(1));
        let divider = mouse_area(HResizeHandle)
            .on_press(split_drag);
        let diff_panel = diff_panel.width(Length::FillPortion(right_portion.max(1)));
        row![chat_panel.width(Length::FillPortion(left_portion.max(1))), divider, diff_panel]
            .spacing(spacing)
    } else {
        row![chat_panel].spacing(spacing)
    };

    if state.show_file_manager {
        let file_manager_divider = mouse_area(HResizeHandle)
            .on_press(file_drag.clone());
        let file_manager_panel = container(files())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(content_panel_style)
            .padding(content_pad)
            .clip(true)
            .width(Length::Fixed(state.file_manager_width))
            .height(Length::Fill);
        r = r.push(file_manager_divider).push(file_manager_panel);
    }

    let content: Element<'_, Message> = r.width(Length::Fill).into();
    content
}
