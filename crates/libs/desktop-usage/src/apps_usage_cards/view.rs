use chrono::{DateTime, Local};
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, text, tooltip},
};
use vw_api_types::{
    agent_apps::AppId,
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryOutcome,
        UsageSnapshotResponse, UsageSource,
    },
};
use vw_api_app_auth::AuthService;

use super::{Cards, Event, Failure, Scope, Target};
#[derive(Clone)]
pub enum Message { Event(Event), OpenSettings(AppId, String), OpenUrl(String) }
use crate::apps_usage_details;
use desktop_assets::assets::Icon;
use desktop_ui_controls::{
    icons::icon_svg,
    system_settings_common::{
        icons::icon_btn_svg_style, rounded_action_btn_style, settings_icon_tile_style,
        settings_muted_text_style,
    },
    tooltip::tooltip_dark_style,
};

pub fn provider_view<'a>(i18n: desktop_i18n::I18n,
    scope: Option<&Scope>,
    cards: &'a Cards,
    app_id: AppId,
    id: &str,
) -> Element<'a, Message> {
    view(i18n, scope, cards, Target::Provider(app_id, id.into()))
}

pub fn account_view<'a>(i18n: desktop_i18n::I18n,
    scope: Option<&Scope>,
    cards: &'a Cards,
    service: AuthService,
    id: &str,
) -> Element<'a, Message> {
    view(i18n, scope, cards, Target::Account(service, id.into()))
}

fn view<'a>(i18n: desktop_i18n::I18n, scope: Option<&Scope>, cards: &'a Cards, target: Target) -> Element<'a, Message> {
    let entry =
        cards.entries.get(&target).filter(|_| scope.is_some() && cards.scope.as_ref() == scope);
    let codex = target.is_codex_account();
    let busy = entry.is_some_and(|entry| entry.pending.is_some());
    let request = |label: &'static str, event, enabled: bool| {
        let label = text(label).size(13);
        let content = if codex {
            row![icon_svg(Icon::ArrowClockwise, 13.0).style(icon_btn_svg_style), label]
                .align_y(Alignment::Center)
                .spacing(6)
                .into()
        } else {
            Element::from(label)
        };
        button(content)
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(Message::Event(event)))
    };
    let label = if codex && entry.is_some_and(|entry| entry.result.is_some()) {
        i18n.__("刷新额度")
    } else {
        i18n.__("读取额度")
    };
    let mut header = row![
        text(i18n.__("额度")).size(13),
        request(label, Event::Read(target.clone()), scope.is_some() && !busy),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if let Target::Provider(_, id) = &target {
        header =
            header.push(request(i18n.__("刷新额度"), Event::Refresh(target.clone(), id.clone()), !busy));
    }
    if let Target::Provider(app, id) = &target {
        header = header.push(
            button(text(i18n.__("查询设置")).size(13))
                .padding([6, 10])
                .style(rounded_action_btn_style)
                .on_press_maybe(scope.is_some().then(|| {
                    Message::OpenSettings(*app, id.clone())
                })),
        );
    }
    let now = Local::now();
    if codex
        && let Some(Ok(entries)) = entry.and_then(|entry| entry.result.as_ref())
        && let Some(snapshot) = entries
            .first()
            .and_then(|entry| entry.snapshot.as_ref().ok())
            .and_then(|response| response.snapshot.as_ref())
    {
        if let Some(queried_at) =
            snapshot.result.queried_at.and_then(|at| DateTime::from_timestamp_millis(at.0))
        {
            let local = queried_at.with_timezone(&now.timezone());
            let stamp = local.format("%F %T %:z").to_string();
            let age = now.signed_duration_since(local);
            let label = match age.num_minutes() {
                ..0 => stamp.clone(),
                0 => i18n.__("刚刚").into(),
                minutes @ 1..60 => desktop_i18n::chat::relative_time(i18n, (1, minutes as u64)),
                60..1440 => desktop_i18n::chat::relative_time(i18n, (2, age.num_hours() as u64)),
                1440..10080 => desktop_i18n::chat::relative_time(i18n, (3, age.num_days() as u64)),
                _ => stamp.clone(),
            };
            let label = i18n.format("查询于 %s", &[label.into()]).expect("valid display template");
            header = header.push(
                tooltip(line(label), text(stamp).size(13), tooltip::Position::Top)
                    .gap(6)
                    .style(tooltip_dark_style),
            );
        }
        if snapshot.stale
            && matches!(
                &snapshot.result.outcome,
                UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(_) },
            )
        {
            header = header.push(line(i18n.__("刷新失败，已有数据可能不是最新状态")));
        }
    }
    let header: Element<'_, Message> = if codex { header.wrap().into() } else { header.into() };
    let mut body = column![header].spacing(if codex { 9 } else { 5 });
    if entry.is_some_and(|entry| entry.pending.is_some()) {
        let refreshing = if codex {
            entry.is_some_and(|entry| entry.result.is_some())
        } else {
            entry.is_some_and(|entry| entry.refreshing.is_some())
        };
        body = body.push(line(if refreshing { i18n.__("正在刷新额度…") } else { i18n.__("正在读取额度…") }));
    }
    match entry.and_then(|entry| entry.result.as_ref().map(|result| (entry, result))) {
        Some((_, Err(error))) => body = body.push(line(i18n.__(error.label()))),
        Some((_, Ok(entries))) if entries.is_empty() => {
            body = body.push(line(i18n.__("此账号暂无绑定供应商")))
        }
        Some((card, Ok(entries))) => {
            for entry in entries {
                if !codex && matches!(target, Target::Account(..)) {
                    body = body.push(
                        row![
                            text(entry.provider()).size(13),
                            request(
                                i18n.__("刷新额度"),
                                Event::Refresh(target.clone(), entry.provider().to_owned()),
                                !busy
                            ),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    );
                }
                if let Some(error) = entry.refresh_failure {
                    body = body.push(line(i18n.__(error.label())));
                }
                if codex
                    && let Ok(response) = &entry.snapshot
                    && let Some(snapshot) = &response.snapshot
                    && let UsageQueryOutcome::Success {
                        data: UsageQueryData::OfficialSubscription(subscription),
                    } = &snapshot.result.outcome
                {
                    body = body.push(apps_usage_details::account_subscription(i18n, subscription, &now));
                    continue;
                }
                for value in summary(i18n, &entry.snapshot) {
                    body = body.push(line(value));
                }
                if let Ok(response) = &entry.snapshot
                    && let Some(snapshot) = &response.snapshot
                {
                    body = body.push(apps_usage_details::view(i18n,
                        snapshot,
                        &now,
                        card.expanded.contains(entry.provider()),
                        apps_usage_details::Message::Toggle,
                    ).map({
                        let target = target.clone();
                        let provider = entry.provider().to_owned();
                        move |event| match event {
                            apps_usage_details::Message::Toggle => Message::Event(Event::ToggleDetails(target.clone(), provider.clone())),
                            apps_usage_details::Message::OpenUrl(url) => Message::OpenUrl(url),
                        }
                    }));
                }
            }
        }
        None => {}
    }
    if codex {
        container(body).padding([9, 12]).width(Length::Fill).style(settings_icon_tile_style).into()
    } else {
        body.into()
    }
}

fn line<'a>(value: impl iced::widget::text::IntoFragment<'a>) -> Element<'a, Message> {
    text(value)
        .size(13)
        .style(settings_muted_text_style)
        .wrapping(text::Wrapping::WordOrGlyph)
        .into()
}

pub(super) fn summary(i18n: desktop_i18n::I18n, result: &Result<UsageSnapshotResponse, Failure>) -> Vec<String> {
    let response = match result {
        Ok(response) => response,
        Err(error) => return vec![i18n.__(error.label()).into()],
    };
    let Some(snapshot) = &response.snapshot else {
        return vec![
            if response.capabilities.is_empty() {
                i18n.__("暂无可用额度查询")
            } else {
                i18n.__("暂无额度快照")
            }
            .into(),
        ];
    };
    let source = match &snapshot.result.source {
        UsageSource::Official(_) => i18n.__("官方额度"),
        UsageSource::Provider(_) => i18n.__("供应商查询"),
        UsageSource::Script(_) => i18n.__("脚本查询"),
        UsageSource::Estimate(_) => i18n.__("估算"),
        UsageSource::Billing(_) => i18n.__("账单"),
    };
    let source = i18n.__(source);
    let mut lines = vec![if snapshot.stale {
        i18n.format("%s · 旧快照，最近刷新失败", &[(source.to_string()).into()]).expect("valid display template")
    } else {
        source.into()
    }];
    match &snapshot.result.outcome {
        UsageQueryOutcome::Failure { error, .. } => {
            let label = match snapshot.result.credential_state {
                UsageCredentialState::Expired | UsageCredentialState::RefreshPending => {
                    i18n.__("账号需要重新授权")
                }
                UsageCredentialState::NotFound => i18n.__("缺少查询凭据"),
                UsageCredentialState::ParseError => i18n.__("无法读取查询凭据"),
                UsageCredentialState::Valid => i18n.__("额度查询失败"),
            };
            lines.push(format!("{label} · {:?}", error.code));
        }
        UsageQueryOutcome::Success { data } => {
            let windows =
                match data {
                    UsageQueryData::OfficialSubscription(subscription) => {
                        lines.extend(subscription.subscription_type.iter().cloned());
                        &subscription.windows[..]
                    }
                    UsageQueryData::CodingPlan(windows) => &windows[..],
                    UsageQueryData::Balance(wallets) => {
                        lines.extend(wallets.iter().map(|wallet| {
                            remaining(i18n, i18n.__("余额"), wallet.remaining, Some(&wallet.currency))
                        }));
                        &[]
                    }
                    UsageQueryData::CustomScript(packages) => {
                        lines.extend(packages.iter().map(|package| {
                            let label = package.plan_name.as_deref().unwrap_or(i18n.__("套餐"));
                            if package.is_valid == Some(false) {
                                i18n.format("%s · 失效", &[(label.to_string()).into()]).expect("valid display template")
                            } else {
                                remaining(i18n, label, package.remaining, package.unit.as_deref())
                            }
                        }));
                        &[]
                    }
                };
            lines.extend(windows.iter().map(|window| {
                if let Some(utilization) = window.utilization {
                    i18n.format("%s · 剩余 %s%%", &[(window.name.to_string()).into(), (format!("{:.0}", (100.0 - f64::from(utilization)).clamp(0.0, 100.0))).into()]).expect("valid display template")
                } else {
                    remaining(i18n, &window.name, window.remaining, window.unit.as_deref())
                }
            }));
            if lines.len() == 1 {
                lines.push(i18n.__("暂无额度明细").into());
            }
        }
    }
    lines
}

fn remaining(i18n: desktop_i18n::I18n, label: &str, amount: Option<UsageNumber>, unit: Option<&str>) -> String {
    match amount {
        Some(amount) => {
            i18n.format("%s · 剩余 %s %s", &[(label.to_string()).into(), (format!("{}", f64::from(amount))).into(), (unit.unwrap_or("").to_string()).into()]).expect("valid display template").trim_end().into()
        }
        None => i18n.format("%s · 剩余额度未知", &[(label.to_string()).into()]).expect("valid display template"),
    }
}
