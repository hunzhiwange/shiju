use vw_api_app_identity::{
    AppId,
    AppInstallationRuntimeStatus,
};
// Application selection stays separate from the selected application configuration.


use super::{catalog::CatalogRow, drag, preferences, presentation::{self, Presence}, styles::{self, Tone}, Action, ViewState};
use desktop_ui_controls::system_settings_common::*;
use desktop_assets::assets::{self, Icon};
use iced::{
    Alignment, Element, Length, Theme,
    widget::{
        button, column, container, mouse_area, row, svg, text, text::Wrapping, text_input, tooltip,
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
    let visible = preferences::visible_rows(state);
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
    let heading = row![
        container(settings_page_heading("第三方智能体", "选择智能体，管理它的配置与运行环境。"))
            .width(Length::Fill),
        action(
            if state.error.is_some() { "重试" } else { "刷新" },
            Icon::ArrowClockwise,
            Action::Refresh,
            !state.loading && state.drag.source.is_none(), message
        ),
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
        icon_btn(Icon::Link, "高级连接", Some(message(Action::ConnectionOpen))),
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
    let mut content = column![
        heading, domains, filters, routing
    ].spacing(20);
    if let Some(error) = state.error {
        content = content.push(settings_error_banner(error.clone()));
    }
    if state.lifecycle.request.is_some() || state.lifecycle.error.is_some() {
        content = content.push(super::lifecycle_view::progress(state.lifecycle, message));
    }
    if state.loading {
        content = content.push(empty(
            "正在读取应用",
            "正在检查安装状态和版本，请稍候。",
            Icon::ArrowClockwise,
        ));
    } else if state.rows.is_empty() {
        let mut panel = column![empty(
            "让常用工具触手可及",
            "本机应用会自动发现，无需填写主机或令牌。",
            Icon::Terminal
        )]
        .spacing(12)
        .align_x(Alignment::Center);
        if local_available {
            panel = panel.push(
                button(text("重新连接本机").size(12))
                    .padding([8, 14])
                    .style(primary_action_btn_style)
                    .on_press(message(Action::LocalConnect)),
            );
        }
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
            let mut cards = column![].spacing(8);
            for app in apps {
                cards = cards.push(card(state, app, message));
            }
            let section = column![section_title, cards].spacing(10);
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
    settings_section_content(content).into()
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
    let content: Element<'static, M> = if let Some(glyph) = glyph {
        if monochrome { icon(glyph, 28.).into() } else { icon_svg(glyph, 28.).into() }
    } else {
        match app {
            AppId::Codex => svg::Svg::new(assets::get_provider_icon("openai"))
                .width(28.)
                .height(28.)
                .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
                .into(),
            AppId::Gemini => {
                static GEMINI: std::sync::LazyLock<svg::Handle> = std::sync::LazyLock::new(|| {
                    svg::Handle::from_memory(
                        include_bytes!("../../../../../assets/icons/app/gemini-brand.svg")
                            .as_slice(),
                    )
                });
                svg::Svg::new(GEMINI.clone()).width(28.).height(28.).into()
            }
            AppId::GrokBuild => svg::Svg::new(assets::get_provider_icon("xai"))
                .width(28.)
                .height(28.)
                .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
                .into(),
            AppId::Mcode => {
                svg::Svg::new(assets::get_provider_icon("minimax")).width(28.).height(28.).into()
            }
            AppId::Dsh => {
                svg::Svg::new(assets::get_provider_icon("deepseek")).width(28.).height(28.).into()
            }
            AppId::Hermes => iced::widget::image(assets::get_image(Icon::AppHermes))
                .width(28.)
                .height(28.)
                .into(),
            _ => icon(Icon::Terminal, 28.).into(),
        }
    };
    container(content)
        .width(44.)
        .height(44.)
        .center_x(44.)
        .center_y(44.)
        .style(settings_icon_tile_style)
        .into()
}

fn card<'a, M: Clone + 'static>(state: &ViewState<'a>, app: &'a CatalogRow, message: impl Fn(Action) -> M + Copy + 'static) -> Element<'a, M> {
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
            Presence::Missing if app.entry.id == AppId::ClaudeDesktop => "桌面应用",
            Presence::Missing => "命令行智能体",
            Presence::Unverified => "暂未确认安装状态",
        };
        meta = meta.push(text(label).size(12).style(settings_muted_text_style));
    }
    let description =
        column![text(&app.entry.name).size(16).wrapping(Wrapping::Glyph), meta.wrap(),]
            .spacing(8)
            .width(Length::Fill);
    let select = button(text(if expanded { "已选择" } else { "选择" }).size(SETTINGS_CONTROL_TEXT_SIZE))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe((!expanded && state.drag.source.is_none())
            .then_some(message(Action::ToggleDetails(app.entry.id))));
    let summary = row![handle, brand(app.entry.id), description, select]
        .spacing(12)
        .align_y(Alignment::Center);
    let body = container(summary).padding([12, 14]);
    let card =
        container(body).width(Length::Fill).style(move |theme| styles::card(theme, selected));
    if installed {
        mouse_area(card)
            .on_enter(message(Action::Drag(drag::DragMessage::Hover(app.entry.id))))
            .on_exit(message(Action::Drag(drag::DragMessage::Leave(app.entry.id))))
            .into()
    } else {
        card.into()
    }
}

/// Returns the selected application within the current visibility and search scope.
pub fn selected_app<'a>(state: &ViewState<'a>) -> Option<&'a CatalogRow> {
    preferences::visible_rows(state).into_iter().find(|app| {
        Some(app.entry.id) == state.expanded && presentation::matches_query(app, state.query)
    })
}
