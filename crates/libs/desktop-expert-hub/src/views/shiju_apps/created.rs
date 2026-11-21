//! 我创建的应用表格，对应平台可管理应用列表。
use desktop_app_launch::{ShijuAppDetailTarget, ShijuAppsLoadState, ShijuAppsState};
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use iced::widget::{button, column, container, row, rule, text, tooltip};
use iced::{Alignment, Element, Font, Length};

use super::{
    Event, Message, app_icon, date, heading, icon_svg, list_feedback, matches_query, muted,
    page_range, pagination, pill, search, styles, title,
};

pub(super) fn view(i18n: I18n, state: &ShijuAppsState, width: f32) -> Element<'static, Message> {
    let browse = &state.browse;
    let tabs = ["", "草稿", "审核中", "已发布"].into_iter().fold(row![].spacing(4), |tabs, key| {
        tabs.push(pill(
            i18n.__(if key.is_empty() { "全部" } else { key }).into(),
            browse.lifecycle == key,
            Event::Lifecycle(key.into()),
        ))
    });
    let body = column![
        heading(
            i18n.__("我创建的应用"),
            i18n.__("管理你创建或拥有管理权限的应用，包括开发、发布、权限与版本。")
        ),
        column![
            container(tabs.wrap()).padding(4).style(styles::pill_track),
            search(i18n, browse, width.min(360.0)),
        ]
        .spacing(12),
    ]
    .spacing(24)
    .width(Length::Fill);
    let ShijuAppsLoadState::Loaded(created) = &state.created else {
        return body.push(list_feedback(i18n, &state.created)).into();
    };
    let query = browse.query.trim().to_lowercase();
    let entries: Vec<_> = created
        .iter()
        .enumerate()
        .filter(|(_, app)| {
            (browse.lifecycle.is_empty() || app.lifecycle() == browse.lifecycle)
                && matches_query(
                    &query,
                    &[
                        &app.title,
                        &app.app_id,
                        app.description.as_deref().unwrap_or_default(),
                        app.created_by_user_name.as_deref().unwrap_or_default(),
                        app.created_by_user_id.as_deref().unwrap_or_default(),
                    ],
                )
        })
        .collect();
    let compact = width < 680.0;
    let mut header = row![
        muted(i18n.__("应用"), 12).width(Length::FillPortion(27)),
        muted(i18n.__("状态"), 12).width(Length::FillPortion(10)),
        muted(i18n.__("版本"), 12).width(Length::FillPortion(10)),
    ]
    .spacing(12);
    if !compact {
        header = header
            .push(muted(i18n.__("访问权限"), 12).width(Length::FillPortion(13)))
            .push(muted(i18n.__("更新时间"), 12).width(Length::FillPortion(27)));
    }
    header = header.push(container(muted(i18n.__("操作"), 12)).align_right(72));
    let mut table = column![
        container(header).width(Length::Fill).padding([13, 16]).style(styles::table_header)
    ];
    let total = entries.len();
    if total == 0 {
        table = table.push(list_feedback(i18n, &state.created));
    }
    for &(index, app) in &entries[page_range(total, browse)] {
        let id = muted(app.app_id.clone(), 12);
        let id = if app.app_id.is_ascii() { id.font(Font::MONOSPACE) } else { id };
        let status = app.lifecycle();
        let version = if app.active_version.is_empty() {
            app.published_version.as_deref().unwrap_or_default()
        } else {
            &app.active_version
        };
        let version = if version.is_empty() {
            "—".into()
        } else {
            format!("v{}", version.trim_start_matches('v'))
        };
        let mut line = row![
            row![
                app_icon(40.0, state.artwork(app.icon_url.as_deref())),
                column![title(app.title.clone(), 14), id].spacing(4),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .width(Length::FillPortion(27))
            .clip(true),
            container(
                container(title(i18n.__(status), 12))
                    .padding([4, 8])
                    .style(move |theme| styles::lifecycle_style(theme, status))
            )
            .width(Length::FillPortion(10)),
            text(version).size(13).width(Length::FillPortion(10)),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        if !compact {
            let scope = match app.access_scope.as_deref() {
                Some("company") => i18n.__("全员"),
                Some("restricted") => i18n.__("指定成员"),
                _ => "—",
            };
            line = line.push(text(scope).size(13).width(Length::FillPortion(13))).push(
                text(date(i18n, app.updated_at.as_deref(), true))
                    .size(13)
                    .width(Length::FillPortion(27)),
            );
        }
        let launch = match &state.mine {
            ShijuAppsLoadState::Loaded(mine) => mine
                .apps
                .iter()
                .find(|item| Some(item.installation_id.as_str()) == app.installation_id.as_deref())
                .and_then(|item| super::open_installed(&item.installation_id)),
            _ => None,
        };
        let tools = row![
            table_action(
                Icon::Eye,
                i18n.__("详情"),
                Some(Event::OpenDetail(ShijuAppDetailTarget::Created(index)))
            ),
            table_action(Icon::ArrowRight, i18n.__("打开"), launch),
        ]
        .spacing(5);
        line = line.push(container(tools).align_right(72));
        table = table
            .push(rule::horizontal(1).style(styles::divider_style))
            .push(container(line).width(Length::Fill).padding([13, 16]));
    }
    body.push(
        container(table.push(pagination(i18n, total, browse)))
            .width(Length::Fill)
            .clip(true)
            .style(styles::card_style),
    )
    .into()
}

fn table_action(
    icon: Icon,
    label: &'static str,
    event: Option<Event>,
) -> Element<'static, Message> {
    let control = button(icon_svg(icon, 15.0).style(styles::muted_icon))
        .padding(8)
        .style(styles::quiet_button_style)
        .on_press_maybe(event.map(Message::from));
    tooltip(control, text(label).size(12), tooltip::Position::Top).style(styles::panel_style).into()
}
