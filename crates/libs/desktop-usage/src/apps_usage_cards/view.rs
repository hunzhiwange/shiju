use chrono::Local;
use iced::{
    Alignment, Element,
    widget::{button, column, row, text},
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
use desktop_ui_controls::system_settings_common::{rounded_action_btn_style, settings_muted_text_style};

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
    let busy = entry.is_some_and(|entry| entry.pending.is_some())
        || scope.is_some_and(|scope| cards.batch_pending(scope));
    let request = |label: &'static str, event, enabled: bool| {
        button(text(i18n.__(label)).size(13))
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
    let mut body = column![header].spacing(5);
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
            let now = Local::now();
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
    body.into()
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
        i18n.format("%s · 旧快照，最近刷新失败", &[(format!("{}", source)).into()]).expect("valid display template")
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
                                i18n.format("%s · 失效", &[(format!("{}", label)).into()]).expect("valid display template")
                            } else {
                                remaining(i18n, label, package.remaining, package.unit.as_deref())
                            }
                        }));
                        &[]
                    }
                };
            lines.extend(windows.iter().map(|window| {
                if let Some(utilization) = window.utilization {
                    i18n.format("%s · 剩余 %s%%", &[(format!("{}", window.name)).into(), (format!("{:.0}", (100.0 - f64::from(utilization)).clamp(0.0, 100.0))).into()]).expect("valid display template")
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
            i18n.format("%s · 剩余 %s %s", &[(format!("{}", label)).into(), (format!("{}", f64::from(amount))).into(), (format!("{}", unit.unwrap_or(""))).into()]).expect("valid display template").trim_end().into()
        }
        None => i18n.format("%s · 剩余额度未知", &[(format!("{}", label)).into()]).expect("valid display template"),
    }
}
