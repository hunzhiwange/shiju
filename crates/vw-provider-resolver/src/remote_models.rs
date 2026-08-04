//! 已连接 provider 的受限 OpenAI 兼容模型发现。

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::time::Duration;

use serde::Deserialize;

use crate::provider::{
    ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
    ModelCostCache, ModelLimit, default_adapter,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) struct DiscoveryLimits {
    timeout: Duration,
    max_response_bytes: usize,
}

impl DiscoveryLimits {
    #[cfg(test)]
    pub(crate) fn new(timeout: Duration, max_response_bytes: usize) -> Self {
        Self { timeout, max_response_bytes }
    }
}

impl Default for DiscoveryLimits {
    fn default() -> Self {
        Self { timeout: DEFAULT_TIMEOUT, max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES }
    }
}

/// 不含凭据、URL 或响应载荷的模型发现错误。
#[derive(Debug)]
pub struct DiscoveryError {
    category: &'static str,
}

impl DiscoveryError {
    fn new(category: &'static str) -> Self {
        Self { category }
    }

    /// 返回可安全写入日志的稳定错误类别。
    pub fn category(&self) -> &'static str {
        self.category
    }
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "remote model discovery failed: {}", self.category)
    }
}

impl std::error::Error for DiscoveryError {}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: Option<String>,
}

/// 判断凭据是否可用于远端 Bearer 认证。
pub fn is_real_bearer(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value != vw_shared::auth::OAUTH_DUMMY_KEY
}

/// 从 API 基址派生 `/models`，并发现 OpenAI 兼容模型 id。
pub async fn discover_openai_compatible_models(
    api_base: &str,
    bearer: &str,
) -> Result<Vec<String>, DiscoveryError> {
    discover_openai_compatible_models_with_limits(api_base, bearer, DiscoveryLimits::default())
        .await
}

pub(crate) async fn discover_openai_compatible_models_with_limits(
    api_base: &str,
    bearer: &str,
    limits: DiscoveryLimits,
) -> Result<Vec<String>, DiscoveryError> {
    if !is_real_bearer(bearer) {
        return Ok(Vec::new());
    }

    let api_base = api_base.trim();
    if api_base.is_empty() {
        return Ok(Vec::new());
    }
    let models_url = format!("{}/models", api_base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(limits.timeout)
        .build()
        .map_err(|_| DiscoveryError::new("client"))?;
    let mut response =
        client.get(models_url).bearer_auth(bearer.trim()).send().await.map_err(request_error)?;

    if response.status().is_redirection() {
        return Err(DiscoveryError::new("redirect"));
    }
    if !response.status().is_success() {
        return Err(DiscoveryError::new("http_status"));
    }
    if response.content_length().is_some_and(|length| length > limits.max_response_bytes as u64) {
        return Err(DiscoveryError::new("response_too_large"));
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(request_error)? {
        if body.len().saturating_add(chunk.len()) > limits.max_response_bytes {
            return Err(DiscoveryError::new("response_too_large"));
        }
        body.extend_from_slice(&chunk);
    }

    let response: ModelsResponse =
        serde_json::from_slice(&body).map_err(|_| DiscoveryError::new("invalid_response"))?;
    let ids = response
        .data
        .into_iter()
        .filter_map(|entry| entry.id)
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(ids)
}

fn request_error(error: reqwest::Error) -> DiscoveryError {
    if error.is_timeout() { DiscoveryError::new("timeout") } else { DiscoveryError::new("request") }
}

/// 选择配置 API 基址；未配置时使用模型元数据中的确定性非空候选。
pub fn resolve_api_base(provider: &Info, configured_api: Option<&str>) -> Option<String> {
    if let Some(api) = configured_api.map(str::trim).filter(|api| !api.is_empty()) {
        return Some(api.trim_end_matches('/').to_string());
    }

    provider
        .models
        .values()
        .map(|model| model.api.url.trim())
        .filter(|url| !url.is_empty())
        .map(|url| url.trim_end_matches('/').to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .next()
}

/// 合并远端模型；显式配置状态优先，静态元数据保持不变。
pub fn merge_remote_models(
    provider: &mut Info,
    api_base: &str,
    explicit_model_ids: &HashSet<String>,
    remote_ids: Vec<String>,
) -> usize {
    let mut changed = 0;
    for remote_id in remote_ids {
        let existing_id = if provider.models.contains_key(&remote_id) {
            Some(remote_id.clone())
        } else {
            provider
                .models
                .iter()
                .filter(|(_, model)| model.api.id == remote_id)
                .map(|(id, _)| id.clone())
                .min()
        };

        if let Some(existing_id) = existing_id {
            if explicit_model_ids.contains(&existing_id) {
                continue;
            }
            if let Some(model) = provider.models.get_mut(&existing_id)
                && model.status != "active"
            {
                model.status = "active".to_string();
                changed += 1;
            }
            continue;
        }

        provider
            .models
            .insert(remote_id.clone(), safe_remote_model(&provider.id, api_base, remote_id));
        changed += 1;
    }
    changed
}

/// 发现并合并单个 provider；无真实凭据或 API 基址时直接跳过。
pub async fn discover_and_merge_provider(
    provider: &mut Info,
    configured_api: Option<&str>,
    explicit_model_ids: &HashSet<String>,
    bearer: &str,
) -> Result<usize, DiscoveryError> {
    if !is_real_bearer(bearer) {
        return Ok(0);
    }
    let Some(api_base) = resolve_api_base(provider, configured_api) else {
        return Ok(0);
    };
    let remote_ids = discover_openai_compatible_models(&api_base, bearer).await?;
    Ok(merge_remote_models(provider, &api_base, explicit_model_ids, remote_ids))
}

fn safe_remote_model(provider_id: &str, api_base: &str, model_id: String) -> Model {
    Model {
        id: model_id.clone(),
        provider_id: provider_id.to_string(),
        api: ApiInfo {
            id: model_id.clone(),
            url: api_base.trim_end_matches('/').to_string(),
            adapter: default_adapter(),
        },
        name: model_id,
        family: None,
        capabilities: Capabilities {
            temperature: false,
            reasoning: false,
            attachment: false,
            toolcall: true,
            input: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            output: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: InterleavedCapability::Bool(false),
        },
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache: ModelCostCache { read: 0.0, write: 0.0 },
            experimental_over_200k: None,
        },
        limit: ModelLimit { context: 0, input: None, output: 0 },
        status: "active".to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: String::new(),
        variants: HashMap::new(),
    }
}
