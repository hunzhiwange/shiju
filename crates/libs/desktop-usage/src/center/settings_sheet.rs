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

impl std::fmt::Display for ScanInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "每 {} 秒扫描", self.0)
    }
}

fn scan_controls(state: &State, ready: bool) -> Element<'_, Outcome<Event>> {
    let active = state.job.as_ref().is_some_and(|job| !scan::terminal(job));
    let preferences = state.preferences;
    let enabled = preferences.is_some_and(|preferences| preferences.enabled);
    let status = if state.last.as_ref().is_some_and(|last| last.last_complete_scan.is_some()) {
        state.sync_label()
    } else {
        "尚未完成同步".into()
    };
    let scanning = column![
        row![
            row![
                text("自动扫描会话日志").size(15),
                tooltip(
                    icon_svg(Icon::QuestionCircle, 14.0),
                    text("扫描当前主机已授权的本地会话日志").size(12),
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
        text(format!("启动时扫描一次，后台每 10 分钟扫描一次。打开用量页时立即扫描，随后按所选间隔扫描。上次同步：{status}"))
            .size(12).style(text::secondary),
        text("Hermes、OpenClaw 暂不支持用量统计").size(12).style(text::secondary),
        row![
            settings_select(
                [5, 10, 30, 60].map(ScanInterval),
                preferences.map(|preferences| ScanInterval(preferences.refresh_seconds)),
                move |interval| Outcome::Event(Event::Run(Action::Save(
                    gateway_usage_scan::UsageScanPreferences { enabled, refresh_seconds: interval.0 },
                ))),
            )
            .placeholder("读取扫描间隔")
            .width(144)
            .disabled(!ready || !enabled),
            Space::new().width(Length::Fill),
            action(if active { "任务进行中…" } else { "立即同步" }, Event::Run(Action::Sync), ready && !active),
        ].spacing(8).align_y(iced::Alignment::Center),
    ].spacing(10);

    let routing = row![
        column![
            text("记录请求用量").size(15),
            text("在当前主机的本地路由设置中查看记录状态。").size(12).style(text::secondary),
        ]
        .spacing(4)
        .width(Length::Fill),
        action("路由设置 ›", Event::Navigate(Navigation::Routing), ready),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center);
    let mut maintenance = column![
        row![
            text("Codex 用量维护").size(15).width(Length::Fill),
            action("重建 Codex 用量", Event::ConfirmRebuild, ready && !active),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
        text(
            "重建会替换当前项目与主机的原生 Codex 用量。主机接口不提供备份凭证，请先自行确认备份。"
        )
        .size(12)
        .style(text::secondary),
    ]
    .spacing(9);
    if state.confirm_rebuild {
        maintenance = maintenance.push(
            card(
                column![
                    text("确认重建 Codex 用量？").size(14),
                    text("重建会重新读取会话记录。关闭设置不会中断已开始的任务。").size(12),
                    row![
                        action("取消", Event::DismissRebuild, true),
                        action("确认重建", Event::Run(Action::Rebuild), ready && !active)
                            .style(primary_action_btn_style),
                    ]
                    .spacing(8),
                ]
                .spacing(8),
            )
            .padding(10),
        );
    }
    if let Some(job) = &state.job {
        // Job payloads may include paths. Show only the task status.
        let phase = job
            .progress
            .get("phase")
            .or_else(|| job.progress.get("status"))
            .and_then(|value| value.as_str())
            .unwrap_or("等待主机反馈");
        maintenance = maintenance.push(text(format!("任务状态  {phase}")).size(12));
        if active {
            maintenance = maintenance.push(
                row![
                    action("取消任务", Event::Run(Action::Cancel), ready),
                    action("刷新进度", Event::Run(Action::Poll), ready),
                ]
                .spacing(8),
            );
        }
    }
    let mut body = column![
        container(scanning).padding(14),
        rule::horizontal(1),
        container(routing).padding(14),
        rule::horizontal(1),
        container(maintenance).padding(14),
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
    let display_settings = action("汇率设置 ›", Event::Navigate(Navigation::Display), true);
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
                    text("按模型设置每百万 Tokens 的价格。")
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
