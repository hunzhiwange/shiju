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

use gateway_types::{GatewayRateLimiter, IdempotencyStore, RATE_LIMIT_MAX_KEYS_DEFAULT, IDEMPOTENCY_MAX_KEYS_DEFAULT};
use gateway_pairing::pairing::PairingGuard;
use crate::util::normalize_max_keys;
use std::time::Duration;
use crate::route_catalog::{GatewayRouteSummaryOptions, gateway_route_summary};
pub struct GatewayAdmission {
    pub pairing: Arc<PairingGuard>,
    pub rate_limiter: Arc<GatewayRateLimiter>,
    pub idempotency_store: Arc<IdempotencyStore>,
}
pub fn gateway_admission(config: &Config) -> GatewayAdmission {
    let gateway_skeys = config.gateway.skeys.clone();
    let pairing = Arc::new(PairingGuard::from_skeys(config.gateway.auth_enabled, &gateway_skeys));
    let rate_limit_max_keys =
        normalize_max_keys(config.gateway.rate_limit_max_keys, RATE_LIMIT_MAX_KEYS_DEFAULT);
    let rate_limiter = Arc::new(GatewayRateLimiter::new(
        config.gateway.pair_rate_limit_per_minute,
        config.gateway.webhook_rate_limit_per_minute,
        rate_limit_max_keys,
    ));
    let idempotency_max_keys =
        normalize_max_keys(config.gateway.idempotency_max_keys, IDEMPOTENCY_MAX_KEYS_DEFAULT);
    let idempotency_store = Arc::new(IdempotencyStore::new(
        Duration::from_secs(config.gateway.idempotency_ttl_secs.max(1)),
        idempotency_max_keys,
    ));

    GatewayAdmission { pairing, rate_limiter, idempotency_store }
}
pub fn gateway_cors() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{AllowOrigin, Any, CorsLayer};
    use axum::http::HeaderValue;
    let cors_whitelist: Arc<Vec<String>> = Arc::new(Vec::new());
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
            crate::middleware::cors_origin_allowed(origin, cors_whitelist.as_ref())
        }))
        .allow_methods(Any)
        .allow_headers(Any);

    cors
}
pub fn print_gateway_startup(display_addr: &str, tunnel_url: Option<&str>, pairing: &PairingGuard, options: GatewayRouteSummaryOptions) {
    println!("VibeWindow Gateway listening on http://{display_addr}");
    if let Some(ref url) = tunnel_url {
        println!("  Public URL: {url}");
    }
    for line in gateway_route_summary(options) {
        println!("{line}");
    }
    if pairing.auth_enabled() {
        println!(
            "  🔒 Gateway auth: ENABLED (Authorization: Bearer <skey> required, active skeys: {})",
            pairing.active_skey_count()
        );
    } else {
        println!("  Gateway auth: DISABLED (all requests accepted)");
    }
    println!("  Press Ctrl+C to stop.\n");

}
