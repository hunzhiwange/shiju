use iced::{
    Alignment, Element,
    widget::{button, column, row, text},
};
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::AuthService,
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryOutcome,
        UsageSnapshotResponse, UsageSource,
    },
};

use super::{Cards, Event, Failure, ListMessage, Message, Scope, Target, message};
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_muted_text_style,
};

pub(crate) fn provider_view<'a>(
    scope: Option<&Scope>,
    cards: &'a Cards,
    app_id: AppId,
    id: &str,
) -> Element<'a, Message> {
    view(scope, cards, Target::Provider(app_id, id.into()))
}

pub(crate) fn account_view<'a>(
    scope: Option<&Scope>,
    cards: &'a Cards,
    service: AuthService,
    id: &str,
) -> Element<'a, Message> {
    view(scope, cards, Target::Account(service, id.into()))
}

fn view<'a>(scope: Option<&Scope>, cards: &'a Cards, target: Target) -> Element<'a, Message> {
    let entry =
        cards.entries.get(&target).filter(|_| scope.is_some() && cards.scope.as_ref() == scope);
    let busy = entry.is_some_and(|entry| entry.refreshing);
    let request = |label: &'static str, event, enabled: bool| {
        button(text(label).size(13))
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(message(ListMessage::Usage(event))))
    };
    let mut header =
        row![text("额度").size(13), request("读取额度", Event::Read(target.clone()), !busy)]
            .spacing(8)
            .align_y(Alignment::Center);
    if let Target::Provider(_, id) = &target {
        header =
            header.push(request("刷新额度", Event::Refresh(target.clone(), id.clone()), !busy));
    }
    let mut body = column![header].spacing(5);
    if entry.is_some_and(|entry| entry.pending.is_some()) {
        body =
            body.push(line(if busy { "正在刷新额度…" } else { "正在读取额度…" }));
    }
    match entry.and_then(|entry| entry.result.as_ref()) {
        Some(Err(error)) => body = body.push(line(error.label())),
        Some(Ok(entries)) if entries.is_empty() => body = body.push(line("此账号暂无绑定供应商")),
        Some(Ok(entries)) => {
            for entry in entries {
                if matches!(target, Target::Account(..)) {
                    body = body.push(
                        row![
                            text(entry.provider.as_str()).size(13),
                            request(
                                "刷新额度",
                                Event::Refresh(target.clone(), entry.provider.clone()),
                                !busy
                            ),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    );
                }
                if let Some(error) = entry.refresh_failure {
                    body = body.push(line(error.label()));
                }
                for value in summary(&entry.snapshot) {
                    body = body.push(line(value));
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

pub(super) fn summary(result: &Result<UsageSnapshotResponse, Failure>) -> Vec<String> {
    let response = match result {
        Ok(response) => response,
        Err(error) => return vec![error.label().into()],
    };
    let Some(snapshot) = &response.snapshot else {
        return vec![
            if response.capabilities.is_empty() {
                "暂无可用额度查询"
            } else {
                "暂无额度快照"
            }
            .into(),
        ];
    };
    let source = match &snapshot.result.source {
        UsageSource::Official(_) => "官方额度",
        UsageSource::Provider(_) => "供应商查询",
        UsageSource::Script(_) => "脚本查询",
        UsageSource::Estimate(_) => "估算",
        UsageSource::Billing(_) => "账单",
    };
    let mut lines = vec![if snapshot.stale {
        format!("{source} · 旧快照，最近刷新失败")
    } else {
        source.into()
    }];
    match &snapshot.result.outcome {
        UsageQueryOutcome::Failure { error, .. } => {
            let label = match snapshot.result.credential_state {
                UsageCredentialState::Expired | UsageCredentialState::RefreshPending => {
                    "账号需要重新授权"
                }
                UsageCredentialState::NotFound => "缺少查询凭据",
                UsageCredentialState::ParseError => "无法读取查询凭据",
                UsageCredentialState::Valid => "额度查询失败",
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
                            remaining("余额", wallet.remaining, Some(&wallet.currency))
                        }));
                        &[]
                    }
                    UsageQueryData::CustomScript(packages) => {
                        lines.extend(packages.iter().map(|package| {
                            let label = package.plan_name.as_deref().unwrap_or("套餐");
                            if package.is_valid == Some(false) {
                                format!("{label} · 失效")
                            } else {
                                remaining(label, package.remaining, package.unit.as_deref())
                            }
                        }));
                        &[]
                    }
                };
            lines.extend(windows.iter().map(|window| {
                if let Some(utilization) = window.utilization {
                    format!(
                        "{} · 剩余 {:.0}%",
                        window.name,
                        (100.0 - f64::from(utilization)).clamp(0.0, 100.0)
                    )
                } else {
                    remaining(&window.name, window.remaining, window.unit.as_deref())
                }
            }));
            if lines.len() == 1 {
                lines.push("暂无额度明细".into());
            }
        }
    }
    lines
}

fn remaining(label: &str, amount: Option<UsageNumber>, unit: Option<&str>) -> String {
    match amount {
        Some(amount) => {
            format!("{label} · 剩余 {} {}", f64::from(amount), unit.unwrap_or("")).trim_end().into()
        }
        None => format!("{label} · 剩余额度未知"),
    }
}
