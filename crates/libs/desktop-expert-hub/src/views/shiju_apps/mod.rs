use desktop_app_launch::{ShijuAppsBrowse, ShijuAppsState, ShijuAppsTab};
use desktop_apps_ui::shiju_apps::ShijuAppsMessage as Event;
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, icon_svg, settings_select,
    settings_text_input_style,
};
use iced::widget::{
    Column, Space, button, column, container, responsive, row, rule, scrollable, text, text_input,
};
use iced::{Alignment, Element, Font, Length};

pub mod api_key_binding;
mod copy;
mod created;
pub mod detail;
pub mod mine;
pub mod store;
pub mod styles;

#[derive(Debug, Clone)]
pub enum Message {
    ShijuApps(Event),
    StartBrowserLogin,
}

impl From<Event> for Message {
    fn from(event: Event) -> Self {
        Self::ShijuApps(event)
    }
}

pub struct ViewInput<'a> {
    pub shiju_apps: &'a ShijuAppsState,
    pub desktop_locale: config_ui::DesktopLocale,
}

pub fn tab_specs() -> &'static [(&'static str, ShijuAppsTab)] {
    &[
        ("应用广场", ShijuAppsTab::Mine),
        ("我收藏的应用", ShijuAppsTab::Favorites),
        ("应用商店", ShijuAppsTab::Store),
        ("我创建的应用", ShijuAppsTab::Created),
    ]
}

fn title(
    label: impl iced::widget::text::IntoFragment<'static>,
    size: u32,
) -> iced::widget::Text<'static> {
    text(label).size(size).font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT })
}

fn muted(
    label: impl iced::widget::text::IntoFragment<'static>,
    size: u32,
) -> iced::widget::Text<'static> {
    text(label).size(size).style(styles::muted_text_style)
}

fn action(label: &'static str, event: Option<Event>) -> iced::widget::Button<'static, Message> {
    button(title(label, 13))
        .padding([6, 13])
        .style(styles::quiet_button_style)
        .on_press_maybe(event.map(Message::from))
}

pub(super) fn open_installed(installation_id: &str) -> Option<Event> {
    gateway_shiju_app::valid_installation_id(installation_id)
        .then(|| Event::OpenInstalled(installation_id.to_owned()))
}

fn pill(label: String, selected: bool, event: Event) -> iced::widget::Button<'static, Message> {
    button(title(label, 14))
        .padding([9, 16])
        .style(move |theme, status| styles::tab_style(theme, status, selected))
        .on_press(event.into())
}

fn heading(title_text: &'static str, description: &'static str) -> Element<'static, Message> {
    column![title(title_text, 24), muted(description, 14)].spacing(5).into()
}

fn search(i18n: I18n, browse: &ShijuAppsBrowse, width: f32) -> Element<'static, Message> {
    text_input(i18n.__("按名称、描述、应用 ID 或开发者搜索"), &browse.query)
        .icon(text_input::Icon {
            font: Font::DEFAULT,
            code_point: '⌕',
            size: Some(iced::Pixels(16.0)),
            spacing: 8.0,
            side: text_input::Side::Left,
        })
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .width(width)
        .style(settings_text_input_style)
        .on_input(|value| Event::Search(value).into())
        .into()
}

fn matches_query(query: &str, values: &[&str]) -> bool {
    query.is_empty() || values.iter().any(|value| value.to_lowercase().contains(query))
}

fn card_grid(cards: Vec<Element<'static, Message>>, columns: usize) -> Column<'static, Message> {
    let mut grid = column![].spacing(16).width(Length::Fill);
    let mut cards = cards.into_iter();
    loop {
        let chunk: Vec<_> = cards.by_ref().take(columns).collect();
        if chunk.is_empty() {
            break;
        }
        let count = chunk.len();
        let mut line = row(chunk).spacing(16);
        for _ in count..columns {
            line = line.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(line);
    }
    grid
}

fn columns(width: f32, max: usize, minimum: f32) -> usize {
    (((width + 16.0) / (minimum + 16.0)) as usize).clamp(1, max)
}

fn page_range(total: usize, browse: &ShijuAppsBrowse) -> std::ops::Range<usize> {
    let page = browse.page.min(total.saturating_sub(1) / browse.page_size);
    let start = page * browse.page_size;
    start..(start + browse.page_size).min(total)
}

fn pagination(i18n: I18n, total: usize, browse: &ShijuAppsBrowse) -> Element<'static, Message> {
    let range = page_range(total, browse);
    let page = range.start / browse.page_size;
    let summary = i18n
        .format(
            "共 %s 条，当前显示 %s–%s",
            &[
                total.to_string().into(),
                (range.start + usize::from(total > 0)).to_string().into(),
                range.end.to_string().into(),
            ],
        )
        .unwrap_or_else(|_| total.to_string());
    let page_label = move |size: usize| {
        i18n.format("%s 条", &[size.to_string().into()]).unwrap_or_else(|_| size.to_string())
    };
    let page_size = settings_select(
        [10, 20, 50, 100].map(page_label),
        Some(page_label(browse.page_size)),
        move |value| {
            [10, 20, 50, 100]
                .into_iter()
                .find(|size| page_label(*size) == value)
                .map(Event::PageSize)
                .unwrap_or(Event::PageSize(20))
                .into()
        },
    )
    .width(90);
    column![
        rule::horizontal(1).style(styles::divider_style),
        container(
            row![
                muted(summary, 13),
                Space::new().width(Length::Fill),
                row![
                    muted(i18n.__("每页"), 13),
                    page_size,
                    action("‹", (page > 0).then(|| Event::Page(page.saturating_sub(1)))),
                    container(title((page + 1).to_string(), 13))
                        .padding([6, 12])
                        .style(styles::pill_track),
                    action("›", (range.end < total).then_some(Event::Page(page + 1))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
        )
        .padding([12, 16])
    ]
    .into()
}

fn feedback(
    i18n: I18n,
    message: impl Into<String>,
    event: Option<(&'static str, Message)>,
) -> Element<'static, Message> {
    let mut content = column![muted(message.into(), 14)].spacing(12).align_x(Alignment::Center);
    if let Some((label, event)) = event {
        content = content.push(
            button(title(i18n.__(label), 13))
                .padding([7, 12])
                .style(styles::quiet_button_style)
                .on_press(event),
        );
    }
    container(content).center_x(Length::Fill).padding(24).style(styles::card_style).into()
}

fn list_feedback<T>(
    i18n: I18n,
    state: &desktop_app_launch::ShijuAppsLoadState<T>,
) -> Element<'static, Message> {
    use desktop_app_launch::ShijuAppsLoadState::*;
    match state {
        Error(error) => feedback(i18n, error.clone(), Some(("重试", Event::Retry.into()))),
        NeedsLogin => feedback(
            i18n,
            i18n.__("登录识矩账户后可浏览应用"),
            Some(("登录", Message::StartBrowserLogin)),
        ),
        Idle | Loading { .. } => feedback(i18n, i18n.__("正在加载应用…"), None),
        Loaded(_) => feedback(i18n, i18n.__("未找到匹配的应用"), None),
    }
}

fn app_icon(size: f32, artwork: Option<iced::widget::image::Handle>) -> Element<'static, Message> {
    if let Some(handle) = artwork {
        return container(iced::widget::image(handle).width(size).height(size).border_radius(9.0))
            .width(size)
            .height(size)
            .into();
    }
    container(icon_svg(Icon::LayoutTextWindow, size * 0.52).style(styles::accent_icon))
        .center(Length::Fixed(size))
        .style(styles::app_icon_style)
        .into()
}

fn date(i18n: I18n, value: Option<&str>, time: bool) -> String {
    let Some(value) = value.filter(|value| !value.is_empty()) else { return "—".into() };
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(value) else { return value.to_owned() };
    let local = parsed.with_timezone(&chrono::Local);
    if time {
        return local.format("%Y-%m-%d %H:%M").to_string();
    }
    let days = (chrono::Local::now().date_naive() - local.date_naive()).num_days();
    match days {
        0 => i18n.__("今天更新").into(),
        1..=7 => i18n.format("%s天前更新", &[days.to_string().into()]).unwrap_or_default(),
        _ => {
            i18n.format("%s 更新", &[local.format("%m-%d").to_string().into()]).unwrap_or_default()
        }
    }
}

fn scroll(body: Element<'static, Message>) -> Element<'static, Message> {
    use iced::widget::scrollable::{Direction, Scrollbar};
    scrollable(container(body).width(Length::Fill).padding([0, styles::SCROLLABLE_SIDE_PADDING]))
        .direction(Direction::Vertical(
            Scrollbar::new()
                .width(styles::SCROLLBAR_WIDTH)
                .scroller_width(styles::SCROLLBAR_WIDTH)
                .spacing(3),
        ))
        .height(Length::Fill)
        .into()
}

pub fn view<'a, M: Clone + 'static>(
    app: &ViewInput<'a>,
    tabs: Element<'a, M>,
    map: impl Fn(Message) -> M + Copy + 'static,
    layout: impl FnOnce(Element<'a, M>) -> Element<'a, M>,
) -> Element<'a, M> {
    let state = app.shiju_apps.clone();
    let i18n = I18n::new(app.desktop_locale);
    let content = responsive(move |size| {
        let width = (size.width - 14.0).max(0.0);
        let body = if let Some(target) = state.detail {
            detail::view(i18n, &state, target, width)
        } else {
            let navigation = tab_specs().iter().fold(row![].spacing(4), |tabs, (label, tab)| {
                tabs.push(pill(i18n.__(label).into(), state.tab == *tab, Event::SelectTab(*tab)))
            });
            let body = match state.tab {
                ShijuAppsTab::Mine | ShijuAppsTab::Favorites => mine::view(i18n, &state, width),
                ShijuAppsTab::Store => store::view(i18n, &state, width),
                ShijuAppsTab::Created => created::view(i18n, &state, width),
            };
            column![container(navigation.wrap()).padding(4).style(styles::pill_track), body]
                .spacing(24)
                .width(Length::Fill)
                .into()
        };
        scroll(body)
    });
    let base = container(super::shell::view(tabs, Element::from(content).map(map)))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(styles::page_style)
        .into();
    let base = copy::with_overlay(app, layout(base), map);
    api_key_binding::with_overlay(app, base, map)
}
