use vw_api_app_identity::{AppId, AppInstallationRuntimeStatus};
// Application selection stays separate from the selected application configuration.

use super::{
    Action, ViewState,
    catalog::CatalogRow,
    drag,
    presentation::{self, Presence},
    styles::{self, Tone},
};
use desktop_assets::assets::{self, Icon};
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Alignment, Element, Length, Theme,
    widget::{
        Space, button, column, container, mouse_area, row, scrollable, svg, text, text::Wrapping,
        text_input, tooltip,
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

pub fn view<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
    state: &ViewState<'a>,
    help_heading: impl Fn(&'static str, &'static str) -> Element<'a, M>,
    local_available: bool,
    domains: Element<'a, M>,
    routing: Element<'a, M>,
    details: Option<Element<'a, M>>,
    message: impl Fn(Action) -> M + Copy + 'static,
) -> Element<'a, M> {
    view_with_updates(i18n,
        state,
        None,
        false,
        help_heading,
        local_available,
        Some(domains),
        Some(routing),
        details,
        message,
    )
}

pub fn view_with_updates<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
    state: &ViewState<'a>,
    updates: Option<&'a desktop_apps_ui::catalog_state::UpdatesState>,
    compact: bool,
    help_heading: impl Fn(&'static str, &'static str) -> Element<'a, M>,
    local_available: bool,
    domains: Option<Element<'a, M>>,
    routing: Option<Element<'a, M>>,
    mut details: Option<Element<'a, M>>,
    message: impl Fn(Action) -> M + Copy + 'static,
) -> Element<'a, M> {
    let mut visible: Vec<_> = state.rows.iter().collect();
    if let Some(saved) = &state.preferences.saved {
        visible.sort_by_key(|app| {
            saved
                .user
                .app_order
                .iter()
                .position(|id| *id == presentation::key(app.entry.id))
                .unwrap_or(usize::MAX)
        });
    }
    let checking = updates.is_some_and(|updates| updates.checking);
    let ready = !state.loading && state.drag.source.is_none();
    let upgrade_count = updates.map_or(0, |updates| {
        updates
            .results
            .iter()
            .filter(|candidate| {
                candidate.supported
                    && state
                        .rows
                        .iter()
                        .any(|app| presentation::preferred(app) == Some(&candidate.installation))
            })
            .count()
    });
    let more = settings_select(
        ["刷新目录", "诊断安装冲突", "手动安装命令", "手动卸载命令", "应用设置"],
        None,
        move |choice| {
            message(match choice {
                "刷新目录" => Action::Refresh,
                "诊断安装冲突" => Action::CommandsOpen(3),
                "手动安装命令" => Action::CommandsOpen(1),
                "手动卸载命令" => Action::CommandsOpen(2),
                _ => Action::PreferencesOpen,
            })
        },
    )
    .format_labels(|source| i18n.__(source).to_owned())
    .menu_icon(Icon::DotsThreeVertical, 168.0)
    .width(34)
    .height(34.0)
    .disabled(!ready);
    let more = tooltip(
        more,
        container(text(i18n.__("更多应用操作")).size(12)).padding([6, 8]).style(settings_panel_style),
        tooltip::Position::Bottom,
    );
    let update_buttons = row![
        button(
            container(text(if checking { i18n.__("检查中…") } else { i18n.__("检查更新") }).size(13))
                .center_x(Length::Fill),
        )
        .width(92)
        .padding([7, 10])
        .style(rounded_action_btn_style)
        .on_press_maybe(
            (ready && !checking && !state.rows.is_empty()).then_some(message(Action::CheckUpdates))
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let update_buttons = if upgrade_count > 0 {
        update_buttons.push(
            button(text(i18n.sprintf("全部升级（%s）", &[format!("{}", upgrade_count).into()])).size(13))
                .padding([7, 10])
                .style(primary_action_btn_style)
                .on_press_maybe(
                    (ready
                        && !checking
                        && !state.lifecycle.active()
                        && state.lifecycle.preview.is_none()
                        && state.lifecycle.batch.as_ref().is_none_or(|batch| batch.stopped))
                    .then_some(message(Action::BatchUpgrade)),
                ),
        )
    } else {
        update_buttons
    };
    let title = row![
        icon(Icon::Grid1x2, 20.0),
        help_heading(i18n.__("应用"), i18n.__("管理第三方应用。显示开关控制应用是否出现在智能侧栏。")),
    ]
    .spacing(9)
    .align_y(Alignment::Center);
    let heading = if compact {
        column![
            row![title, Space::new().width(Length::Fill), more].align_y(Alignment::Center),
            update_buttons.wrap(),
        ]
        .spacing(8)
    } else {
        column![
            row![title, Space::new().width(Length::Fill), update_buttons, more,]
                .spacing(8)
                .align_y(Alignment::Center)
        ]
    };
    let search = text_input(i18n.__("搜索应用名称或命令"), state.query)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .on_input(move |query| message(Action::QueryInput(query)))
        .width(Length::Fill);
    let search = container(search).max_width(300);
    let mut context = column![].spacing(3);
    if let Some(elapsed) =
        updates.and_then(|updates| updates.checked_at).and_then(|at| at.elapsed().ok())
    {
        let minutes = elapsed.as_secs() / 60;
        let status = if minutes == 0 {
            i18n.__("刚刚检查过更新").to_owned()
        } else {
            i18n.sprintf("上次检查 · %s 分钟前", &[format!("{}", minutes).into()])
        };
        context = context.push(text(status).size(12).style(settings_muted_text_style));
    }
    let filters = if compact {
        column![search, context].spacing(6)
    } else {
        column![
            row![search, Space::new().width(Length::Fill), context]
                .spacing(12)
                .align_y(Alignment::Center)
        ]
    };
    let mut toolbar = column![heading, filters].spacing(12);
    if state.drag.source.is_some() {
        toolbar = toolbar
            .push(text(i18n.__("松开鼠标完成排序 · Esc 取消")).size(12).style(settings_muted_text_style));
    }
    let mut content = column![].spacing(16).width(Length::Fill);
    if let Some(error) = updates.and_then(|updates| updates.error.as_ref()) {
        content = content.push(settings_error_banner(error.clone()));
    }
    if let Some(domains) = domains {
        content = content.push(domains);
    }
    if let Some(error) = state.error {
        content = content.push(settings_error_banner(error.clone()));
    }
    if state.lifecycle.batch.is_some()
        || state.lifecycle.request.is_some()
        || state.lifecycle.error.is_some()
    {
        content = content.push(super::lifecycle_view::progress(i18n, state.lifecycle, message));
    }
    if state.loading && state.rows.is_empty() {
        content = content.push(empty(
            i18n.__("正在读取应用"),
            i18n.__("正在检查安装状态和版本，请稍候。"),
            Icon::ArrowClockwise,
        ));
    } else if state.rows.is_empty() {
        content =
            content.push(empty(i18n.__("暂无应用"), i18n.__("请刷新应用目录。"), Icon::Terminal));
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
            let label = match presence {
                Presence::Installed => i18n.__("已安装"),
                Presence::Missing => i18n.__("未安装"),
                Presence::Unverified => i18n.__("待确认"),
            };
            let section_title = row![
                text(label).size(14),
                text(format!("· {}", apps.len())).size(13).style(settings_muted_text_style),
            ]
            .spacing(6)
            .align_y(Alignment::Center);
            let header: Element<'_, M> = if compact {
                section_title.into()
            } else {
                row![
                    container(section_title).width(Length::Fill),
                    row_columns(
                        text(i18n.__("版本 / 状态")).size(12).style(settings_muted_text_style).into(),
                        Space::new().into(),
                        text(i18n.__("侧栏显示")).size(12).style(settings_muted_text_style).into(),
                        Space::new().into(),
                    )
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            };
            let mut cards = column![].width(Length::Fill);
            for (index, app) in apps.into_iter().enumerate() {
                if index > 0 {
                    cards = cards.push(settings_divider());
                }
                cards = cards.push(card(i18n, state, updates, app, compact, local_available, message));
                if !state.loading && state.expanded == Some(app.entry.id) {
                    let mut panel =
                        column![super::details::actions(i18n, state, app, local_available, message),]
                            .spacing(10);
                    if let Some(details) = details.take() {
                        panel = panel.push(details);
                    }
                    cards = cards.push(settings_divider()).push(
                        container(panel)
                            .padding([12, 16])
                            .width(Length::Fill)
                            .style(styles::detail_surface),
                    );
                }
            }
            let section = column![
                container(header).padding([0, 12]).width(Length::Fill),
                container(cards).width(Length::Fill).style(settings_content_panel_style),
            ]
            .spacing(7);
            let section: Element<'_, M> = if presence == Presence::Installed
                && state.drag.source.is_some()
            {
                mouse_area(section).on_exit(message(Action::Drag(drag::DragMessage::Cancel))).into()
            } else {
                section.into()
            };
            content = content.push(section);
        }
        if found == 0 {
            content = content.push(empty(
                i18n.__("没有找到匹配的应用"),
                i18n.__("试试其他名称，或清空搜索内容。"),
                Icon::Search,
            ));
        }
    }
    if let Some(routing) = routing {
        content = content.push(routing);
    }
    let scroller = scrollable(container(content).width(Length::Fill).padding([4, 7]))
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ));
    settings_section_content(
        column![container(toolbar).padding([0, 7]).width(Length::Fill), scroller]
            .spacing(12)
            .height(Length::Fill),
    )
    .max_width(1080)
    .height(Length::Fill)
    .into()
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
    .padding([28, 16])
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

fn card<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
    state: &ViewState<'a>,
    updates: Option<&'a desktop_apps_ui::catalog_state::UpdatesState>,
    app: &'a CatalogRow,
    compact: bool,
    local_available: bool,
    message: impl Fn(Action) -> M + Copy + 'static,
) -> Element<'a, M> {
    let installed = presentation::presence(app) == Presence::Installed;
    let expanded = state.expanded == Some(app.entry.id);
    let selected = expanded
        || state.drag.source == Some(app.entry.id)
        || state.drag.target == Some(app.entry.id);
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
        .height(42.)
        .center_y(42.)
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
                    i18n.__("清空搜索后可拖动排序")
                } else {
                    i18n.__("拖动排序")
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
    let presence = presentation::presence(app);
    let preferred = presentation::preferred(app);
    let candidate = updates.and_then(|updates| {
        updates.results.iter().find(|candidate| preferred == Some(&candidate.installation))
    });
    let version_label = match (preferred, presence) {
        (Some(installation), _) => installation.version.as_deref().unwrap_or(i18n.__("版本待确认")),
        (None, Presence::Installed) => i18n.__("已发现桌面应用"),
        (None, Presence::Missing) => i18n.__("未安装"),
        (None, Presence::Unverified) => i18n.__("安装状态待确认"),
    };
    let mut version = column![
        text(version_label).size(12).wrapping(Wrapping::Glyph).style(settings_muted_text_style),
    ]
    .spacing(3);
    if let Some(latest) = candidate
        .filter(|candidate| candidate.supported)
        .and_then(|candidate| candidate.latest.as_deref())
    {
        version =
            version.push(text(i18n.sprintf("可升级至 %s", &[format!("{}", latest).into()])).size(12).wrapping(Wrapping::Glyph));
    } else if preferred.is_some_and(|installation| {
        !matches!(installation.runtime_status, AppInstallationRuntimeStatus::Runnable)
    }) {
        version = version.push(pill(i18n.__("需要检查"), Tone::Warning));
    }
    let toggle_details =
        (state.drag.source.is_none()).then_some(message(Action::ToggleDetails(app.entry.id)));
    let select = icon_btn(
        if expanded { Icon::ChevronUp } else { Icon::ChevronDown },
        if expanded { i18n.__("收起应用详情") } else { i18n.__("展开应用详情") },
        toggle_details.clone(),
    );
    let mut title = row![text(&app.entry.name).size(15).wrapping(Wrapping::Glyph)]
        .spacing(7)
        .align_y(Alignment::Center);
    if let Some(installation) = preferred {
        title = title.push(pill(presentation::source_label(i18n, installation.source), Tone::Neutral));
    }
    let mut description = column![title.wrap()].spacing(4).width(Length::Fill);
    let path = preferred.map(|installation| installation.path.as_str()).or_else(|| {
        app.observation
            .as_ref()
            .ok()?
            .desktop
            .as_ref()?
            .app_bundles
            .as_ref()?
            .first()
            .map(String::as_str)
    });
    if let Some(path) = path {
        description = description
            .push(text(path).size(12).wrapping(Wrapping::Glyph).style(settings_muted_text_style));
    }
    if let Some(count) = app
        .observation
        .as_ref()
        .ok()
        .map(|observation| observation.installations.len().saturating_sub(1))
        .filter(|count| *count > 0)
    {
        description = description.push(
            button(text(i18n.sprintf("另有 %s 处安装", &[format!("{}", count).into()])).size(12))
                .padding([3, 7])
                .style(rounded_action_btn_style)
                .on_press_maybe(toggle_details),
        );
    }
    let saved = state.preferences.saved.as_ref();
    let visible = saved.is_some_and(|saved| saved.user.visible_apps.contains(&app.entry.id));
    let enabled = saved.is_some()
        && !state.preferences.busy
        && !state.preferences.open
        && state.drag.source.is_none()
        && (!visible || saved.is_some_and(|saved| saved.user.visible_apps.len() > 1));
    let id = app.entry.id;
    let toggle = settings_toggle(visible).on_toggle_maybe(
        enabled.then_some(move |visible| message(Action::SetVisible(id, visible))),
    );
    let toggle = if compact { toggle.label(i18n.__("侧栏显示")).text_size(12) } else { toggle };
    let toggle = tooltip(
        toggle,
        container(text(i18n.__("在智能侧栏显示")).size(12)).padding([6, 8]).style(settings_panel_style),
        tooltip::Position::Top,
    );
    let primary: Element<'_, M> =
        if let Some(candidate) = candidate.filter(|candidate| candidate.supported) {
            let request = vw_api_app_operations::AppActionPreviewRequest {
                app_id: app.entry.id,
                action: vw_api_app_operations::AppLifecycleAction::Update,
                target: vw_api_app_operations::AppActionTarget::Installation {
                    installation_id: candidate.installation.id.clone(),
                },
            };
            button(text(i18n.__("升级")).size(13))
                .padding([7, 10])
                .style(primary_action_btn_style)
                .on_press_maybe(
                    (!state.loading
                        && !updates.is_some_and(|updates| updates.checking)
                        && !state.lifecycle.active()
                        && state.lifecycle.preview.is_none()
                        && state.drag.source.is_none())
                    .then_some(message(Action::Preview(request))),
                )
                .into()
        } else {
            super::details::install_action(i18n, state, app, local_available, message)
                .unwrap_or_else(|| Space::new().into())
        };
    let identity = row![
        handle,
        container(brand_icon(app.entry.id, 22.0)).center_x(26).center_y(42),
        description,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill);
    let summary: Element<'_, M> = if compact {
        column![
            row![identity, select].spacing(8).align_y(Alignment::Center),
            row![version, primary, toggle].spacing(10).align_y(Alignment::Center).wrap(),
        ]
        .spacing(6)
        .into()
    } else {
        row![identity, row_columns(version.into(), primary, toggle.into(), select)]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
    };
    let body = container(summary).padding([9, 12]).width(Length::Fill).height(Length::Shrink);
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

fn row_columns<'a, M: 'a>(
    version: Element<'a, M>,
    action: Element<'a, M>,
    visibility: Element<'a, M>,
    details: Element<'a, M>,
) -> iced::widget::Row<'a, M> {
    row![
        container(version).width(112),
        container(action).center_x(88),
        container(visibility).center_x(68),
        container(details).center_x(34),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
}

/// Returns the selected application within the catalog search scope.
pub fn selected_app<'a>(state: &ViewState<'a>) -> Option<&'a CatalogRow> {
    state.rows.iter().find(|app| {
        Some(app.entry.id) == state.expanded && presentation::matches_query(app, state.query)
    })
}
