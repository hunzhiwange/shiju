//! Catalog presentation only; host discovery and pagination stay in the SDK.
use super::{Event, State, message, delete};
use desktop_ui_controls::system_settings_common::*;
use super::Event as Message;

use chrono::{Local, TimeZone};
use iced::{Element, Length, widget::{button, column, container, row, scrollable, text, text_input, checkbox, hover}};
use iced::widget::scrollable::{Direction, Scrollbar};
use std::collections::BTreeMap;
use vw_api_external_session::SessionMeta;

pub(super) fn view(state: &State, loopback: bool) -> Element<'_, Message> {
    if !state.open {
        return button(text("浏览外部会话"))
            .padding([8, 12]).style(rounded_action_btn_style)
            .on_press(message(Event::Open)).into();
    }
    let mut header = column![
        row![button(text("返回应用")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press(message(Event::Close))].spacing(12),
        row![
            action_menu(vec![Action::Refresh, Action::Select, Action::Terminal, Action::Sources],
                "会话菜单", |action| match action {
                    Action::Refresh => Event::Refresh,
                    Action::Select => Event::Delete(delete::Event::Enter),
                    Action::Terminal => Event::Resume(super::resume::Event::Preferences),
                    _ => Event::Path(super::paths::Event::Sources(true)),
                }),
            button(text(if state.by_time { "项目" } else { "项目 ✓" })).padding([8, 12])
                .style(rounded_action_btn_style).on_press(Event::GroupByTime(false)),
            button(text(if state.by_time { "时间 ✓" } else { "时间" })).padding([8, 12])
                .style(rounded_action_btn_style).on_press(Event::GroupByTime(true)),
        ].spacing(8).wrap(),
    ].spacing(8);
    header = header.push(row![
        text_input("搜索标题、摘要、目录、来源路径或 ID", &state.search_draft)
            .on_input(Event::SearchDraft).on_submit(Event::Search)
            .font(iced::Font::DEFAULT).style(settings_text_input_style).padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE),
        button(text("搜索")).padding([8, 12]).style(primary_action_btn_style).on_press(Event::Search),
        button(text("清空")).padding([8, 12]).style(rounded_action_btn_style).on_press(Event::ClearSearch),
    ].spacing(8));
    let mut apps = column![button(text(if state.provider.is_none() { "全部应用 ✓" } else { "全部应用" }))
        .padding([8, 12]).style(rounded_action_btn_style).on_press(Event::Provider(None))].spacing(8);
    let mut providers = vec![vw_api_external_session::SessionProvider::Claude,
        vw_api_external_session::SessionProvider::Codex, vw_api_external_session::SessionProvider::Opencode,
        vw_api_external_session::SessionProvider::Hermes, vw_api_external_session::SessionProvider::Pi,
        vw_api_external_session::SessionProvider::Gemini, vw_api_external_session::SessionProvider::Openclaw,
        vw_api_external_session::SessionProvider::Grokbuild, vw_api_external_session::SessionProvider::Kimi,
        vw_api_external_session::SessionProvider::Omp, vw_api_external_session::SessionProvider::Mcode,
        vw_api_external_session::SessionProvider::Dsh, vw_api_external_session::SessionProvider::Agy];
    providers.sort_by_key(|provider| std::cmp::Reverse(state.app_counts.as_ref().map(|(groups, _)|
        groups.iter().filter(|group| group.provider == *provider).map(|group| group.count).sum::<usize>()).unwrap_or(0)));
    for provider in providers {
        let count = state.app_counts.as_ref().map(|(groups, truncated)| {
            let count = groups.iter().filter(|group| group.provider == provider).map(|group| group.count).sum::<usize>();
            if *truncated { format!("{count}+（扫描截断）") } else { count.to_string() }
        }).unwrap_or_else(|| "未知".into());
        apps = apps.push(button(row![provider_icon(provider, 18.0),
            text(format!("{provider:?}{}", if state.provider == Some(provider) { " ✓" } else { "" })).width(Length::Fill),
            text(count.clone()).width(Length::Fill).align_x(iced::alignment::Horizontal::Right)
                .style(move |theme| if count == "0" { settings_muted_text_style(theme) } else { iced::widget::text::default(theme) }),
        ].spacing(8))
            .padding([8, 12]).style(rounded_action_btn_style).on_press(Event::Provider(Some(provider))));
    }
    header = header.push(button(row![
        text(state.provider.map(|provider| format!("{provider:?}")).unwrap_or_else(|| "全部应用".into())),
        text("▾"),
    ].spacing(12)).padding([8, 12]).style(rounded_action_btn_style).on_press(Event::Applications(true)));
    if state.busy { header = header.push(text("正在读取会话…").style(settings_muted_text_style)); }
    if state.error { header = header.push(settings_error_banner("外部会话读取失败，请刷新重试；已加载内容保留。")); }
    let available = !state.busy && !state.deletion.busy();
    let mut footer = column![].spacing(8);
    if state.deletion.selecting {
        footer = footer.push(row![
            text(format!("已选择 {} 项", state.deletion.selected.len())),
            button(text("全选匹配结果")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe(available.then(|| message(Event::Delete(delete::Event::SelectAll)))),
            button(text("清空")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe(available.then(|| message(Event::Delete(delete::Event::Clear)))),
            button(text("退出选择")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe(available.then(|| message(Event::Delete(delete::Event::Exit)))),
            button(text("删除所选")).padding([8, 12]).style(primary_action_btn_style)
                .on_press_maybe((available && !state.deletion.selected.is_empty()).then(|| message(Event::Delete(delete::Event::Ask(None))))),
        ].spacing(8).wrap());
    }
    if state.deletion.busy() {
        header = header.push(text("正在处理所选会话，请稍候…").style(settings_muted_text_style));
    }
    let mut content = column![].spacing(10).padding([0, 7]);
    for (identity, reason) in &state.deletion.failures {
        content = content.push(settings_error_banner(format!("{:?} · {} · {}：{}",
            identity.provider, identity.instance_id, identity.session_id.0, reason)));
    }
    if let Some(page) = &state.page {
        header = header.push(text(format!("主机：{} · 共 {} 项 · 已加载 {} 项", state.host, page.total, page.sessions.len())));
        if page.sessions.is_empty() { content = content.push(text(if state.search.is_empty() && state.provider.is_none() { "当前授权范围内没有外部会话" } else { "没有匹配的会话" })); }
        for (dir, label, sessions) in groups(state) {
            let count = if !state.by_time && state.search.is_empty() && !page.groups.is_empty() {
                Some(page.groups.iter().filter(|group| group.project_dir == dir
                    && state.provider.is_none_or(|provider| provider == group.provider))
                    .map(|group| group.count).sum::<usize>())
            } else { None };
            let collapsed = !state.by_time && state.collapsed.contains(&dir);
            let title = format!("{}{} · 已加载 {} 项{}", if collapsed { "▸ " } else { "▾ " }, label, sessions.len(),
                count.map(|count| format!(" / {} {count} 项", if page.truncated { "至少" } else { "共" })).unwrap_or_default());
            let recent = sessions.first().map(|session| relative_time(session)).unwrap_or_default();
            let project_name = label.rsplit(['/', '\\']).find(|part| !part.is_empty()).unwrap_or(&label);
            let mut group = column![text(format!("{project_name} · {recent}")).size(14), button(text(title).size(12)).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe((!state.by_time).then(|| Event::ToggleProject(dir)))].spacing(8);
            if collapsed { content = content.push(group); continue; }
            for session in sessions {
                let mut body = column![row![
                    text(session.title.as_deref().unwrap_or(&session.identity.session_id.0)).size(15).width(Length::Fill),
                    text(relative_time(session)).size(12).style(settings_muted_text_style),
                ].spacing(8)].spacing(4);
                let archived = session.source_path.as_deref().is_some_and(|path|
                    path.replace('\\', "/").contains("/archived_sessions/"));
                body = body.push(text(format!("{}{}", if archived { "已归档 · " } else { "" },
                    session.summary.as_deref().unwrap_or("暂无摘要"))).size(12).style(settings_muted_text_style));
                let read = button(body).width(Length::Fill).padding(iced::Padding { top: 8.0, right: if state.deletion.selecting { 12.0 } else { 104.0 }, bottom: 8.0, left: 12.0 }).style(rounded_action_btn_style)
                    .on_press_maybe((session.capabilities.read && !state.deletion.selecting)
                        .then(|| Event::Read(session.identity.clone())));
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
                let menu = action_menu(actions, "更多", move |action| {
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
                    line = line.push(checkbox(state.deletion.selected.iter().any(|selected| selected.identity == identity))
                        .on_toggle(move |_| Event::Delete(delete::Event::Toggle(identity.clone()))));
                }
                if state.deletion.selecting {
                    line = line.push(read);
                } else {
                    line = line.push(hover(read,
                        container(super::resume::quick_controls(session, available))
                            .align_right(Length::Fill).center_y(Length::Fill).padding([0, 8])))
                        .push(menu);
                }
                group = group.push(line).push(settings_divider());
            }
            content = content.push(group);
        }
        if page.truncated { content = content.push(text("来源扫描已截断，目录可能不完整").style(settings_muted_text_style)); }
    } else if !state.busy && !state.error {
        content = content.push(text("点击刷新，读取当前项目和授权主机的外部会话。"));
    }
    let more = state.page.as_ref().is_some_and(|page| page.next_cursor.is_some());
    let base = column![header,
        scrollable(container(content).width(Length::Fill))
            .id("external-session-list")
            .on_scroll(|viewport| message(Event::ListScrolled(viewport.absolute_offset())))
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
        footer,
        button(text("加载更多")).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((more && !state.busy && !state.deletion.busy()).then(|| message(Event::Next))),
    ].spacing(10).height(Length::Fill).into();
    if !state.apps_open { return base; }
    let close = Event::Applications(false);
    let card = settings_modal_card(column![
        row![text("选择应用").size(18), settings_close_button(close.clone())].spacing(12),
        scrollable(apps).height(Length::Fixed(340.0))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
    ].spacing(12)).max_width(400);
    settings_modal_overlay_with_escape_and_backdrop_dismiss(base, close, card)
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
        groups.into_iter().map(|(dir, mut sessions)| {
            sessions.sort_by_key(|session| std::cmp::Reverse(session.last_active_at.or(session.created_at)));
            let label = dir.clone().unwrap_or_else(|| "未知目录".into());
            (dir, label, sessions)
        }).collect()
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
pub enum Action { CopyMarkdown, ExportMarkdown, Options, Refresh, Select, Terminal, Sources, CopyId, CopyPath, Reveal, Delete, Maintenance, Compression }
impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CopyMarkdown => "复制整段 Markdown", Self::ExportMarkdown => "导出整段 Markdown", Self::Options => "显示与复制选项",
            Self::Refresh => "刷新", Self::Select => "多选", Self::Terminal => "首选终端",
            Self::Sources => "会话记录在哪里", Self::CopyId => "复制会话 ID", Self::CopyPath => "复制来源路径",
            Self::Reveal => "在 Finder 中显示", Self::Delete => "删除…", Self::Maintenance => "历史维护设置",
            Self::Compression => "压缩设置与磁盘统计",
        })
    }
}
pub fn action_menu<'a>(actions: Vec<Action>, placeholder: &'a str, on_select: impl Fn(Action) -> Event + 'a) -> Element<'a, Event> {
    settings_select(actions, None::<Action>, on_select).placeholder(placeholder)
        .width(Length::Fixed(if placeholder == "更多" { 80.0 } else { 128.0 })).into()
}
pub(super) fn relative_time(session: &SessionMeta) -> String {
    let Some(ms) = session.last_active_at.or(session.created_at) else { return "时间未知".into(); };
    let minutes = Local::now().timestamp_millis().saturating_sub(ms).max(0) / 60_000;
    match minutes { 0 => "刚刚".into(), 1..=59 => format!("{minutes} 分钟前"), 60..=1439 => format!("{} 小时前", minutes / 60), _ => format!("{} 天前", minutes / 1440) }
}
