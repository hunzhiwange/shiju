//! 任务看板视图模块，负责看板列、拖拽预览和整体页面组织。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::{column, container, mouse_area, opaque, row, scrollable, stack};
use iced::{Background, Color, Element, Length, Theme};

use crate::app::components::system_settings_common::settings_modal_backdrop_style;
use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardViewMode;
use crate::app::{App, Message};

mod board;
mod common;
mod control;
mod drag;
mod modals;
mod panel;

/// 渲染对应界面。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn view(app: &App) -> Element<'_, Message> {
    let header = control::build_control_buttons(app);
    let content = if app.task_board_worktree_pixel_office {
        column![header].spacing(0).width(Length::Fill).height(Length::Fill)
    } else {
        let kanban = board::build_kanban_board(app);
        column![header, iced::widget::Space::new().height(12.0), kanban]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill)
    };

    let main_surface: Element<'_, Message> = if app.task_board_worktree_pixel_office
        && app.task_board_view_mode == TaskBoardViewMode::Kanban
    {
        scrollable(content)
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        content.into()
    };

    let main_mouse_area = iced::widget::mouse_area(main_surface)
        .on_press(Message::TaskBoard(TaskBoardMessage::ContextMenuClosed));

    let base = container(main_mouse_area)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(16)
        .style(task_board_container_style);

    let drag_preview_layer = drag::build_drag_preview_layer(app);

    let main_content: Element<'_, Message> = if app.task_board_create_modal_open {
        let panel = panel::build_task_panel(app, false);
        let main_content = stack![base, drag_preview_layer];
        row![container(main_content).width(Length::FillPortion(2)).height(Length::Fill), panel]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else if let Some(_task) = &app.task_board_viewing_logs {
        let detail_panel = panel::build_task_panel(app, true);
        let main_content = stack![base, drag_preview_layer];
        let dismiss_layer = mouse_area(
            container(iced::widget::Space::new().width(Length::Fill).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::TaskBoard(TaskBoardMessage::CloseTaskLogs));
        let detail_layer = row![iced::widget::Space::new().width(Length::Fill), detail_panel]
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill);
        stack![main_content, dismiss_layer, detail_layer].into()
    } else {
        stack![base, drag_preview_layer].into()
    };

    if app.task_board_settings_modal_open {
        let overlay = opaque(
            mouse_area(
                container(iced::widget::Space::new().width(Length::Fill).height(Length::Fill))
                    .style(settings_modal_backdrop_style),
            )
            .on_press(Message::TaskBoard(TaskBoardMessage::CloseSettingsModal)),
        );

        let modal = modals::build_task_settings_modal(app);
        let modal_layer: Element<'_, Message> = opaque(
            container(modal)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        );

        return stack![main_content, stack![overlay, modal_layer]].into();
    }

    main_content
}

fn task_board_container_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            if theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5
            {
                theme.extended_palette().background.base.color.scale_alpha(0.94)
            } else {
                Color::from_rgba8(246, 248, 252, 0.98)
            },
        )),
        ..Default::default()
    }
}
