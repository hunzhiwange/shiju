use super::*;
use tokio::{net::TcpListener, time::Duration};

// Snapshot reads are an external port; validation, error summaries and persistence remain real.
struct SavedConfigHost;
#[async_trait::async_trait]
impl crate::delivery::DeliveryHost for SavedConfigHost {
    fn validate_webhook(&self, _: &str) -> anyhow::Result<()> { panic!("unexpected webhook validation") }
    async fn latest_config(&self, config: &Config) -> anyhow::Result<Config> { Ok(config.clone()) }
    async fn send_validated(&self, _: &Config, _: &str, _: &str, _: &str) -> anyhow::Result<()> { panic!("unexpected authenticated send") }
}
#[async_trait::async_trait]
impl crate::scheduler::SchedulerHost for SavedConfigHost {
    async fn run_gateway_agent_job(&self, _: &Config, _: &CronJob, _: &str, _: Option<String>) -> anyhow::Result<String> { panic!("unexpected gateway") }
    async fn deliver_announcement(&self, config: &Config, channel: &str, target: &str, output: &str) -> anyhow::Result<()> {
        crate::announcement::deliver_announcement(self, config, channel, target, output).await
    }
}

#[tokio::test]
async fn persist_job_result_delivery_failure_wecom_respects_best_effort_for_shell_and_agent() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
        for job_type in [JobType::Shell, JobType::Agent] {
            for best_effort in [false, true] {
                let job = cron::add_shell_job(
                    &config,
                    None,
                    Schedule::Every { every_ms: 60_000 },
                    "echo result",
                )
                .unwrap();
                let mut job = job;
                job.job_type = job_type.clone();
                job.delivery = DeliveryConfig {
                    mode: "announce".into(),
                    channel: Some("wecom".into()),
                    to: Some("user".into()),
                    best_effort,
                };
                let now = Utc::now();
                assert_eq!(
                    crate::scheduler::persist_job_result(&SavedConfigHost, &config, &job, true, "result", now, now).await,
                    best_effort
                );
                let runs = cron::list_runs(&config, &job.id, 10).unwrap();
                assert_eq!(runs[0].status, if best_effort { "ok" } else { "error" });
                assert!(runs[0].output.as_deref().unwrap().contains("Bot ID"));
            }
        }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn cron_wecom_delivery_none_makes_zero_webhook_requests() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let job =
        cron::add_shell_job(&config, None, Schedule::Every { every_ms: 60_000 }, "echo result")
            .unwrap();
    let mut job = job;
    job.delivery = DeliveryConfig {
        mode: "none".into(),
        channel: Some("wecom_webhook".into()),
        to: Some("default".into()),
        best_effort: false,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    deliver_if_configured(&config, &job, "result").await.unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(100), listener.accept()).await.is_err());
}


#[test]
fn cron_wecom_delivery_configuration_validation_fails_closed() {
    let mut config = Config::default();
    config.channels_config.wecom = Some(serde_json::from_value(serde_json::json!({"bot_id":"bot","secret":"secret"})).unwrap());
    config.channels_config.wecom_webhook = Some(serde_json::from_value(serde_json::json!({"webhook_url":"https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture-secret"})).unwrap());
    let delivery = DeliveryConfig { mode: "announce".into(), channel: Some("wecom".into()), to: Some("user".into()), best_effort: false };
        for channel in ["wecom", "wecom_webhook"] {
            let mut d = delivery.clone();
            d.channel = Some(channel.into());
            d.to = Some(if channel == "wecom" { "user" } else { "default" }.into());
            let mut broken = config.clone();
            broken.channels_config.unavailable_channels.insert(format!("{channel}:default"));
            assert!(crate::delivery::validate(&TestHost, &broken, &d).is_err());
            d.to = None;
            assert!(crate::delivery::validate(&TestHost, &config, &d).is_err());
            d.mode = "none".into();
            crate::delivery::validate(&TestHost, &broken, &d).unwrap();
        }

}
