//! 定时任务投递的宿主配置与已认证通道适配。
use super::DeliveryConfig;
use crate::config::Config;
use anyhow::{Result, bail};
pub(crate) use agent_cron::delivery::{is_wecom, failure_summary};

pub(crate) fn validate(config: &Config, delivery: &DeliveryConfig) -> Result<()> {
    agent_cron::delivery::validate(&super::scheduler::Host, config, delivery)
}
pub(crate) async fn validate_saved(config: &Config, delivery: &DeliveryConfig) -> Result<()> {
    agent_cron::delivery::validate_saved(&super::scheduler::Host, config, delivery).await
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn send(config: &Config, channel: &str, target: &str, output: &str) -> Result<()> {
    agent_cron::delivery::send(&super::scheduler::Host, config, channel, target, output).await
}
pub(crate) async fn apply_result(config: &Config, job: &super::CronJob, success: bool, output: &str) -> (bool, String) {
    agent_cron::delivery::apply_result(&super::scheduler::Host, config, job, success, output).await
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn latest_config(config: &Config) -> Result<Config> {
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

#[cfg(all(test, not(target_arch = "wasm32")))]
tokio::task_local! {
    pub(crate) static TEST_WEBHOOK_ENDPOINT: reqwest::Url;
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn send_validated(config: &Config, channel: &str, target: &str, output: &str) -> Result<()> {
    match channel.to_ascii_lowercase().as_str() {
        "wecom" => {
            let scope = crate::channels::wecom::runtime::WeComRuntimeScope::from_config(
                config,
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
