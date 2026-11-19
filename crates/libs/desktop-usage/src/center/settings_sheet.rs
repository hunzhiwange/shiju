//! Right-hand usage settings. The scan owner retains jobs after dismissal.
use super::{
    Outcome,
    scan::{self, Action, Event, Navigation, State},
};
use crate::Identity;
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Element, Length, Theme,
    widget::{Space, button, column, container, row, rule, text, tooltip},
};

fn action<'a>(
    label: &'static str,
    event: Event,
    enabled: bool,
) -> iced::widget::Button<'a, Outcome<Event>> {
    button(text(label).size(13))
        .padding([6, 10])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(Outcome::Event(event)))
}

fn card<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> iced::widget::Container<'a, M> {
    container(content).width(Length::Fill).style(super::view::panel_style)
}

fn section<'a, M: 'a>(title: &'static str, content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    column![text(title).size(14).style(text::secondary), card(content)].spacing(8).into()
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ScanInterval(u32);

const HOUR: u32 = 3_600;
const DAY: u32 = vw_api_usage::DEFAULT_USAGE_SCAN_REFRESH_SECONDS;
const SCAN_INTERVALS: [u32; 11] = [
    5, 10, 30, 60, HOUR, 8 * HOUR, 12 * HOUR, DAY, 2 * DAY, 7 * DAY, 30 * DAY,
];

impl std::fmt::Display for ScanInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            seconds if seconds == 30 * DAY => f.write_str("每 1 月同步（30 天）"),
            seconds if seconds == 7 * DAY => f.write_str("每 1 星期同步"),
            seconds if seconds % DAY == 0 => write!(f, "每 {} 天同步", seconds / DAY),
            seconds if seconds % HOUR == 0 => write!(f, "每 {} 小时同步", seconds / HOUR),
            seconds if seconds % 60 == 0 => write!(f, "每 {} 分钟同步", seconds / 60),
            seconds => write!(f, "每 {seconds} 秒同步"),
        }
    }
}

fn scan_controls(state: &State, ready: bool) -> Element<'_, Outcome<Event>> {
    let active = state.job.as_ref().is_some_and(|job| !scan::terminal(job));
    let preferences = state.preferences;
    let enabled = preferences.is_some_and(|preferences| preferences.enabled);
    let status = state.last_sync_time().map_or_else(
        || "尚未完成同步".into(),
        |time| time.format("%Y-%m-%d %H:%M:%S").to_string(),
    );
    let mut scanning = column![
        row![
            row![
                text("自动扫描会话日志").size(15),
                tooltip(
                    icon_svg(Icon::QuestionCircle, 14.0),
                    text("扫描可用的本地会话日志").size(12),
                    tooltip::Position::Bottom,
                ),
            ].spacing(6).align_y(iced::Alignment::Center),
            Space::new().width(Length::Fill),
            settings_toggle(enabled).on_toggle_maybe(
                preferences.filter(|_| ready).map(|preferences| move |enabled| {
                    Outcome::Event(Event::Run(Action::Save(gateway_usage_scan::UsageScanPreferences {
                        enabled,
                        ..preferences
                    })))
                })
            ),
        ].spacing(12).align_y(iced::Alignment::Center),
        text("支持 Claude Code、Codex、Gemini CLI、Grok Build、OpenCode、Pi、Oh My Pi、MiniMax Code、Kimi Code、DeepSeek Harness 和 Antigravity 的日志格式。")
            .size(12).style(text::secondary),
        text(format!("启动和打开用量页时各触发一次同步，定时同步按所选间隔执行。上次同步：{status}"))
            .size(12).style(text::secondary),
        text("Hermes、OpenClaw 暂不支持用量统计").size(12).style(text::secondary),
        row![
            settings_select(
                SCAN_INTERVALS.map(ScanInterval),
                preferences.map(|preferences| ScanInterval(preferences.refresh_seconds)),
                move |interval| Outcome::Event(Event::Run(Action::Save(
                    gateway_usage_scan::UsageScanPreferences { enabled, refresh_seconds: interval.0 },
                ))),
            )
            .placeholder("每 1 天同步")
            .width(192)
            .disabled(!ready || !enabled),
            Space::new().width(Length::Fill),
            action(if active { "任务进行中…" } else { "立即同步" }, Event::Run(Action::Sync), ready && !active),
        ].spacing(8).align_y(iced::Alignment::Center),
    ].spacing(10);

    if let Some(job) = &state.job {
        // Job payloads may include paths. Show only the task status.
        let phase = job
            .progress
            .get("phase")
            .or_else(|| job.progress.get("status"))
            .and_then(|value| value.as_str())
            .unwrap_or("等待主机反馈");
        scanning = scanning.push(text(format!("任务状态  {phase}")).size(12));
        if active {
            scanning = scanning.push(
                row![
                    action("取消任务", Event::Run(Action::Cancel), ready),
                    action("刷新进度", Event::Run(Action::Poll), ready),
                ]
                .spacing(8),
            );
        }
    }
    let routing = row![
        column![
            text("记录请求用量").size(15),
            text("在本地路由设置中查看记录状态。").size(12).style(text::secondary),
        ]
        .spacing(4)
        .width(Length::Fill),
        action("路由设置 ›", Event::Navigate(Navigation::Routing), ready),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center);
    let mut body = column![
        container(scanning).padding(14),
        rule::horizontal(1),
        container(routing).padding(14),
    ];
    if state.error.is_some() {
        body = body.push(
            container(
                column![
                    settings_error_banner("操作失败，请重试或重新读取设置。"),
                    action("重新读取设置", Event::Run(Action::Load), ready),
                ]
                .spacing(8),
            )
            .padding(14),
        );
    }
    body.into()
}

pub fn overlay<'a, M: Clone + 'static>(
    state: &'a State,
    current: Option<Identity>,
    base: Element<'a, M>,
    display: Element<'a, M>,
    map: fn(Outcome<Event>) -> M,
) -> Element<'a, M> {
    if !state.open {
        return base;
    }
    let ready = current.is_some() && state.scope == current && !state.pending;
    let display_settings = action("币种与显示 ›", Event::Navigate(Navigation::Display), true);
    let pricing = action("管理定价 ›", Event::Navigate(Navigation::Pricing), true);
    let body = column![
        section("数据来源", scan_controls(state, ready).map(map)),
        section(
            "显示",
            container(
                column![
                    display,
                    container(Element::from(display_settings).map(map)).align_right(Length::Fill),
                ]
                .spacing(10)
            )
            .padding(14)
        ),
        section(
            "定价",
            container(
                row![
                    text("模型同步、计费规则与模型单价")
                        .size(13)
                        .style(text::secondary)
                        .width(Length::Fill),
                    Element::from(pricing).map(map),
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center)
            )
            .padding(14)
        ),
    ]
    .spacing(18);
    let close = map(Outcome::Event(Event::Close));
    let header = column![
        settings_modal_header(text("用量设置").size(21), settings_close_button(close.clone())),
        rule::horizontal(1),
    ]
    .spacing(12);
    let drawer = settings_modal_layout(header, body, None).width(500).style(|theme: &Theme| {
        let mut style = settings_modal_card_style(theme).background(theme.palette().background);
        style.border.radius = iced::border::left(18);
        style
    });
    settings_right_drawer_overlay(base, Some((close, drawer.into())))
}
