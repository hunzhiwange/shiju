//! Catalog presentation only; host discovery and pagination stay in the SDK.
use super::{Event, State, message, delete};
use desktop_ui_controls::system_settings_common::*;
use super::Event as Message;

use chrono::{Local, TimeZone};
use desktop_assets::assets::Icon;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use iced::{Element, Length, widget::{button, column, container, row, scrollable, text, text_input, checkbox, hover}};
use iced::widget::scrollable::{Direction, Scrollbar};
use std::collections::BTreeMap;
use vw_api_external_session::{SessionMeta, SessionProvider};

pub(super) fn view(i18n: desktop_i18n::I18n, state: &State, loopback: bool) -> Element<'_, Message> {
    if !state.open {
        return button(text(i18n.__("浏览外部会话")))
            .padding([8, 12]).style(rounded_action_btn_style)
            .on_press(message(Event::Open)).into();
    }
    let title = row![
        icon_btn(Icon::ArrowLeft, i18n.__("返回应用"), Some(Event::Close)),
        icon_svg(Icon::Clock, 19.0),
        text(i18n.__("会话")).size(21).font(iced::Font { weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT }),
        icon_btn(Icon::QuestionCircle, i18n.__("会话记录在哪里"), Some(Event::Path(super::paths::Event::Sources(true)))),
        iced::widget::space::horizontal(),
        action_menu(i18n, vec![Action::Refresh, Action::Select, Action::Terminal, Action::Sources],
            i18n.__("会话菜单"), |action| match action {
                Action::Refresh => Event::Refresh,
                Action::Select => Event::Delete(delete::Event::Enter),
                Action::Terminal => Event::Resume(super::resume::Event::Preferences),
                _ => Event::Path(super::paths::Event::Sources(true)),
            }),
    ].spacing(9).align_y(iced::Alignment::Center);
    let search = row![
        text_input(i18n.__("搜索标题、摘要、目录、来源路径或会话 ID"), &state.search_draft)
            .on_input(Event::SearchDraft).on_submit(Event::Search)
            .font(iced::Font::DEFAULT).style(settings_text_input_style).padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE).width(Length::Fill),
        icon_btn(Icon::Search, i18n.__("搜索会话"), Some(Event::Search)),
    ].spacing(3).align_y(iced::Alignment::Center);
    let mut search = search;
    if !state.search_draft.is_empty() || !state.search.is_empty() {
        search = search.push(icon_btn(Icon::X, i18n.__("清空搜索"), Some(Event::ClearSearch)));
    }
    let controls = row![
        applications(i18n, state),
        container(search).width(Length::Fill),
        container(row![
            segment(i18n.__("按时间"), state.by_time, Event::GroupByTime(true)),
            segment(i18n.__("按项目"), !state.by_time, Event::GroupByTime(false)),
        ].spacing(2)).padding(3).style(subtle_surface),
    ].spacing(9).align_y(iced::Alignment::Center);
    let mut header = column![title, settings_divider(), controls].spacing(10);
    if state.busy { header = header.push(text(i18n.__("正在读取会话…")).style(settings_muted_text_style)); }
    if let Some(error) = &state.error {
        header = header.push(settings_error_banner(error.as_str()));
    }
    let available = !state.busy && !state.deletion.busy();
    let mut footer = column![].spacing(7);
    if state.deletion.selecting {
        header = header.push(row![
            text(i18n.sprintf("%s 个匹配会话", &[format!("{}", state.page.as_ref().map_or(0, |page| page.total)).into()])).size(12)
                .style(settings_muted_text_style),
            button(text(i18n.__("全选匹配结果")).size(12)).padding([4, 7])
                .style(|theme, status| flat_button(theme, status, false))
                .on_press_maybe(available.then_some(Event::Delete(delete::Event::SelectAll))),
            button(text(i18n.__("清空选择")).size(12)).padding([4, 7])
                .style(|theme, status| flat_button(theme, status, false))
                .on_press_maybe(available.then_some(Event::Delete(delete::Event::Clear))),
        ].spacing(5).align_y(iced::Alignment::Center));
        let selected = state.deletion.selected.len();
        footer = footer.push(settings_divider()).push(row![
            text(i18n.sprintf("已选 %s 个会话", &[format!("{}", selected).into()])).size(13),
            iced::widget::space::horizontal(),
            button(text(i18n.__("取消")).size(13)).padding([6, 11]).style(rounded_action_btn_style)
                .on_press_maybe(available.then_some(Event::Delete(delete::Event::Exit))),
            button(text(i18n.__("删除所选")).size(13)).padding([6, 11]).style(danger_action_btn_style)
                .on_press_maybe((available && selected > 0).then_some(Event::Delete(delete::Event::Ask(None)))),
        ].spacing(8).align_y(iced::Alignment::Center));
    }
    if state.deletion.busy() {
        header = header.push(text(i18n.__("正在处理所选会话，请稍候…")).style(settings_muted_text_style));
    }
    let mut content = column![].padding([4, 7]);
    for (identity, reason) in &state.deletion.failures {
        content = content.push(settings_error_banner(format!("{:?} · {} · {}：{}",
            identity.provider, identity.instance_id, identity.session_id.0, reason)));
    }
    if let Some(page) = &state.page {

        if page.sessions.is_empty() { content = content.push(text(if state.search.is_empty() && state.provider.is_none() { i18n.__("当前项目没有外部会话") } else { i18n.__("没有匹配的会话") })); }
        for (dir, label, sessions) in groups(state) {
            let count = if !state.by_time && state.search.is_empty() && !page.groups.is_empty() {
                Some(page.groups.iter().filter(|group| group.project_dir == dir
                    && state.provider.is_none_or(|provider| provider == group.provider))
                    .map(|group| group.count).sum::<usize>())
            } else { None };
            let collapsed = !state.by_time && state.collapsed.contains(&dir);

            let recent = sessions.first().map(|session| relative_time(i18n, session)).unwrap_or_default();
            let display_label = if state.by_time || dir.is_none() { i18n.__(&label) } else { &label };
            let project_name = display_label.rsplit(['/', '\\']).find(|part| !part.is_empty()).unwrap_or(display_label);
            let total = count.unwrap_or(sessions.len());
            let metadata = i18n.sprintf("%s%s 个会话 · %s", &[format!("{}", total).into(), format!("{}", if page.truncated { "+" } else { "" }).into(), format!("{}", recent).into()]);
            let mut heading = row![].spacing(9).align_y(iced::Alignment::Center);
            if !state.by_time {
                heading = heading.push(icon_svg(if collapsed { Icon::ChevronRight } else { Icon::ChevronDown }, 13.0));
            }
            heading = heading.push(text(project_name.to_owned()).size(14)
                .font(iced::Font { weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT }));
            if !state.by_time && dir.is_some() {
                heading = heading.push(container(SingleLineLabel::new(label.clone(), 12.0)).style(|theme|
                    container::Style { text_color: settings_muted_text_style(theme).color, ..Default::default() })
                    .width(Length::Fill));
            } else {
                heading = heading.push(iced::widget::space::horizontal());
            }
            heading = heading.push(text(metadata).size(12).style(settings_muted_text_style));
            let heading: Element<'_, Event> = if state.by_time {
                container(heading).padding([8, 10]).style(subtle_surface).into()
            } else {
                button(heading).width(Length::Fill).padding([8, 10])
                    .style(|theme, status| flat_button(theme, status, true))
                    .on_press(Event::ToggleProject(dir)).into()
            };
            let mut group = column![heading, settings_divider()];
            if collapsed { content = content.push(group); continue; }
            for session in sessions {
                let mut body = column![row![
                    container(SingleLineLabel::new(session.title.as_deref()
                        .unwrap_or(&session.identity.session_id.0).to_owned(), 14.0)).width(Length::Fill),
                    text(relative_time(i18n, session)).size(12).style(settings_muted_text_style),
                ].spacing(8)].spacing(4);
                let archived = session.source_path.as_deref().is_some_and(|path|
                    path.replace('\\', "/").contains("/archived_sessions/"));
                let mut summary = row![].spacing(8).align_y(iced::Alignment::Center);
                if archived { summary = summary.push(text(i18n.__("已归档")).size(11).style(settings_muted_text_style)); }
                if let Some(value) = session.summary.as_deref().filter(|value| !value.trim().is_empty()) {
                    summary = summary.push(container(SingleLineLabel::new(preview(value, 220), 12.0)).style(|theme|
                        container::Style { text_color: settings_muted_text_style(theme).color, ..Default::default() }));
                }
                if archived || session.summary.as_deref().is_some_and(|value| !value.trim().is_empty()) {
                    body = body.push(summary);
                }
                let selected = state.deletion.selected.iter().any(|item| item.identity == session.identity);
                let read_event = if state.deletion.selecting {
                    session.capabilities.delete.then(|| Event::Delete(delete::Event::Toggle(session.identity.clone())))
                } else {
                    session.capabilities.read.then(|| Event::Read(session.identity.clone()))
                };
                let read = button(body).width(Length::Fill)
                    .padding(iced::Padding { top: 7.0, right: if state.deletion.selecting { 10.0 } else { 82.0 }, bottom: 7.0, left: 10.0 })
                    .style(move |theme, status| flat_button(theme, status, selected))
                    .on_press_maybe(read_event);
                let mut actions = vec![Action::CopyId, Action::CopyPath];
                if loopback && cfg!(target_os = "macos") && session.identity.target_host == local_app_auth::HOST_ID {
                    actions.push(Action::Reveal);
                }
                if session.identity.provider == vw_api_external_session::SessionProvider::Codex {
                    actions.extend([Action::Maintenance, Action::Compression]);
                }
                if session.capabilities.delete && !state.deletion.selecting { actions.push(Action::Delete); }
                let identity = session.identity.clone();
                let generation = state.generation;
                let menu = action_menu(i18n, actions, i18n.__("更多"), move |action| {
                    let target = vw_api_external_session::HistoryMaintenanceTarget {
                        scope: identity.scope.clone(), target_host: identity.target_host.clone(), instance_id: identity.instance_id.clone(),
                    };
                    match action {
                        Action::CopyId => Event::Path(super::paths::Event::Copy(identity.clone(), generation, super::paths::Location::Identity)),
                        Action::CopyPath => Event::Path(super::paths::Event::Copy(identity.clone(), generation, super::paths::Location::Source)),
                        Action::Reveal => Event::Path(super::paths::Event::Reveal(identity.clone(), generation, super::paths::Location::Source)),
                        Action::Delete => Event::Delete(delete::Event::Ask(Some(identity.clone()))),
                        Action::Maintenance => Event::Maintenance(super::maintenance::Event::Open(target)),
                        _ => Event::Compression(super::compression::Event::Open(target)),
                    }
                });
                let mut line = row![].spacing(8).align_y(iced::Alignment::Center);
                if state.deletion.selecting && session.capabilities.delete {
                    let identity = session.identity.clone();
                    line = line.push(checkbox(selected).style(settings_checkbox_style)
                        .on_toggle(move |_| Event::Delete(delete::Event::Toggle(identity.clone()))));
                }
                if state.deletion.selecting {
                    line = line.push(read);
                } else {
                    line = line.push(hover(read,
                        container(super::resume::quick_controls(i18n, session, available))
                            .align_right(Length::Fill).center_y(Length::Fill).padding([0, 8])))
                        .push(menu);
                }
                group = group.push(container(line).padding([0, 4])).push(settings_divider());
            }
            content = content.push(group);
        }
        if page.truncated { content = content.push(text(i18n.__("来源扫描已截断，目录可能不完整")).style(settings_muted_text_style)); }
    } else if !state.busy && state.error.is_none() {
        content = content.push(text(i18n.__("点击刷新，读取当前项目的外部会话。")));
    }
    let more = state.page.as_ref().is_some_and(|page| page.next_cursor.is_some());
    if more {
        content = content.push(container(button(text(i18n.__("加载更多")).size(13)).padding([7, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(available.then_some(Event::Next)))
            .width(Length::Fill).align_x(iced::alignment::Horizontal::Center).padding(8));
    }
    column![header,
        container(scrollable(container(content).width(Length::Fill))
            .id("external-session-list")
            .on_scroll(|viewport| message(Event::ListScrolled(viewport.absolute_offset())))
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))))
            .style(catalog_frame).clip(true),
        footer,
    ].spacing(10).height(Length::Fill).into()
}

fn applications(i18n: desktop_i18n::I18n, state: &State) -> Element<'_, Event> {
    use SessionProvider::*;
    let mut providers = [Codex, Claude, Opencode, Hermes, Pi, Gemini, Openclaw, Grokbuild, Kimi, Omp, Mcode, Dsh, Agy];
    let count = |provider: Option<SessionProvider>| state.app_counts.as_ref().map(|(groups, truncated)| {
        let count: usize = groups.iter().filter(|group| provider.is_none_or(|app| group.provider == app))
            .map(|group| group.count).sum();
        (count, *truncated)
    });
    providers.sort_by_key(|provider| std::cmp::Reverse(count(Some(*provider)).map_or(0, |(count, _)| count)));
    let count_label = |provider| match count(provider) {
        Some((0, false)) if provider.is_some() => i18n.__("无会话").to_owned(),
        Some((n, truncated)) => format!("{n}{}", if truncated { "+" } else { "" }),
        None => "—".to_owned(),
    };
    let app_icon = |provider| match provider {
        Some(provider) => provider_icon(provider, 17.0),
        None => icon_svg(Icon::Grid1x2, 17.0).into(),
    };
    let trigger = button(row![
        app_icon(state.provider),
        text(state.provider.map_or(i18n.__("全部应用"), provider_name)).size(13),
        text(count_label(state.provider)).size(12).style(settings_muted_text_style),
        icon_svg(Icon::ChevronDown, 12.0),
    ].spacing(7).height(Length::Fill).align_y(iced::Alignment::Center))
        .height(SETTINGS_CONTROL_HEIGHT).padding([0, 9]).style(rounded_action_btn_style)
        .on_press(Event::Applications(!state.apps_open));
    let mut menu = column![].spacing(2);
    for provider in std::iter::once(None).chain(providers.into_iter().map(Some)) {
        if provider == Some(providers[0]) { menu = menu.push(settings_divider()); }
        let active = state.provider == provider;
        let check: Element<'_, Event> = if active {
            icon_svg(Icon::Check, 13.0).width(16).into()
        } else {
            iced::widget::space::horizontal().width(16).into()
        };
        menu = menu.push(button(row![
            app_icon(provider),
            text(provider.map_or(i18n.__("全部应用"), provider_name)).size(13).width(Length::Fill),
            text(count_label(provider)).size(12).style(settings_muted_text_style),
            check,
        ].spacing(8).align_y(iced::Alignment::Center)).width(Length::Fill).padding([6, 8])
            .style(move |theme, status| flat_button(theme, status, active))
            .on_press(Event::Provider(provider)));
    }
    let menu = container(scrollable(menu.padding([0, 7])).height(Length::Shrink).direction(
        Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)),
    )).width(252).max_height(386).padding([6, 0]).style(settings_modal_card_style);
    BelowOverlay::new(trigger, menu).show(state.apps_open).gap(4.0)
        .on_close(Event::Applications(false)).dismiss_on_escape().into()
}

pub(super) fn provider_name(provider: SessionProvider) -> &'static str {
    use SessionProvider::*;
    match provider {
        Codex => "Codex", Claude => "Claude Code", Opencode => "OpenCode", Hermes => "Hermes",
        Pi => "Pi", Gemini => "Gemini CLI", Openclaw => "OpenClaw", Grokbuild => "Grok Build",
        Kimi => "Kimi Code", Omp => "Oh My Pi", Mcode => "MiniMax Code",
        Dsh => "DeepSeek Harness", Agy => "Antigravity",
    }
}

pub(super) fn preview(value: &str, limit: usize) -> String {
    let line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = line.chars();
    let mut result: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() { result.push('…'); }
    result
}

pub(super) fn subtle_surface(theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme.palette().text.scale_alpha(0.055).into()),
        border: iced::border::rounded(8),
        ..Default::default()
    }
}

fn catalog_frame(theme: &iced::Theme) -> container::Style {
    container::Style {
        border: iced::Border { width: 1.0, color: theme.palette().text.scale_alpha(0.13),
            radius: 9.0.into() },
        ..Default::default()
    }
}

pub(super) fn flat_button(theme: &iced::Theme, status: button::Status, selected: bool) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    style.border = iced::border::rounded(5);
    style.background = match status {
        button::Status::Hovered | button::Status::Pressed => style.background,
        _ if selected => Some(theme.palette().text.scale_alpha(0.065).into()),
        _ => None,
    };
    style
}

pub(super) fn segment(label: &'static str, selected: bool, event: Event) -> iced::widget::Button<'static, Event> {
    button(text(label).size(13)).padding([5, 10]).on_press(event)
        .style(move |theme, status| {
            let mut style = flat_button(theme, status, false);
            if selected { style.background = Some(theme.palette().background.into()); }
            style
        })
}

fn time_group(session: &SessionMeta) -> &'static str {
    let Some(date) = session.last_active_at.or(session.created_at)
        .and_then(|ms| Local.timestamp_millis_opt(ms).single()) else { return "时间未知"; };
    let days = Local::now().date_naive().signed_duration_since(date.date_naive()).num_days();
    match days { ..=0 => "今天", 1 => "昨天", 2..=6 => "近七天", _ => "更早" }
}

// Both presentation and reader navigation consume this order.
fn groups(state: &State) -> Vec<(Option<String>, String, Vec<&SessionMeta>)> {
    let Some(page) = &state.page else { return Vec::new(); };
    if state.by_time {
        let mut sessions: Vec<_> = page.sessions.iter().collect();
        sessions.sort_by_key(|session| std::cmp::Reverse(session.last_active_at.or(session.created_at)));
        let mut groups: Vec<(Option<String>, String, Vec<&SessionMeta>)> = Vec::new();
        for session in sessions {
            let label = time_group(session);
            if let Some(last) = groups.last_mut().filter(|group| group.1 == label) {
                last.2.push(session);
            } else { groups.push((None, label.to_owned(), vec![session])); }
        }
        groups
    } else {
        let mut groups: BTreeMap<Option<String>, Vec<&SessionMeta>> = BTreeMap::new();
        for session in &page.sessions {
            groups.entry(session.project_dir.clone()).or_default().push(session);
        }
        let mut groups: Vec<_> = groups.into_iter().map(|(dir, mut sessions)| {
            sessions.sort_by_key(|session| std::cmp::Reverse(session.last_active_at.or(session.created_at)));
            let label = dir.clone().unwrap_or_else(|| "未知目录".into());
            (dir, label, sessions)
        }).collect();
        groups.sort_by_key(|(_, _, sessions)| std::cmp::Reverse(
            sessions[0].last_active_at.or(sessions[0].created_at)));
        groups
    }
}

pub(super) fn readable_sessions(state: &State) -> Vec<&SessionMeta> {
    groups(state).into_iter()
        .filter(|(dir, _, _)| state.by_time || !state.collapsed.contains(dir))
        .flat_map(|(_, _, sessions)| sessions).filter(|session| session.capabilities.read).collect()
}

/// Shared brand presentation for the list, source locations and reader toolbar.
pub fn provider_icon<'a>(provider: vw_api_external_session::SessionProvider, size: f32) -> Element<'a, Event> {
    desktop_ui_controls::icons::acp_agent_icon(&format!("{provider:?}").to_lowercase(), size)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action { CopyMarkdown,
    #[cfg(not(target_arch = "wasm32"))]
    ExportMarkdown,
    Options, Reload, Refresh, Select, CopyResume, Terminal, Sources, CopyId, CopyPath, Reveal, Delete, Maintenance, Compression }
impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CopyMarkdown => "复制整段 Markdown",
            #[cfg(not(target_arch = "wasm32"))]
            Self::ExportMarkdown => "导出整段 Markdown",
            Self::Options => "显示与复制选项",
            Self::Reload => "重新读取", Self::Refresh => "刷新列表", Self::Select => "选择多个", Self::Terminal => "首选终端",
            Self::CopyResume => "复制恢复命令", Self::Sources => "会话记录在哪里", Self::CopyId => "复制会话 ID", Self::CopyPath => "复制来源路径",
            Self::Reveal => "在 Finder 中显示", Self::Delete => "删除…", Self::Maintenance => "历史维护设置",
            Self::Compression => "压缩设置与磁盘统计",
        })
    }
}
pub fn action_menu<'a>(i18n: desktop_i18n::I18n, actions: Vec<Action>, placeholder: &'a str, on_select: impl Fn(Action) -> Event + 'a) -> Element<'a, Event> {
    iced::widget::tooltip(
        settings_select(actions, None::<Action>, on_select)
            .format_labels(|action| i18n.__(&action.to_string()).to_owned())
            .menu_icon(Icon::DotsThreeVertical, 218.0).width(Length::Fixed(30.0)),
        text(placeholder).size(12), iced::widget::tooltip::Position::Top,
    ).into()
}
pub(super) fn relative_time(i18n: desktop_i18n::I18n, session: &SessionMeta) -> String {
    let Some(ms) = session.last_active_at.or(session.created_at) else { return i18n.__("时间未知").into(); };
    let minutes = Local::now().timestamp_millis().saturating_sub(ms).max(0) / 60_000;
    match minutes { 0 => i18n.__("刚刚").into(), 1..=59 => i18n.sprintf("%s 分钟前", &[format!("{}", minutes).into()]), 60..=1439 => i18n.sprintf("%s 小时前", &[format!("{}", minutes / 60).into()]), _ => i18n.sprintf("%s 天前", &[format!("{}", minutes / 1440).into()]) }
}
