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
