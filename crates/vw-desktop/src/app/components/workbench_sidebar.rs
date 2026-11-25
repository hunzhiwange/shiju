//! 「我的工作台」和 Git 工作台共享的侧栏控件。
use crate::app::Message;
use desktop_assets::assets::Icon;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::{
    icon_svg, icons::icon_btn_style, settings_muted_text_style,
};
use desktop_ui_controls::tooltip::project_tooltip;
use iced::widget::{Column, button, column, container, row, scrollable, svg, text, tooltip};
use iced::{Alignment, Border, Element, Length, Theme};

pub(crate) fn project_row<'a>(
    name: &'a str,
    directory: &'a str,
    selected: bool,
    action: Option<Message>,
    tooltip_position: tooltip::Position,
) -> Element<'a, Message> {
    let icon = icon_svg(if selected { Icon::FolderOpen } else { Icon::FolderClosed }, 15.0).style(
        |theme: &Theme, _| svg::Style {
            color: Some(theme.extended_palette().primary.strong.color),
        },
    );
    let badge = container(icon).center_x(23).center_y(23).style(|theme: &Theme| container::Style {
        background: Some(theme.extended_palette().primary.base.color.scale_alpha(0.12).into()),
        border: Border::default().rounded(6),
        ..Default::default()
    });
    let content = row![badge, SingleLineLabel::new(name.to_owned(), 13.0)]
        .spacing(7)
        .align_y(Alignment::Center);
    project_tooltip(sidebar_button(content, action, selected), name, directory, tooltip_position)
}

pub(crate) fn sidebar_section<'a>(
    header: Element<'a, Message>,
    rows: Column<'a, Message>,
) -> Element<'a, Message> {
    container(column![header, vertical(rows).height(Length::Fill)])
        .style(sidebar_surface)
        .padding([2, 2])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub(crate) fn sidebar_header<'a>(
    title: &'static str,
    action: Element<'a, Message>,
) -> Element<'a, Message> {
    container(row![sidebar_note(title).width(Length::Fill), action].align_y(Alignment::Center))
        .padding([3, 7])
        .width(Length::Fill)
        .into()
}

pub(crate) fn sidebar_note(label: &str) -> iced::widget::Text<'_> {
    text(label).size(12).style(settings_muted_text_style)
}

pub(crate) fn sidebar_button<'a>(
    content: iced::widget::Row<'a, Message>,
    action: Option<Message>,
    selected: bool,
) -> button::Button<'a, Message> {
    button(content).padding([4, 6]).width(Length::Fill).on_press_maybe(action).style(
        move |theme, status| {
            let mut style = icon_btn_style(theme, status);
            if selected && matches!(status, button::Status::Active | button::Status::Disabled) {
                style.background =
                    Some(theme.extended_palette().primary.base.color.scale_alpha(0.12).into());
            }
            style
        },
    )
}

pub(crate) fn sidebar_surface(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        ..Default::default()
    }
}

pub(crate) fn vertical<'a>(content: Column<'a, Message>) -> scrollable::Scrollable<'a, Message> {
    scrollable(container(content).padding([2, 7]).width(Length::Fill)).direction(
        scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4)),
    )
}
