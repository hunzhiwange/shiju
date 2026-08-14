//! 顶部代码/工作界面模式选择器。

use iced::widget::text::LineHeight;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Font, Length, Theme};

use super::widgets::{icon_svg, menu_container_style, workspace_mode_button_style};
use crate::app::assets::Icon;
use crate::app::components::overlays::BelowOverlay;
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, message};

const MODE_MENU_WIDTH: f32 = 248.0;

pub(super) const fn copy(mode: WorkspaceMode) -> (&'static str, &'static str) {
    match mode {
        WorkspaceMode::Code => ("代码", "构建、调试与审查"),
        WorkspaceMode::Work => ("工作", "对话、整理与协作"),
    }
}

pub(super) const fn mode_icon(mode: WorkspaceMode) -> Icon {
    match mode {
        WorkspaceMode::Code => Icon::Code,
        WorkspaceMode::Work => Icon::ChatTextFill,
    }
}

pub(super) const fn mode_label_font() -> Font {
    Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT }
}

fn centered_icon(mode: WorkspaceMode, selected: bool) -> Element<'static, Message> {
    container(icon_svg(mode_icon(mode)).style(move |theme: &Theme, _| iced::widget::svg::Style {
        color: Some(theme.palette().text.scale_alpha(if selected { 1.0 } else { 0.72 })),
    }))
    .width(Length::Fixed(18.0))
    .height(Length::Fixed(18.0))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

fn mode_item_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        iced::widget::button::Status::Hovered => {
            Some(palette.background.strong.color.scale_alpha(0.72).into())
        }
        iced::widget::button::Status::Pressed => {
            Some(palette.background.strong.color.scale_alpha(0.88).into())
        }
        _ if selected => Some(palette.background.strong.color.scale_alpha(0.54).into()),
        _ => None,
    };

    iced::widget::button::Style {
        background,
        text_color: theme.palette().text,
        border: iced::Border { radius: 7.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn mode_item(mode: WorkspaceMode, selected: bool) -> Element<'static, Message> {
    let (label, description) = copy(mode);
    let check: Element<'static, Message> = if selected {
        container(icon_svg(Icon::Check))
            .width(Length::Fixed(18.0))
            .height(Length::Fixed(18.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    } else {
        Space::new().width(Length::Fixed(18.0)).into()
    };

    button(
        container(
            row![
                centered_icon(mode, selected),
                column![
                    text(label)
                        .size(14)
                        .line_height(LineHeight::Relative(1.0))
                        .font(mode_label_font())
                        .style(move |theme: &Theme| iced::widget::text::Style {
                            color: Some(theme.palette().text.scale_alpha(if selected {
                                1.0
                            } else {
                                0.88
                            }),),
                        }),
                    text(description).size(12).line_height(LineHeight::Relative(1.0)).style(
                        |theme: &Theme| iced::widget::text::Style {
                            color: Some(theme.palette().text.scale_alpha(0.58)),
                        }
                    )
                ]
                .spacing(4)
                .width(Length::Fill),
                check
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        )
        .height(Length::Fixed(44.0))
        .align_y(iced::alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding([0, 10])
    .style(move |theme: &Theme, status| mode_item_style(theme, status, selected))
    .on_press(Message::View(message::ViewMessage::WorkspaceModeSelected(mode)))
    .into()
}

pub(super) fn menu(app: &App) -> Element<'_, Message> {
    container(
        column![
            mode_item(WorkspaceMode::Work, app.workspace_mode == WorkspaceMode::Work),
            mode_item(WorkspaceMode::Code, app.workspace_mode == WorkspaceMode::Code),
        ]
        .spacing(3),
    )
    .padding(5)
    .width(Length::Fixed(MODE_MENU_WIDTH))
    .style(menu_container_style)
    .into()
}

pub(super) fn selector(app: &App) -> Element<'_, Message> {
    let (label, _) = copy(app.workspace_mode);
    let chevron = container(icon_svg(Icon::ChevronDown).style(|theme: &Theme, _| {
        iced::widget::svg::Style { color: Some(theme.palette().text.scale_alpha(0.62)) }
    }))
    .width(Length::Fixed(16.0))
    .height(Length::Fixed(18.0))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center);
    let toggle_content = container(
        row![
            centered_icon(app.workspace_mode, true),
            text(label).size(13).line_height(LineHeight::Relative(1.0)).font(mode_label_font()),
            chevron
        ]
        .spacing(5)
        .height(Length::Fill)
        .align_y(iced::Alignment::Center),
    )
    .height(Length::Fill)
    .padding([0, 9])
    .align_y(iced::alignment::Vertical::Center);
    let toggle = button(toggle_content)
        .height(Length::Fixed(24.0))
        .padding(0)
        .style(move |theme: &Theme, status| {
            workspace_mode_button_style(theme, status, app.show_workspace_mode_popover)
        })
        .on_press(Message::View(message::ViewMessage::ToggleWorkspaceModePopover));

    BelowOverlay::new(toggle, menu(app))
        .show(app.show_workspace_mode_popover)
        .gap(4.0)
        .on_close(Message::View(message::ViewMessage::CloseWorkspaceModePopover))
        .into()
}
