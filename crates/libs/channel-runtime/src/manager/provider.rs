//! 通道运行时的 provider 选择与懒加载。
//!
//! 该模块把模型 provider 的创建、缓存和工具规格过滤集中在一处，避免每条
//! 通道消息重复执行可能阻塞的初始化逻辑。

use super::*;

/// 根据通道运行时策略返回可暴露给模型的工具规格。
///
/// 参数：
/// - `tools_registry`：运行时已注册的全部工具。
/// - `excluded_tools`：当前通道或策略要求隐藏的工具 ID。
///
/// 返回值：过滤后的工具规格列表。
///
/// 错误处理：该函数只读取内存结构，不产生错误。
pub use channel_support::prompt::filtered_tool_specs_for_runtime;

/// 从缓存或配置创建指定 provider。
///
/// 参数：
/// - `ctx`：通道运行时上下文，提供默认 provider、缓存和可靠性配置。
/// - `provider_name`：路由选择出的 provider 名称。
///
/// 返回值：可共享的 provider 实例。
///
/// 错误处理：provider 创建失败、初始化任务 join 失败或底层配置错误会以
/// `anyhow::Error` 返回；warmup 失败只记录警告，因为 provider 仍可能在首轮请求中恢复。
pub async fn get_or_create_provider(
    ctx: &ChannelRuntimeContext,
    provider_name: &str,
) -> anyhow::Result<Arc<dyn Provider>> {
    if let Some(existing) =
        ctx.provider_cache.lock().unwrap_or_else(|e| e.into_inner()).get(provider_name).cloned()
    {
        return Ok(existing);
    }

    if provider_name == ctx.default_provider.as_str() {
        return Ok(Arc::clone(&ctx.provider));
    }

    let defaults = runtime_defaults_snapshot(ctx);
    let api_url = if provider_name == defaults.default_provider.as_str() {
        defaults.api_url.as_deref()
    } else {
        None
    };

    // 非默认 provider 不继承默认 api_url，避免把某个后端的自定义地址泄露到其他 provider。
    let provider = create_resilient_provider_nonblocking(
        provider_name,
        ctx.api_key.clone(),
        api_url.map(ToString::to_string),
        ctx.reliability.as_ref().clone(),
        ctx.provider_runtime_options.clone(),
    )
    .await?;
    let provider: Arc<dyn Provider> = Arc::from(provider);

    if let Err(err) = provider.warmup().await {
        tracing::warn!(provider = provider_name, "Provider warmup failed: {err}");
    }

    // 二次写缓存时仍使用 entry，防止并发消息同时初始化后覆盖已有实例。
    let mut cache = ctx.provider_cache.lock().unwrap_or_else(|e| e.into_inner());
    let cached = cache.entry(provider_name.to_string()).or_insert_with(|| Arc::clone(&provider));
    Ok(Arc::clone(cached))
}

/// 在不阻塞异步执行器的情况下创建具备可靠性包装的 provider。
///
/// 参数包含 provider 名称、凭据、可选 API 地址、可靠性策略和运行期选项。
///
/// 返回值：新建的 provider trait 对象。
///
/// 错误处理：同步创建过程中的错误和后台任务 join 错误都会向上传递。
pub async fn create_resilient_provider_nonblocking(
    provider_name: &str,
    api_key: Option<String>,
    api_url: Option<String>,
    reliability: agent_config::runtime_schema::ReliabilityConfig,
    provider_runtime_options: agent_providers::ProviderRuntimeOptions,
) -> anyhow::Result<Box<dyn Provider>> {
    let provider_name = provider_name.to_string();

    // 原生目标把同步 provider 构造放进阻塞线程，避免卡住通道消息的 async runtime。
    #[cfg(not(target_arch = "wasm32"))]
    let provider = tokio::task::spawn_blocking(move || {
        agent_providers::create_resilient_provider_with_options(
            &provider_name,
            api_key.as_deref(),
            api_url.as_deref(),
            &reliability,
            &provider_runtime_options,
        )
    })
    .await
    .context("failed to join provider initialization task")??;

    #[cfg(target_arch = "wasm32")]
    let provider = agent_providers::create_resilient_provider_with_options(
        &provider_name,
        api_key.as_deref(),
        api_url.as_deref(),
        &reliability,
        &provider_runtime_options,
    )?;

    Ok(provider)
}

/// 在不阻塞异步执行器的情况下创建带模型路由的 provider。
///
/// 参数包含 provider 基本配置、模型路由表、默认模型以及运行期选项。
///
/// 返回值：新建的 routed provider trait 对象。
///
/// 错误处理：路由配置或 provider 构造失败会以 `anyhow::Error` 返回。
pub async fn create_routed_provider_nonblocking(
    provider_name: &str,
    api_key: Option<String>,
    api_url: Option<String>,
    reliability: agent_config::runtime_schema::ReliabilityConfig,
    model_routes: Vec<agent_config::runtime_schema::ModelRouteConfig>,
    default_model: String,
    provider_runtime_options: agent_providers::ProviderRuntimeOptions,
) -> anyhow::Result<Box<dyn Provider>> {
    let provider_name = provider_name.to_string();

    // 原生目标同样把 routed provider 构造放进阻塞线程，避免影响消息处理。
    #[cfg(not(target_arch = "wasm32"))]
    let provider = tokio::task::spawn_blocking(move || {
        agent_providers::create_routed_provider_with_options(
            &provider_name,
            api_key.as_deref(),
            api_url.as_deref(),
            &reliability,
            &model_routes,
            &default_model,
            &provider_runtime_options,
        )
    })
    .await
    .context("failed to join routed provider initialization task")??;

    #[cfg(target_arch = "wasm32")]
    let provider = agent_providers::create_routed_provider_with_options(
        &provider_name,
        api_key.as_deref(),
        api_url.as_deref(),
        &reliability,
        &model_routes,
        &default_model,
        &provider_runtime_options,
    )?;

    Ok(provider)
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
