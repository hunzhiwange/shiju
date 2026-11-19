use vw_api_app_identity::{
    AppId,
    AppInstallationRuntimeStatus,
};
// Application selection stays separate from the selected application configuration.


use super::{catalog::CatalogRow, drag, presentation::{self, Presence}, styles::{self, Tone}, Action, ViewState};
use desktop_ui_controls::system_settings_common::*;
use desktop_assets::assets::{self, Icon};
use iced::{
    Alignment, Element, Length, Theme,
    widget::{
        button, column, container, mouse_area, row, svg, text, text::Wrapping, text_input, tooltip, scrollable, rule,
    },
};

fn pill<'a, M: 'a>(label: impl Into<String>, tone: Tone) -> Element<'a, M> {
    container(text(label.into()).size(12).wrapping(Wrapping::Glyph))
        .padding([3, 7])
        .style(move |theme| styles::badge(theme, tone))
        .into()
}
fn icon(icon: Icon, size: f32) -> svg::Svg<'static> {
    icon_svg(icon, size).style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
}
pub(super) fn action<'a, M: Clone + 'static>(
    label: &'a str,
    glyph: Icon,
    event: Action,
    enabled: bool,
    message: impl Fn(Action) -> M + Copy + 'static,
) -> Element<'a, M> {
    button(
        row![icon(glyph, 14.), text(label).size(SETTINGS_CONTROL_TEXT_SIZE)]
            .spacing(6)
            .align_y(Alignment::Center),
    )
    .padding([8, 11])
    .style(rounded_action_btn_style)
    .on_press_maybe(enabled.then_some(message(event)))
    .into()
}

pub fn view<'a, M: Clone + 'static>(state: &ViewState<'a>, help_heading: impl Fn(&'static str, &'static str) -> Element<'a, M>, local_available: bool, domains: Element<'a, M>, routing: Element<'a, M>, details: Option<Element<'a, M>>, message: impl Fn(Action) -> M + Copy + 'static) -> Element<'a, M> {
    view_with_updates(state, None, help_heading, local_available, domains, routing, details, message)
}

pub fn view_with_updates<'a, M: Clone + 'static>(state: &ViewState<'a>, updates: Option<&'a desktop_apps_ui::catalog_state::UpdatesState>, help_heading: impl Fn(&'static str, &'static str) -> Element<'a, M>, local_available: bool, domains: Element<'a, M>, routing: Element<'a, M>, details: Option<Element<'a, M>>, message: impl Fn(Action) -> M + Copy + 'static) -> Element<'a, M> {
    let mut visible: Vec<_> = state.rows.iter().collect();
    if let Some(saved) = &state.preferences.saved {
        visible.sort_by_key(|app| saved.user.app_order.iter()
            .position(|id| *id == presentation::key(app.entry.id)).unwrap_or(usize::MAX));
    }
    let installed =
        visible.iter().filter(|a| presentation::presence(a) == Presence::Installed).count();
    let missing = visible.iter().filter(|a| presentation::presence(a) == Presence::Missing).count();
    let location = if state.host == local_app_auth::HOST_ID {
        "本机"
    } else if state.host.is_empty() {
        "正在连接"
    } else {
        state.host
    };
    let upgrade_count = updates.map_or(0, |u| u.results.iter().filter(|candidate|
        candidate.supported && state.rows.iter().any(|row|
            presentation::preferred(row) == Some(&candidate.installation))).count());
    let heading = row![
        container(settings_page_heading("应用", "管理执行主机上的第三方应用、配置与运行环境。"))
            .width(Length::Fill),
        button(text(if updates.is_some_and(|u| u.checking) { "检查中…" } else { "检查更新" }).size(SETTINGS_CONTROL_TEXT_SIZE))
            .width(108.).padding([8, 11]).style(rounded_action_btn_style)
            .on_press_maybe((!state.loading && !updates.is_some_and(|u| u.checking) && state.drag.source.is_none() && !state.rows.is_empty()).then_some(message(Action::CheckUpdates))),
        button(text(format!("全部升级（{upgrade_count}）")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 11]).style(primary_action_btn_style)
            .on_press_maybe((upgrade_count > 0 && !state.loading && !updates.is_some_and(|u| u.checking)
                && !state.lifecycle.active() && state.lifecycle.preview.is_none()
                && state.lifecycle.batch.as_ref().is_none_or(|b| b.stopped)
                && state.drag.source.is_none()).then_some(message(Action::BatchUpgrade))),
        action(
            if state.error.is_some() { "重试" } else { "刷新" },
            Icon::ArrowClockwise,
            Action::Refresh,
            !state.loading && state.drag.source.is_none(), message
        ),
        settings_select(
            ["诊断安装冲突", "手动安装命令", "手动卸载命令", "应用设置"],
            None,
            move |choice| message(match choice {
                "诊断安装冲突" => Action::CommandsOpen(3),
                "手动安装命令" => Action::CommandsOpen(1),
                "手动卸载命令" => Action::CommandsOpen(2),
                _ => Action::PreferencesOpen,
            }),
        ).placeholder("⋯").width(56.)
            .disabled(state.loading || state.drag.source.is_some()),
        action(
            "应用设置",
            Icon::Sliders,
            Action::PreferencesOpen,
            !state.loading && !state.rows.is_empty() && state.drag.source.is_none(), message
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let search = text_input("搜索应用名称或命令…", state.query)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .on_input(move |q| message(Action::QueryInput(q)))
        .width(Length::Fill);
    let search_row = row![
        icon(Icon::Search, 16.),
        search,
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let status = row![
        pill(format!("执行主机 · {location}"), Tone::Neutral),
        pill(format!("已安装 {installed}"), Tone::Success),
        pill(format!("待安装 {missing}"), Tone::Neutral),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();
    let filters =
        settings_group_card(container(column![search_row, status].spacing(12)).padding([14, 0]));
    let check_status = updates.map(|updates| {
        if let Some(error) = &updates.error { return error.clone(); }
        match updates.checked_at.and_then(|at| at.elapsed().ok()) {
            Some(elapsed) => format!("上次成功检查：{} 分钟前", elapsed.as_secs() / 60),
            None => "尚未检查更新".into(),
        }
    }).unwrap_or_default();
    let toolbar = column![heading, text(check_status).size(12).style(settings_muted_text_style), domains, filters].spacing(16);
    let mut content = column![routing].spacing(20);
    if let Some(error) = state.error {
        content = content.push(settings_error_banner(error.clone()));
    }
    if state.lifecycle.batch.is_some() || state.lifecycle.request.is_some() || state.lifecycle.error.is_some() {
        content = content.push(super::lifecycle_view::progress(state.lifecycle, message));
    }
    if state.loading && state.rows.is_empty() {
        content = content.push(empty(
            "正在读取应用",
            "正在检查安装状态和版本，请稍候。",
            Icon::ArrowClockwise,
        ));
    } else if state.rows.is_empty() {
        let panel = column![empty(
            "让常用工具触手可及",
            "本机应用会自动发现，无需填写主机或令牌。",
            Icon::Terminal
        )]
        .spacing(12)
        .align_x(Alignment::Center);
        content = content.push(panel);
    } else {
        let mut found = 0;
        for presence in [Presence::Installed, Presence::Missing, Presence::Unverified] {
            let apps: Vec<_> = visible
                .iter()
                .copied()
                .filter(|a| {
                    presentation::presence(a) == presence
                        && presentation::matches_query(a, &state.query)
                })
                .collect();
            if apps.is_empty() {
                continue;
            }
            found += apps.len();
            let heading = match presence {
                Presence::Installed => {
                    let title = help_heading("已安装", "拖动左侧手柄可调整顺序");
                    let notice = if state.drag.source.is_some() {
                        Some("松开鼠标完成排序 · Esc 取消")
                    } else if state.preferences.busy {
                        Some("正在同步设置…")
                    } else {
                        None
                    };
                    if let Some(notice) = notice {
                        column![title, text(notice).size(13).style(settings_muted_text_style)]
                            .spacing(6)
                            .into()
                    } else {
                        title
                    }
                }
                Presence::Missing => help_heading("待安装", "发现更多适合你的工具"),
                Presence::Unverified => settings_group_heading("待确认", "打开详情查看检测结果"),
            };
            let section_title =
                row![container(heading).width(Length::Fill), settings_value_badge(apps.len()),]
                    .spacing(8)
                    .align_y(Alignment::Center);
            let mut cards = column![
                container(row![
                    container(text("应用 / 安装信息").size(12).style(settings_muted_text_style))
                        .width(Length::Fill),
                    text("在智能侧栏显示").size(12).style(settings_muted_text_style),
                ].spacing(12).align_y(Alignment::Center)).padding([8, 14]),
                rule::horizontal(1),
            ];
            for (index, app) in apps.into_iter().enumerate() {
                if index > 0 { cards = cards.push(rule::horizontal(1)); }
                cards = cards.push(card(state, updates, app, local_available, message));
            }
            let section = column![section_title, settings_group_card(cards)].spacing(10);
            let section: Element<'_, M> =
                if presence == Presence::Installed && state.drag.source.is_some() {
                    mouse_area(section).on_exit(message(Action::Drag(drag::DragMessage::Cancel))).into()
                } else {
                    section.into()
                };
            content = content.push(section);
        }
        if found == 0 {
            content = content.push(empty(
                "没有找到匹配的应用",
                "试试其他名称，或清空搜索内容。",
                Icon::Search,
            ));
        }
    }
    if !state.loading {
        if let Some(app) = selected_app(state) {
            let mut panel = column![
                container(super::details::actions(state, app, local_available, message)).padding([16, 18]),
            ].spacing(10);
            if let Some(details) = details {
                panel = panel.push(details);
            }
            content = content.push(settings_group_card(panel));
        } else if !state.rows.is_empty() {
            content = content.push(empty(
                "选择一个智能体",
                "从上方列表选择，查看此智能体的配置与操作。",
                Icon::Sliders,
            ));
        }
    }
    settings_section_content(column![
        toolbar,
        scrollable(container(content).padding([0, 7]))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill),
    ].spacing(20).height(Length::Fill)).height(Length::Fill).into()
}

fn empty<'a, M: 'static>(title: &'a str, description: &'a str, glyph: Icon) -> Element<'a, M> {
    container(
        column![
            container(icon(glyph, 26.)).padding(16).style(settings_icon_tile_style),
            text(title).size(16),
            text(description).size(13).center().style(settings_muted_text_style)
        ]
        .spacing(12)
        .align_x(Alignment::Center),
    )
    .padding([36, 16])
    .width(Length::Fill)
    .center_x(Length::Fill)
    .style(|theme| styles::card(theme, false))
    .into()
}

pub fn brand<M: 'static>(app: AppId) -> Element<'static, M> {
    container(brand_icon(app, 28.0))
        .width(44.)
        .height(44.)
        .center_x(44.)
        .center_y(44.)
        .style(settings_icon_tile_style)
        .into()
}

pub fn brand_icon<M: 'static>(app: AppId, size: f32) -> Element<'static, M> {
    let (glyph, monochrome) = match app {
        AppId::Claude | AppId::ClaudeDesktop => (Some(Icon::AppClaudeCode), false),
        AppId::Codex | AppId::Gemini => (None, false),
        AppId::OpenCode => (Some(Icon::AppOpenCode), true),
        AppId::OpenClaw => (Some(Icon::AppOpenClaw), false),
        AppId::Pi | AppId::Omp => (Some(Icon::AppPi), false),
        AppId::Kimi => (Some(Icon::AppKimiCode), false),
        AppId::Agy => (Some(Icon::AppAntigravity), false),
        _ => (None, false),
    };
    if let Some(glyph) = glyph {
        if monochrome { icon(glyph, size).into() } else { icon_svg(glyph, size).into() }
    } else {
        match app {
            AppId::Codex => svg::Svg::new(assets::get_provider_icon("openai"))
                .width(size)
                .height(size)
                .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
                .into(),
            AppId::Gemini => {
                static GEMINI: std::sync::LazyLock<svg::Handle> = std::sync::LazyLock::new(|| {
                    svg::Handle::from_memory(
                        include_bytes!("../../../../../assets/icons/app/gemini-brand.svg")
                            .as_slice(),
                    )
                });
                svg::Svg::new(GEMINI.clone()).width(size).height(size).into()
            }
            AppId::GrokBuild => svg::Svg::new(assets::get_provider_icon("xai"))
                .width(size)
                .height(size)
                .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
                .into(),
            AppId::Mcode => {
                svg::Svg::new(assets::get_provider_icon("minimax")).width(size).height(size).into()
            }
            AppId::Dsh => {
                svg::Svg::new(assets::get_provider_icon("deepseek")).width(size).height(size).into()
            }
            AppId::Hermes => iced::widget::image(assets::get_image(Icon::AppHermes))
                .width(size)
                .height(size)
                .into(),
            _ => icon(Icon::Terminal, size).into(),
        }
    }
}

fn card<'a, M: Clone + 'static>(state: &ViewState<'a>, updates: Option<&'a desktop_apps_ui::catalog_state::UpdatesState>, app: &'a CatalogRow, local_available: bool, message: impl Fn(Action) -> M + Copy + 'static) -> Element<'a, M> {
    let installed = presentation::presence(app) == Presence::Installed;
    let expanded = state.expanded == Some(app.entry.id);
    let selected =
        expanded || state.drag.source == Some(app.entry.id) || state.drag.target == Some(app.entry.id);
    let handle: Element<'_, M> = if installed {
        static GRIP: std::sync::LazyLock<svg::Handle> = std::sync::LazyLock::new(|| {
            svg::Handle::from_memory(br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 18" fill="currentColor"><circle cx="3" cy="3" r="1.5"/><circle cx="9" cy="3" r="1.5"/><circle cx="3" cy="9" r="1.5"/><circle cx="9" cy="9" r="1.5"/><circle cx="3" cy="15" r="1.5"/><circle cx="9" cy="15" r="1.5"/></svg>"#.as_slice())
        });
        let handle = container(
            svg::Svg::new(GRIP.clone())
                .width(10.)
                .height(16.)
                .opacity(if state.hovered == Some(app.entry.id) || state.drag.source.is_some() {
                    1.0_f32
                } else {
                    0.0_f32
                })
                .style(|theme: &Theme, _| svg::Style {
                    color: Some(theme.palette().text.scale_alpha(0.38)),
                }),
        )
        .width(14.)
        .height(44.)
        .center_y(44.)
        .center_x(14.);
        let enabled = drag::can_start(state, app.entry.id);
        let area = mouse_area(handle).interaction(if !enabled {
            iced::mouse::Interaction::None
        } else if state.drag.source.is_some() {
            iced::mouse::Interaction::Grabbing
        } else {
            iced::mouse::Interaction::Grab
        });
        let area: Element<'_, M> = if enabled {
            area.on_press(message(Action::Drag(drag::DragMessage::Start(app.entry.id)))).into()
        } else {
            area.into()
        };
        tooltip(
            area,
            container(
                text(if !state.query.trim().is_empty() {
                    "清空搜索后可拖动排序"
                } else {
                    "拖动排序"
                })
                .size(11),
            )
            .padding([5, 8])
            .style(settings_panel_style),
            tooltip::Position::Top,
        )
        .into()
    } else {
        container(text("")).width(14.).into()
    };
    let mut meta = row![].spacing(6).align_y(Alignment::Center);
    if let Some(installation) = presentation::preferred(app) {
        let ready = matches!(installation.runtime_status, AppInstallationRuntimeStatus::Runnable);
        meta = meta
            .push(pill(presentation::source_label(installation.source), Tone::Neutral))
            .push(
                text(installation.version.as_deref().unwrap_or("版本待确认"))
                    .size(12)
                    .wrapping(Wrapping::Glyph)
                    .style(settings_muted_text_style),
            )
            .push(pill(
                if ready { "可运行" } else { "需要检查" },
                if ready { Tone::Success } else { Tone::Warning },
            ));
    } else {
        let label = match presentation::presence(app) {
            Presence::Installed => "桌面应用 · 已发现",
            Presence::Missing => "未安装",
            Presence::Unverified => "暂未确认安装状态",
        };
        meta = meta.push(text(label).size(12).style(settings_muted_text_style));
    }
    let candidate = updates.and_then(|u| u.results.iter().find(|candidate|
        presentation::preferred(app) == Some(&candidate.installation)));
    if let Some(candidate) = candidate {
        if let Some(latest) = &candidate.latest { meta = meta.push(pill(format!("最新 {latest}"), Tone::Neutral)); }
        meta = meta.push(text(candidate.status.clone()).size(12).style(settings_muted_text_style));
    }
    let mut description = column![text(&app.entry.name).size(16).wrapping(Wrapping::Glyph), meta.wrap()]
        .spacing(8).width(Length::Fill);
    if let Some(installation) = presentation::preferred(app) {
        description = description.push(text(&installation.path).size(12)
            .wrapping(Wrapping::Glyph).style(settings_muted_text_style));
    } else if let Ok(observation) = &app.observation {
        if let Some(paths) = observation.desktop.as_ref().and_then(|desktop| desktop.app_bundles.as_ref()) {
            for path in paths { description = description.push(text(path).size(12)
                .wrapping(Wrapping::Glyph).style(settings_muted_text_style)); }
        }
    }
    let select = button(text(if expanded { "收起详情" } else { "详情" }).size(SETTINGS_CONTROL_TEXT_SIZE))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe((state.drag.source.is_none())
            .then_some(message(Action::ToggleDetails(app.entry.id))));
    let saved = state.preferences.saved.as_ref();
    let visible = saved.is_some_and(|saved| saved.user.visible_apps.contains(&app.entry.id));
    let enabled = saved.is_some()
        && !state.preferences.busy
        && !state.preferences.open
        && state.drag.source.is_none()
        && (!visible || saved.is_some_and(|saved| saved.user.visible_apps.len() > 1));
    let id = app.entry.id;
    let toggle = container(settings_toggle(visible).on_toggle_maybe(
        enabled.then_some(move |visible| message(Action::SetVisible(id, visible))),
    )).width(96.).center_x(96.);
    let mut actions = column![super::details::actions(state, app, local_available, message)].spacing(8);
    if let Some(candidate) = candidate.filter(|candidate| candidate.supported) {
        let request = vw_api_app_operations::AppActionPreviewRequest {
            app_id: app.entry.id,
            action: vw_api_app_operations::AppLifecycleAction::Update,
            target: vw_api_app_operations::AppActionTarget::Installation { installation_id: candidate.installation.id.clone() },
        };
        actions = actions.push(button(text("升级").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12]).style(primary_action_btn_style)
            .on_press_maybe((!state.loading && !updates.is_some_and(|u| u.checking) && !state.lifecycle.active() && state.lifecycle.preview.is_none() && state.drag.source.is_none()).then_some(message(Action::Preview(request)))));
    }
    let summary = row![handle, brand(app.entry.id), description,
        actions.push(select)
            .spacing(8).width(Length::FillPortion(1)), toggle]
        .spacing(12)
        .align_y(Alignment::Center)
        .wrap();
    let body = container(summary).padding([12, 14]);
    let card =
        container(body).width(Length::Fill).style(move |theme| styles::list_row(theme, selected));
    if installed {
        mouse_area(card)
            .on_enter(message(Action::Drag(drag::DragMessage::Hover(app.entry.id))))
            .on_exit(message(Action::Drag(drag::DragMessage::Leave(app.entry.id))))
            .into()
    } else {
        card.into()
    }
}

/// Returns the selected application within the catalog search scope.
pub fn selected_app<'a>(state: &ViewState<'a>) -> Option<&'a CatalogRow> {
    state.rows.iter().find(|app| {
        Some(app.entry.id) == state.expanded && presentation::matches_query(app, state.query)
    })
}
