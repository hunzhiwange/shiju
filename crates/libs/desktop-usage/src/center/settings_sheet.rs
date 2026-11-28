//! Right-hand usage settings. The scan owner retains jobs after dismissal.
use super::{
    Outcome,
    scan::{self, Action, Event, Navigation, State},
};
use crate::Identity;
use desktop_ui_controls::system_settings_common::help::settings_hover_help_label;
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Element, Length, Theme,
    widget::{Space, button, column, container, row, rule, text},
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

fn section<'a, M: Clone + 'static>(
    title: &'static str,
    content: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    column![settings_group_header(text(title).size(14).style(text::secondary)), content.into()]
        .spacing(8)
        .width(Length::Fill)
        .into()
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

fn scan_controls(i18n: desktop_i18n::I18n, state: &State, ready: bool) -> Element<'_, Outcome<Event>> {
    let active = state.job.as_ref().is_some_and(|job| !scan::terminal(job));
    let preferences = state.preferences;
    let enabled = preferences.is_some_and(|preferences| preferences.enabled);
    let status = state.last_sync_time().map_or_else(
        || i18n.__("尚未完成同步").into(),
        |time| time.format("%Y-%m-%d %H:%M:%S").to_string(),
    );
    let mut scanning = column![
        row![
            settings_hover_help_label(
                text(i18n.__("自动扫描会话日志")).size(15),
                text(i18n.__("扫描可用的本地会话日志")).size(12).into(),
            ),
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
        text(i18n.__("支持 Claude Code、Codex、Gemini CLI、Grok Build、OpenCode、Pi、Oh My Pi、MiniMax Code、Kimi Code、DeepSeek Harness 和 Antigravity 的日志格式。"))
            .size(12).style(text::secondary),
        text(i18n.format("启动和打开用量页时各触发一次同步，定时同步按所选间隔执行。上次同步：%s", &[(status.to_string()).into()]).expect("valid display template"))
            .size(12).style(text::secondary),
        text(i18n.__("Hermes、OpenClaw 暂不支持用量统计")).size(12).style(text::secondary),
        row![
            settings_select(
                SCAN_INTERVALS.map(ScanInterval),
                preferences.map(|preferences| ScanInterval(preferences.refresh_seconds)),
                move |interval| Outcome::Event(Event::Run(Action::Save(
                    gateway_usage_scan::UsageScanPreferences { enabled, refresh_seconds: interval.0 },
                ))),
            )
            .format_labels(|interval| match interval.0 {
                seconds if seconds == 30 * DAY => i18n.__("每 1 月同步（30 天）").to_owned(),
                seconds if seconds == 7 * DAY => i18n.__("每 1 星期同步").to_owned(),
                seconds if seconds % DAY == 0 => i18n.format("每 %s 天同步", &[format!("{}", seconds / DAY).into()]).expect("valid interval"),
                seconds if seconds % HOUR == 0 => i18n.format("每 %s 小时同步", &[format!("{}", seconds / HOUR).into()]).expect("valid interval"),
                seconds => i18n.format("每 %s 分钟同步", &[format!("{}", seconds / 60).into()]).expect("valid interval"),
            })
            .placeholder(i18n.__("每 1 天同步"))
            .width(192)
            .disabled(!ready || !enabled),
            Space::new().width(Length::Fill),
            action(if active { i18n.__("任务进行中…") } else { i18n.__("立即同步") }, Event::Run(Action::Sync), ready && !active),
        ].spacing(8).align_y(iced::Alignment::Center),
    ].spacing(10);

    if let Some(job) = &state.job {
        // Job payloads may include paths. Show only the task status.
        let phase = job
            .progress
            .get("phase")
            .or_else(|| job.progress.get("status"))
            .and_then(|value| value.as_str())
            .unwrap_or(i18n.__("等待主机反馈"));
        scanning = scanning.push(text(i18n.format("任务状态  %s", &[(phase.to_string()).into()]).expect("valid display template")).size(12));
        if active {
            scanning = scanning.push(
                row![
                    action(i18n.__("取消任务"), Event::Run(Action::Cancel), ready),
                    action(i18n.__("刷新进度"), Event::Run(Action::Poll), ready),
                ]
                .spacing(8),
            );
        }
    }
    let routing = row![
        column![
            text(i18n.__("记录请求用量")).size(15),
            text(i18n.__("在本地路由设置中查看记录状态。")).size(12).style(text::secondary),
        ]
        .spacing(4)
        .width(Length::Fill),
        action(i18n.__("路由设置 ›"), Event::Navigate(Navigation::Routing), ready),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center);
    let mut body = column![
        container(scanning).padding([14, 0]),
        rule::horizontal(1),
        container(routing).padding([14, 0]),
    ];
    if state.error.is_some() {
        body = body.push(
            container(
                column![
                    settings_error_banner(i18n.__("操作失败，请重试或重新读取设置。")),
                    action(i18n.__("重新读取设置"), Event::Run(Action::Load), ready),
                ]
                .spacing(8),
            )
            .padding([14, 0]),
        );
    }
    body.into()
}

pub fn overlay<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n,
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
    let display_settings = action(i18n.__("币种与显示 ›"), Event::Navigate(Navigation::Display), true);
    let pricing = action(i18n.__("管理定价 ›"), Event::Navigate(Navigation::Pricing), true);
    let body = column![
        section(i18n.__("数据来源"), scan_controls(i18n, state, ready).map(map)),
        section(
            i18n.__("显示"),
            container(
                column![
                    display,
                    container(Element::from(display_settings).map(map)).align_right(Length::Fill),
                ]
                .spacing(10)
            )
            .padding([14, 0])
        ),
        section(
            i18n.__("定价"),
            container(
                row![
                    text(i18n.__("模型同步、计费规则与模型单价"))
                        .size(13)
                        .style(text::secondary)
                        .width(Length::Fill),
                    Element::from(pricing).map(map),
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center)
            )
            .padding([14, 0])
        ),
    ]
    .spacing(18);
    let close = map(Outcome::Event(Event::Close));
    let header = column![
        settings_modal_header(text(i18n.__("用量设置")).size(21), settings_close_button(close.clone())),
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
