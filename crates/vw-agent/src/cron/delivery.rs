//! 企业微信定时结果投递：配置预检、即时读取与现有连接复用。
use super::DeliveryConfig;
use crate::config::Config;
use anyhow::{Result, bail};

pub(crate) fn is_wecom(channel: &str) -> bool {
    channel.eq_ignore_ascii_case("wecom") || channel.eq_ignore_ascii_case("wecom_webhook")
}

fn validate_target(delivery: &DeliveryConfig) -> Result<()> {
    let channel = delivery.channel.as_deref().unwrap_or_default();
    let target = delivery.to.as_deref().unwrap_or_default().trim();
    if channel.eq_ignore_ascii_case("wecom") && target.is_empty() {
        bail!("企微投递需要明确的 userid/chatid，请填写会话 ID");
    }
    if channel.eq_ignore_ascii_case("wecom_webhook") && target != "default" {
        bail!("企微群机器人投递目标必须为 default，请在定时任务通知配置中保存群机器人 Webhook");
    }
    Ok(())
}

pub(crate) fn validate(config: &Config, delivery: &DeliveryConfig) -> Result<()> {
    if !delivery.mode.eq_ignore_ascii_case("announce") {
        return Ok(());
    }
    let channel = delivery.channel.as_deref().unwrap_or_default().to_ascii_lowercase();
    if !is_wecom(&channel) {
        return Ok(());
    }
    validate_target(delivery)?;
    if config.channels_config.unavailable_channels.contains(&format!("{channel}:default")) {
        bail!("企微通道密钥不可用，请重新保存通道凭据");
    }
    match channel.as_str() {
        "wecom" => {
            if !config.channels_config.wecom.as_ref().is_some_and(|c| c.has_required_credentials())
            {
                bail!("企微智能机器人未配置，请保存 Bot ID 和 Secret");
            }
        }
        "wecom_webhook" => {
            let Some(group) = &config.channels_config.wecom_webhook else {
                bail!("企微群机器人未配置，请在定时任务通知配置中保存 Webhook");
            };
            #[cfg(not(target_arch = "wasm32"))]
            crate::channels::wecom::webhook::WeComWebhook::new(&group.webhook_url)?;
            #[cfg(target_arch = "wasm32")]
            let _ = group;
        }
        _ => unreachable!(),
    }
    Ok(())
}

/// 与通道监听热重载共用租户、项目覆盖和隔离解密来源，禁止读取失败后回退旧秘密。
#[cfg(not(target_arch = "wasm32"))]
async fn latest_config(config: &Config) -> Result<Config> {
    let mut watcher = crate::config::schema::ChannelConfigWatcher::new(config, true)
        .await
        .map_err(|_| anyhow::anyhow!("企微配置读取失败，请检查配置存储后重试"))?;
    let mut latest = config.clone();
    latest.channels_config = watcher
        .load_for_delivery(config)
        .await
        .map_err(|_| anyhow::anyhow!("企微配置读取失败，请检查配置存储后重试"))?;
    Ok(latest)
}

pub(crate) async fn validate_saved(config: &Config, delivery: &DeliveryConfig) -> Result<()> {
    // 即使停用投递，任务记录也不得携带群机器人的秘密 URL。
    if delivery
        .channel
        .as_deref()
        .is_some_and(|channel| channel.eq_ignore_ascii_case("wecom_webhook"))
        && delivery.to.as_deref().is_some_and(|target| target.trim() != "default")
    {
        validate_target(delivery)?;
    }
    if !delivery.mode.eq_ignore_ascii_case("announce")
        || !delivery.channel.as_deref().is_some_and(is_wecom)
    {
        return Ok(());
    }
    validate_target(delivery)?;
    #[cfg(not(target_arch = "wasm32"))]
    return validate(&latest_config(config).await?, delivery);
    #[cfg(target_arch = "wasm32")]
    validate(config, delivery)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
tokio::task_local! {
    pub(crate) static TEST_WEBHOOK_ENDPOINT: reqwest::Url;
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn send(config: &Config, channel: &str, target: &str, output: &str) -> Result<()> {
    let delivery = DeliveryConfig {
        mode: "announce".into(),
        channel: Some(channel.into()),
        to: Some(target.into()),
        best_effort: false,
    };
    validate_target(&delivery)?;
    let config = latest_config(config).await?;
    validate(&config, &delivery)?;
    match channel.to_ascii_lowercase().as_str() {
        "wecom" => {
            let scope = crate::channels::wecom::runtime::WeComRuntimeScope::from_config(
                &config,
                crate::session::ui_store::current_tenant_scope(),
            )?;
            let credentials = config.channels_config.wecom.as_ref().expect("validated credentials");
            let handle = crate::channels::wecom::runtime::connected_with_credentials(
                &scope,
                &credentials.bot_id,
                &credentials.secret,
            )?;
            handle.send_proactive(target.trim(), output).await
        }
        "wecom_webhook" => {
            let group =
                config.channels_config.wecom_webhook.as_ref().expect("validated credentials");
            let sender = crate::channels::wecom::webhook::WeComWebhook::new(&group.webhook_url)?;
            #[cfg(test)]
            let sender = TEST_WEBHOOK_ENDPOINT
                .try_with(|endpoint| {
                    crate::channels::wecom::webhook::WeComWebhook::new_for_test(
                        endpoint.clone(),
                        std::time::Duration::from_secs(2),
                    )
                })
                .unwrap_or(sender);
            sender.send(output).await
        }
        _ => bail!("unsupported WeCom delivery channel"),
    }
}

/// 只展示本模块明确产生的安全错误；第三方错误不可直接写入任务历史或日志。
pub(crate) fn failure_summary(error: &anyhow::Error) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(progress) =
        error.downcast_ref::<crate::channels::outbound_delivery::ChannelDeliveryError>()
    {
        return format!("企微投递未完成：{progress}；请检查会话权限及连接，回执不明时请勿盲目重发");
    }
    let message = error.to_string();
    if message.starts_with("企微") || message.starts_with("当前进程没有已认证的企微连接")
    {
        message
    } else {
        "企微投递失败，请检查通道凭据、会话权限和连接".into()
    }
}

/// Shell 和 Agent 在执行/入池结束后汇合；投递不在执行重试循环中。
pub(crate) async fn apply_result(
    config: &Config,
    job: &super::CronJob,
    mut success: bool,
    output: &str,
) -> (bool, String) {
    let mut recorded_output = output.to_string();
    // 尝试投递结果到配置的通道
    if let Err(e) = super::scheduler::deliver_if_configured(config, job, output).await {
        // 通道网络错误可能包含带令牌的 URL；历史只保留可公开的配置错误或失败摘要。
        let error = e.to_string();
        let channel = job.delivery.channel.as_deref().unwrap_or_default().to_ascii_lowercase();
        let wecom_summary;
        let summary = if is_wecom(&channel) {
            wecom_summary = failure_summary(&e);
            wecom_summary.as_str()
        } else if error == format!("{channel} channel not configured") {
            error.as_str()
        } else if error.starts_with("channel credentials unavailable;") {
            "channel credentials unavailable; repair channel configuration before delivery"
        } else if error.starts_with("unsupported delivery channel:") {
            "unsupported delivery channel"
        } else {
            "请检查通道凭证、投递目标和网络连接"
        };
        recorded_output.push_str(&format!("\nCron delivery failed: {summary}"));
        if job.delivery.best_effort {
            // best_effort 模式：投递失败不影响整体状态
            tracing::warn!("Cron delivery failed (best_effort): {summary}");
        } else {
            // 非 best_effort 模式：投递失败会导致任务失败
            success = false;
            tracing::warn!("Cron delivery failed: {summary}");
        }
    }

    (success, recorded_output)
}
