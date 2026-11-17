use super::{
    AppState, MAX_BODY_SIZE,
    agent, api, handlers, health, middleware, node_control, openai_compat, request_timeout_middleware, skey_auth_middleware, sse,
    webhook, ws,
};
use crate::app::agent::config::{self, Config};
use crate::app::agent::memory::{self, Memory};
use crate::app::agent::runtime as agent_runtime;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::traits::ToolSpec;
use crate::app::agent::tools::{self, Tool};
use anyhow::{Context, Result};
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{delete, get, post, put};
use parking_lot::Mutex;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::limit::RequestBodyLimitLayer;
use vw_api_transport::id::TenantScope;

use agent_gateway::bootstrap::gateway_provider;

/// Run the HTTP gateway using axum with proper HTTP/1.1 compliance.
pub async fn run_gateway(host: &str, port: u16, config: Config) -> Result<()> {
    run_gateway_until(host, port, config, async {
        crate::daemon::wait_for_shutdown_signal().await.map(|_| ())
    })
    .await
}

pub(crate) async fn run_gateway_until(
    host: &str,
    port: u16,
    config: Config,
    shutdown: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
    let sync = crate::device_sync::lifecycle::SyncLifecycle::default();
    let result = crate::app_routing::gateway_owner::run_with_routing(
        // The gateway startup future contains large config and router state.
        async |routing| Box::pin(serve_gateway(host, port, config, sync.clone(), routing)).await,
        shutdown,
    )
    .await;
    sync.shutdown().await.map_err(|_| anyhow::anyhow!("device sync shutdown failed"))?;
    result
}

#[allow(clippy::too_many_lines)]
async fn serve_gateway(
    host: &str,
    port: u16,
    mut config: Config,
    sync: crate::device_sync::lifecycle::SyncLifecycle,
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))] routing: &Arc<
        crate::app_routing::RoutingListener,
    >,
) -> Result<()> {
    agent_gateway::bootstrap::validate_bind(host, &config)?;
    config.gateway.skeys = config::load_gateway_skeys_for_tenant(&TenantScope::default())
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    #[cfg(target_os = "macos")]
    tokio::task::spawn_blocking(api::handlers::agent_apps::recover_providers_on_startup)
        .await
        .context("provider startup recovery task failed")?
        .map_err(anyhow::Error::msg)?;
    let config_state = Arc::new(Mutex::new(config.clone()));

    let hook_runtime = crate::app::agent::hooks::build_hook_runtime(&config)
        .context("failed to build gateway hook runtime")?;

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind gateway on {host}:{port}"))?;
    let actual_port = listener.local_addr()?.port();
    let display_addr = format!("{host}:{actual_port}");
    let cors = agent_gateway::bootstrap::gateway_cors();

    let provider = gateway_provider(&config);
    let model = config.default_model.clone().unwrap_or_else(|| "anthropic/claude-sonnet-4".into());
    let temperature = config.default_temperature;
    let mem: Arc<dyn Memory> = Arc::from(memory::create_memory_with_storage(
        &config.memory,
        Some(&config.storage.provider.config),
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);
    let runtime: Arc<dyn runtime_types::RuntimeAdapter> =
        Arc::from(agent_runtime::create_runtime(&config.runtime)?);
    let security = Arc::new(SecurityPolicy::from_configs(
        &config.autonomy,
        &config.security,
        &config.workspace_dir,
    ));

    #[cfg(unix)]
    let _computer_control =
        tools::computer_control::NativeControlRuntime::start(config.computer_control.clone())?;

    let tools_registry_exec: Arc<Vec<Box<dyn Tool>>> = Arc::new(tools::all_tools_with_runtime(
        Arc::new(config.clone()),
        &security,
        runtime,
        Arc::clone(&mem),
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
    let max_iterations =
        config_agent::effective_main_agent_config(&config.agents).max_iterations;
    let multimodal_config = config.multimodal.clone();

    let (event_tx, _event_rx) = tokio::sync::broadcast::channel::<serde_json::Value>(256);
    #[cfg(target_os = "macos")]
    routing.set_failover_events(event_tx.clone());
    let channels = super::channel_reload::build_gateway_channels(&config.channels_config);

    let agent_gateway::bootstrap::GatewayAdmission { pairing, rate_limiter, idempotency_store } =
        agent_gateway::bootstrap::gateway_admission(&config);

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

    agent_gateway::bootstrap::print_gateway_startup(
        &display_addr, tunnel_url.as_deref(), &pairing,
        GatewayRouteSummaryOptions {
            whatsapp_enabled: channels.whatsapp.is_some(),
            linq_enabled: channels.linq.is_some(),
            wati_enabled: channels.wati.is_some(),
            nextcloud_talk_enabled: channels.nextcloud_talk.is_some(),
            qq_webhook_enabled: channels.qq_webhook_enabled,
            node_control_enabled: config.gateway.node_control.enabled,
        },
    );

    crate::app::agent::health::mark_component_ok("gateway");

    if let Some(hooks) = hook_runtime.runner_snapshot() {
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
        pairing,
        trust_forwarded_headers: config.gateway.trust_forwarded_headers,
        rate_limiter,
        idempotency_store,
        channels: Arc::new(Mutex::new(channels)),
        observer: broadcast_observer,
        tools_registry,
        tools_registry_exec,
        multimodal: multimodal_config,
        max_iterations,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: Some(hook_runtime),
    };

    let openai_compat_route = Router::<AppState>::new()
        .route("/chat/completions", post(openai_compat::handle_v1_chat_completions));

    let dashboard_config_route = Router::<AppState>::new()
        .route("/config", get(api::handle_api_config_get).put(api::handle_api_config_put))
        .layer(RequestBodyLimitLayer::new(1_048_576));

    let app = Router::<AppState>::new()
        .merge(external_router().layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE)))
        .merge(client_router(state.clone(), openai_compat_route, dashboard_config_route))
        .with_state(state.clone())
        // Model providers validate client payload and context sizes.
        .layer(DefaultBodyLimit::disable())
        .layer(from_fn(request_timeout_middleware))
        .merge(super::router::management_auth_router()
            .layer(from_fn_with_state(state.clone(), skey_auth_middleware)))
        .layer(cors)
        .layer(axum::Extension(sync));

    #[cfg(target_os = "macos")]
    let refresh_service = api::handlers::agent_apps::init_usage_refresh().await;
    #[cfg(target_os = "macos")]
    let app = if let Some(refresh) = &refresh_service {
        app.layer(axum::Extension(Arc::clone(refresh)))
    } else {
        app
    };

    let usage_refresh = async {
        #[cfg(target_os = "macos")]
        if let (Some(refresh), Some(policy)) =
            (refresh_service, std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE"))
        {
            api::handlers::agent_apps::run_usage_refresh(refresh, policy.into()).await;
        }
        std::future::pending::<()>().await;
    };
    #[cfg(target_os = "macos")]
    let app = api::handlers::app_routing::configure(app, routing).await;
    #[cfg(not(target_arch = "wasm32"))]
    crate::worktree::initialize_host();
    #[cfg(not(target_arch = "wasm32"))]
    let _worktree_cleanup = crate::worktree::cleanup_host::HostCleanup::start()
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let periodic_backups = async {
        #[cfg(target_os = "macos")]
        if let Some(policy) = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE") {
            api::handlers::agent_apps_data::periodic::run(policy.into()).await;
        }
        std::future::pending::<()>().await;
    };
    let usage_scans = gateway_external_sessions::usage_scan::run();
    tokio::select! {
        () = usage_scans => {}
        result = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()) => result?,
        result = super::channel_reload::watch_gateway_channels(state, config) => result?,
        () = usage_refresh => {}
        () = periodic_backups => {}
    }

    Ok(())
}

pub use agent_gateway::route_catalog::{GatewayRouteEntry, documented_gateway_routes};
use agent_gateway::route_catalog::GatewayRouteSummaryOptions;

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
    let handler_router = stateful_handler_router();
    #[cfg(target_os = "macos")]
    let handler_router = handler_router
        .merge(super::router::app_routing_router().with_state(()))
        .merge(api::handlers::app_routing::agent_apps_high_availability::router().with_state(()));
    let handler_router = handler_router
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
        .route("/v1/mcp/apps/call", post(api::handle_api_mcp_app_call))
        .route("/v1/cron/history", get(api::handle_api_cron_history))
        .route("/v1/cron", get(api::handle_api_cron_list))
        .route("/v1/cron", post(api::handle_api_cron_add))
        .route("/v1/cron/preview", post(api::handle_api_cron_preview))
        .route("/api/cron/preview", post(api::handle_api_cron_preview))
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
        .route("/v1/api-health", get(api::handle_api_health))
        .nest("/v1/dashboard", dashboard_config_route)
        // 未使用 skey 的客户端入口也必须解析租户。
        .layer(from_fn(middleware::tenant_scope_middleware));

    standalone_client_router.nest("/api", api::voice_router()).nest("/v1", handler_router)
}

pub(super) fn stateful_handler_router() -> Router<AppState> {
    let router = Router::<AppState>::new();
    #[cfg(target_os = "macos")]
    let router = router.merge(api::handlers::agent_apps_diagnostics::router().with_state(()));
    router
        .merge(api::handlers::agent_apps::router().with_state(()))
        .merge(api::handlers::usage::router().with_state(()))
        .merge(api::handlers::computer_control::router().with_state(()))
        .merge(api::handlers::app_providers::router().with_state(()))
        .merge(api::handlers::model_catalog::router().with_state(()))
        .merge(api::handlers::auth::router())
        .merge(api::handlers::channel_qr::router())
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
        .merge(api::handlers::task_pool::router())
        .merge(api::handlers::agent_scheduler::router())
        .merge(api::handlers::task_lifecycle::router())
        .merge(api::handlers::task_plan_run::router())
        .merge(api::handlers::workflow::router())
        .merge(api::handlers::session::router())
        .merge(api::handlers::dynamic_tools::router())
        .merge(api::handlers::config::stateful_router())
        .merge(api::handlers::shiju_account::router())
        .merge(api::handlers::shiju_apps::router())
        .merge(api::handlers::shiju_skills::router())
        .merge(api::handlers::skill_market::router())
        .merge(api::handlers::desktop_skills::router())
        .merge(api::handlers::plugins::router())
        .merge(api::handlers::plugins::stateful_router())
        .merge(api::handlers::hooks::router())
        .merge(api::handlers::desktop::router())
}

#[cfg(test)]
#[path = "../../../libs/agent-gateway/src/bootstrap_tests.rs"]
mod bootstrap_tests;

#[cfg(test)]
pub(crate) use bootstrap_tests::dynamic_tools_acceptance_router;
