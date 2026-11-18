use super::{event::Event, overview, scan, state::DetailTab};
use desktop_assets::assets::{Icon, get_icon};
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_HEIGHT, rounded_action_btn_style, settings_modal_overlay_with_backdrop_dismiss,
    settings_right_drawer_overlay,
};
use iced::{
    Element, Length, Theme,
    widget::{
        Space, button, column, container, responsive, row, rule, scrollable, stack, svg, text, tooltip,
    },
};

pub fn view<'a>(
    state: &'a super::state::State,
    known_cost: Option<String>,
    i18n: desktop_i18n::I18n,
) -> Element<'a, Event> {
    let action = |label: &'static str, event| {
        button(text(label).size(13))
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press(event)
    };
    if !state.open {
        return action("第三方智能体用量统计 ›", Event::Load(super::range_picker::Preset::Month))
            .into();
    }
    let sync_active = state.scan.job.as_ref().is_some_and(|job| !scan::terminal(job));
    let last_sync = state.scan.last_sync_time();
    let sync_label = if sync_active {
        "同步中…".into()
    } else if let Some(time) = &last_sync {
        let format = if time.date_naive() == chrono::Local::now().date_naive() {
            "%H:%M:%S"
        } else {
            "%Y-%m-%d %H:%M:%S"
        };
        format!("同步于 {}", time.format(format))
    } else {
        "同步".into()
    };
    let sync_help = last_sync.map_or_else(
        || "立即同步".into(),
        |time| format!("立即同步 · 上次同步 {}", time.format("%Y-%m-%d %H:%M:%S")),
    );
    let header_icon = |icon| {
        svg(get_icon(icon))
            .width(16)
            .height(16)
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
    };
    let sync = tooltip(
        button(container(
            row![header_icon(Icon::ArrowRepeat), text(sync_label).size(13)]
                .spacing(6)
                .align_y(iced::Alignment::Center),
        ).center_y(Length::Fill))
            .height(SETTINGS_CONTROL_HEIGHT)
            .padding([0, 10])
            .style(|theme, status| header_segment_style(theme, status, true))
            .on_press_maybe((!state.scan.pending && !sync_active).then_some(Event::Sync)),
        text(sync_help).size(12),
        tooltip::Position::Bottom,
    )
    .snap_within_viewport(true);
    let settings = tooltip(
        button(container(header_icon(Icon::Sliders)).center_y(Length::Fill))
            .height(SETTINGS_CONTROL_HEIGHT)
            .padding([0, 10])
            .style(|theme, status| header_segment_style(theme, status, false))
            .on_press(Event::Scan(scan::Event::Open)),
        text("用量设置").size(12),
        tooltip::Position::Bottom,
    )
    .snap_within_viewport(true);
    let sync_controls = container(
        row![sync, rule::vertical(1), settings]
            .height(SETTINGS_CONTROL_HEIGHT)
            .align_y(iced::Alignment::Center),
    )
    .style(|theme| {
        let style = rounded_action_btn_style(theme, button::Status::Active);
        container::Style { background: style.background, border: style.border, ..Default::default() }
    });
    let heading = row![
        svg(get_icon(Icon::BarChartFill))
            .width(22)
            .height(22)
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) }),
        text("用量统计").size(24),
    ]
    .spacing(9)
    .align_y(iced::Alignment::Center);
    let header = row![
        heading,
        Space::new().width(Length::Fill),
        sync_controls,
    ]
    .spacing(7)
    .align_y(iced::Alignment::Center);
    let mut options =
        vec![overview::ScopeOption {
            label: "供应商 / 模型".into(), provider: None, model: None
        }];
    if let Some(Ok((providers, models))) = &state.groups.result {
        for provider in providers {
            let label = match &provider.provider {
                vw_api_usage::UsageProvider::Unknown => "未知供应商".into(),
                vw_api_usage::UsageProvider::Known { provider_id } => provider_id.clone(),
            };
            options.push(overview::ScopeOption {
                label,
                provider: Some(provider.provider.clone()),
                model: None,
            });
        }
        for model in models {
            if let Some(name) = &model.model {
                options.push(overview::ScopeOption {
                    label: name.clone(),
                    provider: state.query.filter.provider.clone(),
                    model: Some(name.clone()),
                });
            }
        }
    }
    // Own the option vector in the picker; do not return a borrow of local data.
    let filters = overview::primary_filters(&state.query, &options).map(Event::Filter);
    let summary = state.result.as_ref().and_then(|r| r.as_ref().ok());
    let hero =
        super::hero::view_with_cost(&state.hero, summary, state.loading, known_cost.as_deref())
            .map(Event::Hero);
    let mut chart_section = column![].width(Length::Fill).spacing(8);
    match &state.result {
        Some(Err(error)) => {
            chart_section = chart_section.push(text(format!("用量加载失败：{error}")).size(12));
        }
        Some(Ok(summary)) if summary.counts.unknown_cost_requests > 0 => {
            chart_section = chart_section.push(
                text(format!("{} 个请求的费用未知", summary.counts.unknown_cost_requests))
                    .size(12)
                    .style(text::secondary),
            );
        }
        _ => {}
    }
    let chart = super::charts::view_range(
        &state.charts,
        state.hero.metric,
        Some(matches!(&state.query.range, vw_api_usage::UsageRange::All)),
    )
    .map(Event::Charts);
    let overview_card = container(column![hero, chart_section.push(chart).padding([14, 16])])
        .width(Length::Fill)
        .clip(true)
        .style(panel_style);
    let counts = state.groups.result.as_ref().and_then(|result| result.as_ref().ok());
    let counts = [
        super::logs::request_count(&state.logs),
        counts.map(|(providers, _)| providers.len() as u64),
        counts.map(|(_, models)| models.len() as u64),
    ];
    let tabs: Element<'_, Event> = if state.tab == DetailTab::Logs {
        responsive(move |size| {
            let tabs = detail_tabs(state.tab, counts);
            let filters = container(super::logs::toolbar(&state.logs).map(Event::Logs))
                .align_right(Length::Fill)
                .padding([0, 8]);
            if size.width >= 500.0 {
                row![tabs, filters].spacing(8).align_y(iced::Alignment::Center).into()
            } else {
                column![tabs, filters].spacing(4).into()
            }
        })
        .height(Length::Shrink)
        .into()
    } else {
        detail_tabs(state.tab, counts)
    };
    let detail: Element<'_, Event> = match state.tab {
        DetailTab::Logs => super::logs::table(&state.logs, i18n).map(Event::Logs),
        DetailTab::Providers => container(super::groups::provider_view(&state.groups).map(Event::Groups))
            .padding(14).into(),
        DetailTab::Models => container(super::groups::model_view(&state.groups).map(Event::Groups))
            .padding(14).into(),
    };
    let details = container(column![tabs, rule::horizontal(1), detail])
        .width(Length::Fill)
        .style(panel_style);
    let filters = column![
        row![state.range_picker.view().map(Event::Range), filters,]
            .spacing(12)
            .align_y(iced::Alignment::Center),
        overview::source_filters(&state.query).map(Event::Filter),
    ]
    .spacing(6)
    .width(Length::Fill);
    let body = column![
        column![header, rule::horizontal(1), filters].spacing(11),
        overview_card,
        details,
    ]
    .spacing(16);
    let page = scrollable(container(body).padding([7, 7]).max_width(1280).center_x(Length::Fill))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Fill);
    let drawer = if state.tab == DetailTab::Logs {
        super::logs::detail_drawer(&state.logs)
            .map(|(close, drawer)| (Event::Logs(close), drawer.map(Event::Logs)))
    } else {
        None
    };
    let page = settings_right_drawer_overlay(page.into(), drawer);
    // A top-level modal suppresses the underlying chart and filter overlays.
    match state.range_picker.dialog() {
        Some(dialog) => settings_modal_overlay_with_backdrop_dismiss(
            page,
            Event::Range(super::range_picker::Event::Cancel),
            dialog.map(Event::Range),
        ),
        None => stack![page].into(),
    }
}

fn header_segment_style(theme: &Theme, status: button::Status, first: bool) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    style.border.width = 0.0;
    style.border.radius = if first { iced::border::left(10) } else { iced::border::right(10) };
    if status == button::Status::Active {
        style.background = None;
    }
    style
}

fn detail_tabs(active: DetailTab, counts: [Option<u64>; 3]) -> Element<'static, Event> {
    let mut tabs = row![].spacing(4).padding(iced::padding::left(12));
    for ((tab, label), count) in [
        (DetailTab::Logs, "请求日志"),
        (DetailTab::Providers, "供应商"),
        (DetailTab::Models, "模型"),
    ].into_iter().zip(counts) {
        let selected = active == tab;
        let mut content = row![text(label).size(13)]
            .spacing(6)
            .align_y(iced::Alignment::Center);
        if let Some(count) = count {
            content = content.push(
                container(text(super::metrics::format_count(count)).size(12).style(text::secondary))
                    .padding([2, 6])
                    .style(|theme: &Theme| {
                        container::Style::default()
                            .background(theme.extended_palette().background.weak.color)
                            .border(iced::border::rounded(5))
                    }),
            );
        }
        let label = container(content).padding([0, 8]).center_y(SETTINGS_CONTROL_HEIGHT);
        let indicator = container(Space::new().height(2)).width(Length::Fill).style(move |theme: &Theme| {
            container::Style::default().background(if selected {
                theme.palette().text
            } else {
                iced::Color::TRANSPARENT
            }).border(iced::border::rounded(1))
        });
        tabs = tabs.push(
            button(stack![label, container(indicator).align_bottom(Length::Fill)])
                .padding(0)
                .style(move |theme, status| {
                    let mut style = rounded_action_btn_style(theme, status);
                    style.border.width = 0.0;
                    style.border.radius = iced::border::top(7);
                    style.background = None;
                    let alpha = match status {
                        button::Status::Hovered | button::Status::Pressed => 1.0,
                        _ if selected => 0.95,
                        _ => 0.73,
                    };
                    style.text_color = theme.palette().text.scale_alpha(alpha);
                    style
                })
                .on_press(Event::Tab(tab)),
        );
    }
    tabs.into()
}

pub(super) fn panel_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style::default().background(palette.background).border(iced::Border {
        color: palette.text.scale_alpha(0.14),
        width: 1.0,
        radius: 14.0.into(),
    })
}
