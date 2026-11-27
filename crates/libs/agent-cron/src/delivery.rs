//! 企业微信投递目标的纯校验规则。
use crate::DeliveryConfig;
use anyhow::{Result, bail};

pub fn is_wecom(channel: &str) -> bool {
    channel.eq_ignore_ascii_case("wecom")
        || channel.eq_ignore_ascii_case("wecom_webhook")
        || channel.eq_ignore_ascii_case("wecom_cron")
}

pub fn is_task_webhook(target: &str) -> bool {
    target.starts_with("enc2:")
}

pub fn public_target(delivery: &DeliveryConfig) -> Option<&str> {
    let stored = delivery.to.as_deref();
    if delivery.channel.as_deref().is_some_and(|channel| channel.eq_ignore_ascii_case("wecom_cron")) {
        return Some(stored.filter(|target| !target.trim().is_empty())
            .unwrap_or(agent_config::schema::WeComConfig::CRON_INSTANCE));
    }
    let target = stored?;
    if is_task_webhook(target) {
        return Some("configured");
    }
    if delivery.channel.as_deref().is_some_and(|channel| channel.eq_ignore_ascii_case("wecom_webhook")) {
        return None;
    }
    Some(target)
}

#[cfg(not(target_arch = "wasm32"))]
fn webhook_store(config: &Config) -> agent_security::SecretStore {
    agent_security::SecretStore::new(&config.workspace_dir.join("cron"), true)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn protect_task_webhook(host: &dyn DeliveryHost, config: &Config, url: &str) -> Result<String> {
    let url = url.trim();
    if url.is_empty() {
        bail!("请输入有效的企业微信群机器人 Webhook");
    }
    host.validate_webhook(url)
        .map_err(|_| anyhow::anyhow!("请输入有效的企业微信群机器人 Webhook"))?;
    webhook_store(config).encrypt(url)
        .map_err(|_| anyhow::anyhow!("无法加密定时任务的 Webhook"))
}

#[cfg(not(target_arch = "wasm32"))]
fn task_webhook(config: &Config, target: &str) -> Result<String> {
    webhook_store(config).decrypt(target)
        .map_err(|_| anyhow::anyhow!("企微任务 Webhook 无法解密，请重新填写"))
}

fn cron_wecom_instance(target: &str) -> Result<&str> {
    use agent_config::schema::WeComConfig;
    let target = target.trim();
    if target.is_empty() {
        return Ok(WeComConfig::CRON_INSTANCE);
    }
    if !WeComConfig::is_cron_instance(target) {
        bail!("企微通知只能选择一个实例，请重新选择");
    }
    Ok(target)
}

pub fn validate_target(delivery: &DeliveryConfig) -> Result<()> {
    let channel = delivery.channel.as_deref().unwrap_or_default();
    let target = delivery.to.as_deref().unwrap_or_default().trim();
    if channel.eq_ignore_ascii_case("wecom_cron") {
        cron_wecom_instance(target)?;
    }
    if channel.eq_ignore_ascii_case("wecom") && target.is_empty() {
        bail!("企微投递需要明确的 userid/chatid，请填写会话 ID");
    }
    if channel.eq_ignore_ascii_case("wecom_webhook") && !is_task_webhook(target) {
        bail!("企微群机器人需要在当前任务中配置 Webhook");
    }
    Ok(())
}


pub(super) fn validate_stored_target(delivery: &DeliveryConfig) -> Result<()> {
    if delivery.channel.as_deref().is_some_and(|channel| channel.eq_ignore_ascii_case("wecom_cron")) {
        return validate_target(delivery);
    }
    if delivery.channel.as_deref().is_some_and(|channel| channel.eq_ignore_ascii_case("wecom_webhook"))
        && (delivery.mode.eq_ignore_ascii_case("announce") || delivery.to.is_some())
    {
        validate_target(delivery)?;
    }
    Ok(())
}

use agent_config::runtime::Config;
/// 只展示本模块明确产生的安全错误；第三方错误不可直接写入任务历史或日志。
pub fn failure_summary(error: &anyhow::Error) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(progress) =
        error.downcast_ref::<channel_support::outbound_delivery::ChannelDeliveryError>()
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
pub async fn apply_result(
    host: &dyn crate::scheduler::SchedulerHost,
    config: &Config,
    job: &super::CronJob,
    mut success: bool,
    output: &str,
) -> (bool, String) {
    let mut recorded_output = output.to_string();
    // 尝试投递结果到配置的通道
    if let Err(e) = super::scheduler::deliver_if_configured(host, config, job, output).await {
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

pub fn validate(host: &dyn DeliveryHost, config: &Config, delivery: &DeliveryConfig) -> Result<()> {
    if !delivery.mode.eq_ignore_ascii_case("announce") {
        return Ok(());
    }
    match delivery.channel.as_deref().unwrap_or_default().to_ascii_lowercase().as_str() {
        "wecom_cron" => {
            #[cfg(not(target_arch = "wasm32"))]
            validate_target(delivery)?;
            #[cfg(target_arch = "wasm32")]
            bail!("当前平台不支持企微定时任务机器人");
        }
        "wecom_webhook" => {
            validate_target(delivery)?;
            #[cfg(not(target_arch = "wasm32"))]
            {
                let target = delivery.to.as_deref().expect("validated task webhook");
                host.validate_webhook(&task_webhook(config, target)?)?;
            }
            #[cfg(target_arch = "wasm32")]
            bail!("当前平台不支持定时任务 Webhook");
        }
        "wecom" => {
            validate_target(delivery)?;
            if config.channels_config.unavailable_channels.contains("wecom:default") {
                bail!("企微通道密钥不可用，请重新保存通道凭据");
            }
            if !config.channels_config.wecom.as_ref().is_some_and(|c| c.has_required_credentials()) {
                bail!("企微智能机器人未配置，请在通道设置中完成授权");
            }
        }
        _ => {}
    }
    Ok(())
}

pub async fn validate_saved(host: &dyn DeliveryHost, config: &Config, delivery: &DeliveryConfig) -> Result<()> {
    let channel = delivery.channel.as_deref().unwrap_or_default();
    if channel.eq_ignore_ascii_case("wecom_cron") || channel.eq_ignore_ascii_case("wecom_webhook") {
        validate_stored_target(delivery)?;
        return validate(host, config, delivery);
    }
    if !delivery.mode.eq_ignore_ascii_case("announce")
        || !channel.eq_ignore_ascii_case("wecom")
    {
        return Ok(());
    }
    validate_target(delivery)?;
    #[cfg(not(target_arch = "wasm32"))]
    return validate(host, &host.latest_config(config).await?, delivery);
    #[cfg(target_arch = "wasm32")]
    validate(host, config, delivery)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn send(host: &dyn DeliveryHost, config: &Config, channel: &str, target: &str, output: &str) -> Result<()> {
    let delivery = DeliveryConfig { mode: "announce".into(), channel: Some(channel.into()), to: (!target.is_empty()).then(|| target.into()), best_effort: false };
    validate_target(&delivery)?;
    if channel.eq_ignore_ascii_case("wecom_webhook") {
        let url = task_webhook(config, target)?;
        host.validate_webhook(&url)?;
        return host.send_validated(config, channel, &url, output).await;
    }
    let config = host.latest_config(config).await?;
    if channel.eq_ignore_ascii_case("wecom_cron") {
        return host.send_validated(&config, channel, cron_wecom_instance(target)?, output).await;
    }
    validate(host, &config, &delivery)?;
    host.send_validated(&config, channel, target, output).await
}

#[async_trait::async_trait]
pub trait DeliveryHost: Send + Sync {
    #[cfg(not(target_arch = "wasm32"))]
    fn validate_webhook(&self, url: &str) -> Result<()>;
    #[cfg(not(target_arch = "wasm32"))]
    async fn latest_config(&self, config: &Config) -> Result<Config>;
    #[cfg(not(target_arch = "wasm32"))]
    async fn send_validated(&self, config: &Config, channel: &str, target: &str, output: &str) -> Result<()>;
}
