//! 已连接供应商和原生应用目录共用的受限 HTTP 模型发现。

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::time::Duration;

use reqwest::{
    Client, ClientBuilder, StatusCode, Url,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serde::Deserialize;
use serde_json::Value;

use crate::provider::{
    ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
    ModelCostCache, ModelLimit, default_adapter,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);
const CATALOG_TIMEOUT: Duration = Duration::from_secs(15);
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
    data: Option<Vec<ModelEntry>>,
    models: Option<Value>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: Option<String>,
    owned_by: Option<Value>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DiscoveredModel {
    pub id: String,
    pub owned_by: Option<String>,
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
    let models_url = Url::parse(&format!("{}/models", api_base.trim_end_matches('/')))
        .map_err(|_| DiscoveryError::new("request"))?;
    let mut authorization = HeaderValue::from_str(&format!("Bearer {}", bearer.trim()))
        .map_err(|_| DiscoveryError::new("request"))?;
    authorization.set_sensitive(true);
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, authorization);
    let body =
        fetch_at(&[models_url], &headers, limits, Client::builder()).await.map_err(|error| {
            if error.category() == "unauthorized" {
                DiscoveryError::new("http_status")
            } else {
                error
            }
        })?;
    Ok(parse_models(&body, false)?.into_iter().map(|model| model.id).collect())
}

/// Request only the supplied, preauthorized candidates with the supplied protocol headers.
/// Catalog requests connect directly to avoid sending credentials through an implicit proxy.
pub async fn discover_catalog_models(
    candidates: &[Url],
    headers: &HeaderMap,
) -> Result<Vec<DiscoveredModel>, DiscoveryError> {
    let limits = DiscoveryLimits { timeout: CATALOG_TIMEOUT, ..DiscoveryLimits::default() };
    let body = fetch_at(candidates, headers, limits, Client::builder().no_proxy()).await?;
    parse_models(&body, true)
}

async fn fetch_at(
    candidates: &[Url],
    headers: &HeaderMap,
    limits: DiscoveryLimits,
    builder: ClientBuilder,
) -> Result<Vec<u8>, DiscoveryError> {
    let client = builder
        .redirect(reqwest::redirect::Policy::none())
        .referer(false)
        .timeout(limits.timeout)
        .build()
        .map_err(|_| DiscoveryError::new("client"))?;
    for url in candidates {
        let response =
            client.get(url.clone()).headers(headers.clone()).send().await.map_err(request_error)?;
        match response.status() {
            StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED => continue,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(DiscoveryError::new("unauthorized"));
            }
            status if status.is_redirection() => return Err(DiscoveryError::new("redirect")),
            status if status.is_success() => {
                return read_body(response, limits.max_response_bytes).await;
            }
            _ => return Err(DiscoveryError::new("http_status")),
        }
    }
    Err(DiscoveryError::new("http_status"))
}

async fn read_body(
    mut response: reqwest::Response,
    max_response_bytes: usize,
) -> Result<Vec<u8>, DiscoveryError> {
    if response.content_length().is_some_and(|length| length > max_response_bytes as u64) {
        return Err(DiscoveryError::new("response_too_large"));
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(request_error)? {
        if body.len().saturating_add(chunk.len()) > max_response_bytes {
            return Err(DiscoveryError::new("response_too_large"));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn parse_models(
    body: &[u8],
    include_catalog_entries: bool,
) -> Result<Vec<DiscoveredModel>, DiscoveryError> {
    let response: ModelsResponse =
        serde_json::from_slice(body).map_err(|_| DiscoveryError::new("invalid_response"))?;
    let mut models: Vec<_> = match (response.data, response.models) {
        (Some(data), _) => data
            .into_iter()
            .filter_map(|entry| {
                Some(DiscoveredModel {
                    id: model_id(entry.id.as_deref())?.to_owned(),
                    owned_by: entry.owned_by.and_then(|owner| match owner {
                        Value::String(owner) => Some(owner),
                        _ => None,
                    }),
                })
            })
            .collect(),
        (None, Some(Value::Array(entries))) if include_catalog_entries => {
            entries.iter().filter_map(catalog_entry).collect()
        }
        _ => return Err(DiscoveryError::new("invalid_response")),
    };
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    Ok(models)
}

fn catalog_entry(entry: &Value) -> Option<DiscoveredModel> {
    let field = |name| entry.get(name).and_then(Value::as_str);
    let id = model_id(field("slug"))
        .or_else(|| model_id(field("id")))
        .or_else(|| model_id(field("name")?.strip_prefix("models/")))?;
    Some(DiscoveredModel { id: id.to_owned(), owned_by: None })
}

fn model_id(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|id| !id.is_empty())
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
    catalog: &HashMap<String, crate::models::Provider>,
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

        let mut model = safe_remote_model(&provider.id, api_base, remote_id.clone());
        if provider.id == "shiju"
            && let Some(metadata) = crate::models::shiju_model_metadata(catalog, &remote_id)
        {
            // Only model metadata crosses providers. Routing, credentials, prices,
            // headers and provider-specific request options stay on Shiju.
            let raw_provider = crate::models::Provider {
                npm: None,
                id: provider.id.clone(),
                api: Some(api_base.to_string()),
                adapter: Some(default_adapter()),
                name: provider.name.clone(),
                env: Vec::new(),
                models: HashMap::new(),
            };
            let matched = crate::provider::from_models_dev_model(&raw_provider, metadata);
            model.name = matched.name;
            model.family = matched.family;
            model.capabilities = matched.capabilities;
            model.limit = matched.limit;
            model.release_date = matched.release_date;
        }
        provider.models.insert(remote_id, model);
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
    let catalog = if provider.id == "shiju" { crate::models::get().await } else { HashMap::new() };
    Ok(merge_remote_models(provider, &api_base, explicit_model_ids, remote_ids, &catalog))
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
            native_tool_search: false,
            catalog_execution: None,
            temperature: false,
            reasoning: false,
            reasoning_options: None,
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
