//! Provider 模块 - 模型提供商管理与发现系统
//!
//! 本模块负责 AI 模型提供商的完整生命周期管理，包括：
//! - 从多种来源加载提供商配置（环境变量、配置文件、认证系统）
//! - 管理模型元数据（能力、成本、限制等）
//! - 提供模型查找与智能建议功能
//! - 支持动态配置覆盖与合并
//!
//! ## 核心概念
//!
//! - **Provider（提供商）**: AI 模型服务提供方（如 OpenAI、Anthropic）
//! - **Model（模型）**: 提供商下的具体模型实例
//! - **Capability（能力）**: 模型支持的功能特性
//! - **Cost（成本）**: 模型使用的计费信息
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::app::agent::provider::provider;
//!
//! // 获取所有可用提供商
//! let providers = provider::list().await;
//!
//! // 获取特定模型
//! let model = provider::get_model("openai", "gpt-4").await?;
//! ```

use crate::app::agent::auth;
use crate::app::agent::config;
use crate::app::agent::env;
use crate::app::agent::project::instance;
use crate::app::agent::util::log;
use serde_json::{Map, Value};
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, LazyLock, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use vw_provider_resolver::remote_models::{discover_and_merge_provider, is_real_bearer};

pub use vw_shared::provider::state::{
    State, as_string_map, builtin_model_limit, default_model_capabilities, from_models_dev_model,
    from_models_dev_provider, normalize_adapter, resolve_model_capabilities, resolve_model_limit,
};
pub use vw_shared::provider::types::{
    ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
    ModelCostCache, ModelCostOver200k, ModelLimit, ModelNotFoundError, ParsedModelRef,
    ProviderSource, default_adapter,
};
pub use vw_shared::provider::types::{parse_model, sort};

/// Provider 模块的日志记录器
///
/// 使用延迟初始化模式创建，所有日志都会带上 `service: provider` 标签，
/// 便于在日志系统中过滤和追踪 provider 相关的操作。
static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("provider".to_string()));
        m
    }))
});

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ConfigFingerprint(u64);

static CONFIG_FINGERPRINTS: LazyLock<Mutex<HashMap<String, ConfigFingerprint>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn instance_state()
-> impl Fn() -> crate::app::agent::project::BoxFuture<Arc<State>> + Send + Sync + 'static {
    instance::state(
        "provider",
        || async { load_state_filtered().await },
        None::<fn(Arc<State>) -> crate::app::agent::project::BoxFuture<()>>,
    )
}

/// 创建设置页面的 provider 状态访问器
///
/// 与 `instance_state` 类似，但不会过滤掉 disabled 的模型，
/// 用于设置页面显示所有可用模型（包括未启用的）。
///
/// # 返回值
///
/// 返回一个闭包，调用它会返回 `Arc<State>` 的 Future
fn instance_state_for_settings()
-> impl Fn() -> crate::app::agent::project::BoxFuture<Arc<State>> + Send + Sync + 'static {
    instance::state(
        "provider_settings",
        || async { load_state_for_settings().await },
        None::<fn(Arc<State>) -> crate::app::agent::project::BoxFuture<()>>,
    )
}

pub async fn list() -> HashMap<String, Info> {
    invalidate_cache_if_config_changed().await;
    instance_state()().await.providers.clone()
}

/// 获取设置页面用的提供商列表
///
/// 与 `list` 类似，但包含所有模型（包括未启用的），
/// 用于设置界面显示完整选项。
///
/// # 返回值
///
/// 返回提供商 ID 到提供商信息的映射
pub async fn list_for_settings() -> HashMap<String, Info> {
    invalidate_cache_if_config_changed().await;
    instance_state_for_settings()().await.providers.clone()
}

/// 使缓存失效
///
/// 清除 provider 状态的缓存，强制下次访问时重新加载。
/// 通常在配置变更后调用。
pub async fn invalidate_cache() {
    let directory = instance::directory();
    if let Ok(mut guard) = CONFIG_FINGERPRINTS.lock() {
        guard.remove(&directory);
    }
    crate::app::agent::project::state::dispose(&directory).await;
}

async fn current_config_fingerprint() -> ConfigFingerprint {
    config_fingerprint(&config::get().await)
}

fn config_fingerprint(config: &config::Config) -> ConfigFingerprint {
    let payload = serde_json::to_vec(&config.providers).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    payload.hash(&mut hasher);
    ConfigFingerprint(hasher.finish())
}

async fn invalidate_cache_if_config_changed() {
    let directory = instance::directory();
    let next = current_config_fingerprint().await;
    let changed = {
        let mut guard = CONFIG_FINGERPRINTS.lock().unwrap_or_else(|e| e.into_inner());
        let changed = guard.get(&directory).is_some_and(|current| current != &next);
        guard.insert(directory.clone(), next);
        changed
    };

    if changed {
        crate::app::agent::project::state::dispose(&directory).await;
    }
}

/// 获取指定提供商信息
///
/// # 参数
///
/// - `provider_id`: 提供商 ID
///
/// # 返回值
///
/// 如果找到则返回 `Some(Info)`，否则返回 `None`
pub async fn get_provider(provider_id: &str) -> Option<Info> {
    invalidate_cache_if_config_changed().await;
    instance_state()().await.providers.get(provider_id).cloned()
}

/// 获取指定模型信息
///
/// 查找并返回指定提供商下的模型信息。如果模型 ID 不存在，
/// 会尝试通过 API ID 匹配（别名查找），并提供相似模型建议。
///
/// # 参数
///
/// - `provider_id`: 提供商 ID
/// - `model_id`: 模型 ID
///
/// # 返回值
///
/// 成功返回 `Ok(Model)`，失败返回 `ModelNotFoundError`
///
/// # 错误
///
/// - 提供商不存在时返回错误，并包含相似提供商建议
/// - 模型不存在时返回错误，并包含相似模型建议
pub async fn get_model(provider_id: &str, model_id: &str) -> Result<Model, ModelNotFoundError> {
    invalidate_cache_if_config_changed().await;
    let state = instance_state()().await;

    // 首先查找提供商
    let Some(provider) = state.providers.get(provider_id) else {
        let suggestions =
            suggest(provider_id, state.providers.keys().map(|s| s.as_str()).collect());
        return Err(ModelNotFoundError {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
            suggestions,
        });
    };

    // 尝试通过模型 ID 查找
    let Some(model) = provider.models.get(model_id).cloned() else {
        // 尝试通过 API ID 别名查找
        let alias_matches =
            provider.models.values().filter(|m| m.api.id == model_id).cloned().collect::<Vec<_>>();

        // 如果只有一个别名匹配，直接返回
        if alias_matches.len() == 1 {
            return Ok(alias_matches[0].clone());
        }

        // 生成建议列表
        let suggestions = if alias_matches.len() > 1 {
            // 多个别名匹配，返回这些模型的 ID
            let mut ids = alias_matches.into_iter().map(|m| m.id).collect::<Vec<_>>();
            ids.sort();
            ids.truncate(3);
            ids
        } else {
            // 无别名匹配，通过字符串相似度建议
            suggest(model_id, provider.models.keys().map(|s| s.as_str()).collect())
        };

        return Err(ModelNotFoundError {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
            suggestions,
        });
    };

    Ok(model)
}

/// 获取默认模型
///
/// 按以下优先级确定默认模型：
/// 1. 配置文件中的 `default_model` 设置
/// 2. 第一个可用提供商的第一个模型（按排序规则）
///
/// # 返回值
///
/// 成功返回 `Ok(ParsedModelRef)`，失败返回错误消息
///
/// # 错误
///
/// - 没有可用提供商时返回 "没有可用的 provider"
/// - 提供商没有模型时返回 "没有可用的模型"
pub async fn default_model() -> Result<ParsedModelRef, String> {
    let cfg = config::get().await;

    // 优先使用配置中的默认模型
    if let Some(m) = cfg.default_model.as_deref() {
        return Ok(parse_model(m));
    }

    // 回退到第一个可用提供商的第一个模型
    let providers = list().await;
    let mut candidates = providers.values().collect::<Vec<_>>();
    candidates.sort_by(|a, b| a.id.cmp(&b.id));

    let Some(p) = candidates.first() else {
        return Err("没有可用的 provider".to_string());
    };

    let mut models = p.models.values().cloned().collect::<Vec<_>>();
    models = sort(models);

    let Some(m) = models.first() else {
        return Err("没有可用的模型".to_string());
    };

    Ok(ParsedModelRef { provider_id: p.id.clone(), model_id: m.id.clone() })
}

/// 获取指定提供商的小型模型
///
/// 查找适合快速任务的轻量级模型。
/// 会根据提供商类型和模型名称优先级进行匹配。
///
/// # 参数
///
/// - `provider_id`: 提供商 ID
///
/// # 返回值
///
/// 如果找到小型模型返回 `Some(Model)`，否则返回 `None`
///
/// # 优先级顺序
///
/// 1. Claude Haiku 系列
/// 2. Gemini Flash 系列
/// 3. GPT Nano 系列
///
/// 对于特定提供商会有调整（如 GitHub Copilot 会优先使用 GPT Mini）
pub async fn get_small_model(provider_id: &str) -> Option<Model> {
    invalidate_cache_if_config_changed().await;
    let _cfg = config::get().await;
    let state = instance_state()().await;
    let provider = state.providers.get(provider_id)?;

    // 根据提供商类型设置优先级列表
    let mut priority = vec![
        "claude-haiku-4-5",
        "claude-haiku-4.5",
        "3-5-haiku",
        "3.5-haiku",
        "gemini-3-flash",
        "gemini-2.5-flash",
        "gpt-5-nano",
    ];

    // VibeWindow 提供商只使用 GPT Nano
    if provider_id.starts_with("vibewindow") {
        priority = vec!["gpt-5-nano"];
    }

    // GitHub Copilot 提供商优先使用 GPT Mini
    if provider_id.starts_with("github-copilot") {
        priority.insert(0, "gpt-5-mini");
        priority.insert(1, "claude-haiku-4.5");
    }

    // 按优先级查找匹配的模型
    for item in priority {
        for (id, model) in &provider.models {
            if id.contains(item) {
                return Some(model.clone());
            }
        }
    }

    None
}

fn suggest(query: &str, candidates: Vec<&str>) -> Vec<String> {
    let mut scored = candidates
        .into_iter()
        .map(|c| (c, strsim::jaro_winkler(query, c)))
        .filter(|(_, s)| *s > 0.70)
        .collect::<Vec<_>>();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scored.into_iter().take(3).map(|(c, _)| c.to_string()).collect()
}

async fn load_provider_overrides() -> Vec<(String, Value)> {
    config::get()
        .await
        .providers
        .into_iter()
        .filter_map(|(provider_id, provider_cfg)| {
            provider_cfg.as_object().map(|obj| (provider_id, Value::Object(obj.clone())))
        })
        .collect()
}

fn unique_provider_api(models: &HashMap<String, Model>) -> Option<(String, String)> {
    let mut api_candidates = models.values().filter_map(|model| {
        let url = model.api.url.trim();
        (!url.is_empty()).then(|| (url.to_string(), model.api.adapter.clone()))
    });
    let first = api_candidates.next()?;
    api_candidates.all(|candidate| candidate == first).then_some(first)
}

async fn load_state_impl(apply_enabled_filter: bool) -> State {
    // 初始化模型基线数据源
    super::models::init();
    let _cfg = config::get().await;
    let models_dev = super::models::get().await;

    // 将模型基线数据转换为内部格式
    let mut database = models_dev
        .into_iter()
        .map(|(k, v)| (k, from_models_dev_provider(v)))
        .collect::<HashMap<_, _>>();

    // 将所有模型初始化为 disabled 状态
    for provider in database.values_mut() {
        for model in provider.models.values_mut() {
            model.status = "disabled".to_string();
        }
    }

    let mut providers: HashMap<String, Info> = HashMap::new();
    let config_providers: Vec<(String, serde_json::Value)> = load_provider_overrides().await;

    // 处理配置文件中的提供商覆盖
    for (provider_id, provider) in config_providers.iter() {
        let existing = database.get(provider_id).cloned();

        // 记录基础数据缺失的情况
        if existing.is_none() {
            LOGGER.info(
                "provider_base_missing",
                Some({
                    let mut m = Map::new();
                    m.insert("providerID".to_string(), Value::String(provider_id.to_string()));
                    m
                }),
            );
        }

        // 解析提供商名称（优先级：配置 > 基础数据 > ID）
        let name = provider
            .get("name")
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .or_else(|| existing.as_ref().map(|p| p.name.clone()))
            .unwrap_or_else(|| provider_id.to_string());

        // 解析环境变量列表
        let env_arr = provider
            .get("env")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>())
            .or_else(|| existing.as_ref().map(|p| p.env.clone()))
            .unwrap_or_default();

        // 合并提供商级别选项
        let mut options = existing.as_ref().map(|p| p.options.clone()).unwrap_or_default();
        if let Some(o) = provider.get("options").and_then(Value::as_object) {
            for (k, v) in o {
                options.insert(k.clone(), v.clone());
            }
        }

        // 获取基础模型集合
        let mut models = existing.as_ref().map(|p| p.models.clone()).unwrap_or_default();
        let inherited_api = unique_provider_api(&models);

        // 处理模型级别配置
        if let Some(models_cfg) = provider.get("models").and_then(Value::as_object) {
            for (model_id, model_cfg) in models_cfg {
                // 尝试通过 ID 或别名查找基础模型
                let base = models.get(model_id).cloned().or_else(|| {
                    model_cfg
                        .get("id")
                        .and_then(Value::as_str)
                        .and_then(|id| models.get(id).cloned())
                });

                // 解析 API ID（优先级：配置 > 基础模型 > 模型 ID）
                let api_id = model_cfg
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| base.as_ref().map(|m| m.api.id.clone()))
                    .unwrap_or_else(|| model_id.to_string());

                // 解析 API URL
                let api_url = provider
                    .get("api")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| base.as_ref().map(|m| m.api.url.clone()))
                    .or_else(|| inherited_api.as_ref().map(|(url, _)| url.clone()))
                    .unwrap_or_default();

                // 解析适配器类型（优先级：模型配置 > 提供商配置 > 基础模型）
                let api_adapter = model_cfg
                    .get("provider")
                    .and_then(|p| p.get("adapter"))
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| {
                        provider.get("adapter").and_then(Value::as_str).map(|s| s.to_string())
                    })
                    .or_else(|| base.as_ref().map(|m| m.api.adapter.clone()))
                    .or_else(|| inherited_api.as_ref().map(|(_, adapter)| adapter.clone()))
                    .unwrap_or_else(default_adapter);
                let api_adapter = normalize_adapter(&api_adapter);

                // 解析状态（默认为 disabled，需要显式设置为 active）
                let status = model_cfg
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "disabled".to_string());

                // 解析显示名称
                let name = model_cfg
                    .get("name")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| base.as_ref().map(|m| m.name.clone()))
                    .unwrap_or_else(|| model_id.to_string());

                // 解析模型家族
                let family = model_cfg
                    .get("family")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| base.as_ref().and_then(|m| m.family.clone()));

                // 解析发布日期
                let release_date = model_cfg
                    .get("release_date")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .or_else(|| base.as_ref().map(|m| m.release_date.clone()))
                    .unwrap_or_default();

                // 解析自定义请求头
                let headers = model_cfg
                    .get("headers")
                    .map(as_string_map)
                    .or_else(|| base.as_ref().map(|m| m.headers.clone()))
                    .unwrap_or_default();

                // 合并模型级别选项
                let mut options_model =
                    base.as_ref().map(|m| m.options.clone()).unwrap_or_default();
                if let Some(o) = model_cfg.get("options").and_then(Value::as_object) {
                    for (k, v) in o {
                        options_model.insert(k.clone(), v.clone());
                    }
                }

                // 解析限制配置（优先级：配置 > 基础模型 > 内置默认）
                let config_limit = model_cfg.get("limit");
                let configured_context = config_limit
                    .and_then(|limit| limit.get("context"))
                    .and_then(Value::as_u64)
                    .filter(|value| *value > 0);
                let configured_input = config_limit
                    .and_then(|limit| limit.get("input"))
                    .and_then(Value::as_u64)
                    .filter(|value| *value > 0);
                let configured_output = config_limit
                    .and_then(|limit| limit.get("output"))
                    .and_then(Value::as_u64);
                let limit = vw_shared::provider::state::resolve_model_limit_with_base(
                    configured_context,
                    configured_input,
                    configured_output,
                    base.as_ref().map(|model| &model.limit),
                );

                // 解析能力配置（使用基础模型或默认值）
                let cap = resolve_model_capabilities(
                    &model_cfg,
                    base.as_ref().map(|model| &model.capabilities),
                );

                // 解析成本配置（使用基础模型或默认值）
                let cost = base.as_ref().map(|m| m.cost.clone()).unwrap_or(ModelCost {
                    input: 0.0,
                    output: 0.0,
                    cache: ModelCostCache { read: 0.0, write: 0.0 },
                    experimental_over_200k: None,
                });

                // 构建最终模型配置
                let model = Model {
                    id: model_id.to_string(),
                    provider_id: provider_id.to_string(),
                    api: ApiInfo { id: api_id, url: api_url, adapter: api_adapter },
                    name,
                    family,
                    capabilities: cap,
                    cost,
                    limit,
                    status,
                    options: options_model,
                    headers,
                    release_date,
                    variants: HashMap::new(),
                };
                models.insert(model_id.to_string(), model);
            }
        }

        // 更新数据库中的提供商信息
        database.insert(
            provider_id.to_string(),
            Info {
                id: provider_id.to_string(),
                name,
                source: ProviderSource::Config,
                env: env_arr,
                key: None,
                options,
                models,
            },
        );
    }

    // 检测环境变量中的认证信息
    let env_vars = env::all();
    for (_provider_id, provider) in database.iter_mut() {
        let key =
            provider.env.iter().filter_map(|k| env_vars.get(k).cloned()).find(|v| !v.is_empty());
        if let Some(k) = key {
            provider.source = ProviderSource::Env;
            // 只有单个环境变量时才存储密钥
            if provider.env.len() == 1 {
                provider.key = Some(k);
            }
        }
    }

    // 集成认证系统的提供商信息
    for (provider_id, info) in auth::all() {
        let (next_source, next_key) = match info {
            auth::Info::Api(api) => (ProviderSource::Api, Some(api.key)),
            auth::Info::Oauth(_) => (ProviderSource::Api, Some(auth::OAUTH_DUMMY_KEY.to_string())),
            auth::Info::Wellknown(_) => {
                (ProviderSource::Api, Some(auth::OAUTH_DUMMY_KEY.to_string()))
            }
        };

        if let Some(p) = database.get_mut(&provider_id) {
            p.source = next_source;
            // 只在当前密钥为空时更新
            if p.key.as_deref().unwrap_or_default().trim().is_empty() {
                p.key = next_key;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    for (provider_id, provider) in database.iter_mut() {
        let configured = config_providers
            .iter()
            .find(|(configured_id, _)| configured_id == provider_id)
            .map(|(_, value)| value);
        let configured_api = configured
            .and_then(|value| value.get("api"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let explicit_model_ids = configured
            .and_then(|value| value.get("models"))
            .and_then(Value::as_object)
            .map(|models| models.keys().cloned().collect::<HashSet<_>>())
            .unwrap_or_default();
        let bearer =
            provider.key.as_deref().filter(|key| is_real_bearer(key)).map(str::to_string).or_else(
                || {
                    provider
                        .env
                        .iter()
                        .filter_map(|name| env_vars.get(name))
                        .find(|key| is_real_bearer(key))
                        .cloned()
                },
            );
        let Some(bearer) = bearer else {
            continue;
        };

        if let Err(error) = discover_and_merge_provider(
            provider,
            configured_api.as_deref(),
            &explicit_model_ids,
            &bearer,
        )
        .await
        {
            LOGGER.warn(
                "remote_model_discovery_failed",
                Some({
                    let mut fields = Map::new();
                    fields.insert("providerID".to_string(), Value::String(provider_id.clone()));
                    fields.insert(
                        "category".to_string(),
                        Value::String(error.category().to_string()),
                    );
                    fields
                }),
            );
        }
    }

    // 构建最终提供商列表，应用过滤器
    for (provider_id, provider) in database.into_iter() {
        let mut provider = provider;

        // 根据参数决定是否过滤 disabled 模型
        if apply_enabled_filter {
            provider.models.retain(|_, m| m.status == "active");
        }

        // 跳过没有模型的提供商
        if provider.models.is_empty() {
            continue;
        }

        providers.insert(provider_id, provider);
    }

    State { providers }
}

/// 加载过滤后的状态
///
/// 只包含 status 为 "active" 的模型，用于正常运行时。
///
/// # 返回值
///
/// 返回过滤后的 `State`
async fn load_state_filtered() -> State {
    load_state_impl(true).await
}

/// 加载设置页面用的状态
///
/// 包含所有模型（包括 disabled），用于设置界面显示完整选项。
///
/// # 返回值
///
/// 返回包含所有模型的 `State`
async fn load_state_for_settings() -> State {
    load_state_impl(false).await
}

/// 初始化 provider 模块
///
/// 预加载 provider 状态，确保后续访问时数据已就绪。
/// 这是在应用启动时调用的初始化函数。
pub fn init() {
    let _ = instance_state();
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
