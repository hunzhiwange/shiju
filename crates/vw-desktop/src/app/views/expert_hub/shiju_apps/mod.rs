use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

use crate::app::message::ShijuAppsMessage;
use crate::app::shiju_apps::ShijuAppsTab;
use crate::app::{App, Message};

mod detail;
pub(super) mod mine;
mod store;
pub(super) mod styles;

const CARDS_PER_ROW: usize = 4;
const APP_TITLE_MAX_WIDTH: usize = 22;

pub(super) fn app_title(name: &str) -> String {
    let width = |character: char| usize::from(!character.is_ascii()) + 1;
    if name.chars().map(width).sum::<usize>() <= APP_TITLE_MAX_WIDTH {
        return name.to_string();
    }

    let mut title = String::new();
    let mut used = 0;
    let content_width = APP_TITLE_MAX_WIDTH - "...".len();
    for character in name.chars() {
        let character_width = width(character);
        if used + character_width > content_width {
            break;
        }
        title.push(character);
        used += character_width;
    }
    title.push_str("...");
    title
}

pub(super) fn card_grid(cards: Vec<Element<'static, Message>>) -> Element<'static, Message> {
    let mut grid = column![].spacing(12);
    let mut cards = cards.into_iter();

    loop {
        let cards_in_row = cards.by_ref().take(CARDS_PER_ROW).collect::<Vec<_>>();
        if cards_in_row.is_empty() {
            break;
        }
        grid = grid.push(row(cards_in_row).spacing(12).wrap());
    }

    grid.into()
}

pub(super) fn tab_specs() -> &'static [(&'static str, ShijuAppsTab)] {
    &[("我的应用", ShijuAppsTab::Mine), ("应用商店", ShijuAppsTab::Store)]
}

fn tab_button(label: &'static str, tab: ShijuAppsTab, selected: bool) -> Element<'static, Message> {
    button(text(label).size(14))
        .on_press(Message::ShijuApps(ShijuAppsMessage::SelectTab(tab)))
        .padding([8, 14])
        .style(move |theme, status| styles::tab_style(theme, status, selected))
        .into()
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let content = if let Some(target) = app.expert_hub.shiju_apps.detail {
        detail::view(app, target)
    } else {
        let tabs = tab_specs().iter().fold(row!().spacing(8), |tabs, (label, tab)| {
            tabs.push(tab_button(label, *tab, app.expert_hub.shiju_apps.tab == *tab))
        });
        let body = match app.expert_hub.shiju_apps.tab {
            ShijuAppsTab::Store => store::view(&app.expert_hub.shiju_apps.store),
            ShijuAppsTab::Mine => mine::view(&app.expert_hub.shiju_apps.mine),
        };

        column![
            text("识矩应用").size(24),
            text("浏览平台应用，或查看当前账户已经安装的应用。")
                .size(13)
                .style(styles::muted_text_style),
            tabs,
            body,
        ]
        .spacing(14)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    };

    container(content).width(Length::Fill).height(Length::Fill).padding([24, 28]).into()
}
