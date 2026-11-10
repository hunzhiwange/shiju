use iced::keyboard::{Event, Key, Modifiers, key};
use iced::widget::pane_grid::{Direction, Pane};
use iced::widget::{button, column, container, row, svg, text};
use iced::{Border, Element, Length, Point, Shadow, Theme, Vector};

use super::{TAB_HEADER_HEIGHT, toolbar_icon_button};
use crate::app::assets::{self, Icon};
use crate::app::components::overlays::point_below::PointBelowOverlay;
use crate::app::i18n::I18n;
use crate::app::message::TerminalMessage;
use crate::app::{App, Message};

const MACOS: bool = cfg!(target_os = "macos");
const DIRECTIONS: [(Direction, key::Named, Icon, &str, &str); 4] = [
    (Direction::Left, key::Named::ArrowLeft, Icon::ArrowLeft, "向左拆分", "←"),
    (Direction::Right, key::Named::ArrowRight, Icon::ArrowRight, "向右拆分", "→"),
    (Direction::Up, key::Named::ArrowUp, Icon::ArrowUp, "向上拆分", "↑"),
    (Direction::Down, key::Named::ArrowDown, Icon::ArrowDown, "向下拆分", "↓"),
];

pub(super) fn shortcut(event: &Event) -> Option<Direction> {
    let Event::KeyPressed { key, physical_key, modifiers, .. } = event else {
        return None;
    };
    if MACOS {
        if *modifiers == Modifiers::LOGO
            && matches!(key, Key::Character(c) if c.eq_ignore_ascii_case("d"))
        {
            return Some(Direction::Right);
        }
    } else if *modifiers == Modifiers::CTRL | Modifiers::SHIFT
        && (*physical_key == key::Physical::Code(key::Code::Digit5)
            || matches!(key, Key::Character(c) if c == "5"))
    {
        return Some(Direction::Right);
    }
    let arrow_modifiers = Modifiers::CTRL | if MACOS { Modifiers::ALT } else { Modifiers::SHIFT };
    if *modifiers != arrow_modifiers {
        return None;
    }
    DIRECTIONS
        .iter()
        .find_map(|(direction, arrow, ..)| (*key == Key::Named(*arrow)).then_some(*direction))
}

pub(super) fn split_items(
    app: &App,
    on_split: impl Fn(Direction) -> TerminalMessage,
) -> [Element<'static, Message>; 4] {
    let i18n = I18n::new(app.desktop_locale);
    DIRECTIONS.map(|(direction, _, icon, label, arrow)| {
        let shortcut = match (MACOS, direction) {
            (true, Direction::Right) => "⌘D".into(),
            (false, Direction::Right) => "Ctrl+Shift+5".into(),
            (true, _) => format!("⌃⌥{arrow}"),
            (false, _) => format!("Ctrl+Shift+{arrow}"),
        };
        let icon =
            svg::Svg::new(assets::get_icon(icon)).width(13).height(13).style(|theme: &Theme, _| {
                svg::Style { color: Some(theme.palette().text.scale_alpha(0.75)) }
            });
        let item = row![
            icon,
            text(i18n.__(label)).size(13).width(Length::Fill),
            text(shortcut).size(12).style(|theme: &Theme| text::Style {
                color: Some(theme.palette().text.scale_alpha(0.72)),
            }),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        button(item)
            .width(Length::Fill)
            .height(30)
            .padding([4, 8])
            .style(|theme: &Theme, status| {
                let palette = theme.extended_palette();
                let background = match status {
                    button::Status::Hovered | button::Status::Pressed => {
                        Some(palette.primary.base.color.scale_alpha(0.15).into())
                    }
                    _ => None,
                };
                button::Style {
                    background,
                    text_color: theme.palette().text,
                    border: Border::default().rounded(5),
                    ..Default::default()
                }
            })
            .on_press(Message::Terminal(on_split(direction)))
            .into()
    })
}

pub(super) fn split_button(app: &App, pane: Pane) -> Element<'static, Message> {
    let items = column![].extend(split_items(app, |direction| {
        TerminalMessage::Split(app.terminal.panes.instance.clone(), pane, direction)
    }));
    let menu =
        container(items).width(if MACOS { 200 } else { 240 }).padding(4).style(|theme: &Theme| {
            let palette = theme.extended_palette().background;
            container::Style {
                background: Some(palette.base.color.into()),
                border: Border::default()
                    .rounded(8)
                    .width(1)
                    .color(palette.strong.color.scale_alpha(0.7)),
                shadow: Shadow {
                    color: iced::Color::BLACK.scale_alpha(0.22),
                    offset: Vector::new(0.0, 3.0),
                    blur_radius: 9.0,
                },
                ..Default::default()
            }
        });
    PointBelowOverlay::new(
        toolbar_icon_button(
            Icon::Columns,
            Message::Terminal(TerminalMessage::ToggleSplitMenu(pane)),
        ),
        menu,
    )
    .show(app.terminal.panes.split_menu == Some(pane))
    .anchor(Point::new(-14.0, TAB_HEADER_HEIGHT))
    .gap(2.0)
    .capture_keyboard()
    .on_close(Message::Terminal(TerminalMessage::CloseSplitMenu))
    .into()
}
