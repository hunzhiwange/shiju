use iced::widget::{Space, button, column, container, row, rule, scrollable, text};
use iced::{Border, Element, Length, Theme};

use super::split_menu::split_items;
use desktop_ui_controls::overlays::point_below::PointBelowOverlay;
use desktop_ui_controls::right_click_area::RightClickArea;
use desktop_i18n::I18n;
use crate::message::TerminalMessage;
use crate::message::ContentAction;
use crate::TerminalTab;
use crate::{ViewInput, message::Message};

pub(super) fn terminal_surface<'a>(app: ViewInput<'a>, tab: &'a TerminalTab) -> Element<'a, Message> {
    surface(&tab.term, tab.id, app.desktop_locale, app.content_menu, true)
}

pub fn surface<'a>(
    term: &'a iced_term::Terminal,
    id: u64,
    locale: config_ui::DesktopLocale,
    content_menu: Option<(u64, iced::Point)>,
    chat_actions: bool,
) -> Element<'a, Message> {
    let terminal = container(
        iced_term::TerminalView::show(term)
            .map(|event| Message::Terminal(TerminalMessage::Event(event))),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(super::terminal_content_padding(id));
    let surface = RightClickArea::new(
        terminal.into(),
        Box::new(move |position| Message::Terminal(TerminalMessage::ContentMenuOpen(id, position))),
    )
    .preserve_on_right_click();
    let position = content_menu.filter(|(menu_id, _)| *menu_id == id);
    let menu = if position.is_some() { menu(locale, id, term.has_selection(), chat_actions) } else { Space::new().into() };

    PointBelowOverlay::new(surface, menu)
        .show(position.is_some())
        .anchor(position.map_or(iced::Point::ORIGIN, |(_, position)| position))
        .gap(2.0)
        .close_on_target_click(true)
        .capture_keyboard()
        .on_close(Message::Terminal(TerminalMessage::ContentMenuClose))
        .into()
}

fn menu(
    locale: config_ui::DesktopLocale,
    id: u64,
    selected: bool,
    chat_actions: bool,
) -> Element<'static, Message> {
    let i18n = I18n::new(locale);
    let item = |label, shortcut, action, enabled: bool| {
        button(
            row![
                text(label).size(13),
                Space::new().width(Length::Fill),
                text(shortcut).size(12).style(|theme: &Theme| text::Style {
                    color: Some(theme.palette().text.scale_alpha(0.65)),
                }),
            ]
            .height(Length::Fill)
            .align_y(iced::Alignment::Center)
            .spacing(12),
        )
        .width(Length::Fill)
        .height(30)
        .padding([4, 9])
        .on_press_maybe(
            enabled.then_some(Message::Terminal(TerminalMessage::ContentMenuAction(id, action))),
        )
        .style(|theme: &Theme, status| {
            let palette = theme.extended_palette().background;
            button::Style {
                background: match status {
                    button::Status::Hovered | button::Status::Pressed => {
                        Some(palette.weak.color.into())
                    }
                    _ => None,
                },
                border: Border::default().rounded(5),
                text_color: match status {
                    button::Status::Disabled => palette.base.text.scale_alpha(0.42),
                    _ => palette.base.text,
                },
                ..Default::default()
            }
        })
    };
    let shortcut = |mac, other| if cfg!(target_os = "macos") { mac } else { other };
    let [split_left, split_right, split_up, split_down] = split_items(locale, |direction| {
        TerminalMessage::ContentMenuAction(id, ContentAction::Split(direction))
    });
    let mut items = column![
        item(i18n.__("新建终端"), "", ContentAction::NewTerminal, true),
        rule::horizontal(1),
        item(i18n.__("复制"), shortcut("⌘ C", "Ctrl+Shift+C"), ContentAction::Copy, selected),
        item(i18n.__("粘贴"), "", ContentAction::Paste, true),
        item(
            i18n.__("粘贴纯文本"),
            shortcut("⌘ V", "Ctrl+Shift+V"),
            ContentAction::PasteText,
            true,
        ),
        item(i18n.__("全选"), shortcut("⌘ A", "Ctrl+Shift+A"), ContentAction::SelectAll, true),
        item(i18n.__("清屏"), shortcut("⌘ K", "Ctrl+Shift+K"), ContentAction::Clear, true),
    ];
    if chat_actions {
        items = items.push(rule::horizontal(1))
            .push(item(i18n.__("在聊天中求助"), "", ContentAction::AskInChat, true))
            .push(item(i18n.__("插入到聊天"), "", ContentAction::InsertInChat, selected));
    }
    let items = items.extend([
        rule::horizontal(1).into(),
        split_left,
        split_right,
        split_up,
        split_down,
        rule::horizontal(1).into(),
    ])
    .push(item(i18n.__("关闭终端标签页"), "", ContentAction::CloseTab, true))
    .spacing(2);
    container(scrollable(items).direction(scrollable::Direction::Vertical(
        scrollable::Scrollbar::new().width(4).scroller_width(4).spacing(3),
    )))
    .width(240)
    .padding(4)
    .style(|theme: &Theme| {
        let palette = theme.extended_palette().background;
        container::Style {
            background: Some(palette.base.color.into()),
            border: Border::default().rounded(8).width(1).color(palette.strong.color),
            shadow: iced::Shadow {
                color: iced::Color::BLACK.scale_alpha(0.18),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..Default::default()
        }
    })
    .into()
}
