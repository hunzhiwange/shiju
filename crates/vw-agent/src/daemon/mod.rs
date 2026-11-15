//! Application composition for daemon services.
use crate::config::Config;
use anyhow::Result;
use tokio_util::sync::CancellationToken;

pub use agent_daemon::state_file_path;
pub(crate) use agent_daemon::wait_for_shutdown_signal;

struct DaemonServices;

#[async_trait::async_trait]
impl agent_daemon::DaemonHost for DaemonServices {
    async fn run_gateway(&self, host: &str, port: u16, config: Config, shutdown: CancellationToken) -> Result<()> {
        crate::gateway::run_gateway_until(host, port, config, async {
            shutdown.cancelled().await;
            Ok(())
        }).await
    }

    async fn start_channels(&self, config: Config) -> Result<()> {
        crate::channels::start_channels(config).await
    }

    async fn run_scheduler(&self, config: Config) -> Result<()> {
        crate::cron::scheduler::run(config).await
    }

    async fn run_agent(&self, config: Config, message: Option<String>, provider: Option<String>, model: Option<String>, temperature: f64) -> Result<String> {
        crate::agent::run(config, message, provider, model, temperature).await
    }

    async fn deliver_announcement(&self, config: &Config, channel: &str, target: &str, announcement: &str) -> Result<()> {
        crate::cron::scheduler::deliver_announcement(config, channel, target, announcement).await
    }
}

pub async fn run(config: Config, host: String, port: u16) -> Result<()> {
    agent_daemon::run(config, host, port, std::sync::Arc::new(DaemonServices)).await
}
