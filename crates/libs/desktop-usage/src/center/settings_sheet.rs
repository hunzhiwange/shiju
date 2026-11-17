//! Right-hand usage settings. The scan owner retains jobs after dismissal.
use super::{Outcome, scan::{self, Action, Event, Navigation, State}};
use crate::Identity;
use desktop_ui_controls::system_settings_common::*;
use iced::{Element, Length, widget::{button, checkbox, column, container, pick_list, row, text}};

fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Outcome<Event>> {
    button(text(label).size(13)).padding([6, 10]).style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(Outcome::Event(event))).into()
}

pub fn overlay<'a, M: Clone + 'static>(state: &'a State, current: Option<Identity>, base: Element<'a, M>, map: fn(Outcome<Event>) -> M) -> Element<'a, M> {
    if !state.open { return base; }
    let ready = current.is_some() && state.scope == current && !state.pending;
    let active = state.job.as_ref().is_some_and(|job| !scan::terminal(job));
    let last = state.last.as_ref().and_then(|last| last.last_complete_scan)
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|t| chrono::DateTime::from_timestamp(t.as_secs() as i64, 0))
        .map(|t| t.to_rfc3339()).unwrap_or_else(|| "尚无完整同步".into());
    let mut sources = column![
        row![text("会话日志扫描").size(15), checkbox(!state.paused).label("自动扫描")
            .style(settings_checkbox_style)
            .on_toggle_maybe(ready.then_some(|enabled| Outcome::Event(Event::Run(Action::Pause(!enabled)))))].spacing(12),
        text("覆盖 11 个应用：Claude Code、Codex、Gemini CLI、Grok Build、OpenCode、Pi、OMP、MCode、Kimi、Dsh、Antigravity。").size(12),
        text("Hermes / OpenClaw 暂不支持会话用量扫描。").size(12),
        text(format!("上次完整同步（UTC）：{last}")).size(12),
        action(if active { "正在同步…" } else { "立即同步" }, Event::Run(Action::Sync), ready && !active),
    ].spacing(10);
    // Preserve individual host source policies instead of overwriting all apps
    // with a single optimistic device setting. Failed CAS saves keep old values.
    for source in &state.sources {
        let name = source.app.clone();
        let interval_name = name.clone();
        let interval: Element<'_, Outcome<Event>> = if ready && source.settings.scan_enabled {
            pick_list(vec![5u32, 10, 30, 60], source.settings.refresh_seconds, move |seconds|
                Outcome::Event(Event::Run(Action::Interval(interval_name.clone(), seconds))))
                .padding(SETTINGS_CONTROL_PADDING).text_size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_pick_list_style).into()
        } else { text(source.settings.refresh_seconds.map(|n| format!("{n} 秒")).unwrap_or_else(|| "自动刷新关闭".into())).size(12).into() };
        sources = sources.push(column![
            checkbox(source.settings.scan_enabled).label(source.app.clone()).style(settings_checkbox_style)
                .on_toggle_maybe(ready.then_some(move |enabled| Outcome::Event(Event::Run(Action::Source(name.clone(), enabled))))),
            row![text("扫描间隔（秒）").size(12), interval].spacing(8),
        ].spacing(6));
    }
    let mut body = column![text("数据来源").size(13), settings_modal_card(sources),
        action("记录请求用量 ›", Event::Navigate(Navigation::Routing), ready),
        text("请求日志记录状态请在当前主机的本地路由设置中查看和修改。").size(12),
        text("Codex 用量维护").size(13),
        text("当前主机接口不提供备份凭证，重建前请自行确认备份。").size(12),
        text("重建将替换当前项目与主机的原生 Codex 用量，请先确认备份。关闭抽屉不会取消后台任务。").size(12),
        action("重建 Codex…", Event::ConfirmRebuild, ready && !active),
    ].spacing(12);
    if state.confirm_rebuild {
        body = body.push(settings_modal_card(column![
            text("确认重建 Codex 用量？").size(15),
            text("此操作会重新读取会话记录并替换原生用量。已有记录可能发生变化，请确认主机已备份后继续。").size(12),
            row![action("取消", Event::DismissRebuild, true), action("确认重建", Event::Run(Action::Rebuild), ready && !active)].spacing(8),
        ].spacing(10)));
    }
    if let Some(job) = &state.job {
        // Do not render arbitrary host payloads: errors can contain source paths.
        let phase = job.progress.get("phase").or_else(|| job.progress.get("status"))
            .and_then(|v| v.as_str()).unwrap_or("等待主机反馈");
        body = body.push(text(format!("任务状态：{phase}")).size(12));
        if active { body = body.push(row![action("取消任务", Event::Run(Action::Cancel), ready), action("刷新进度", Event::Run(Action::Poll), ready)].spacing(8)); }
    }
    if state.error.is_some() { body = body.push(text("操作失败，原设置保留。请重试或重新读取设置。").size(12)); }
    body = body.push(action("重新读取设置", Event::Run(Action::Load), ready))
        .push(text("显示").size(13)).push(action("币种与显示 ›", Event::Navigate(Navigation::Display), true))
        .push(text("定价").size(13)).push(action("模型定价 ›", Event::Navigate(Navigation::Pricing), true));
    let card = settings_modal_layout(
        settings_modal_header(text("用量设置"), settings_close_button(Outcome::Event(Event::Close))),
        body, Some(action("关闭", Event::Close, true)),
    ).width(Length::Fixed(440.0)).height(Length::Fill);
    let drawer = container(Element::from(card).map(map)).align_right(Length::Fill).height(Length::Fill);
    settings_modal_overlay(Some(base), map(Outcome::Event(Event::Close)), drawer)
}
