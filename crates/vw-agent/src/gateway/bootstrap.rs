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
    let security = Arc::new(SecurityPolicy::from_configs(
        &config.autonomy,
        &config.security,
        &config.workspace_dir,
    ));

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

    println!("VibeWindow Gateway listening on http://{display_addr}");
    if let Some(ref url) = tunnel_url {
        println!("  Public URL: {url}");
    }
    for line in gateway_route_summary(GatewayRouteSummaryOptions {
        whatsapp_enabled: whatsapp_channel.is_some(),
        linq_enabled: linq_channel.is_some(),
        wati_enabled: wati_channel.is_some(),
        nextcloud_talk_enabled: nextcloud_talk_channel.is_some(),
        qq_webhook_enabled,
        node_control_enabled: config.gateway.node_control.enabled,
    }) {
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

#[derive(Debug, Clone, Copy)]
pub(super) struct GatewayRouteSummaryOptions {
    pub(super) whatsapp_enabled: bool,
    pub(super) linq_enabled: bool,
    pub(super) wati_enabled: bool,
    pub(super) nextcloud_talk_enabled: bool,
    pub(super) qq_webhook_enabled: bool,
    pub(super) node_control_enabled: bool,
}

pub(super) fn gateway_route_summary(options: GatewayRouteSummaryOptions) -> Vec<String> {
    let routes = gateway_route_entries(options);
    let mut lines = vec![format!("  Gateway routes: {} total", routes.len())];
    lines.push("  Public probes and third-party webhooks (no tenant headers):".to_string());
    lines.extend(
        routes.iter().filter(|route| !route.requires_tenant).map(GatewayRouteEntry::summary_line),
    );
    lines.push("  Client APIs (require X-Platform-ID and X-Company-ID):".to_string());
    lines.extend(
        routes.iter().filter(|route| route.requires_tenant).map(GatewayRouteEntry::summary_line),
    );
    lines
}

#[derive(Debug, Clone, Copy)]
pub(super) struct GatewayRouteEntry {
    pub(super) method: &'static str,
    pub(super) path: &'static str,
    pub(super) description: &'static str,
    pub(super) requires_tenant: bool,
}

impl GatewayRouteEntry {
    fn summary_line(&self) -> String {
        format!("    {:<6} {:<58} — {}", self.method, self.path, self.description)
    }
}

pub(super) fn gateway_route_entries(options: GatewayRouteSummaryOptions) -> Vec<GatewayRouteEntry> {
    let mut routes = Vec::new();
    routes.extend(public_route_entries());
    if options.whatsapp_enabled {
        routes.extend([
            public_route("GET", "/v1/whatsapp", "Meta webhook verification"),
            public_route("POST", "/v1/whatsapp", "WhatsApp message webhook"),
        ]);
    }
    if options.linq_enabled {
        routes.push(public_route("POST", "/v1/linq", "Linq message webhook"));
    }
    if options.wati_enabled {
        routes.extend([
            public_route("GET", "/v1/wati", "WATI webhook verification"),
            public_route("POST", "/v1/wati", "WATI message webhook"),
        ]);
    }
    if options.nextcloud_talk_enabled {
        routes.push(public_route("POST", "/v1/nextcloud-talk", "Nextcloud Talk bot webhook"));
    }
    if options.qq_webhook_enabled {
        routes.push(public_route("POST", "/v1/qq", "QQ Bot webhook"));
    }
    routes.extend(client_route_entries());
    if options.node_control_enabled {
        routes.push(client_route("POST", "/v1/node-control", "node-control RPC"));
    }
    routes
}

const fn public_route(
    method: &'static str,
    path: &'static str,
    description: &'static str,
) -> GatewayRouteEntry {
    GatewayRouteEntry { method, path, description, requires_tenant: false }
}

const fn client_route(
    method: &'static str,
    path: &'static str,
    description: &'static str,
) -> GatewayRouteEntry {
    GatewayRouteEntry { method, path, description, requires_tenant: true }
}

fn public_route_entries() -> Vec<GatewayRouteEntry> {
    vec![
        public_route("GET", "/v1/health", "health probe"),
        public_route("POST", "/v1/webhook", "generic webhook prompt"),
    ]
}

fn client_route_entries() -> Vec<GatewayRouteEntry> {
    vec![
        client_route("GET", "/v1/metrics", "Prometheus metrics"),
        client_route("POST", "/v1/agent", "tool-enabled agent chat"),
        client_route("POST", "/v1/chat/completions", "OpenAI-compatible chat"),
        client_route("GET", "/v1/models", "list available models"),
        client_route("GET", "/v1/ws/chat", "WebSocket agent chat"),
        client_route("GET", "/v1/events", "gateway event stream"),
        client_route("GET", "/v1/status", "runtime status"),
        client_route("GET", "/v1/tools", "tool catalog"),
        client_route("GET", "/v1/mcp/inspect", "MCP inspection"),
        client_route("GET", "/v1/cron", "cron jobs"),
        client_route("POST", "/v1/cron", "create cron job"),
        client_route("GET", "/v1/cron/{id}/runs", "cron job runs"),
        client_route("GET", "/v1/cron/runs/{id}", "cron run detail"),
        client_route("PATCH", "/v1/cron/{id}", "update cron job"),
        client_route("POST", "/v1/cron/{id}", "update cron job"),
        client_route("PUT", "/v1/cron/{id}", "update cron job"),
        client_route("DELETE", "/v1/cron/{id}", "delete cron job"),
        client_route("GET", "/v1/integrations", "integrations"),
        client_route("GET", "/v1/integrations/settings", "integration settings"),
        client_route("PUT", "/v1/integrations/{id}/credentials", "update credentials"),
        client_route("GET", "/v1/doctor", "diagnostics"),
        client_route("POST", "/v1/doctor", "diagnostics"),
        client_route("GET", "/v1/memory", "memory query"),
        client_route("POST", "/v1/memory", "store memory"),
        client_route("DELETE", "/v1/memory/{key}", "delete memory"),
        client_route("GET", "/v1/cli-tools", "CLI tool discovery"),
        client_route("GET", "/v1/api-health", "API health summary"),
        client_route("GET", "/v1/dashboard/config", "dashboard config"),
        client_route("PUT", "/v1/dashboard/config", "update dashboard config"),
        client_route("PUT", "/v1/auth/{provider_id}", "set provider auth"),
        client_route("DELETE", "/v1/auth/{provider_id}", "remove provider auth"),
        client_route("GET", "/v1/config", "runtime config"),
        client_route("PATCH", "/v1/config", "update runtime config"),
        client_route("GET", "/v1/config/providers", "provider config catalog"),
        client_route("GET", "/v1/global/health", "global health"),
        client_route("GET", "/v1/global/event", "global event stream"),
        client_route("GET", "/v1/global/config", "global config"),
        client_route("PATCH", "/v1/global/config", "update global config"),
        client_route("GET", "/v1/global/config/acp", "ACP config"),
        client_route("POST", "/v1/global/dispose", "dispose global instance"),
        client_route("GET", "/v1/path", "workspace path"),
        client_route("GET", "/v1/vcs", "VCS status"),
        client_route("GET", "/v1/event", "instance event stream"),
        client_route("POST", "/v1/instance/dispose", "dispose instance"),
        client_route("GET", "/v1/provider", "provider list"),
        client_route("GET", "/v1/provider/auth", "provider auth methods"),
        client_route("POST", "/v1/provider/{provider_id}/oauth/authorize", "start provider OAuth"),
        client_route("POST", "/v1/provider/{provider_id}/oauth/callback", "finish provider OAuth"),
        client_route("GET", "/v1/doc", "documentation placeholder"),
        client_route("GET", "/v1/command", "command catalog"),
        client_route("GET", "/v1/agent", "agent catalog"),
        client_route("GET", "/v1/skill", "skill catalog"),
        client_route("GET", "/v1/find", "find text"),
        client_route("GET", "/v1/find/file", "find file"),
        client_route("GET", "/v1/find/symbol", "find symbol placeholder"),
        client_route("GET", "/v1/file", "file list"),
        client_route("POST", "/v1/file/read", "read file"),
        client_route("GET", "/v1/file/content", "read file content"),
        client_route("POST", "/v1/file/write", "write file"),
        client_route("GET", "/v1/file/status", "file status"),
        client_route("POST", "/v1/files/move", "move file"),
        client_route("POST", "/v1/files/copy", "copy file"),
        client_route("POST", "/v1/files/delete", "delete files"),
        client_route("POST", "/v1/git/commit", "git commit"),
        client_route("POST", "/v1/git/command", "git command"),
        client_route("POST", "/v1/git/merge", "git merge"),
        client_route("GET", "/v1/permission", "permission requests"),
        client_route("POST", "/v1/permission/{request_id}/reply", "approve permission request"),
        client_route("GET", "/v1/question", "questions"),
        client_route("POST", "/v1/question/{request_id}/reply", "answer question"),
        client_route("POST", "/v1/question/{request_id}/reject", "reject question"),
        client_route("GET", "/v1/pty", "PTY list"),
        client_route("POST", "/v1/pty", "create PTY"),
        client_route("GET", "/v1/pty/{pty_id}", "PTY detail"),
        client_route("PUT", "/v1/pty/{pty_id}", "update PTY"),
        client_route("DELETE", "/v1/pty/{pty_id}", "remove PTY"),
        client_route("GET", "/v1/pty/{pty_id}/connect", "PTY WebSocket"),
        client_route("GET", "/v1/projects", "project list"),
        client_route("POST", "/v1/projects/resolve", "resolve project"),
        client_route("GET", "/v1/projects/{project_id}", "project detail"),
        client_route("PATCH", "/v1/projects/{project_id}", "update project"),
        client_route("GET", "/v1/projects/{project_id}/change-records", "project change records"),
        client_route("GET", "/v1/projects/{project_id}/worktrees", "project worktrees"),
        client_route("POST", "/v1/projects/{project_id}/worktrees", "create worktree"),
        client_route("GET", "/v1/projects/{project_id}/worktrees/{worktree_id}", "worktree detail"),
        client_route(
            "DELETE",
            "/v1/projects/{project_id}/worktrees/{worktree_id}",
            "delete worktree",
        ),
        client_route(
            "POST",
            "/v1/projects/{project_id}/worktrees/{worktree_id}/reset",
            "reset worktree",
        ),
        client_route("GET", "/v1/knowledge/status", "knowledge status"),
        client_route("GET", "/v1/knowledge/embedding/status", "embedding status"),
        client_route("GET", "/v1/knowledge/embedding/presets", "embedding presets"),
        client_route("POST", "/v1/knowledge/embedding/test", "test embedding"),
        client_route("GET", "/v1/knowledge/datasets", "knowledge datasets"),
        client_route("POST", "/v1/knowledge/datasets", "create knowledge dataset"),
        client_route("GET", "/v1/knowledge/datasets/{dataset_id}", "knowledge dataset"),
        client_route("DELETE", "/v1/knowledge/datasets/{dataset_id}", "delete knowledge dataset"),
        client_route("GET", "/v1/knowledge/datasets/{dataset_id}/documents", "knowledge documents"),
        client_route(
            "POST",
            "/v1/knowledge/datasets/{dataset_id}/documents",
            "create knowledge document",
        ),
        client_route(
            "POST",
            "/v1/knowledge/datasets/{dataset_id}/documents/import",
            "import document",
        ),
        client_route(
            "POST",
            "/v1/knowledge/datasets/{dataset_id}/documents/import-async",
            "import document async",
        ),
        client_route("GET", "/v1/knowledge/index-jobs/{job_id}", "knowledge index job"),
        client_route("POST", "/v1/knowledge/documents/batch", "batch update documents"),
        client_route("PATCH", "/v1/knowledge/documents/{document_id}", "update document"),
        client_route("DELETE", "/v1/knowledge/documents/{document_id}", "delete document"),
        client_route("GET", "/v1/knowledge/documents/{document_id}/chunks", "document chunks"),
        client_route("POST", "/v1/knowledge/documents/{document_id}/reindex", "reindex document"),
        client_route("POST", "/v1/knowledge/retrieve", "knowledge retrieve"),
        client_route("POST", "/v1/knowledge/retrieval-test", "knowledge retrieval test"),
        client_route("POST", "/v1/task-pool/worktrees/snapshot", "worktree pool snapshot"),
        client_route("POST", "/v1/task-pool/worktrees/maintain", "maintain worktree pool"),
        client_route("POST", "/v1/task-pool/worktrees/acquire", "acquire worktree"),
        client_route("POST", "/v1/task-pool/worktrees/release", "release worktree"),
        client_route("POST", "/v1/task-pool/worktrees/recycle", "recycle worktree"),
        client_route("POST", "/v1/task-pool/worktrees/slots/recycle", "recycle worktree slots"),
        client_route("POST", "/v1/task-pool/worktrees/merge-lock", "lock worktree merge"),
        client_route("POST", "/v1/task-pool/worktrees/merge-unlock", "unlock worktree merge"),
        client_route("GET", "/v1/task-pool/matrix/events", "Matrix task-pool events"),
        client_route("POST", "/v1/task-pool/matrix/events", "publish Matrix task-pool event"),
        client_route(
            "GET",
            "/v1/agent-scheduler/projects/{project_id}/workflow",
            "scheduler workflow",
        ),
        client_route("GET", "/v1/agent-scheduler/projects/{project_id}/tasks", "scheduler tasks"),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/tasks",
            "create scheduler task",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/commands",
            "scheduler command",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/commands/bulk",
            "bulk scheduler commands",
        ),
        client_route(
            "PATCH",
            "/v1/agent-scheduler/projects/{project_id}/tasks/{task_id}/metadata",
            "update task metadata",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/legacy-import",
            "legacy task import",
        ),
        client_route(
            "GET",
            "/v1/agent-scheduler/projects/{project_id}/workers",
            "scheduler workers",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/workers",
            "upsert scheduler worker",
        ),
        client_route("GET", "/v1/agent-scheduler/projects/{project_id}/teams", "scheduler teams"),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/teams",
            "upsert scheduler team",
        ),
        client_route(
            "GET",
            "/v1/agent-scheduler/projects/{project_id}/tasks/{task_id}/logs",
            "task logs",
        ),
        client_route(
            "GET",
            "/v1/agent-scheduler/projects/{project_id}/tasks/{task_id}/artifacts",
            "task artifacts",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/tasks/{task_id}/runs/{run_id}/worker-events",
            "worker run event",
        ),
        client_route("GET", "/v1/agent-scheduler/projects/{project_id}/events", "scheduler events"),
        client_route(
            "GET",
            "/v1/agent-scheduler/projects/{project_id}/events/stream",
            "scheduler event stream",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/wake",
            "wake scheduler project",
        ),
        client_route(
            "POST",
            "/v1/agent-scheduler/projects/{project_id}/outbox/drain",
            "drain scheduler outbox",
        ),
        client_route("GET", "/v1/staff/stage-capabilities", "staff stage capabilities"),
        client_route(
            "GET",
            "/v1/staff/projects/{project_id}/capabilities",
            "staff project capabilities",
        ),
        client_route(
            "PUT",
            "/v1/staff/projects/{project_id}/capabilities",
            "update staff project capabilities",
        ),
        client_route(
            "GET",
            "/v1/staff/projects/{project_id}/orchestration",
            "staff project orchestration",
        ),
        client_route(
            "PUT",
            "/v1/staff/projects/{project_id}/orchestration",
            "update staff project orchestration",
        ),
        client_route(
            "GET",
            "/v1/staff/projects/{project_id}/workflows/{workflow_id}/capabilities",
            "staff workflow capabilities",
        ),
        client_route(
            "PUT",
            "/v1/staff/projects/{project_id}/workflows/{workflow_id}/capabilities",
            "update staff workflow capabilities",
        ),
        client_route(
            "GET",
            "/v1/staff/projects/{project_id}/workflows/{workflow_id}/orchestration",
            "staff workflow orchestration",
        ),
        client_route(
            "PUT",
            "/v1/staff/projects/{project_id}/workflows/{workflow_id}/orchestration",
            "update staff workflow orchestration",
        ),
        client_route("GET", "/v1/staff/projects/{project_id}/projection", "staff projection"),
        client_route("POST", "/v1/staff/projects/{project_id}/commands", "staff command"),
        client_route(
            "POST",
            "/v1/staff/projects/{project_id}/auto-schedule-plan",
            "staff auto-schedule plan",
        ),
        client_route("POST", "/v1/staff/ipc/messages", "staff IPC message"),
        client_route("GET", "/v1/apiQL/{entity_class}", "APIQL list"),
        client_route("GET", "/v1/apiQL/{entity_class}/", "APIQL list"),
        client_route("GET", "/v1/apiQL/{entity_class}/list-only", "APIQL list rows"),
        client_route("GET", "/v1/apiQL/{entity_class}/list-only/", "APIQL list rows"),
        client_route("GET", "/v1/apiQL/{entity_class}/list-count", "APIQL list count"),
        client_route("GET", "/v1/apiQL/{entity_class}/list-count/", "APIQL list count"),
        client_route("GET", "/v1/apiQL/{entity_class}/list-page", "APIQL list page"),
        client_route("GET", "/v1/apiQL/{entity_class}/list-page/", "APIQL list page"),
        client_route("POST", "/v1/workflow/resume", "resume workflow"),
        client_route("POST", "/v1/workflow/applications/chat-messages", "workflow chat"),
        client_route(
            "POST",
            "/v1/workflow/applications/{uuid}/chat-messages",
            "workflow application chat",
        ),
        client_route("GET", "/v1/workflow/applications", "workflow applications"),
        client_route("POST", "/v1/workflow/applications", "create workflow application"),
        client_route("GET", "/v1/workflow/applications/{uuid}", "workflow application"),
        client_route("PUT", "/v1/workflow/applications/{uuid}", "update workflow application"),
        client_route("DELETE", "/v1/workflow/applications/{uuid}", "delete workflow application"),
        client_route("POST", "/v1/chat/stream", "session chat stream"),
        client_route("POST", "/v1/task-plan-run/stream", "task plan-run stream"),
        client_route("GET", "/v1/session", "sessions"),
        client_route("POST", "/v1/session", "create session"),
        client_route("GET", "/v1/session/status", "session status"),
        client_route("GET", "/v1/session/{session_id}", "session"),
        client_route("PATCH", "/v1/session/{session_id}", "update session"),
        client_route("DELETE", "/v1/session/{session_id}", "delete session"),
        client_route("GET", "/v1/session/{session_id}/children", "session children"),
        client_route("GET", "/v1/session/{session_id}/todo", "session todo"),
        client_route("PUT", "/v1/session/{session_id}/todo", "update session todo"),
        client_route("POST", "/v1/session/{session_id}/fork", "fork session"),
        client_route("POST", "/v1/session/{session_id}/reset", "reset session"),
        client_route("GET", "/v1/session/{session_id}/diff", "session diff"),
        client_route("POST", "/v1/session/{session_id}/summarize", "summarize session"),
        client_route("GET", "/v1/session/{session_id}/message", "session messages"),
        client_route("GET", "/v1/session/{session_id}/message/{message_id}", "session message"),
        client_route(
            "PATCH",
            "/v1/session/{session_id}/message/{message_id}/part/{part_id}",
            "update message part",
        ),
        client_route(
            "DELETE",
            "/v1/session/{session_id}/message/{message_id}/part/{part_id}",
            "delete message part",
        ),
        client_route("GET", "/v1/session/ui-previews", "session UI previews"),
        client_route("GET", "/v1/session/archived", "archived sessions"),
        client_route("PUT", "/v1/session/archived", "update archived sessions"),
        client_route("GET", "/v1/session/scope", "session scope"),
        client_route("PUT", "/v1/session/scope", "update session scope"),
        client_route("GET", "/v1/session/{session_id}/ui", "session UI"),
        client_route("PUT", "/v1/session/{session_id}/ui", "save session UI"),
        client_route("GET", "/v1/session/{session_id}/preview", "session preview"),
        client_route("GET", "/v1/session/{session_id}/path", "session path"),
        client_route("GET", "/v1/session/{session_id}/any", "session by any ID"),
        client_route("POST", "/v1/session/{session_id}/title", "generate session title"),
        client_route("GET", "/v1/data/settings", "data settings"),
        client_route("PUT", "/v1/data/settings", "update data settings"),
        client_route("GET", "/v1/data/connections", "data connections"),
        client_route("POST", "/v1/data/connections", "create data connection"),
        client_route("GET", "/v1/data/connections/{connection_id}", "data connection"),
        client_route("PUT", "/v1/data/connections/{connection_id}", "update data connection"),
        client_route("DELETE", "/v1/data/connections/{connection_id}", "delete data connection"),
        client_route(
            "POST",
            "/v1/data/connections/{connection_id}/activate",
            "activate data connection",
        ),
        client_route("POST", "/v1/data/connections/{connection_id}/test", "test data connection"),
        client_route(
            "GET",
            "/v1/data/connections/{connection_id}/catalog",
            "data connection catalog",
        ),
        client_route("GET", "/v1/data/reports", "data reports"),
        client_route("POST", "/v1/data/reports", "create data report"),
        client_route("GET", "/v1/data/reports/{report_id}", "data report"),
        client_route("PUT", "/v1/data/reports/{report_id}", "update data report"),
        client_route("DELETE", "/v1/data/reports/{report_id}", "delete data report"),
        client_route("POST", "/v1/data/query", "data query"),
        client_route("POST", "/v1/data/ai/query", "AI data query"),
        client_route("GET", "/v1/task-lifecycle/workflows", "task lifecycle workflows"),
        client_route("POST", "/v1/task-lifecycle/workflows", "create task lifecycle workflow"),
        client_route("GET", "/v1/task-lifecycle/workflows/{id}", "task lifecycle workflow"),
        client_route("PUT", "/v1/task-lifecycle/workflows/{id}", "update task lifecycle workflow"),
        client_route(
            "DELETE",
            "/v1/task-lifecycle/workflows/{id}",
            "delete task lifecycle workflow",
        ),
        client_route(
            "POST",
            "/v1/task-lifecycle/workflows/{id}/reset",
            "reset task lifecycle workflow",
        ),
        client_route("POST", "/v1/shiju/account/login", "start SHIJU account login"),
        client_route("POST", "/v1/shiju/account/login/poll", "poll SHIJU account login"),
        client_route("GET", "/v1/shiju/account/profile", "SHIJU account profile"),
        client_route("GET", "/v1/shiju/apps/catalog", "SHIJU application catalog"),
        client_route("GET", "/v1/shiju/apps", "installed SHIJU applications"),
        client_route(
            "POST",
            "/v1/shiju/apps/{installation_id}/launch",
            "prepare SHIJU application launch",
        ),
        client_route("GET", "/v1/shiju/skills/search", "search SHIJU skills"),
        client_route("GET", "/v1/shiju/skills/installed", "installed SHIJU skills"),
        client_route("GET", "/v1/shiju/skills/system", "SHIJU system skills"),
        client_route("GET", "/v1/shiju/skills/company", "SHIJU company skills"),
        client_route("GET", "/v1/shiju/skills/maintained", "maintained SHIJU skills"),
        client_route("GET", "/v1/shiju/skills/{skill_id}", "SHIJU skill detail"),
        client_route("GET", "/v1/shiju/skills/{skill_id}/versions", "SHIJU skill versions"),
        client_route("POST", "/v1/shiju/skills/{skill_id}/install", "install SHIJU skill"),
        client_route("DELETE", "/v1/shiju/skills/{skill_id}/install", "uninstall SHIJU skill"),
        client_route("GET", "/v1/skill-market/sources", "skill market sources"),
        client_route("GET", "/v1/skill-market/showcase/recommended", "recommended market skills"),
        client_route("GET", "/v1/skill-market/skills", "market skill catalog"),
        client_route("GET", "/v1/skill-market/categories", "market skill categories"),
        client_route("GET", "/v1/skill-market/skills/{slug}", "market skill detail"),
        client_route("GET", "/v1/skill-market/skills/{slug}/file", "market skill file"),
        client_route("GET", "/v1/skill-market/skillsets", "market skill-set catalog"),
        client_route("GET", "/v1/skill-market/skillsets/{slug}", "market skill-set detail"),
        client_route("GET", "/v1/skills", "skills catalog"),
        client_route("GET", "/v1/skills/detail", "skill detail"),
        client_route("POST", "/v1/skills/create", "create skill"),
        client_route("POST", "/v1/skills/install-built-in", "install built-in skill"),
        client_route("POST", "/v1/skills/set-enabled", "set skill enabled"),
        client_route("POST", "/v1/skills/delete", "delete skill"),
        client_route("GET", "/v1/plugins", "plugins catalog"),
        client_route("GET", "/v1/plugins/detail", "plugin detail"),
        client_route("POST", "/v1/plugins/set-enabled", "set plugin enabled"),
        client_route("POST", "/v1/plugins/delete", "delete plugin"),
        client_route("GET", "/v1/desktop/preferences", "desktop preferences"),
        client_route("PATCH", "/v1/desktop/preferences", "update desktop preferences"),
        client_route("GET", "/v1/desktop/tool-content/{tool_type}", "desktop tool content"),
        client_route("PUT", "/v1/desktop/tool-content/{tool_type}", "update desktop tool content"),
        client_route("GET", "/v1/desktop/skills", "desktop skills catalog"),
        client_route("GET", "/v1/desktop/skills/detail", "desktop skill detail"),
        client_route("POST", "/v1/desktop/skills/create", "create desktop skill"),
        client_route(
            "POST",
            "/v1/desktop/skills/install-built-in",
            "install desktop built-in skill",
        ),
        client_route("POST", "/v1/desktop/skills/set-enabled", "set desktop skill enabled"),
        client_route("POST", "/v1/desktop/skills/delete", "delete desktop skill"),
        client_route("GET", "/v1/desktop/external-apps", "external apps"),
        client_route("POST", "/v1/desktop/external-apps/open", "open external app"),
        client_route("POST", "/v1/desktop/external-path/reveal", "reveal external path"),
        client_route("POST", "/v1/desktop/service/{command}", "desktop service command"),
        client_route("GET", "/v1/desktop/project-preferences", "desktop project preferences"),
        client_route(
            "PUT",
            "/v1/desktop/project-preferences",
            "update desktop project preferences",
        ),
    ]
}

pub(super) fn external_router() -> Router<AppState> {
    Router::<AppState>::new()
        // 健康探针不属于受管客户端 API，不能要求租户 header。
        .route("/v1/health", get(health::handle_health))
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

    standalone_client_router.nest("/v1", handler_router)
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
        .merge(api::handlers::task_plan_run::router())
        .merge(api::handlers::workflow::router())
        .merge(api::handlers::session::router())
        .merge(api::handlers::config::stateful_router())
        .merge(api::handlers::shiju_account::router())
        .merge(api::handlers::shiju_apps::router())
        .merge(api::handlers::shiju_skills::router())
        .merge(api::handlers::skill_market::router())
        .merge(api::handlers::desktop_skills::router())
        .merge(api::handlers::plugins::router())
        .merge(api::handlers::desktop::router())
}

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod bootstrap_tests;
