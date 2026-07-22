use super::*;
use crate::app::agent::channels::traits::ChannelMessage;
use crate::app::agent::config::schema::{NostrConfig, WebhookConfig};
use std::future::pending;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy)]
enum HealthBehavior {
    Ready(bool),
    Pending,
}

struct HealthChannel {
    name: &'static str,
    behavior: HealthBehavior,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for HealthChannel {
    fn name(&self) -> &str {
        self.name
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn health_check(&self) -> bool {
        match self.behavior {
            HealthBehavior::Ready(value) => value,
            HealthBehavior::Pending => pending::<bool>().await,
        }
    }
}

fn configured_channel(name: &'static str, behavior: HealthBehavior) -> ConfiguredChannel {
    ConfiguredChannel::new(name, Arc::new(HealthChannel { name, behavior }))
}

#[test]
fn classify_health_result_maps_success_failure_and_timeout() {
    assert_eq!(classify_health_result(&Ok(true)), ChannelHealthState::Healthy);
    assert_eq!(classify_health_result(&Ok(false)), ChannelHealthState::Unhealthy);
}

#[tokio::test]
async fn classify_health_result_maps_elapsed_to_timeout() {
    let elapsed = tokio::time::timeout(Duration::from_millis(0), pending::<()>()).await;

    assert_eq!(classify_health_result(&elapsed.map(|_| true)), ChannelHealthState::Timeout);
}

#[tokio::test]
async fn doctor_channels_returns_ok_when_no_realtime_channels_are_configured() {
    doctor_channels(Config::default()).await.expect("empty config should be a no-op");
}

#[tokio::test]
async fn doctor_channels_reports_nostr_initialization_failure() {
    let mut config = Config::default();
    config.channels_config.nostr = Some(NostrConfig {
        private_key: "not-a-valid-key".to_string(),
        relays: Vec::new(),
        allowed_pubkeys: Vec::new(),
    });

    let err = doctor_channels(config).await.expect_err("invalid nostr should fail");

    assert!(err.to_string().contains("All configured channels failed"));
}

#[tokio::test]
async fn run_channel_health_checks_counts_channel_states_and_init_failures() {
    let summary = run_channel_health_checks(
        vec![
            configured_channel("Healthy", HealthBehavior::Ready(true)),
            configured_channel("Unhealthy", HealthBehavior::Ready(false)),
            configured_channel("Timeout", HealthBehavior::Pending),
        ],
        vec!["Nostr init failed during health check: invalid key".to_string()],
        true,
        Duration::from_millis(1),
    )
    .await
    .expect("mixed health report should complete");

    assert_eq!(summary, ChannelDoctorSummary { healthy: 1, unhealthy: 2, timeout: 1 });
}

#[tokio::test]
async fn run_channel_health_checks_fails_when_only_init_failures_exist() {
    let err = run_channel_health_checks(
        Vec::new(),
        vec!["Nostr init failed during health check: invalid key".to_string()],
        false,
        Duration::from_millis(1),
    )
    .await
    .expect_err("init-only failures should fail");

    assert!(err.to_string().contains("All configured channels failed"));
}

#[tokio::test]
async fn doctor_channels_treats_webhook_only_config_as_no_realtime_channels() {
    // Webhook 不是实时运行通道，因此即使配置了 webhook，
    // 没有其它通道与初始化失败时仍应作为无运行时通道的 no-op 返回。
    let mut config = Config::default();
    config.channels_config.webhook = Some(WebhookConfig { port: 8787, secret: None });

    doctor_channels(config).await.expect("webhook-only config should be a no-op");
}

#[tokio::test]
async fn run_channel_health_checks_reports_summary_without_webhook_hint() {
    let summary = run_channel_health_checks(
        vec![configured_channel("Healthy", HealthBehavior::Ready(true))],
        Vec::new(),
        false,
        Duration::from_millis(1),
    )
    .await
    .expect("healthy-only report should complete");

    assert_eq!(summary, ChannelDoctorSummary { healthy: 1, unhealthy: 0, timeout: 0 });
}

#[test]
fn channel_doctor_summary_default_is_all_zero() {
    assert_eq!(
        ChannelDoctorSummary::default(),
        ChannelDoctorSummary { healthy: 0, unhealthy: 0, timeout: 0 }
    );
}
