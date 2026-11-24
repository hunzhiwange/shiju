//! Expand the authorized snapshot in place on provider and account cards.

use chrono::{DateTime, TimeZone, Utc};
use iced::Element;
use iced::widget::{Column, button, column, text};
use vw_api_apps_usage::apps_usage::{
    UsageNumber, UsageQueryData, UsageQueryOutcome, UsageQuotaWindow, UsageScriptPackage,
    UsageSnapshot, UsageSource, UsageSubscription, UsageWallet,
    presentation::{
        credits::{CreditsUsdEstimate, credits_line, reset_credits_line},
        tiers::{QuotaDisplay, tier_lines},
    },
    recharge_detail_url,
};

use desktop_ui_controls::system_settings_common::{rounded_action_btn_style, settings_muted_text_style};
#[derive(Clone, Debug)]
pub enum Message { Toggle, OpenUrl(String) }

pub fn view(i18n: desktop_i18n::I18n,
    snapshot: &UsageSnapshot,
    now: &DateTime<impl TimeZone>,
    expanded: bool,
    toggle: Message,
) -> Element<'static, Message> {
    let label = if expanded { i18n.__("收起额度明细") } else { i18n.__("展开额度明细") };
    let mut body = column![
        button(text(i18n.__(label)).size(13))
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
        UsageSource::Official(source) => (i18n.__("官方额度"), source),
        UsageSource::Provider(source) => (i18n.__("供应商查询"), source),
        UsageSource::Script(source) => (i18n.__("脚本查询"), source),
        UsageSource::Estimate(source) => (i18n.__("估算"), source),
        UsageSource::Billing(source) => (i18n.__("账单"), source),
    };
    let label = i18n.__(label);
    let queried_at = result.queried_at.and_then(|at| DateTime::from_timestamp_millis(at.0));
    body = body.push(line(i18n.format("来源 · %s · %s · 查询时间 %s", &[(label.to_string()).into(), (source.to_string()).into(), (queried_at.map_or_else(|| i18n.__("未知").into(), |at| display_time(at, now)).to_string()).into()]).expect("valid display template")));
    if let Some(failure) = &snapshot.refresh_failure
        && let Some(at) = failure.queried_at.and_then(|at| DateTime::from_timestamp_millis(at.0))
    {
        body = body.push(line(i18n.format("最近刷新失败 · %s", &[(display_time(at, now).to_string()).into()]).expect("valid display template")));
    }

    if let UsageQueryOutcome::Success { data } = &result.outcome {
        body = match data {
            UsageQueryData::OfficialSubscription(subscription) => {
                subscription_rows(i18n, body, subscription, now)
            }
            UsageQueryData::CodingPlan(windows) => window_rows(i18n, body, windows, now),
            UsageQueryData::Balance(wallets) => wallet_rows(i18n,
                body.push(line(i18n.__("余额明细").into())),
                wallets,
                matches!(result.source, UsageSource::Provider(_)),
            ),
            UsageQueryData::CustomScript(packages) => {
                for package in packages {
                    body = body.push(script_package_line(i18n, package));
                }
                body
            }
        };
    }
    body.into()
}

pub(super) fn account_subscription<M: 'static>(i18n: desktop_i18n::I18n,
    subscription: &UsageSubscription,
    now: &DateTime<impl TimeZone>,
) -> Element<'static, M> {
    let body = subscription_rows(i18n, column![].spacing(5), subscription, now);
    if subscription.windows.is_empty()
        && subscription.credits_balance.is_none()
        && subscription.reset_credits.is_none()
    {
        body.push(line(i18n.__("暂无额度明细").into())).into()
    } else {
        body.into()
    }
}

fn subscription_rows<M: 'static>(i18n: desktop_i18n::I18n,
    mut body: Column<'static, M>,
    subscription: &UsageSubscription,
    now: &DateTime<impl TimeZone>,
) -> Column<'static, M> {
    if let Some(tier) = &subscription.rate_limit_tier {
        body = body.push(line(i18n.format("限额档位 · %s", &[(tier.to_string()).into()]).expect("valid display template")));
    }
    body = window_rows(i18n, body, &subscription.windows, now);
    if let Some(credits) = credits_line(subscription.credits_balance) {
        let estimate = match credits.usd_estimate {
            CreditsUsdEstimate::UnderOneDollar => "< $1".into(),
            CreditsUsdEstimate::RoundedDollars(dollars) => format!("${dollars}"),
        };
        body =
            body.push(line(i18n.format("Credits · %s · 估算 %s", &[(format!("{}", f64::from(credits.balance))).into(), (estimate.to_string()).into()]).expect("valid display template")));
    }
    if let Some(credits) = reset_credits_line(subscription.reset_credits.as_deref(), now) {
        body = body.push(line(i18n.format("可用重置 · %u 次", &[credits.count.into()]).expect("valid display template")));
        for group in credits.groups {
            let day = group.expiry_day.map_or_else(|| i18n.__("永不过期").into(), |day| day.to_string());
            body = body.push(line(i18n.format("%s · %u 次", &[day.into(), group.count.into()]).expect("valid display template")));
        }
    }
    if let Some(extra) = &subscription.extra_usage
        && extra.is_enabled
    {
        let mut values = vec![i18n.__("超额用量").into()];
        let unit = extra.currency.as_deref();
        values.extend(amount(i18n.__("已用"), extra.used_credits, unit));
        values.extend(amount(i18n.__("月度上限"), extra.monthly_limit, unit));
        if let Some(utilization) = extra.utilization {
            values.push(i18n.format("已用 %s%%", &[(format!("{}", f64::from(utilization))).into()]).expect("valid display template"));
        }
        body = body.push(line(values.join(" · ")));
    }
    body
}

fn window_rows<M: 'static>(i18n: desktop_i18n::I18n,
    mut body: Column<'static, M>,
    windows: &[UsageQuotaWindow],
    now: &DateTime<impl TimeZone>,
) -> Column<'static, M> {
    for tier in tier_lines(windows, QuotaDisplay::Remaining) {
        let window = tier.window;
        let name = window.plan_label.as_deref().unwrap_or(match window.name.as_str() {
            "five_hour" => i18n.__("5 小时"),
            "seven_day" => i18n.__("每周"),
            "30_day" => i18n.__("30 天"),
            name => name,
        });
        let name = if window.plan_label.is_some() { name } else { i18n.__(name) };
        let mut values = vec![name.into()];
        let unit = window.unit.as_deref();
        if let Some(percent) = tier.bar_percent() {
            values.push(i18n.format("剩余 %s%%", &[(format!("{:.0}", percent)).into()]).expect("valid display template"));
        }
        values.extend(amount(i18n.__("剩余"), window.remaining, unit));
        values.extend(amount(i18n.__("已用"), window.used, unit));
        values.extend(amount(i18n.__("总额"), window.total, unit));
        if let Some(at) = window
            .resets_at
            .as_deref()
            .and_then(|at| at.parse::<DateTime<chrono::FixedOffset>>().ok())
        {
            values.push(i18n.format("重置时间 %s", &[(display_time(at.to_utc(), now).to_string()).into()]).expect("valid display template"));
        }
        body = body.push(line(values.join(" · ")));
    }
    body
}

fn wallet_rows(i18n: desktop_i18n::I18n,
    mut body: Column<'static, Message>,
    wallets: &[UsageWallet],
    allow_link: bool,
) -> Column<'static, Message> {
    let mut links = Vec::new();
    for wallet in wallets {
        let mut values = vec![wallet.currency.clone()];
        let unit = Some(wallet.currency.as_str());
        values.push(amount(i18n.__("剩余"), wallet.remaining, unit).unwrap_or_else(|| i18n.__("余额未知").into()));
        values.extend(amount(i18n.__("总额"), wallet.total, unit));
        values.extend(amount(i18n.__("已用"), wallet.used, unit));
        values.extend(amount(i18n.__("赠金"), wallet.granted, unit));
        values.extend(amount(i18n.__("充值"), wallet.topped_up, unit));
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
            button(text(i18n.__("在浏览器核对额度")).size(13))
                .padding([6, 10])
                .style(rounded_action_btn_style)
                .on_press(Message::OpenUrl(url.into())),
        );
    }
    body
}

pub fn script_package_line(i18n: desktop_i18n::I18n, package: &UsageScriptPackage) -> Element<'static, Message> {
    let mut values = vec![package.plan_name.as_deref().unwrap_or(i18n.__("套餐")).into()];
    if package.is_valid == Some(false) {
        values.push(i18n.__("失效").into());
        values.extend(package.invalid_message.iter().cloned());
    } else {
        let unit = package.unit.as_deref();
        values.extend(amount(i18n.__("剩余"), package.remaining, unit));
        values.extend(amount(i18n.__("已用"), package.used, unit));
        values.extend(amount(i18n.__("总额"), package.total, unit));
        values.extend(package.extra.iter().cloned());
    }
    line(values.join(" · "))
}

fn line<M: 'static>(value: String) -> Element<'static, M> {
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
