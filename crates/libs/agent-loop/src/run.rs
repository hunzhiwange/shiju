//! 单次代理运行的配置覆盖、观测与输出编排。
use agent_config::runtime::Config;
use observer_types::{Observer, ObserverEvent};
use anyhow::Result;
use std::time::Instant;

/// 已构造代理的窄运行接口；构造工厂由宿主按原 Config 签名注入。
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait RunAgent {
    fn observer(&self) -> &dyn Observer;
    async fn run_single(&mut self, message: &str) -> Result<String>;
}

pub async fn run_with_agent_factory<A: RunAgent>(
    config: Config,
    message: Option<String>,
    provider_override: Option<String>,
    model_override: Option<String>,
    temperature: f64,
    agent_factory: impl FnOnce(&Config) -> Result<A>,
) -> Result<String> {
    let mut effective_config = config;
    if let Some(p) = provider_override {
        effective_config.default_provider = Some(p);
    }
    if let Some(m) = model_override {
        effective_config.default_model = Some(m);
    }
    effective_config.default_temperature = temperature;

    if message.is_none() {
        anyhow::bail!(
            "Interactive CLI mode moved to vw-cli; provide a message or use the vw-cli binary"
        );
    }

    let start = Instant::now();
    let mut agent = agent_factory(&effective_config)?;

    let provider_name =
        effective_config.default_provider.as_deref().unwrap_or("openrouter").to_string();
    let model_name = effective_config
        .default_model
        .as_deref()
        .unwrap_or("anthropic/claude-sonnet-4-20250514")
        .to_string();

    agent.observer().record_event(&ObserverEvent::AgentStart {
        provider: provider_name.clone(),
        model: model_name.clone(),
    });

    let response = agent.run_single(message.as_deref().expect("checked above")).await?;
    println!("{response}");

    agent.observer().record_event(&ObserverEvent::AgentEnd {
        provider: provider_name,
        model: model_name,
        duration: start.elapsed(),
        tokens_used: None,
        cost_usd: None,
    });

    Ok(response)
}
