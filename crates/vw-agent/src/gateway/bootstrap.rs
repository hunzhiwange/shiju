use super::{
    AppState, GatewayRateLimiter, IDEMPOTENCY_MAX_KEYS_DEFAULT, IdempotencyStore, MAX_BODY_SIZE,
    RATE_LIMIT_MAX_KEYS_DEFAULT, agent, api, handlers, hash_webhook_secret, health, middleware,
    node_control, normalize_max_keys, openai_compat, request_timeout_middleware,
    skey_auth_middleware, sse, webhook, ws,
};
use crate::app::agent::channels::{
    LinqChannel, NextcloudTalkChannel, QQChannel, WatiChannel, WhatsAppChannel,
};
use crate::app::agent::config::{self, Config};
use crate::app::agent::memory::{self, Memory};
use crate::app::agent::providers::{self, Provider};
use crate::app::agent::runtime as agent_runtime;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::security::pairing::{PairingGuard, is_public_bind};
use crate::app::agent::tools::traits::ToolSpec;
use crate::app::agent::tools::{self, Tool};
use anyhow::{Context, Result};
use axum::Router;
use axum::http::HeaderValue;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{delete, get, post, put};
use parking_lot::Mutex;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use vw_api_types::id::TenantScope;

/// Run the HTTP gateway using axum with proper HTTP/1.1 compliance.
#[allow(clippy::too_many_lines)]
pub async fn run_gateway(host: &str, port: u16, mut config: Config) -> Result<()> {
    if is_public_bind(host) && config.tunnel.provider == "none" && !config.gateway.allow_public_bind
    {
        anyhow::bail!(
            "🛑 Refusing to bind to {host} — gateway would be exposed to the internet.\n\
             Fix: use --host 127.0.0.1 (default), configure a tunnel, or set\n\
             gateway.allow_public_bind = true in system settings (NOT recommended)."
        );
    }
    config.gateway.skeys = config::load_gateway_skeys_for_tenant(&TenantScope::default())
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let config_state = Arc::new(Mutex::new(config.clone()));

    let hooks: Option<std::sync::Arc<crate::app::agent::hooks::HookRunner>> =
        if config.hooks.enabled {
            let mut runner = crate::app::agent::hooks::HookRunner::new();
            crate::plugins::register_trusted_plugin_hooks(&mut runner, &config)
                .context("failed to register trusted plugin hooks")?;
            Some(std::sync::Arc::new(runner))
        } else {
            None
        };

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind gateway on {host}:{port}"))?;
    let actual_port = listener.local_addr()?.port();
    let display_addr = format!("{host}:{actual_port}");
    let cors_whitelist: Arc<Vec<String>> = Arc::new(Vec::new());
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
            middleware::cors_origin_allowed(origin, cors_whitelist.as_ref())
        }))
        .allow_methods(Any)
        .allow_headers(Any);

    let provider: Arc<dyn Provider> = Arc::from(providers::create_resilient_provider_with_options(
        config.default_provider.as_deref().unwrap_or("openrouter"),
        config.api_key.as_deref(),
        config.api_url.as_deref(),
        &config.reliability,
        &providers::ProviderRuntimeOptions {
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
    )?);
    let model = config.default_model.clone().unwrap_or_else(|| "anthropic/claude-sonnet-4".into());
    let temperature = config.default_temperature;
    let mem: Arc<dyn Memory> = Arc::from(memory::create_memory_with_storage(
        &config.memory,
        Some(&config.storage.provider.config),
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);
    let runtime: Arc<dyn agent_runtime::RuntimeAdapter> =
        Arc::from(agent_runtime::create_runtime(&config.runtime)?);
    let security = Arc::new(SecurityPolicy::from_config(&config.autonomy, &config.workspace_dir));

    let (composio_key, composio_entity_id) = if config.composio.enabled {
        (config.composio.api_key.as_deref(), Some(config.composio.entity_id.as_str()))
    } else {
        (None, None)
    };

    let tools_registry_exec: Arc<Vec<Box<dyn Tool>>> = Arc::new(tools::all_tools_with_runtime(
        Arc::new(config.clone()),
        &security,
        runtime,
        Arc::clone(&mem),
        composio_key,
        composio_entity_id,
        &config.browser,
        &config.http_request,
        &config.web_fetch,
        &config.workspace_dir,
        &config.agents,
        config.api_key.as_deref(),
        &config,
        None,
    ));
    let tools_registry: Arc<Vec<ToolSpec>> =
        Arc::new(tools_registry_exec.iter().map(|t| t.spec()).collect());
    let max_tool_iterations = config.agent.max_tool_iterations;
    let multimodal_config = config.multimodal.clone();

    let (event_tx, _event_rx) = tokio::sync::broadcast::channel::<serde_json::Value>(256);
    let webhook_secret_hash: Option<Arc<str>> =
        config.channels_config.webhook.as_ref().and_then(|webhook| {
            webhook.secret.as_ref().and_then(|raw_secret| {
                let trimmed_secret = raw_secret.trim();
                (!trimmed_secret.is_empty())
                    .then(|| Arc::<str>::from(hash_webhook_secret(trimmed_secret)))
            })
        });

    let whatsapp_channel: Option<Arc<WhatsAppChannel>> =
        config.channels_config.whatsapp.as_ref().filter(|wa| wa.is_cloud_config()).map(|wa| {
            Arc::new(WhatsAppChannel::new(
                wa.access_token.clone().unwrap_or_default(),
                wa.phone_number_id.clone().unwrap_or_default(),
                wa.verify_token.clone().unwrap_or_default(),
                wa.allowed_numbers.clone(),
            ))
        });

    let whatsapp_app_secret: Option<Arc<str>> = std::env::var("VIBEWINDOW_WHATSAPP_APP_SECRET")
        .ok()
        .and_then(|secret| {
            let secret = secret.trim();
            (!secret.is_empty()).then(|| secret.to_owned())
        })
        .or_else(|| {
            config.channels_config.whatsapp.as_ref().and_then(|wa| {
                wa.app_secret
                    .as_deref()
                    .map(str::trim)
                    .filter(|secret| !secret.is_empty())
                    .map(ToOwned::to_owned)
            })
        })
        .map(Arc::from);

    let linq_channel: Option<Arc<LinqChannel>> = config.channels_config.linq.as_ref().map(|lq| {
        Arc::new(LinqChannel::new(
            lq.api_token.clone(),
            lq.from_phone.clone(),
            lq.allowed_senders.clone(),
        ))
    });

    let linq_signing_secret: Option<Arc<str>> = std::env::var("VIBEWINDOW_LINQ_SIGNING_SECRET")
        .ok()
        .and_then(|secret| {
            let secret = secret.trim();
            (!secret.is_empty()).then(|| secret.to_owned())
        })
        .or_else(|| {
            config.channels_config.linq.as_ref().and_then(|lq| {
                lq.signing_secret
                    .as_deref()
                    .map(str::trim)
                    .filter(|secret| !secret.is_empty())
                    .map(ToOwned::to_owned)
            })
        })
        .map(Arc::from);

    let wati_channel: Option<Arc<WatiChannel>> =
        config.channels_config.wati.as_ref().map(|wati_cfg| {
            Arc::new(WatiChannel::new(
                wati_cfg.api_token.clone(),
                wati_cfg.api_url.clone(),
                wati_cfg.tenant_id.clone(),
                wati_cfg.allowed_numbers.clone(),
            ))
        });

    let qq_channel: Option<Arc<QQChannel>> = config.channels_config.qq.as_ref().map(|qq_cfg| {
        Arc::new(QQChannel::new(
            qq_cfg.app_id.clone(),
            qq_cfg.app_secret.clone(),
            qq_cfg.allowed_users.clone(),
        ))
    });
    let qq_webhook_enabled = config.channels_config.qq.as_ref().is_some_and(|qq| {
        qq.receive_mode == crate::app::agent::config::schema::QQReceiveMode::Webhook
    });

    let nextcloud_talk_channel: Option<Arc<NextcloudTalkChannel>> =
        config.channels_config.nextcloud_talk.as_ref().map(|nc| {
            Arc::new(NextcloudTalkChannel::new(
                nc.base_url.clone(),
                nc.app_token.clone(),
                nc.allowed_users.clone(),
            ))
        });

    let nextcloud_talk_webhook_secret: Option<Arc<str>> =
        std::env::var("VIBEWINDOW_NEXTCLOUD_TALK_WEBHOOK_SECRET")
            .ok()
            .and_then(|secret| {
                let secret = secret.trim();
                (!secret.is_empty()).then(|| secret.to_owned())
            })
            .or_else(|| {
                config.channels_config.nextcloud_talk.as_ref().and_then(|nc| {
                    nc.webhook_secret
                        .as_deref()
                        .map(str::trim)
                        .filter(|secret| !secret.is_empty())
                        .map(ToOwned::to_owned)
                })
            })
            .map(Arc::from);

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

    let tunnel = crate::app::agent::tunnel::create_tunnel(&config.tunnel)?;
    let mut tunnel_url: Option<String> = None;

    if let Some(ref tun) = tunnel {
        println!("🔗 Starting {} tunnel...", tun.name());
        match tun.start(host, actual_port).await {
            Ok(url) => {
                println!("🌐 Tunnel active: {url}");
                tunnel_url = Some(url);
            }
            Err(e) => {
                println!("⚠️  Tunnel failed to start: {e}");
                println!("   Falling back to local-only mode.");
            }
        }
    }

    println!("🦀 VibeWindow Gateway listening on http://{display_addr}");
    if let Some(ref url) = tunnel_url {
        println!("  🌐 Public URL: {url}");
    }
    println!("  POST /v1/webhook           — {{\"message\": \"your prompt\"}}");
    println!(
        "  POST /v1/agent             — tool-enabled agent chat {{\"message\": \"your prompt\"}}"
    );
    if whatsapp_channel.is_some() {
        println!("  GET  /v1/whatsapp          — Meta webhook verification");
        println!("  POST /v1/whatsapp          — WhatsApp message webhook");
    }
    if linq_channel.is_some() {
        println!("  POST /v1/linq              — Linq message webhook (iMessage/RCS/SMS)");
    }
    if wati_channel.is_some() {
        println!("  GET  /v1/wati              — WATI webhook verification");
        println!("  POST /v1/wati              — WATI message webhook");
    }
    if nextcloud_talk_channel.is_some() {
        println!("  POST /v1/nextcloud-talk    — Nextcloud Talk bot webhook");
    }
    if qq_webhook_enabled {
        println!("  POST /v1/qq                — QQ Bot webhook (validation + events)");
    }
    if config.gateway.node_control.enabled {
        println!("  POST /v1/node-control      — experimental node-control RPC scaffold");
    }
    println!("  POST /v1/chat/completions  — OpenAI-compatible chat");
    println!("  GET  /v1/models            — list available models");
    println!("  GET  /v1/*                 — REST API");
    println!("  GET  /v1/ws/chat           — WebSocket agent chat");
    println!("  GET  /v1/health            — health check");
    println!("  GET  /v1/metrics           — Prometheus metrics");
    if pairing.auth_enabled() {
        println!(
            "  🔒 Gateway auth: ENABLED (Authorization: Bearer <skey> required, active skeys: {})",
            pairing.active_skey_count()
        );
    } else {
        println!("  ⚠️  Gateway auth: DISABLED (all requests accepted)");
    }
    println!("  Press Ctrl+C to stop.\n");

    crate::app::agent::health::mark_component_ok("gateway");

    if let Some(ref hooks) = hooks {
        hooks.fire_gateway_start(host, actual_port).await;
    }

    let broadcast_observer: Arc<dyn crate::app::agent::observability::Observer> =
        Arc::new(sse::BroadcastObserver::new(
            crate::app::agent::observability::create_observer(&config.observability),
            event_tx.clone(),
        ));

    let state = AppState {
        config: config_state,
        provider,
        model,
        temperature,
        mem,
        auto_save: config.memory.auto_save,
        webhook_secret_hash,
        pairing,
        trust_forwarded_headers: config.gateway.trust_forwarded_headers,
        rate_limiter,
        idempotency_store,
        whatsapp: whatsapp_channel,
        whatsapp_app_secret,
        linq: linq_channel,
        linq_signing_secret,
        nextcloud_talk: nextcloud_talk_channel,
        nextcloud_talk_webhook_secret,
        wati: wati_channel,
        qq: qq_channel,
        qq_webhook_enabled,
        observer: broadcast_observer,
        tools_registry,
        tools_registry_exec,
        multimodal: multimodal_config,
        max_tool_iterations,
        event_tx,
        session_query_engines: Default::default(),
    };

    let openai_compat_route = Router::<AppState>::new()
        .route("/chat/completions", post(openai_compat::handle_v1_chat_completions))
        .layer(RequestBodyLimitLayer::new(openai_compat::CHAT_COMPLETIONS_MAX_BODY_SIZE));

    let dashboard_config_route = Router::<AppState>::new()
        .route("/config", get(api::handle_api_config_get).put(api::handle_api_config_put))
        .layer(RequestBodyLimitLayer::new(1_048_576));

    let app = Router::<AppState>::new()
        .merge(external_router())
        .merge(client_router(state.clone(), openai_compat_route, dashboard_config_route))
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE))
        .layer(from_fn(request_timeout_middleware))
        .layer(cors);

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}

pub(super) fn apiql_compat_router(auth_state: AppState) -> Router<AppState> {
    api::handlers::apiql::router::<AppState>()
        // 认证（外层）-> 租户解析 -> ApiQL 业务。
        .layer(from_fn(middleware::tenant_scope_middleware))
        .layer(from_fn_with_state(auth_state, skey_auth_middleware))
}

pub(super) fn external_router() -> Router<AppState> {
    Router::<AppState>::new()
        // 根健康探针不属于客户端 API，不能要求租户 header。
        .route("/health", get(health::handle_health))
        // 第三方 webhook 协议无法控制请求头，因此保持在租户 middleware 之外。
        .route("/v1/webhook", post(webhook::handle_webhook))
        .route("/v1/whatsapp", get(handlers::handle_whatsapp_verify))
        .route("/v1/whatsapp", post(handlers::handle_whatsapp_message))
        .route("/v1/linq", post(handlers::handle_linq_webhook))
        .route("/v1/wati", get(handlers::handle_wati_verify))
        .route("/v1/wati", post(handlers::handle_wati_webhook))
        .route("/v1/nextcloud-talk", post(handlers::handle_nextcloud_talk_webhook))
        .route("/v1/qq", post(handlers::handle_qq_webhook))
}

pub(super) fn client_router(
    state: AppState,
    openai_compat_route: Router<AppState>,
    dashboard_config_route: Router<AppState>,
) -> Router<AppState> {
    let handler_router = stateful_handler_router()
        // 认证（外层）-> 租户解析 -> 受保护的业务 handler。
        .layer(from_fn(middleware::tenant_scope_middleware))
        .layer(from_fn_with_state(state.clone(), skey_auth_middleware));

    let standalone_client_router = Router::<AppState>::new()
        .route("/v1/health", get(health::handle_health))
        .route("/v1/metrics", get(health::handle_metrics))
        .route("/v1/agent", post(agent::handle_agent))
        .route("/v1/models", get(openai_compat::handle_v1_models))
        .nest("/v1", openai_compat_route)
        .route("/v1/ws/chat", get(ws::handle_ws_chat))
        .route("/v1/node-control", post(node_control::handle_node_control))
        .route("/v1/events", get(sse::handle_sse_events))
        .route("/v1/status", get(api::handle_api_status))
        .route("/v1/tools", get(api::handle_api_tools))
        .route("/v1/mcp/inspect", get(api::handle_api_mcp_inspect))
        .route("/v1/cron", get(api::handle_api_cron_list))
        .route("/v1/cron", post(api::handle_api_cron_add))
        .route("/v1/cron/{id}/runs", get(api::handle_api_cron_runs))
        .route("/v1/cron/runs/{id}", get(api::handle_api_cron_runs))
        .route(
            "/v1/cron/{id}",
            delete(api::handle_api_cron_delete)
                .patch(api::handle_api_cron_update)
                .post(api::handle_api_cron_update)
                .put(api::handle_api_cron_update),
        )
        .route("/v1/integrations", get(api::handle_api_integrations))
        .route("/v1/integrations/settings", get(api::handle_api_integrations_settings))
        .route(
            "/v1/integrations/{id}/credentials",
            put(api::handle_api_integration_credentials_put),
        )
        .route("/v1/doctor", get(api::handle_api_doctor).post(api::handle_api_doctor))
        .route("/v1/memory", get(api::handle_api_memory_list))
        .route("/v1/memory", post(api::handle_api_memory_store))
        .route("/v1/memory/{key}", delete(api::handle_api_memory_delete))
        .route("/v1/cli-tools", get(api::handle_api_cli_tools))
        .route("/v1/api-health", get(api::handle_api_health))
        .nest("/v1/dashboard", dashboard_config_route)
        // 未使用 skey 的客户端入口也必须解析租户。
        .layer(from_fn(middleware::tenant_scope_middleware));

    standalone_client_router.merge(apiql_compat_router(state)).nest("/v1", handler_router)
}

pub(super) fn stateful_handler_router() -> Router<AppState> {
    Router::<AppState>::new()
        .merge(api::handlers::auth::router())
        .merge(api::handlers::file::router())
        .merge(api::handlers::git::router())
        .merge(api::handlers::global::stateful_router())
        .merge(api::handlers::instance::router())
        .merge(api::handlers::knowledge::router())
        .merge(api::handlers::misc::router())
        .merge(api::handlers::permission::router())
        .merge(api::handlers::project::router::<AppState>())
        .merge(api::handlers::provider::router())
        .merge(api::handlers::pty::router())
        .merge(api::handlers::question::router())
        .merge(api::handlers::data::router())
        .merge(api::handlers::staff_ipc::router())
        .merge(api::handlers::task_pool::router())
        .merge(api::handlers::task_pool_matrix::router())
        .merge(api::handlers::agent_scheduler::router())
        .merge(api::handlers::apiql::router())
        .merge(api::handlers::task_lifecycle::router())
        .merge(api::handlers::workflow::router())
        .merge(api::handlers::session::router())
        .merge(api::handlers::config::stateful_router())
        .merge(api::handlers::desktop_skills::router())
        .merge(api::handlers::plugins::router())
        .merge(api::handlers::desktop::router())
}

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod bootstrap_tests;
