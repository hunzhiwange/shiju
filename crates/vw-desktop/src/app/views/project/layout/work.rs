//! 工作模式布局：组合专属左栏、对话区与项目资源区。

use iced::widget::{Space, container, mouse_area, row, stack};
use iced::{Element, Length};

use crate::app::components::chat_panel;
use crate::app::message::ViewMessage;
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message};

use super::super::handles::HResizeHandle;
use super::chat::chat_area;
use super::chrome::right_column_chrome;
use super::{work_resources, work_sidebar};

/// 构建工作模式布局，复用共享底部终端。
pub fn build_work_layout(
    app: &App,
    settings_panel_width: f32,
    left_rail_width: f32,
    session_panel_width_scale: f32,
    corner_radius: f32,
    spacing: f32,
    chat_content_pad: f32,
) -> Element<'_, Message> {
    if app.terminal.is_fullscreen {
        return super::bottom::fullscreen_terminal(app);
    }

    let left_sidebar = work_sidebar::view(app);
    let chat = container(chat_area(app, spacing, corner_radius, chat_content_pad))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding::default().right(3.0));
    let resources = container(work_resources::view(app, corner_radius))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding::default().left(3.0));

    let content: Element<'_, Message> =
        if !app.work_resources_visible || app.work_chat_fullscreen || app.work_chat_half_fullscreen
        {
            chat.into()
        } else if app.work_resource_fullscreen || app.work_resource_half_fullscreen {
            resources.into()
        } else {
            let left_portion = (app.work_split_ratio.clamp(0.25, 0.75) * 1_000.0) as u16;
            let right_portion = 1_000u16.saturating_sub(left_portion).max(1);
            let divider = mouse_area(HResizeHandle)
                .on_press(Message::View(ViewMessage::WorkSplitDragStarted));
            row![
                chat.width(Length::FillPortion(left_portion.max(1))),
                divider,
                resources.width(Length::FillPortion(right_portion)),
            ]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        };

    let right_column_base: Element<'_, Message> = content;
    let right_overlay: Element<'_, Message> = chat_panel::tool_dialog_overlay(app)
        .unwrap_or_else(|| Space::new().width(Length::Fill).height(Length::Fill).into());
    let right_column =
        stack![right_column_base, right_overlay].width(Length::Fill).height(Length::Fill);
    let right_column = right_column_chrome(right_column.into(), corner_radius, WorkspaceMode::Work);

    let _ = (settings_panel_width, left_rail_width, session_panel_width_scale);
    if app.work_chat_fullscreen || app.work_resource_fullscreen {
        return right_column;
    }
    let sidebar_divider =
        mouse_area(HResizeHandle).on_press(Message::View(ViewMessage::WorkSidebarDragStarted));
    let workspace = row![left_sidebar, sidebar_divider, right_column].spacing(spacing);

    if !app.terminal.is_visible {
        workspace.into()
    } else {
        iced::widget::column![
            container(workspace)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(iced::Padding::default().bottom(super::WORKSPACE_BOTTOM_GAP)),
            super::bottom::bottom_panel(app)
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
