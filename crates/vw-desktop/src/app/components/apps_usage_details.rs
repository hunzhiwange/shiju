//! Expand the authorized snapshot in place on provider and account cards.

use chrono::{DateTime, TimeZone, Utc};
use iced::Element;
use iced::widget::{Column, button, column, text};
use vw_gateway_client::vw_api_types::apps_usage::{
    UsageNumber, UsageQueryData, UsageQueryOutcome, UsageQuotaWindow, UsageSnapshot, UsageSource,
    UsageSubscription, UsageWallet,
    presentation::{
        credits::{CreditsUsdEstimate, credits_line, reset_credits_line},
        tiers::{QuotaDisplay, tier_lines},
    },
    recharge_detail_url,
};

use super::system_settings_common::{rounded_action_btn_style, settings_muted_text_style};
use crate::app::{Message, message::ViewMessage};

pub(super) fn view(
    snapshot: &UsageSnapshot,
    now: &DateTime<impl TimeZone>,
    expanded: bool,
    toggle: Message,
) -> Element<'static, Message> {
    let label = if expanded { "收起额度明细" } else { "展开额度明细" };
    let mut body = column![
        button(text(label).size(13))
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press(toggle)
    ]
    .spacing(5);
    if !expanded {
        return body.into();
    }

    let result = &snapshot.result;
    let (label, source) = match &result.source {
        UsageSource::Official(source) => ("官方额度", source),
        UsageSource::Provider(source) => ("供应商查询", source),
        UsageSource::Script(source) => ("脚本查询", source),
        UsageSource::Estimate(source) => ("估算", source),
        UsageSource::Billing(source) => ("账单", source),
    };
    let queried_at = result.queried_at.and_then(|at| DateTime::from_timestamp_millis(at.0));
    body = body.push(line(format!(
        "来源 · {label} · {source} · 查询时间 {}",
        queried_at.map_or_else(|| "未知".into(), |at| display_time(at, now))
    )));
    if let Some(failure) = &snapshot.refresh_failure
        && let Some(at) = failure.queried_at.and_then(|at| DateTime::from_timestamp_millis(at.0))
    {
        body = body.push(line(format!("最近刷新失败 · {}", display_time(at, now))));
    }

    if let UsageQueryOutcome::Success { data } = &result.outcome {
        body = match data {
            UsageQueryData::OfficialSubscription(subscription) => {
                subscription_rows(body, subscription, now)
            }
            UsageQueryData::CodingPlan(windows) => window_rows(body, windows, now),
            UsageQueryData::Balance(wallets) => wallet_rows(
                body.push(line("余额明细".into())),
                wallets,
                matches!(result.source, UsageSource::Provider(_)),
            ),
            UsageQueryData::CustomScript(packages) => {
                for package in packages {
                    let mut values = vec![package.plan_name.as_deref().unwrap_or("套餐").into()];
                    if package.is_valid == Some(false) {
                        values.push("失效".into());
                        values.extend(package.invalid_message.iter().cloned());
                    } else {
                        let unit = package.unit.as_deref();
                        values.extend(amount("剩余", package.remaining, unit));
                        values.extend(amount("已用", package.used, unit));
                        values.extend(amount("总额", package.total, unit));
                        values.extend(package.extra.iter().cloned());
                    }
                    body = body.push(line(values.join(" · ")));
                }
                body
            }
        };
    }
    body.into()
}

fn subscription_rows(
    mut body: Column<'static, Message>,
    subscription: &UsageSubscription,
    now: &DateTime<impl TimeZone>,
) -> Column<'static, Message> {
    if let Some(tier) = &subscription.rate_limit_tier {
        body = body.push(line(format!("限额档位 · {tier}")));
    }
    body = window_rows(body, &subscription.windows, now);
    if let Some(credits) = credits_line(subscription.credits_balance) {
        let estimate = match credits.usd_estimate {
            CreditsUsdEstimate::UnderOneDollar => "< $1".into(),
            CreditsUsdEstimate::RoundedDollars(dollars) => format!("${dollars}"),
        };
        body =
            body.push(line(format!("Credits · {} · 估算 {estimate}", f64::from(credits.balance))));
    }
    if let Some(credits) = reset_credits_line(subscription.reset_credits.as_deref(), now) {
        body = body.push(line(format!("可用重置 · {} 次", credits.count)));
        for group in credits.groups {
            let day = group.expiry_day.map_or_else(|| "永不过期".into(), |day| day.to_string());
            body = body.push(line(format!("{day} · {} 次", group.count)));
        }
    }
    if let Some(extra) = &subscription.extra_usage
        && extra.is_enabled
    {
        let mut values = vec!["超额用量".into()];
        let unit = extra.currency.as_deref();
        values.extend(amount("已用", extra.used_credits, unit));
        values.extend(amount("月度上限", extra.monthly_limit, unit));
        if let Some(utilization) = extra.utilization {
            values.push(format!("已用 {}%", f64::from(utilization)));
        }
        body = body.push(line(values.join(" · ")));
    }
    body
}

fn window_rows(
    mut body: Column<'static, Message>,
    windows: &[UsageQuotaWindow],
    now: &DateTime<impl TimeZone>,
) -> Column<'static, Message> {
    for tier in tier_lines(windows, QuotaDisplay::Remaining) {
        let window = tier.window;
        let mut values = vec![window.plan_label.as_deref().unwrap_or(&window.name).into()];
        let unit = window.unit.as_deref();
        if let Some(percent) = tier.bar_percent() {
            values.push(format!("剩余 {percent:.0}%"));
        }
        values.extend(amount("剩余", window.remaining, unit));
        values.extend(amount("已用", window.used, unit));
        values.extend(amount("总额", window.total, unit));
        if let Some(at) = window
            .resets_at
            .as_deref()
            .and_then(|at| at.parse::<DateTime<chrono::FixedOffset>>().ok())
        {
            values.push(format!("重置时间 {}", display_time(at.to_utc(), now)));
        }
        body = body.push(line(values.join(" · ")));
    }
    body
}

fn wallet_rows(
    mut body: Column<'static, Message>,
    wallets: &[UsageWallet],
    allow_link: bool,
) -> Column<'static, Message> {
    let mut links = Vec::new();
    for wallet in wallets {
        let mut values = vec![wallet.currency.clone()];
        let unit = Some(wallet.currency.as_str());
        values.push(amount("剩余", wallet.remaining, unit).unwrap_or_else(|| "余额未知".into()));
        values.extend(amount("总额", wallet.total, unit));
        values.extend(amount("已用", wallet.used, unit));
        values.extend(amount("赠金", wallet.granted, unit));
        values.extend(amount("充值", wallet.topped_up, unit));
        body = body.push(line(values.join(" · ")));
        if allow_link
            && let Some(url) = wallet.detail_url.as_deref().and_then(recharge_detail_url)
            && !links.contains(&url)
        {
            links.push(url);
        }
    }
    for url in links {
        body = body.push(
            button(text("在浏览器核对额度").size(13))
                .padding([6, 10])
                .style(rounded_action_btn_style)
                .on_press(Message::View(ViewMessage::OpenUrlExternal(url.into()))),
        );
    }
    body
}

fn line(value: String) -> Element<'static, Message> {
    text(value)
        .size(13)
        .style(settings_muted_text_style)
        .wrapping(text::Wrapping::WordOrGlyph)
        .into()
}

fn amount(label: &str, value: Option<UsageNumber>, unit: Option<&str>) -> Option<String> {
    let value = f64::from(value?);
    Some(match unit.filter(|unit| !unit.is_empty()) {
        Some(unit) => format!("{label} {value} {unit}"),
        None => format!("{label} {value}"),
    })
}

// Use the same local time zone for query times, resets, and expiry groups.
fn display_time(at: DateTime<Utc>, now: &DateTime<impl TimeZone>) -> String {
    at.with_timezone(&now.timezone()).fixed_offset().format("%F %T %:z").to_string()
}
