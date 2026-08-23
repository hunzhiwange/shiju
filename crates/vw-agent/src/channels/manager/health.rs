//! 已配置通道的健康检查、超时分类与汇总输出。
//!
//! 初始化失败与运行期探测结果会合并统计；任一通道不健康时整体检查返回错误。

use super::*;

/// 将探测结果归一为健康、不健康或超时状态。
pub(crate) fn classify_health_result(
    result: &std::result::Result<bool, tokio::time::error::Elapsed>,
) -> ChannelHealthState {
    match result {
        Ok(true) => ChannelHealthState::Healthy,
        Ok(false) => ChannelHealthState::Unhealthy,
        Err(_) => ChannelHealthState::Timeout,
    }
}

/// 一轮通道检查的分类计数。
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ChannelDoctorSummary {
    healthy: u32,
    unhealthy: u32,
    timeout: u32,
}

/// 收集当前配置的通道并执行健康检查。
pub async fn doctor_channels(config: Config) -> Result<()> {
    let mut channels = collect_configured_channels(&config, "health check");
    let mut init_failures = Vec::new();

    if let Some(reason) =
        append_nostr_channel_if_available(&config, &mut channels, "health check").await
    {
        init_failures.push(reason);
    }

    run_channel_health_checks(
        channels,
        init_failures,
        config.channels_config.webhook.is_some(),
        Duration::from_secs(10),
    )
    .await?;
    Ok(())
}

/// 并发探测指定通道，在 `health_timeout` 后将未完成项记为超时。
pub(crate) async fn run_channel_health_checks(
    channels: Vec<ConfiguredChannel>,
    init_failures: Vec<String>,
    has_webhook: bool,
    health_timeout: Duration,
) -> Result<ChannelDoctorSummary> {
    if channels.is_empty() && init_failures.is_empty() {
        println!("No real-time channels configured.");
        return Ok(ChannelDoctorSummary::default());
    }

    println!("🩺 VibeWindow Channel Doctor");
    println!();

    let mut summary = ChannelDoctorSummary {
        healthy: 0,
        unhealthy: u32::try_from(init_failures.len()).unwrap_or(u32::MAX),
        timeout: 0,
    };
    let has_runtime_channels = !channels.is_empty();

    for failure in &init_failures {
        println!("  ❌ {:<9} {failure}", "Nostr");
    }

    for configured in channels {
        let result = tokio::time::timeout(health_timeout, configured.channel.health_check()).await;
        let state = classify_health_result(&result);

        match state {
            ChannelHealthState::Healthy => {
                summary.healthy += 1;
                println!("  ✅ {:<9} healthy", configured.display_name);
            }
            ChannelHealthState::Unhealthy => {
                summary.unhealthy += 1;
                println!("  ❌ {:<9} unhealthy (auth/config/network)", configured.display_name);
            }
            ChannelHealthState::Timeout => {
                summary.timeout += 1;
                println!("  ⏱️  {:<9} timed out (>10s)", configured.display_name);
            }
        }
    }

    if has_webhook {
        println!("  ℹ️  Webhook   check via `vibewindow gateway` then GET /health");
    }

    if !has_runtime_channels && !init_failures.is_empty() {
        println!();
        anyhow::bail!("All configured channels failed during initialization.");
    }

    println!();
    println!(
        "Summary: {} healthy, {} unhealthy, {} timed out",
        summary.healthy, summary.unhealthy, summary.timeout
    );
    Ok(summary)
}

#[cfg(test)]
#[path = "health_tests.rs"]
mod health_tests;
