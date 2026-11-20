//! 应用商店和广场精选共用封面卡片。
use desktop_app_launch::{
    ShijuAppDetailTarget, ShijuAppsLoadState, ShijuAppsState, ShijuMarketplaceAction,
};
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

use super::{
    Event, Message, action, card_grid, columns, heading, icon_svg, list_feedback, matches_query,
    muted, page_range, pagination, search, styles, title,
};

pub fn view(i18n: I18n, state: &ShijuAppsState, width: f32) -> Element<'static, Message> {
    let mut body = column![
        heading(i18n.__("应用商店"), i18n.__("浏览平台已上架、可安装的应用。")),
        search(i18n, &state.browse, width.min(288.0)),
    ]
    .spacing(24);
    let ShijuAppsLoadState::Loaded(catalog) = &state.store else {
        return body.push(list_feedback(i18n, &state.store)).into();
    };
    let query = state.browse.query.trim().to_lowercase();
    let entries: Vec<_> = catalog
        .apps
        .iter()
        .enumerate()
        .filter(|(_, app)| {
            matches_query(
                &query,
                &[
                    &app.name,
                    &app.app_id,
                    app.description.as_deref().unwrap_or_default(),
                    app.presentation.created_by_user_name.as_deref().unwrap_or_default(),
                    app.presentation.created_by_user_id.as_deref().unwrap_or_default(),
                ],
            )
        })
        .collect();
    let total = entries.len();
    let cards = entries[page_range(total, &state.browse)]
        .iter()
        .map(|&(index, app)| {
            let install = (!app.installed && state.action.is_none()).then(|| {
                Event::MarketplaceAction(ShijuMarketplaceAction::Install(app.app_id.clone()))
            });
            cover_card(
                i18n,
                &app.name,
                app.description.as_deref(),
                state
                    .artwork(app.presentation.cover_url.as_deref())
                    .or_else(|| state.artwork(app.presentation.background_url.as_deref())),
                Event::OpenDetail(ShijuAppDetailTarget::Store(index)),
                action(i18n.__(if app.installed { "已安装" } else { "安装" }), install).into(),
            )
        })
        .collect();
    body = if total == 0 {
        body.push(list_feedback(i18n, &state.store))
    } else {
        body.push(card_grid(cards, columns(width, 4, 228.0)))
    };
    body.push(pagination(i18n, total, &state.browse)).into()
}

pub(super) fn featured(
    i18n: I18n,
    state: &ShijuAppsState,
    width: f32,
) -> Element<'static, Message> {
    let mut body = column![title(i18n.__("精选应用"), 18)].spacing(16);
    match &state.featured {
        ShijuAppsLoadState::Loaded(apps) if !apps.is_empty() => {
            let cards = apps
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    let event = Event::OpenDetail(ShijuAppDetailTarget::Featured(index));
                    cover_card(
                        i18n,
                        &entry.app.name,
                        entry.app.description.as_deref(),
                        state
                            .artwork(Some(&entry.cover_url))
                            .or_else(|| state.artwork(entry.background_url.as_deref())),
                        event.clone(),
                        action(i18n.__("详情"), Some(event)).into(),
                    )
                })
                .collect();
            body = body.push(card_grid(cards, columns(width, 4, 228.0)));
        }
        ShijuAppsLoadState::Error(_) => {
            body = body.push(super::feedback(
                i18n,
                i18n.__("精选应用加载失败"),
                Some(("重试", Event::Retry.into())),
            ))
        }
        ShijuAppsLoadState::Loading { .. } => body = body.push(muted(i18n.__("正在加载应用…"), 14)),
        _ => return column![].into(),
    }
    body.into()
}

pub(super) fn cover_card(
    i18n: I18n,
    name: &str,
    description: Option<&str>,
    artwork: Option<iced::widget::image::Handle>,
    detail: Event,
    cta: Element<'static, Message>,
) -> Element<'static, Message> {
    let cover: Element<'static, Message> = match artwork {
        Some(image) => iced::widget::image(image)
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(iced::ContentFit::Cover)
            .into(),
        None => container(icon_svg(Icon::Star, 34.0).style(styles::accent_icon))
            .center(Length::Fill)
            .style(styles::cover_placeholder)
            .into(),
    };
    let cover = button(container(cover).width(Length::Fill).height(Length::Fill).clip(true))
        .on_press(detail.clone().into())
        .padding(iced::Padding { top: 22.0, right: 30.0, bottom: 0.0, left: 30.0 })
        .width(Length::Fill)
        .height(148)
        .style(|theme, status| {
            let ground = styles::cover_ground(theme);
            button::Style { background: ground.background, ..button::text(theme, status) }
        });
    let heading = button(title(name.to_owned(), 17).wrapping(text::Wrapping::None))
        .width(Length::Fill)
        .clip(true)
        .padding(0)
        .style(styles::text_button_style)
        .on_press(detail.into());
    let details = column![
        row![heading, cta].spacing(8).align_y(Alignment::Center),
        container(muted(
            description
                .filter(|value| !value.is_empty())
                .unwrap_or(i18n.__("暂无应用说明"))
                .to_owned(),
            14
        ))
        .height(44)
        .clip(true),
    ]
    .spacing(10);
    container(column![
        cover,
        container(details).padding(iced::Padding {
            top: 12.0,
            right: 16.0,
            bottom: 12.0,
            left: 16.0
        }),
    ])
    .width(Length::FillPortion(1))
    .clip(true)
    .style(styles::card_style)
    .into()
}
