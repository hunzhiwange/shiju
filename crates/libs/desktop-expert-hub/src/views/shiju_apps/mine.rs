//! 广场与收藏共用可运行应用卡片。
use desktop_app_launch::{
    ShijuAppDetailTarget, ShijuAppsLoadState, ShijuAppsState, ShijuAppsTab, ShijuMarketplaceAction,
};
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use gateway_shiju_app::ShijuInstalledAppDto;
use iced::widget::{Space, button, column, container, row, rule, text, tooltip};
use iced::{Alignment, Element, Length};

use super::{
    Event, Message, action, app_icon, card_grid, columns, date, heading, icon_svg, list_feedback,
    matches_query, muted, page_range, pagination, pill, search, settings_select, styles, title,
};

pub fn runtime_readiness(app: &ShijuInstalledAppDto) -> Result<&'static str, String> {
    if app.runtime_ready() {
        Ok("运行就绪")
    } else {
        Err(format!("协调状态：{}；运行状态：{}", app.reconcile_status, app.runtime_status))
    }
}

pub fn open_action_message(app: &ShijuInstalledAppDto) -> Option<Message> {
    app.runtime_ready().then(|| Event::OpenInstalled(app.installation_id.clone()).into())
}

fn installed_card(
    i18n: I18n,
    index: usize,
    app: &ShijuInstalledAppDto,
    state: &ShijuAppsState,
) -> Element<'static, Message> {
    let metadata = &app.presentation;
    let developer = metadata
        .created_by_user_name
        .as_deref()
        .filter(|name| !name.is_empty())
        .or(metadata.created_by_user_id.as_deref());
    let byline = developer.map(|name| format!("@{name}")).unwrap_or_else(|| app.app_id.clone());
    let name = button(title(app.name.clone(), 17).wrapping(text::Wrapping::None))
        .on_press(Event::OpenDetail(ShijuAppDetailTarget::Mine(index)).into())
        .padding(0)
        .width(Length::Shrink)
        .clip(true)
        .style(styles::text_button_style);
    let star_icon: Element<'static, Message> = if metadata.is_favorite {
        text("★").size(16).style(styles::favorite_text_style).into()
    } else {
        icon_svg(Icon::Star, 14.0).style(styles::muted_icon).into()
    };
    let star = button(container(star_icon).center(18))
        .padding(3)
        .style(button::text)
        .on_press_maybe((state.action.is_none()).then(|| {
            Event::MarketplaceAction(ShijuMarketplaceAction::Favorite(
                app.installation_id.clone(),
                !metadata.is_favorite,
            ))
            .into()
        }));
    let star = tooltip(
        star,
        text(i18n.__(if metadata.is_favorite { "取消收藏" } else { "收藏应用" })).size(12),
        tooltip::Position::Top,
    )
    .style(styles::panel_style);
    let source = if market_source(&app.source) {
        format!("{}  |  ", i18n.__("市场应用"))
    } else {
        String::new()
    };
    container(
        column![
            row![
                app_icon(44.0, state.artwork(app.icon_url.as_deref())),
                column![row![name, star].align_y(Alignment::Center).spacing(4), muted(byline, 12)]
                    .spacing(3)
                    .width(Length::Fill),
                action(
                    i18n.__("打开"),
                    app.runtime_ready().then(|| Event::OpenInstalled(app.installation_id.clone()))
                ),
            ]
            .align_y(Alignment::Start)
            .spacing(12),
            container(muted(
                app.description
                    .clone()
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| i18n.__("暂无应用说明").into()),
                14
            ))
            .height(44)
            .clip(true),
            rule::horizontal(1).style(styles::divider_style),
            row![
                muted(format!("{source}{}", date(i18n, app.updated_at.as_deref(), false)), 12),
                Space::new().width(Length::Fill),
                muted(app.version.clone(), 12),
            ],
        ]
        .spacing(10),
    )
    .padding([16, 20])
    .width(Length::FillPortion(1))
    .style(styles::card_style)
    .into()
}

pub(super) fn market_source(source: &str) -> bool {
    matches!(source, "store" | "catalog")
}

#[derive(Clone, PartialEq)]
struct Source {
    value: &'static str,
    label: &'static str,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label)
    }
}

fn filters(i18n: I18n, state: &ShijuAppsState, width: f32) -> Element<'static, Message> {
    let browse = &state.browse;
    let tabs = row![
        pill(
            i18n.__("全部").into(),
            !browse.recent && browse.category.is_empty(),
            Event::Category(String::new())
        ),
        pill(i18n.__("最近使用").into(), browse.recent, Event::Recent),
    ]
    .spacing(3);
    let english = i18n.__("全部") == "All";
    let tabs = state.categories.iter().fold(tabs, |tabs, category| {
        let name = if english && !category.name_en.is_empty() {
            &category.name_en
        } else {
            &category.name
        };
        tabs.push(pill(
            name.clone(),
            browse.category == category.code,
            Event::Category(category.code.clone()),
        ))
    });
    let tabs = container(tabs.wrap()).padding(4).style(styles::pill_track);
    let sources = [
        Source { value: "", label: i18n.__("全部来源") },
        Source { value: "catalog", label: i18n.__("市场应用") },
        Source { value: "native", label: i18n.__("企业应用") },
    ];
    let selected = sources.iter().find(|source| source.value == browse.source).cloned();
    let source_width = if english { 192.0 } else { 144.0 };
    let filter_gap = 12.0;
    let source = settings_select(sources.to_vec(), selected, |source| {
        Event::Source(source.value.into()).into()
    })
    .width(source_width);
    let search_width = (width - source_width - filter_gap).clamp(120.0, 360.0);
    let search = row![search(i18n, browse, search_width), source]
        .spacing(filter_gap)
        .align_y(Alignment::Center);
    if width > 1260.0 {
        row![tabs, Space::new().width(Length::Fill), search]
            .spacing(12)
            .align_y(Alignment::Center)
            .into()
    } else {
        column![tabs, search].spacing(12).into()
    }
}

pub fn view(i18n: I18n, state: &ShijuAppsState, width: f32) -> Element<'static, Message> {
    let favorites = state.tab == ShijuAppsTab::Favorites;
    let browse = &state.browse;
    let mut page = if favorites {
        column![heading(
            i18n.__("我收藏的应用"),
            i18n.__("集中查看已收藏的应用，方便快速打开使用。")
        )]
    } else {
        column![
            row![
                heading(
                    i18n.__("应用广场"),
                    i18n.__("发现并使用适合业务场景的应用，提升企业效能。")
                ),
                Space::new().width(Length::Fill),
                action(i18n.__("应用管理"), Some(Event::SelectTab(ShijuAppsTab::Created))),
            ]
            .spacing(12),
            super::store::featured(i18n, state, width),
            column![title(i18n.__("全部应用"), 18), filters(i18n, state, width)].spacing(16),
        ]
    }
    .spacing(24)
    .width(Length::Fill);
    let ShijuAppsLoadState::Loaded(mine) = &state.mine else {
        return page.push(list_feedback(i18n, &state.mine)).into();
    };
    let query = browse.query.trim().to_lowercase();
    let mut apps: Vec<_> = mine
        .apps
        .iter()
        .enumerate()
        .filter(|(_, app)| {
            if app.installation_id.is_empty() {
                return false;
            }
            let info = &app.presentation;
            if favorites {
                return info.is_favorite;
            }
            (browse.source.is_empty() || (browse.source == "catalog") == market_source(&app.source))
                && (browse.category.is_empty()
                    || info.category_code.as_deref() == Some(browse.category.as_str()))
                && (!browse.recent
                    || info.last_used_at.as_ref().is_some_and(|value| !value.is_empty()))
                && matches_query(
                    &query,
                    &[
                        &app.app_id,
                        &app.name,
                        app.description.as_deref().unwrap_or_default(),
                        info.created_by_user_name.as_deref().unwrap_or_default(),
                        info.created_by_user_id.as_deref().unwrap_or_default(),
                    ],
                )
        })
        .collect();
    if browse.recent {
        apps.sort_by_cached_key(|(_, app)| {
            std::cmp::Reverse(
                app.presentation
                    .last_used_at
                    .as_deref()
                    .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok()),
            )
        });
    }
    let total = apps.len();
    if favorites {
        page = page.push(muted(
            i18n.format("共 %s 个", &[total.to_string().into()]).unwrap_or_default(),
            14,
        ));
    }
    let cards = apps[page_range(total, browse)]
        .iter()
        .map(|&(index, app)| installed_card(i18n, index, app, state))
        .collect();
    if total == 0 {
        page = page.push(list_feedback(i18n, &state.mine));
    } else {
        page = page.push(card_grid(cards, columns(width, 3, 260.0)));
    }
    page.push(pagination(i18n, total, browse)).into()
}
