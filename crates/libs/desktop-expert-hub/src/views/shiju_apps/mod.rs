use iced::{Element};
use iced::widget::{column, row};
pub mod api_key_binding;
pub mod detail;
pub mod mine;
pub mod store;
pub mod styles;
#[derive(Debug, Clone)]
pub enum Message { ShijuApps(desktop_apps_ui::shiju_apps::ShijuAppsMessage), StartBrowserLogin }
pub struct ViewInput<'a> { pub shiju_apps: &'a desktop_app_launch::ShijuAppsState, pub desktop_locale: config_ui::DesktopLocale }
const CARDS_PER_ROW: usize = 4;
const APP_TITLE_MAX_WIDTH: usize = 22;

/// 按中英文显示宽度截断应用标题，超限时追加省略号。
pub fn app_title(name: &str) -> String {
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

/// 将卡片稳定地按每行固定上限分组并允许窄窗口换行。
pub fn card_grid(cards: Vec<Element<'static, Message>>) -> Element<'static, Message> {
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

use iced::widget::{button, text};
use iced::Length;
use desktop_apps_ui::shiju_apps::ShijuAppsMessage;
use desktop_app_launch::ShijuAppsTab;
pub fn tab_specs() -> &'static [(&'static str, ShijuAppsTab)] {
    &[("我的应用", ShijuAppsTab::Mine), ("应用商店", ShijuAppsTab::Store)]
}

fn tab_button(
    i18n: desktop_i18n::I18n,
    label: &'static str,
    tab: ShijuAppsTab,
    selected: bool,
) -> Element<'static, Message> {
    button(text(i18n.__(label)).size(14))
        .on_press(Message::ShijuApps(ShijuAppsMessage::SelectTab(tab)))
        .padding([8, 14])
        .style(move |theme, status| styles::tab_style(theme, status, selected))
        .into()
}

/// 组合应用页签、详情与密钥绑定覆盖层。
pub fn view<'a, M: Clone + 'static>(app: &ViewInput<'a>, tabs: Element<'a, M>, map: fn(Message) -> M) -> Element<'a, M> {
    let content = if let Some(target) = app.shiju_apps.detail {
        detail::view(app, target)
    } else {
        let tabs = tab_specs().iter().fold(row!().spacing(8), |tabs, (label, tab)| {
            tabs.push(tab_button(
                desktop_i18n::I18n::new(app.desktop_locale),
                label,
                *tab,
                app.shiju_apps.tab == *tab,
            ))
        });
        let body = match app.shiju_apps.tab {
            ShijuAppsTab::Store => store::view(
                desktop_i18n::I18n::new(app.desktop_locale),
                &app.shiju_apps.store,
            ),
            ShijuAppsTab::Mine => mine::view(
                desktop_i18n::I18n::new(app.desktop_locale),
                &app.shiju_apps.mine,
            ),
        };

        column![
            text(desktop_i18n::I18n::new(app.desktop_locale).__("识矩应用")).size(24),
            text(
                desktop_i18n::I18n::new(app.desktop_locale)
                    .__("浏览平台应用，或查看当前账户已经安装的应用。")
            )
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

    let base = super::shell::view(tabs, content.map(map));
    api_key_binding::with_overlay(app, base, map)
}
