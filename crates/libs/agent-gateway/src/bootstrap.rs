//! Provider fallback and bind policy used by the gateway host composition.
use agent_config::runtime::Config;
use model_types::Provider;
use std::sync::Arc;
use anyhow::Result;
use gateway_pairing::pairing::is_public_bind;
// Keep management and health endpoints available when the default provider is unusable.
pub fn gateway_provider(config: &Config) -> Arc<dyn Provider> {
    let result = agent_providers::create_resilient_provider_with_options(
        config.default_provider.as_deref().unwrap_or("openrouter"),
        config.api_key.as_deref(),
        config.api_url.as_deref(),
        &config.reliability,
        &agent_providers::ProviderRuntimeOptions {
            configured_provider_ids: config.providers.keys().cloned().collect(),
            auth_profile_override: None,
            provider_api_url: config.api_url.clone(),
            vibewindow_dir: config.config_path.parent().map(std::path::PathBuf::from),
            secrets_encrypt: config.secrets.encrypt,
            reasoning_enabled: config.runtime.reasoning_enabled,
            reasoning_level: config.effective_provider_reasoning_level(),
            custom_provider_api_mode: config.provider_api.map(|mode| mode.as_compatible_mode()),
            max_tokens_override: None,
            model_support_vision: config.model_support_vision,
        },
    );
    match result {
        Ok(provider) => Arc::from(provider),
        Err(_) => {
            // Factory errors may contain configured URLs or credentials; do not log them.
            tracing::warn!("Skipping unavailable default provider; gateway will continue starting");
            Arc::new(UnavailableGatewayProvider)
        }
    }
}

struct UnavailableGatewayProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for UnavailableGatewayProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> Result<String> {
        anyhow::bail!("Default provider is unavailable; update the provider configuration")
    }
}

pub fn validate_bind(host: &str, config: &Config) -> Result<()> {
    if is_public_bind(host) && config.tunnel.provider == "none" && !config.gateway.allow_public_bind
    {
        anyhow::bail!(
            "🛑 Refusing to bind to {host} — gateway would be exposed to the internet.\n\
             Fix: use --host 127.0.0.1 (default), configure a tunnel, or set\n\
             gateway.allow_public_bind = true in system settings (NOT recommended)."
        );
    }
    Ok(())
}
