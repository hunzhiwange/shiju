//! Catalog presentation only; host discovery and pagination stay in the SDK.
use super::{Event, State, message, delete};
use crate::app::{Message, components::system_settings_common::*};
use chrono::{Local, TimeZone};
use iced::{Element, Length, widget::{button, column, container, row, scrollable, text}};
use iced::widget::scrollable::{Direction, Scrollbar};
use std::collections::BTreeMap;
use vw_gateway_client::vw_api_types::external_session::SessionMeta;

pub(super) fn view(state: &State) -> Element<'_, Message> {
    if !state.open {
        return button(text("浏览外部会话"))
            .padding([8, 12]).style(rounded_action_btn_style)
            .on_press(message(Event::Open)).into();
    }
    let mut header = column![
        row![text("外部会话").size(18),
            button(text("返回应用")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press(message(Event::Close))].spacing(12),
        row![
            button(text("刷新")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press(message(Event::Refresh)),
            button(text("按项目分组")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press(message(Event::GroupByTime(false))),
            button(text("按时间分组")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press(message(Event::GroupByTime(true))),
            super::paths::source_button(),
        ].spacing(8).wrap(),
    ].spacing(8);
    if state.busy { header = header.push(text("正在读取会话…").style(settings_muted_text_style)); }
    if state.error { header = header.push(settings_error_banner("外部会话读取失败，请刷新重试；已加载内容保留。")); }
    let available = !state.busy && !state.deletion.busy();
    if state.deletion.selecting {
        header = header.push(row![
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
    } else if state.page.as_ref().is_some_and(|page| page.sessions.iter().any(|session| session.capabilities.delete)) {
        header = header.push(button(text("选择会话")).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe(available.then(|| message(Event::Delete(delete::Event::Enter)))));
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
        if page.sessions.is_empty() { content = content.push(text("当前授权范围内没有外部会话")); }
        let mut groups: BTreeMap<(u8, String), Vec<&SessionMeta>> = BTreeMap::new();
        for session in &page.sessions {
            let key = if state.by_time {
                format!("{} · {:?} · {} · {}", time_group(session), session.identity.provider,
                    session.identity.instance_id, session.identity.target_host)
            } else {
                format!("{} · {:?} · {} · {}", session.project_dir.as_deref().unwrap_or("未知项目"),
                    session.identity.provider, session.identity.instance_id, session.identity.target_host)
            };
            let order = if state.by_time {
                match time_group(session) { "今天" => 0, "昨天" => 1, "近七天" => 2, "更早" => 3, _ => 4 }
            } else { 0 };
            groups.entry((order, key)).or_default().push(session);
        }
        for ((_, label), mut sessions) in groups {
            sessions.sort_by_key(|session| std::cmp::Reverse(session.last_active_at.or(session.created_at)));
            let count = page.groups.iter().filter(|group| sessions.first().is_some_and(|session|
                group.provider == session.identity.provider && group.instance_id == session.identity.instance_id
                    && group.project_dir == session.project_dir)).map(|group| group.count).sum::<usize>();
            let mut group = column![text(format!("{} · 已加载 {} 项{}", label, sessions.len(),
                if !state.by_time && count > 0 { format!(" / 共 {count} 项") } else { String::new() })).size(14)].spacing(8);
            for session in sessions {
                let mut item = column![text(session.title.as_deref().unwrap_or(&session.identity.session_id.0)).size(15)].spacing(4);
                if let Some(summary) = &session.summary { item = item.push(text(summary).size(13).style(settings_muted_text_style)); }
                let date = session.last_active_at.or(session.created_at)
                    .and_then(|ms| Local.timestamp_millis_opt(ms).single())
                    .map(|date| date.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "时间未知".into());
                item = item.push(text(format!("{} · {}", date, session.project_dir.as_deref().unwrap_or("未知项目")))
                    .size(12).style(settings_muted_text_style));
                item = item.push(super::paths::controls(state, session));
                // Source locations are display metadata, never filesystem read authority.
                if session.source_path.as_deref().is_some_and(|path|
                    path.replace('\\', "/").contains("/archived_sessions/")) {
                    item = item.push(text("已归档").size(12).style(settings_muted_text_style));
                }
                item = item.push(button(text("阅读会话")).padding([8, 12]).style(rounded_action_btn_style)
                    .on_press_maybe(session.capabilities.read.then(|| message(Event::Read(session.identity.clone())))));
                item = item.push(super::resume::control(session, available));
                if session.identity.provider == vw_gateway_client::vw_api_types::external_session::SessionProvider::Codex {
                    let target = vw_gateway_client::vw_api_types::external_session::HistoryMaintenanceTarget {
                        scope: session.identity.scope.clone(), target_host: session.identity.target_host.clone(),
                        instance_id: session.identity.instance_id.clone(),
                    };
                    item = item.push(button(text("历史维护设置")).padding([8, 12]).style(rounded_action_btn_style)
                        .on_press_maybe(available.then(|| message(Event::Maintenance(super::maintenance::Event::Open(target.clone()))))));
                    item = item.push(button(text("压缩设置与磁盘统计")).padding([8, 12]).style(rounded_action_btn_style)
                        .on_press_maybe(available.then(|| message(Event::Compression(super::compression::Event::Open(target))))));
                }
                if session.capabilities.delete {
                    let event = if state.deletion.selecting {
                        delete::Event::Toggle(session.identity.clone())
                    } else { delete::Event::Ask(Some(session.identity.clone())) };
                    let label = if state.deletion.selecting {
                        if state.deletion.selected.iter().any(|selected| selected.identity == session.identity) { "取消选中" } else { "选择" }
                    } else { "删除" };
                    item = item.push(button(text(label)).padding([8, 12]).style(rounded_action_btn_style)
                        .on_press_maybe(available.then(|| message(Event::Delete(event)))));
                }
                group = group.push(settings_group_card(item));
            }
            content = content.push(group);
        }
        if page.truncated { content = content.push(text("来源扫描已截断，目录可能不完整").style(settings_muted_text_style)); }
    } else if !state.busy && !state.error {
        content = content.push(text("点击刷新，读取当前项目和授权主机的外部会话。"));
    }
    let more = state.page.as_ref().is_some_and(|page| page.next_cursor.is_some());
    column![header,
        scrollable(container(content).width(Length::Fill))
            .id("external-session-list")
            .on_scroll(|viewport| message(Event::ListScrolled(viewport.absolute_offset())))
            .height(Length::Fixed(420.0))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
        button(text("加载更多")).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((more && !state.busy).then(|| message(Event::Next))),
    ].spacing(10).into()
}

fn time_group(session: &SessionMeta) -> &'static str {
    let Some(date) = session.last_active_at.or(session.created_at)
        .and_then(|ms| Local.timestamp_millis_opt(ms).single()) else { return "时间未知"; };
    let days = Local::now().date_naive().signed_duration_since(date.date_naive()).num_days();
    match days { ..=0 => "今天", 1 => "昨天", 2..=6 => "近七天", _ => "更早" }
}
