//! Runtime status and provider listing for the CLI.

use anyhow::Result;
use vw_agent::config::Config;
use vw_agent::config::schema::ChannelsConfigExt;
use vw_agent::{memory, provider::provider};

/// Display runtime status using the calling application's version.
pub fn show_status(config: &Config, version: &str) -> Result<()> {
    println!("ShiJu Status");
    println!();
    println!("Version:     {}", version);
    println!("Workspace:   {}", config.workspace_dir.display());
    println!("Config:      {}", config.config_path.display());
    println!();
    println!(
        "🤖 Provider:      {}",
        config.default_provider.as_deref().unwrap_or("openrouter")
    );
    println!(
        "   Model:         {}",
        config.default_model.as_deref().unwrap_or("(default)")
    );
    println!("📊 Observability:  {}", config.observability.backend);
    println!(
        "🧾 Trace storage:  {} ({})",
        config.observability.runtime_trace_mode, config.observability.runtime_trace_path
    );
    println!("🛡️  Autonomy:      {:?}", config.autonomy.level);
    println!("⚙️  Runtime:       {}", config.runtime.kind);
    // 获取有效的记忆后端名称
    let effective_memory_backend = memory::effective_memory_backend_name(
        &config.memory.backend,
        Some(&config.storage.provider.config),
    );
    println!(
        "💓 Heartbeat:      {}",
        if config.heartbeat.enabled {
            format!("every {}min", config.heartbeat.interval_minutes)
        } else {
            "disabled".into()
        }
    );
    println!(
        "Memory:         {} (auto-save: {})",
        effective_memory_backend,
        if config.memory.auto_save { "on" } else { "off" }
    );

    println!();
    println!("Security:");
    println!("  Workspace only:    {}", config.autonomy.workspace_only);
    println!(
        "  Allowed roots:     {}",
        if config.autonomy.allowed_roots.is_empty() {
            "(none)".to_string()
        } else {
            config.autonomy.allowed_roots.join(", ")
        }
    );
    println!("  Allowed commands:  {}", config.autonomy.allowed_commands.join(", "));
    println!("  Max actions/hour:  {}", config.autonomy.max_actions_per_hour);
    println!(
        "  Max cost/day:      ${:.2}",
        f64::from(config.autonomy.max_cost_per_day_cents) / 100.0
    );
    println!("  OTP enabled:       {}", config.security.otp.enabled);
    println!("  E-stop enabled:    {}", config.security.estop.enabled);
    println!();
    println!("Channels:");
    println!("  CLI:      ✅ always");
    // 显示各个通道的配置状态
    for (channel, configured) in config.channels_config.channels() {
        println!(
            "  {:9} {}",
            channel.name(),
            if configured { "✅ configured" } else { "❌ not configured" }
        );
    }

    Ok(())
}

/// Display available providers and mark the configured provider.
pub async fn show_providers(config: &Config) -> Result<()> {
    // 获取所有可用的提供者列表
    let provider_map = provider::list().await?;
    let mut providers = provider_map.values().collect::<Vec<_>>();
    // 按提供者 ID 排序
    providers.sort_by(|a, b| a.id.cmp(&b.id));
    // 确定当前活动的提供者
    let current = config
        .default_provider
        .as_deref()
        .unwrap_or("openrouter")
        .trim()
        .to_ascii_lowercase();
    println!("Supported providers ({} total):\n", providers.len());
    println!("  ID (use in config)  DESCRIPTION");
    println!("  ─────────────────── ───────────");
    // 显示每个提供者的信息，标记当前活动的提供者
    for p in &providers {
        let is_active = p.id.eq_ignore_ascii_case(&current);
        let marker = if is_active { " (active)" } else { "" };
        println!("  {:<19} {}{}", p.id, p.name, marker);
    }
    // 显示自定义端点选项
    println!("\n  custom:<URL>   Any OpenAI-compatible endpoint");
    println!("  anthropic-custom:<URL>  Any Anthropic-compatible endpoint");
    Ok(())
}
