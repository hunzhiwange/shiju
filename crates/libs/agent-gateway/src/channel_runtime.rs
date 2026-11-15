//! Webhook 与监听通道复用会话、命令及取消路径；缓存随网关状态存活。
use gateway_types::state::AppState;
use channel_support::{scope::ChannelProjectState, runtime_state::ChannelRuntimeContext};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type GatewayChannels<W = channel_whatsapp::WhatsAppChannel> = gateway_types::state::GatewayChannels<
    Arc<parking_lot::Mutex<HashMap<vw_api_transport::TenantScope, Arc<ChannelRuntimeContext>>>>,
    Arc<W>,
>;

pub fn channel_runtime<E, H, W>(
    state: &AppState<E, GatewayChannels<W>, H>,
    tenant: vw_api_transport::TenantScope,
    prompt: impl FnOnce(&agent_config::runtime::Config, &str, bool) -> anyhow::Result<String>,
    hooks: impl FnOnce() -> Option<Arc<agent_hooks::HookRunner>>,
) -> anyhow::Result<Arc<ChannelRuntimeContext>> {
    let cache = state.channels.lock().runtimes.clone();
    let mut runtimes = cache.lock();
    if let Some(runtime) = runtimes.get(&tenant) {
        return Ok(runtime.clone());
    }
    let config = state.config.lock().clone();
    let system_prompt = prompt(&config, &state.model, state.provider.supports_native_tools())?;
    let provider_name = config.default_provider.clone().unwrap_or_else(|| "openrouter".into());
    let project_state = Arc::new(Mutex::new(ChannelProjectState::default()));
    {
        let mut state = project_state.lock().unwrap_or_else(|e| e.into_inner());
        state.tenant = tenant.clone();
        state.config = config.channels_config.clone();
    }
    let runtime = Arc::new(ChannelRuntimeContext {
        project_state,
        message_snapshot: None,
        session_full_access: false,
        channels_by_name: Arc::new(HashMap::new()),
        provider: state.provider.clone(),
        default_provider: Arc::new(provider_name.clone()),
        memory: state.mem.clone(),
        tools_registry: state.tools_registry_exec.clone(),
        observer: state.observer.clone(),
        system_prompt: Arc::new(system_prompt),
        model: Arc::new(state.model.clone()),
        temperature: state.temperature,
        auto_save_memory: state.auto_save,
        max_iterations: state.max_iterations,
        min_relevance_score: config.memory.min_relevance_score,
        conversation_histories: Default::default(),
        provider_cache: Arc::new(Mutex::new(HashMap::from([(
            provider_name,
            state.provider.clone(),
        )]))),
        route_overrides: Default::default(),
        api_key: config.api_key.clone(),
        api_url: config.api_url.clone(),
        reliability: Arc::new(config.reliability.clone()),
        provider_runtime_options: channel_runtime::manager::start::build_provider_runtime_options(&config),
        workspace_dir: Arc::new(config.workspace_dir.clone()),
        message_timeout_secs: config.channels_config.message_timeout_secs,
        interrupt_on_new_message: false,
        multimodal: config.multimodal.clone(),
        hooks: hooks(),
        non_cli_excluded_tools: Arc::new(Mutex::new(
            config.autonomy.non_cli_excluded_tools.clone(),
        )),
        query_classification: config.query_classification.clone(),
        model_routes: config.model_routes.clone(),
        approval_manager: Arc::new(agent_approval::ApprovalManager::from_config(&config.autonomy)),
    });
    runtimes.insert(tenant, runtime.clone());
    Ok(runtime)
}
