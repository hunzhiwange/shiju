//! Expand the authorized snapshot in place on provider and account cards.

use chrono::{DateTime, TimeZone, Utc};
use iced::widget::{Column, Row, button, column, progress_bar, row, text, tooltip};
use iced::{Alignment, Element, Font, Length, font};
use vw_api_apps_usage::apps_usage::{
    UsageNumber, UsageQueryData, UsageQueryOutcome, UsageQuotaWindow, UsageScriptPackage,
    UsageSnapshot, UsageSource, UsageSubscription, UsageWallet,
    presentation::{
        credits::{CreditsUsdEstimate, credits_line, reset_credits_line},
        tiers::{QuotaDisplay, QuotaTone, tier_lines},
    },
    recharge_detail_url,
};

use desktop_ui_controls::system_settings_common::{rounded_action_btn_style, settings_muted_text_style};
use desktop_ui_controls::tooltip::tooltip_dark_style;
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
    let mut metrics = Vec::new();
    if let Some(tier) = &subscription.rate_limit_tier {
        metrics.push(account_metric(i18n.__("限额档位"), row![metric_value(tier.clone())]));
    }
    for tier in tier_lines(&subscription.windows, QuotaDisplay::Remaining) {
        let window = tier.window;
        let mut values = row![];
        if let Some(percent) = tier.bar_percent() {
            let style = match tier.tone {
                Some(QuotaTone::Danger) => progress_bar::danger,
                Some(QuotaTone::Warning) => progress_bar::warning,
                _ => progress_bar::primary,
            };
            values = values
                .push(progress_bar(0.0..=100.0, percent as f32).length(104).girth(4).style(style))
                .push(metric_value(
                    i18n.format("剩余 %s%%", &[format!("{percent:.0}").into()])
                        .expect("valid display template"),
                ));
        }
        let amounts = window_amounts(i18n, window);
        if !amounts.is_empty() {
            values = values.push(metric_value(amounts.join(" · ")));
        } else if tier.bar_percent().is_none() {
            values = values.push(line(i18n.__("剩余额度未知").into()));
        }
        if let Some(at) = window_reset(window) {
            let stamp = display_time(at, now);
            let exact =
                i18n.format("重置时间 %s", &[stamp.into()]).expect("valid display template");
            values = values.push(match reset_countdown(i18n, at, now) {
                Some(countdown) => {
                    tooltip(line(countdown), text(exact).size(13), tooltip::Position::Top)
                        .gap(6)
                        .style(tooltip_dark_style)
                        .into()
                }
                None => line(exact),
            });
        }
        metrics.push(account_metric(window_name(i18n, window), values));
    }
    if let Some(expiries) = &subscription.reset_credits {
        let available = reset_credits_line(Some(expiries), now);
        let count = available.as_ref().map_or(0, |credits| credits.count);
        let mut values = row![metric_value(
            i18n.format("剩余 %u 次", &[count.into()]).expect("valid display template")
        )];
        if let Some(credits) = available {
            let multiple = credits.groups.len() > 1;
            for group in credits.groups {
                let expiry = group.expiry_day.map_or_else(
                    || i18n.__("永不过期").into(),
                    |day| {
                        i18n.format("%s 到期", &[day.to_string().into()])
                            .expect("valid display template")
                    },
                );
                let detail = if multiple {
                    i18n.format("%s · %u 次", &[expiry.into(), group.count.into()])
                        .expect("valid display template")
                } else {
                    expiry
                };
                values = values.push(if group.tone == QuotaTone::Warning {
                    metric_value(detail)
                } else {
                    line(detail)
                });
            }
        }
        metrics.push(account_metric(i18n.__("额度重置"), values));
    }
    if let Some(credits) = credits_line(subscription.credits_balance) {
        let estimate = i18n
            .format("估算 %s", &[credits_estimate(credits.usd_estimate).into()])
            .expect("valid display template");
        metrics.push(account_metric(
            "Credits",
            row![metric_value(credit_balance(f64::from(credits.balance))), line(estimate),],
        ));
    }
    if let Some(extra) = extra_usage(i18n, subscription) {
        metrics.push(line(extra));
    }
    if metrics.is_empty() {
        metrics.push(line(i18n.__("暂无额度明细").into()));
    }
    column(metrics).spacing(7).width(Length::Fill).into()
}

fn account_metric<M: 'static>(
    label: impl Into<String>,
    values: Row<'static, M>,
) -> Element<'static, M> {
    row![
        text(label.into())
            .size(13)
            .width(104)
            .wrapping(text::Wrapping::WordOrGlyph)
            .style(settings_muted_text_style),
        values
            .width(Length::Fill)
            .spacing(10)
            .align_y(Alignment::Center)
            .wrap()
            .vertical_spacing(4),
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}

fn metric_value<M: 'static>(value: String) -> Element<'static, M> {
    text(value)
        .size(13)
        .font(Font { weight: font::Weight::Semibold, ..Font::DEFAULT })
        .wrapping(text::Wrapping::WordOrGlyph)
        .into()
}

fn credit_balance(balance: f64) -> String {
    let mut value = format!("{balance:.2}");
    let whole_digits = value.len() - 3;
    for position in (1..whole_digits).rev().skip(2).step_by(3) {
        value.insert(position, ',');
    }
    value
}

fn reset_countdown(
    i18n: desktop_i18n::I18n,
    at: DateTime<Utc>,
    now: &DateTime<impl TimeZone>,
) -> Option<String> {
    let remaining = at.signed_duration_since(now);
    if remaining <= chrono::TimeDelta::zero() {
        return None;
    }
    let hours = remaining.num_hours() as u64;
    let minutes = remaining.num_minutes() as u64;
    let label = match (hours, minutes) {
        (24.., _) => {
            i18n.format("%u 天 %u 小时后重置", &[(hours / 24).into(), (hours % 24).into()])
        }
        (1.., _) => i18n.format("%u 小时后重置", &[hours.into()]),
        (_, 1..) => i18n.format("%u 分钟后重置", &[minutes.into()]),
        _ => return Some(i18n.__("即将重置").into()),
    };
    Some(label.expect("valid display template"))
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
        let estimate = credits_estimate(credits.usd_estimate);
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
    if let Some(extra) = extra_usage(i18n, subscription) {
        body = body.push(line(extra));
    }
    body
}

fn credits_estimate(estimate: CreditsUsdEstimate) -> String {
    match estimate {
        CreditsUsdEstimate::UnderOneDollar => "< $1".into(),
        CreditsUsdEstimate::RoundedDollars(dollars) => format!("${dollars}"),
    }
}

fn extra_usage(i18n: desktop_i18n::I18n, subscription: &UsageSubscription) -> Option<String> {
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
        return Some(values.join(" · "));
    }
    None
}

fn window_rows<M: 'static>(i18n: desktop_i18n::I18n,
    mut body: Column<'static, M>,
    windows: &[UsageQuotaWindow],
    now: &DateTime<impl TimeZone>,
) -> Column<'static, M> {
    for tier in tier_lines(windows, QuotaDisplay::Remaining) {
        let window = tier.window;
        let mut values = vec![window_name(i18n, window).into()];
        if let Some(percent) = tier.bar_percent() {
            values.push(i18n.format("剩余 %s%%", &[(format!("{:.0}", percent)).into()]).expect("valid display template"));
        }
        values.extend(window_amounts(i18n, window));
        if let Some(at) = window_reset(window) {
            values.push(
                i18n.format("重置时间 %s", &[display_time(at, now).into()])
                    .expect("valid display template"),
            );
        }
        body = body.push(line(values.join(" · ")));
    }
    body
}

fn window_name<'a>(i18n: desktop_i18n::I18n, window: &'a UsageQuotaWindow) -> &'a str {
    window.plan_label.as_deref().unwrap_or_else(|| {
        i18n.__(match window.name.as_str() {
            "five_hour" => "5 小时",
            "seven_day" => "每周",
            "30_day" => "30 天",
            name => name,
        })
    })
}

fn window_amounts(i18n: desktop_i18n::I18n, window: &UsageQuotaWindow) -> Vec<String> {
    let unit = window.unit.as_deref();
    [
        amount(i18n.__("剩余"), window.remaining, unit),
        amount(i18n.__("已用"), window.used, unit),
        amount(i18n.__("总额"), window.total, unit),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn window_reset(window: &UsageQuotaWindow) -> Option<DateTime<Utc>> {
    let at = window.resets_at.as_deref()?.parse::<DateTime<chrono::FixedOffset>>().ok()?;
    Some(at.to_utc())
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
