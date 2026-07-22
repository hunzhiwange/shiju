//! Composio API 客户端模块
//!
//! 本模块提供与 Composio 平台交互的 HTTP API 客户端功能，包括：
//! - 获取可用的工具/动作列表
//! - 执行工具动作
//! - 管理已连接的账户
//! - 获取授权配置
//! - 生成连接 URL
//!
//! Composio 是一个集成平台，允许 AI 代理连接并操作数百个第三方应用。
//! 本模块通过 Composio 的 v2 和 v3 API 与其后端服务进行通信。

use crate::app::agent::tools::composio::types::{
    ComposioAuthConfigsResponse, ComposioConnectedAccountsResponse, ComposioToolsResponse,
};
use crate::app::agent::tools::composio::util::{
    ensure_https, normalize_app_slug, normalize_tool_slug, response_error,
};
use anyhow::Context;
use reqwest::Client;
use serde_json::json;

/// Composio v3 API 基础 URL
///
/// 用于访问 Composio 平台的最新 API 版本，提供完整的工具管理功能
const COMPOSIO_API_BASE_V3: &str = "https://backend.composio.dev/api/v3";

/// 工具版本标识符
///
/// 指定使用最新版本的工具定义，确保获取最新的工具参数和行为
const COMPOSIO_TOOL_VERSION_LATEST: &str = "latest";

/// 构建 v3 API 工具列表查询参数
///
/// 根据提供的应用名称构建用于查询 Composio v3 API 的查询参数集合。
/// 查询参数包括分页限制和工具版本信息，如果指定了应用名称，还会包含应用过滤条件。
///
/// # 参数
///
/// * `app_name` - 可选的应用名称，用于过滤特定应用的工具。如果为 None 或空字符串，
///   则查询所有可用工具
///
/// # 返回值
///
/// 返回一个键值对向量，包含所有查询参数
///
/// # 示例
///
/// ```ignore
/// let query = build_list_actions_v3_query(Some("gmail"));
/// // 返回: [("limit", "200"), ("toolkit_versions", "latest"), ("toolkits", "gmail"), ("toolkit_slug", "gmail")]
/// ```
pub(crate) fn build_list_actions_v3_query(app_name: Option<&str>) -> Vec<(String, String)> {
    // 构建基础查询参数：限制返回 200 个结果，使用最新版本
    let mut query = vec![
        ("limit".to_string(), "200".to_string()),
        ("toolkit_versions".to_string(), COMPOSIO_TOOL_VERSION_LATEST.to_string()),
    ];

    // 如果指定了应用名称，添加过滤条件
    if let Some(app) = app_name.map(str::trim).filter(|app| !app.is_empty()) {
        query.push(("toolkits".to_string(), app.to_string()));
        query.push(("toolkit_slug".to_string(), app.to_string()));
    }

    query
}

/// 从 Composio v3 API 获取工具列表
///
/// 调用 Composio 的 v3 API 端点获取可用的工具（动作）列表。
/// 可以获取所有工具或特定应用的工具集合。
///
/// # 参数
///
/// * `client` - HTTP 客户端引用，用于发送请求
/// * `api_key` - Composio API 密钥，用于身份验证
/// * `app_name` - 可选的应用名称，用于过滤特定应用的工具
///
/// # 返回值
///
/// 成功时返回 `ComposioToolsResponse`，包含工具列表和分页信息；
/// 失败时返回错误，包括 HTTP 错误或 JSON 解析错误
///
/// # 错误
///
/// - 如果 API 返回非成功状态码，返回包含错误详情的 anyhow 错误
/// - 如果 JSON 响应无法解析为 `ComposioToolsResponse`，返回解析错误
///
/// # 示例
///
/// ```ignore
/// let client = Client::new();
/// let tools = list_actions_v3(&client, "your-api-key", Some("slack")).await?;
/// ```
pub(crate) async fn list_actions_v3(
    client: &Client,
    api_key: &str,
    app_name: Option<&str>,
) -> anyhow::Result<ComposioToolsResponse> {
    // 构建 API URL
    let url = format!("{COMPOSIO_API_BASE_V3}/tools");

    // 构建请求，添加 API 密钥头部和查询参数
    let req =
        client.get(&url).header("x-api-key", api_key).query(&build_list_actions_v3_query(app_name));

    // 发送请求并等待响应
    let resp = req.send().await?;

    // 检查响应状态码
    if !resp.status().is_success() {
        let err = response_error(resp).await;
        anyhow::bail!("Composio v3 API error: {err}");
    }

    // 解析 JSON 响应
    let body: ComposioToolsResponse =
        resp.json().await.context("Failed to decode Composio v3 tools response")?;
    Ok(body)
}

/// 获取已连接的账户列表
///
/// 从 Composio v3 API 获取当前用户已连接的第三方应用账户列表。
/// 支持按应用名称和实体 ID 进行过滤。
///
/// # 参数
///
/// * `client` - HTTP 客户端引用
/// * `api_key` - Composio API 密钥
/// * `app_name` - 可选的应用名称，用于过滤特定应用的连接账户
/// * `entity_id` - 可选的实体 ID（用户标识符），用于过滤特定用户的连接
///
/// # 返回值
///
/// 成功时返回 `ComposioConnectedAccount` 向量，包含所有匹配的连接账户；
/// 失败时返回错误
///
/// # 错误
///
/// - API 返回非成功状态码时返回错误
/// - JSON 解析失败时返回错误
///
/// # 示例
///
/// ```ignore
/// let accounts = list_connected_accounts(&client, "api-key", Some("github"), Some("user-123")).await?;
/// for account in accounts {
///     println!("Connected to: {}", account.app_name);
/// }
/// ```
pub(crate) async fn list_connected_accounts(
    client: &Client,
    api_key: &str,
    app_name: Option<&str>,
    entity_id: Option<&str>,
) -> anyhow::Result<Vec<crate::app::agent::tools::composio::types::ComposioConnectedAccount>> {
    // 构建 API URL
    let url = format!("{COMPOSIO_API_BASE_V3}/connected_accounts");
    let mut req = client.get(&url).header("x-api-key", api_key);

    // 添加基础查询参数：分页、排序和状态过滤
    req = req.query(&[
        ("limit", "50"),
        ("order_by", "updated_at"),
        ("order_direction", "desc"),
        ("statuses", "INITIALIZING"),
        ("statuses", "ACTIVE"),
        ("statuses", "INITIATED"),
    ]);

    // 如果指定了应用名称，添加应用过滤条件
    if let Some(app) = app_name.map(normalize_app_slug).filter(|app| !app.is_empty()) {
        req = req.query(&[("toolkit_slugs", app.as_str())]);
    }

    // 如果指定了实体 ID，添加用户过滤条件
    if let Some(entity) = entity_id {
        req = req.query(&[("user_ids", entity)]);
    }

    // 发送请求并处理响应
    let resp = req.send().await?;
    if !resp.status().is_success() {
        let err = response_error(resp).await;
        anyhow::bail!("Composio v3 connected accounts lookup failed: {err}");
    }

    // 解析响应
    let body: ComposioConnectedAccountsResponse =
        resp.json().await.context("Failed to decode Composio v3 connected accounts response")?;
    Ok(body.items)
}

/// 构建执行工具动作的 v3 API 请求
///
/// 根据提供的参数构建用于调用 Composio v3 执行端点的请求 URL 和请求体。
/// 该函数支持两种参数传递方式：结构化参数或自然语言描述。
///
/// # 参数
///
/// * `tool_slug` - 工具的唯一标识符（slug）
/// * `params` - 结构化的工具参数，作为 JSON 值
/// * `text` - 可选的自然语言描述，当提供时优先使用，让 Composio 的 NLP 引擎解析参数
/// * `entity_id` - 可选的实体 ID，用于标识执行动作的用户
/// * `connected_account_ref` - 可选的已连接账户引用，用于指定特定的连接
///
/// # 返回值
///
/// 返回一个元组，包含：
/// - 完整的 API URL 字符串
/// - JSON 格式的请求体
///
/// # 说明
///
/// Composio v3 执行端点接受两种互斥的参数传递方式：
/// 1. `arguments` - 结构化的 JSON 参数
/// 2. `text` - 自然语言描述
///
/// 优先使用 `text` 方式可以让 Composio 的自然语言处理引擎自动解析正确的参数，
/// 这解决了社区报告的"持续猜测并失败"的问题。
///
/// # 示例
///
/// ```ignore
/// let (url, body) = build_execute_action_v3_request(
///     "GMAIL_SEND_EMAIL",
///     json!({"to": "user@example.com", "subject": "Hello"}),
///     None,
///     Some("user-123"),
///     None,
/// );
/// ```
pub(crate) fn build_execute_action_v3_request(
    tool_slug: &str,
    params: serde_json::Value,
    text: Option<&str>,
    entity_id: Option<&str>,
    connected_account_ref: Option<&str>,
) -> (String, serde_json::Value) {
    // 构建执行端点 URL
    let url = format!("{COMPOSIO_API_BASE_V3}/tools/execute/{tool_slug}");

    // 处理连接账户引用，去除空白字符
    let account_ref = connected_account_ref.and_then(|candidate| {
        let trimmed_candidate = candidate.trim();
        (!trimmed_candidate.is_empty()).then_some(trimmed_candidate)
    });

    // 初始化请求体，指定使用最新版本
    let mut body = json!({
        "version": COMPOSIO_TOOL_VERSION_LATEST,
    });

    // Composio v3 执行端点接受结构化的 `arguments` 或自然语言的 `text` 描述（互斥）
    // 当调用者提供文本时优先使用 `text`，让 Composio 的 NLP 解析正确的参数
    // 这是解决社区报告的"持续猜测并失败"问题的主要修复方案
    if let Some(nl_text) = text {
        body["text"] = json!(nl_text);
    } else {
        body["arguments"] = params;
    }

    // 添加可选的实体 ID
    if let Some(entity) = entity_id {
        body["user_id"] = json!(entity);
    }

    // 添加可选的连接账户引用
    if let Some(account_ref) = account_ref {
        body["connected_account_id"] = json!(account_ref);
    }

    (url, body)
}

/// 执行 Composio 工具动作（v3 API）
///
/// 调用 Composio v3 API 执行指定的工具动作。支持通过结构化参数或自然语言描述
/// 来指定动作参数。
///
/// # 参数
///
/// * `client` - HTTP 客户端引用
/// * `api_key` - Composio API 密钥
/// * `tool_slug` - 要执行的工具的唯一标识符
/// * `params` - 结构化的工具参数（JSON 格式）
/// * `text` - 可选的自然语言描述，用于 NLP 参数解析
/// * `entity_id` - 可选的实体 ID，标识执行用户
/// * `connected_account_ref` - 可选的已连接账户引用
///
/// # 返回值
///
/// 成功时返回工具执行的 JSON 结果；失败时返回错误
///
/// # 错误
///
/// - 如果 URL 不是 HTTPS，返回安全错误
/// - 如果 API 返回非成功状态码，返回包含错误详情的错误
/// - 如果 JSON 解析失败，返回解析错误
///
/// # 安全性
///
/// 该函数会验证 URL 使用 HTTPS 协议，以确保 API 密钥和参数的传输安全
///
/// # 示例
///
/// ```ignore
/// let result = execute_action_v3(
///     &client,
///     "api-key",
///     "SLACK_SEND_MESSAGE",
///     json!({"channel": "general", "text": "Hello!"}),
///     None,
///     Some("user-123"),
///     None,
/// ).await?;
/// ```
pub(crate) async fn execute_action_v3(
    client: &Client,
    api_key: &str,
    tool_slug: &str,
    params: serde_json::Value,
    text: Option<&str>,
    entity_id: Option<&str>,
    connected_account_ref: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    // 构建请求 URL 和请求体
    let (url, body) =
        build_execute_action_v3_request(tool_slug, params, text, entity_id, connected_account_ref);

    // 确保 URL 使用 HTTPS 协议（安全检查）
    ensure_https(&url)?;

    // 发送 POST 请求执行工具
    let resp = client.post(&url).header("x-api-key", api_key).json(&body).send().await?;

    // 检查响应状态
    if !resp.status().is_success() {
        let err = response_error(resp).await;
        anyhow::bail!("Composio v3 action execution failed: {err}");
    }

    // 解析并返回结果
    let result: serde_json::Value =
        resp.json().await.context("Failed to decode Composio v3 execute response")?;
    Ok(result)
}

/// 获取授权连接 URL（v3 API）
///
/// 生成一个授权 URL，用户可以通过该 URL 连接第三方应用到 Composio 平台。
/// 使用 v3 API 的授权配置 ID 来创建连接链接。
///
/// # 参数
///
/// * `client` - HTTP 客户端引用
/// * `api_key` - Composio API 密钥
/// * `auth_config_id` - 授权配置 ID，定义了应用和权限范围
/// * `entity_id` - 实体 ID（用户标识符），用于关联连接
///
/// # 返回值
///
/// 成功时返回 `ComposioConnectionLink`，包含：
/// - `redirect_url` - 用户需要访问的授权 URL
/// - `connected_account_id` - 已连接账户的 ID（如果有）
///
/// # 错误
///
/// - API 调用失败时返回错误
/// - 响应中缺少重定向 URL 时返回错误
/// - JSON 解析失败时返回错误
///
/// # 示例
///
/// ```ignore
/// let link = get_connection_url_v3(&client, "api-key", "auth-config-123", "user-456").await?;
/// println!("请访问以下 URL 进行授权: {}", link.redirect_url);
/// ```
pub(crate) async fn get_connection_url_v3(
    client: &Client,
    api_key: &str,
    auth_config_id: &str,
    entity_id: &str,
) -> anyhow::Result<crate::app::agent::tools::composio::types::ComposioConnectionLink> {
    // 构建 API URL
    let url = format!("{COMPOSIO_API_BASE_V3}/connected_accounts/link");

    // 构建请求体
    let body = json!({
        "auth_config_id": auth_config_id,
        "user_id": entity_id,
    });

    // 发送 POST 请求创建连接链接
    let resp = client.post(&url).header("x-api-key", api_key).json(&body).send().await?;

    // 检查响应状态
    if !resp.status().is_success() {
        let err = response_error(resp).await;
        anyhow::bail!("Composio v3 connect failed: {err}");
    }

    // 解析响应并提取重定向 URL
    let result: serde_json::Value =
        resp.json().await.context("Failed to decode Composio v3 connect response")?;
    let redirect_url = crate::app::agent::tools::composio::util::extract_redirect_url(&result)
        .ok_or_else(|| anyhow::anyhow!("No redirect URL in Composio v3 response"))?;

    // 返回连接链接对象
    Ok(crate::app::agent::tools::composio::types::ComposioConnectionLink {
        redirect_url,
        connected_account_id:
            crate::app::agent::tools::composio::util::extract_connected_account_id(&result),
    })
}

/// 获取授权连接 URL（v2 API，已废弃）
///
/// 使用 Composio v2 API 生成授权 URL。此函数保留用于向后兼容，
/// 但应优先使用 `get_connection_url_v3`。
///
/// # 参数
///
/// * `client` - HTTP 客户端引用
/// * `api_key` - Composio API 密钥
/// * `app_name` - 应用名称/集成 ID
/// * `entity_id` - 实体 ID（用户标识符）
///
/// # 返回值
///
/// 成功时返回 `ComposioConnectionLink`，包含授权 URL 和连接账户 ID
///
/// # 错误
///
/// - API 调用失败时返回错误
/// - 响应中缺少重定向 URL 时返回错误
///
/// # 注意
///
/// 此函数使用已废弃的 v2 API，建议使用 `get_connection_url_v3` 替代
#[allow(dead_code)]
pub(crate) async fn get_connection_url_v2(
    client: &Client,
    api_key: &str,
    app_name: &str,
    entity_id: &str,
) -> anyhow::Result<crate::app::agent::tools::composio::types::ComposioConnectionLink> {
    // 构建 v2 API URL
    let url = "https://backend.composio.dev/api/connectedAccounts";

    // 构建请求体（使用 v2 API 的字段命名）
    let body = json!({
        "integrationId": app_name,
        "entityId": entity_id,
    });

    // 发送 POST 请求
    let resp = client.post(url).header("x-api-key", api_key).json(&body).send().await?;

    // 检查响应状态
    if !resp.status().is_success() {
        let err = response_error(resp).await;
        anyhow::bail!("Composio v2 connect failed: {err}");
    }

    // 解析响应并提取重定向 URL
    let result: serde_json::Value =
        resp.json().await.context("Failed to decode Composio v2 connect response")?;
    let redirect_url = crate::app::agent::tools::composio::util::extract_redirect_url(&result)
        .ok_or_else(|| anyhow::anyhow!("No redirect URL in Composio v2 response"))?;

    // 返回连接链接对象
    Ok(crate::app::agent::tools::composio::types::ComposioConnectionLink {
        redirect_url,
        connected_account_id:
            crate::app::agent::tools::composio::util::extract_connected_account_id(&result),
    })
}

/// 获取工具的参数 schema
///
/// 从 Composio v3 API 获取指定工具的详细参数定义，包括参数类型、
/// 必填字段、描述等信息。这些信息可用于验证和文档生成。
///
/// # 参数
///
/// * `client` - HTTP 客户端引用
/// * `api_key` - Composio API 密钥
/// * `tool_slug` - 工具的唯一标识符
///
/// # 返回值
///
/// 成功时返回工具 schema 的 JSON 表示；失败时返回错误
///
/// # 错误
///
/// - 如果 URL 不是 HTTPS，返回安全错误
/// - 如果工具不存在或 API 调用失败，返回错误
/// - 如果 JSON 解析失败，返回解析错误
///
/// # 安全性
///
/// 该函数会验证 URL 使用 HTTPS 协议
///
/// # 示例
///
/// ```ignore
/// let schema = get_tool_schema(&client, "api-key", "GMAIL_SEND_EMAIL").await?;
/// println!("Tool schema: {}", serde_json::to_string_pretty(&schema)?);
/// ```
pub(crate) async fn get_tool_schema(
    client: &Client,
    api_key: &str,
    tool_slug: &str,
) -> anyhow::Result<serde_json::Value> {
    // 规范化工具 slug
    let slug = normalize_tool_slug(tool_slug);

    // 构建 API URL
    let url = format!("{COMPOSIO_API_BASE_V3}/tools/{slug}");

    // 安全检查：确保使用 HTTPS
    ensure_https(&url)?;

    // 发送 GET 请求获取工具 schema
    let resp = client
        .get(&url)
        .header("x-api-key", api_key)
        .query(&[("version", COMPOSIO_TOOL_VERSION_LATEST)])
        .send()
        .await?;

    // 检查响应状态
    if !resp.status().is_success() {
        let err = response_error(resp).await;
        anyhow::bail!("Composio v3 tool schema lookup failed for '{slug}': {err}");
    }

    // 解析并返回 schema
    let body: serde_json::Value =
        resp.json().await.context("Failed to decode Composio v3 tool schema response")?;
    Ok(body)
}

/// 解析应用的授权配置 ID
///
/// 从 Composio v3 API 获取指定应用的授权配置列表，并返回首选的配置 ID。
/// 优先选择已启用的配置，如果没有则返回第一个可用配置。
///
/// # 参数
///
/// * `client` - HTTP 客户端引用
/// * `api_key` - Composio API 密钥
/// * `app_name` - 应用的唯一标识符（toolkit slug）
///
/// # 返回值
///
/// 成功时返回授权配置 ID 字符串；失败时返回错误
///
/// # 错误
///
/// - 如果没有找到任何授权配置，返回错误提示用户先创建配置
/// - 如果 API 调用失败，返回错误
/// - 如果返回的配置列表中没有可用配置，返回错误
///
/// # 示例
///
/// ```ignore
/// let auth_config_id = resolve_auth_config_id(&client, "api-key", "github").await?;
/// println!("Auth config ID: {}", auth_config_id);
/// ```
pub(crate) async fn resolve_auth_config_id(
    client: &Client,
    api_key: &str,
    app_name: &str,
) -> anyhow::Result<String> {
    // 构建 API URL
    let url = format!("{COMPOSIO_API_BASE_V3}/auth_configs");

    // 发送 GET 请求获取授权配置列表
    let resp = client
        .get(&url)
        .header("x-api-key", api_key)
        .query(&[("toolkit_slug", app_name), ("show_disabled", "true"), ("limit", "25")])
        .send()
        .await?;

    // 检查响应状态
    if !resp.status().is_success() {
        let err = response_error(resp).await;
        anyhow::bail!("Composio v3 auth config lookup failed: {err}");
    }

    // 解析响应
    let body: ComposioAuthConfigsResponse =
        resp.json().await.context("Failed to decode Composio v3 auth configs response")?;

    // 检查是否有配置项
    if body.items.is_empty() {
        anyhow::bail!(
            "No auth config found for toolkit '{app_name}'. Create one in Composio first."
        );
    }

    // 优先选择已启用的配置，否则选择第一个配置
    let preferred = body
        .items
        .iter()
        .find(|cfg| cfg.is_enabled())
        .or_else(|| body.items.first())
        .context("No usable auth config returned by Composio")?;

    Ok(preferred.id.clone())
}

/// 从 v3 API 响应更新工具 slug 缓存
///
/// 处理从 Composio v3 API 获取的工具列表，并将工具的 slug 和名称
/// 映射关系更新到本地缓存中。这有助于后续通过工具名称或 slug 快速查找。
///
/// # 参数
///
/// * `cache` - 工具 slug 缓存的读写锁引用，存储规范化键到原始 slug 的映射
/// * `items` - 从 v3 API 获取的工具列表
///
/// # 缓存策略
///
/// 对于每个工具项：
/// 1. 如果工具有 slug，将规范化的 slug 作为键，原始 slug 作为值存入缓存
/// 2. 如果工具有名称，将规范化的名称作为键，原始 slug 作为值存入缓存
/// 3. 同时存在 slug 和名称时，两者都会建立映射关系
///
/// # 示例
///
/// ```ignore
/// use parking_lot::RwLock;
/// use std::collections::HashMap;
///
/// let cache = RwLock::new(HashMap::new());
/// let tools = vec![/* ... */];
/// update_action_slug_cache_from_v3(&cache, &tools);
///
/// // 现在可以通过名称查找 slug
/// if let Some(slug) = cache.read().get("gmail_send_email") {
///     println!("Found slug: {}", slug);
/// }
/// ```
pub(crate) fn update_action_slug_cache_from_v3(
    cache: &parking_lot::RwLock<std::collections::HashMap<String, String>>,
    items: &[crate::app::agent::tools::composio::types::ComposioV3Tool],
) {
    for item in items {
        // 获取工具的 slug（优先）或名称作为标识符
        let Some(slug) = item.slug.as_deref().or(item.name.as_deref()) else {
            continue;
        };

        // 将规范化的 slug 作为键存入缓存
        if let Some(key) =
            crate::app::agent::tools::composio::util::normalize_action_cache_key(slug)
        {
            cache.write().insert(key, slug.to_string());
        }

        // 如果工具还有名称，也建立名称到 slug 的映射
        if let Some(name) = item.name.as_deref() {
            if let Some(key) =
                crate::app::agent::tools::composio::util::normalize_action_cache_key(name)
            {
                cache.write().insert(key, slug.to_string());
            }
        }
    }
}

/// 测试辅助函数：获取工具版本标识符
///
/// 返回当前使用的工具版本字符串，用于测试验证。
///
/// # 返回值
///
/// 返回 `"latest"` 字符串
#[cfg(test)]
pub(crate) fn composio_tool_version_latest() -> &'static str {
    COMPOSIO_TOOL_VERSION_LATEST
}

/// 测试辅助函数：获取 v3 API 基础 URL
///
/// 返回 Composio v3 API 的基础 URL，用于测试验证。
///
/// # 返回值
///
/// 返回 v3 API 的基础 URL 字符串
#[cfg(test)]
pub(crate) fn composio_api_base_v3() -> &'static str {
    COMPOSIO_API_BASE_V3
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod api_tests;
