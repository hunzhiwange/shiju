//! Composio 工具核心实现模块
//!
//! 本模块提供了与 Composio 托管工具平台交互的核心功能。Composio 是一个外部工具集成平台，
//! 提供了大量预构建的工具和操作，允许代理通过统一接口调用各种第三方服务。
//!
//! # 主要功能
//!
//! - **工具列表查询**：列出可用的 Composio 应用和操作
//! - **操作执行**：代理执行 Composio 平台上的工具操作
//! - **账户连接管理**：管理已连接账户的查询和缓存
//! - **OAuth 连接**：获取第三方服务的 OAuth 连接 URL
//! - **工具模式获取**：获取工具的输入/输出参数模式定义
//!
//! # 架构说明
//!
//! `ComposioTool` 是本模块的核心结构体，负责：
//! - 维护 API 密钥和默认实体标识
//! - 管理已连接账户的运行时缓存
//! - 管理操作名称到 slug 的映射缓存
//! - 通过安全策略控制访问权限

use super::api;
use super::types;
use super::util;
use crate::app::agent::security::SecurityPolicy;
use parking_lot::RwLock;
use reqwest::Client;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use util::{
    build_tool_slug_candidates, infer_app_slug_from_action_name, normalize_action_cache_key,
    normalize_app_slug, normalize_entity_id, normalize_tool_slug,
};

pub(super) type ApiFuture<'a, T> = Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + 'a>>;

pub(super) trait ComposioApiClient: Send + Sync {
    fn list_actions_v3<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        app_name: Option<&'a str>,
    ) -> ApiFuture<'a, types::ComposioToolsResponse>;

    fn list_connected_accounts<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        app_name: Option<&'a str>,
        entity_id: Option<&'a str>,
    ) -> ApiFuture<'a, Vec<types::ComposioConnectedAccount>>;

    fn execute_action_v3<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        tool_slug: &'a str,
        params: serde_json::Value,
        text: Option<&'a str>,
        entity_id: Option<&'a str>,
        connected_account_ref: Option<&'a str>,
    ) -> ApiFuture<'a, serde_json::Value>;

    fn resolve_auth_config_id<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        app_name: &'a str,
    ) -> ApiFuture<'a, String>;

    fn get_connection_url_v3<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        auth_config_id: &'a str,
        entity_id: &'a str,
    ) -> ApiFuture<'a, types::ComposioConnectionLink>;

    fn get_tool_schema<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        slug: &'a str,
    ) -> ApiFuture<'a, serde_json::Value>;
}

struct RealComposioApiClient;

impl ComposioApiClient for RealComposioApiClient {
    fn list_actions_v3<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        app_name: Option<&'a str>,
    ) -> ApiFuture<'a, types::ComposioToolsResponse> {
        Box::pin(api::list_actions_v3(client, api_key, app_name))
    }

    fn list_connected_accounts<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        app_name: Option<&'a str>,
        entity_id: Option<&'a str>,
    ) -> ApiFuture<'a, Vec<types::ComposioConnectedAccount>> {
        Box::pin(api::list_connected_accounts(client, api_key, app_name, entity_id))
    }

    fn execute_action_v3<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        tool_slug: &'a str,
        params: serde_json::Value,
        text: Option<&'a str>,
        entity_id: Option<&'a str>,
        connected_account_ref: Option<&'a str>,
    ) -> ApiFuture<'a, serde_json::Value> {
        Box::pin(api::execute_action_v3(
            client,
            api_key,
            tool_slug,
            params,
            text,
            entity_id,
            connected_account_ref,
        ))
    }

    fn resolve_auth_config_id<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        app_name: &'a str,
    ) -> ApiFuture<'a, String> {
        Box::pin(api::resolve_auth_config_id(client, api_key, app_name))
    }

    fn get_connection_url_v3<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        auth_config_id: &'a str,
        entity_id: &'a str,
    ) -> ApiFuture<'a, types::ComposioConnectionLink> {
        Box::pin(api::get_connection_url_v3(client, api_key, auth_config_id, entity_id))
    }

    fn get_tool_schema<'a>(
        &'a self,
        client: &'a Client,
        api_key: &'a str,
        slug: &'a str,
    ) -> ApiFuture<'a, serde_json::Value> {
        Box::pin(api::get_tool_schema(client, api_key, slug))
    }
}

/// Composio 工具代理结构体
///
/// 该结构体是与 Composio 托管工具平台交互的主要接口。它封装了所有必要的状态和配置，
/// 用于通过 Composio API 调用外部工具和服务。
///
/// # 字段说明
///
/// - `api_key`：Composio API 的认证密钥
/// - `default_entity_id`：默认的实体标识符，用于多租户场景下的用户/组织隔离
/// - `security`：安全策略引用，用于控制工具访问权限和执行约束
/// - `recent_connected_accounts`：最近使用的已连接账户缓存，键为"应用名:实体ID"格式
/// - `action_slug_cache`：操作名称到 slug 的映射缓存，加速重复操作的执行
///
/// # 线程安全
///
/// 所有可变状态都使用 `RwLock` 保护，支持多线程并发读取和单线程写入。
/// 通过 `Arc` 共享的安全策略确保跨线程安全访问。
pub struct ComposioTool {
    /// Composio API 认证密钥
    api_key: String,
    /// 默认实体标识符，用于多租户隔离
    default_entity_id: String,
    /// 安全策略引用
    security: Arc<SecurityPolicy>,
    /// 已连接账户缓存，键格式为 "app_name:entity_id"
    recent_connected_accounts: RwLock<HashMap<String, String>>,
    /// 操作名称到 slug 的映射缓存
    action_slug_cache: RwLock<HashMap<String, String>>,
    api: Arc<dyn ComposioApiClient>,
}

impl ComposioTool {
    /// 创建新的 ComposioTool 实例
    ///
    /// # 参数
    ///
    /// - `api_key`：Composio API 的认证密钥，必须有值
    /// - `default_entity_id`：可选的默认实体标识符，为空时使用 "default"
    /// - `security`：安全策略的共享引用
    ///
    /// # 返回值
    ///
    /// 返回初始化完成的 `ComposioTool` 实例，缓存初始化为空
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// let tool = ComposioTool::new("api_key", Some("user_123"), Arc::new(security_policy));
    /// ```
    pub fn new(
        api_key: &str,
        default_entity_id: Option<&str>,
        security: Arc<SecurityPolicy>,
    ) -> Self {
        Self::new_with_api(api_key, default_entity_id, security, Arc::new(RealComposioApiClient))
    }

    pub(super) fn new_with_api(
        api_key: &str,
        default_entity_id: Option<&str>,
        security: Arc<SecurityPolicy>,
        api: Arc<dyn ComposioApiClient>,
    ) -> Self {
        Self {
            api_key: api_key.to_string(),
            // 对实体 ID 进行规范化处理，确保格式一致
            default_entity_id: normalize_entity_id(default_entity_id.unwrap_or("default")),
            security,
            recent_connected_accounts: RwLock::new(HashMap::new()),
            action_slug_cache: RwLock::new(HashMap::new()),
            api,
        }
    }

    /// 获取安全策略引用
    ///
    /// # 返回值
    ///
    /// 返回当前工具关联的安全策略的只读引用
    pub(crate) fn security(&self) -> &SecurityPolicy {
        &self.security
    }

    /// 获取默认实体标识符
    ///
    /// # 返回值
    ///
    /// 返回默认实体 ID 的字符串切片
    pub(crate) fn default_entity_id(&self) -> &str {
        self.default_entity_id.as_str()
    }

    /// 创建配置好超时的 HTTP 客户端
    ///
    /// 使用运行时代理配置创建客户端，设置适当的连接和请求超时。
    ///
    /// # 返回值
    ///
    /// 返回配置完成的 `reqwest::Client` 实例
    fn client(&self) -> Client {
        // 构建 HTTP 客户端，设置 60 秒总超时和 10 秒连接超时
        crate::app::agent::config::build_runtime_proxy_client_with_timeouts("tool.composio", 60, 10)
    }

    /// 列出当前用户可用的 Composio 应用和操作
    ///
    /// 调用 Composio v3 API 端点获取工具列表。如果指定了应用名称，则只返回该应用的操作；
    /// 否则返回所有可用操作。结果会自动缓存以加速后续调用。
    ///
    /// # 参数
    ///
    /// - `app_name`：可选的应用名称过滤器，用于限定返回特定应用的操作
    ///
    /// # 返回值
    ///
    /// 成功时返回 `ComposioAction` 向量，包含所有匹配的操作信息；
    /// 失败时返回 API 调用错误
    ///
    /// # 错误处理
    ///
    /// 可能因网络错误、API 认证失败或服务器错误而返回错误
    pub async fn list_actions(
        &self,
        app_name: Option<&str>,
    ) -> anyhow::Result<Vec<types::ComposioAction>> {
        // 调用 v3 API 获取操作列表
        let body = self.api.list_actions_v3(&self.client(), &self.api_key, app_name).await?;
        // 更新操作 slug 缓存，加速后续的执行调用
        api::update_action_slug_cache_from_v3(&self.action_slug_cache, &body.items);
        // 将 v3 API 响应转换为统一的 ComposioAction 类型
        Ok(util::map_v3_tools_to_actions(body.items))
    }

    /// 列出用户的已连接账户
    ///
    /// 查询指定应用和实体的已连接账户列表。已连接账户代表用户已授权的第三方服务集成。
    ///
    /// # 参数
    ///
    /// - `app_name`：可选的应用名称过滤器
    /// - `entity_id`：可选的实体标识符过滤器
    ///
    /// # 返回值
    ///
    /// 成功时返回 `ComposioConnectedAccount` 向量；
    /// 失败时返回 API 调用错误
    pub(crate) async fn list_connected_accounts(
        &self,
        app_name: Option<&str>,
        entity_id: Option<&str>,
    ) -> anyhow::Result<Vec<types::ComposioConnectedAccount>> {
        self.api.list_connected_accounts(&self.client(), &self.api_key, app_name, entity_id).await
    }

    /// 缓存已连接账户信息
    ///
    /// 将应用和实体对应的已连接账户 ID 存入缓存，避免重复查询 API。
    /// 缓存键格式为 "app_name:entity_id"。
    ///
    /// # 参数
    ///
    /// - `app_name`：应用名称
    /// - `entity_id`：实体标识符
    /// - `connected_account_id`：已连接账户的 ID
    pub(crate) fn cache_connected_account(
        &self,
        app_name: &str,
        entity_id: &str,
        connected_account_id: &str,
    ) {
        // 构建缓存键
        let key = util::connected_account_cache_key(app_name, entity_id);
        // 写入缓存
        self.recent_connected_accounts.write().insert(key, connected_account_id.to_string());
    }

    /// 从缓存获取已连接账户
    ///
    /// 根据应用名和实体 ID 查找缓存的已连接账户。
    ///
    /// # 参数
    ///
    /// - `app_name`：应用名称
    /// - `entity_id`：实体标识符
    ///
    /// # 返回值
    ///
    /// 如果缓存命中则返回账户 ID，否则返回 `None`
    fn get_cached_connected_account(&self, app_name: &str, entity_id: &str) -> Option<String> {
        let key = util::connected_account_cache_key(app_name, entity_id);
        self.recent_connected_accounts.read().get(&key).cloned()
    }

    /// 解析已连接账户引用
    ///
    /// 根据应用名和实体 ID 查找可用的已连接账户。优先从缓存查找，
    /// 缓存未命中时调用 API 查询并缓存结果。
    ///
    /// # 参数
    ///
    /// - `app_name`：可选的应用名称
    /// - `entity_id`：可选的实体标识符
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Some(account_id)` 或 `None`（当参数缺失或无可用账户时）；
    /// API 调用失败时返回错误
    ///
    /// # 账户选择策略
    ///
    /// 当存在多个已连接账户时，选择第一个可用的账户。
    /// API 返回的账户按更新时间降序排列，因此第一个可用账户是最近活跃的。
    /// 这种策略避免了在多账户场景下因无法选择而导致的"找不到已连接账户"循环问题（详见 issue #959）。
    pub(crate) async fn resolve_connected_account_ref(
        &self,
        app_name: Option<&str>,
        entity_id: Option<&str>,
    ) -> anyhow::Result<Option<String>> {
        // 规范化应用名，过滤空字符串
        let app = app_name.map(normalize_app_slug).filter(|app| !app.is_empty());
        // 规范化实体 ID
        let entity = entity_id.map(normalize_entity_id);
        // 任一参数缺失时直接返回 None
        let (Some(app), Some(entity)) = (app, entity) else {
            return Ok(None);
        };

        // 优先从缓存查找
        if let Some(cached) = self.get_cached_connected_account(&app, &entity) {
            return Ok(Some(cached));
        }

        // 缓存未命中，调用 API 查询
        let accounts = self.list_connected_accounts(Some(&app), Some(&entity)).await?;
        // API 返回的账户按 updated_at 降序排列，第一个可用账户是最近活跃的
        // 始终选择第一个可用账户，避免多账户场景下的选择困境
        let Some(first) = accounts.into_iter().find(|acct| acct.is_usable()) else {
            return Ok(None);
        };

        // 缓存查找到的账户以加速后续调用
        self.cache_connected_account(&app, &entity, &first.id);
        Ok(Some(first.id))
    }

    /// 执行 Composio 操作/工具
    ///
    /// 调用 Composio v3 API 执行指定的工具操作。支持通过结构化参数或自然语言文本执行。
    /// 会自动解析操作名称到正确的 slug，并处理已连接账户的查找。
    ///
    /// # 参数
    ///
    /// - `action_name`：操作名称，可以是完整 slug 或简短名称
    /// - `app_name_hint`：可选的应用名称提示，用于辅助 slug 解析
    /// - `params`：操作参数的 JSON 值
    /// - `text`：可选的自然语言指令，用于 NLP 执行模式
    /// - `entity_id`：可选的实体标识符
    /// - `connected_account_ref`：可选的显式已连接账户引用
    ///
    /// # 返回值
    ///
    /// 成功时返回操作结果的 JSON 值；
    /// 失败时返回详细的错误信息，包括尝试的 slug 候选和失败原因
    ///
    /// # 执行流程
    ///
    /// 1. 规范化应用名提示和实体 ID
    /// 2. 解析或查找已连接账户
    /// 3. 构建操作 slug 候选列表（优先使用缓存）
    /// 4. 依次尝试每个 slug 候选直到成功
    /// 5. 所有候选失败时返回聚合错误信息
    ///
    /// # 错误处理
    ///
    /// 当无法确定操作 slug 时，建议用户先调用 `list_actions` 刷新缓存。
    /// 错误信息包含尝试的所有候选和失败原因，便于调试。
    pub async fn execute_action(
        &self,
        action_name: &str,
        app_name_hint: Option<&str>,
        params: serde_json::Value,
        text: Option<&str>,
        entity_id: Option<&str>,
        connected_account_ref: Option<&str>,
    ) -> anyhow::Result<serde_json::Value> {
        // 规范化应用名提示，或从操作名称推断
        let app_hint = app_name_hint
            .map(normalize_app_slug)
            .filter(|app| !app.is_empty())
            .or_else(|| infer_app_slug_from_action_name(action_name));
        // 规范化实体 ID
        let normalized_entity_id = entity_id.map(normalize_entity_id);
        // 处理显式指定的已连接账户引用
        let explicit_account_ref = connected_account_ref.and_then(|candidate| {
            let trimmed = candidate.trim();
            (!trimmed.is_empty()).then_some(trimmed.to_string())
        });
        // 确定最终使用的账户引用：显式指定优先，否则自动解析
        let resolved_account_ref = if explicit_account_ref.is_some() {
            explicit_account_ref
        } else {
            self.resolve_connected_account_ref(app_hint.as_deref(), normalized_entity_id.as_deref())
                .await?
        };

        // 构建 slug 候选列表
        let mut slug_candidates = self.build_v3_slug_candidates(action_name);
        let mut prime_error = None;

        // 候选列表为空时尝试刷新操作列表
        if slug_candidates.is_empty() {
            if let Some(app) = app_hint.as_deref() {
                match self.list_actions(Some(app)).await {
                    Ok(_) => {
                        // 刷新后重新构建候选列表
                        slug_candidates = self.build_v3_slug_candidates(action_name);
                    }
                    Err(err) => {
                        prime_error =
                            Some(format!("Failed to refresh action list for app '{app}': {err}"));
                    }
                }
            }
        }

        // 仍然为空时返回错误
        if slug_candidates.is_empty() {
            anyhow::bail!(
                "Unable to determine tool slug for '{action_name}'. Run action='list' with the relevant app first to prime the cache.{}",
                prime_error.as_deref().map(|msg| format!(" ({msg})")).unwrap_or_default()
            );
        }

        // 依次尝试每个 slug 候选
        let mut v3_errors = Vec::new();
        for slug in slug_candidates {
            // 缓存成功的 slug 映射
            self.cache_action_slug(action_name, &slug);
            match self
                .api
                .execute_action_v3(
                    &self.client(),
                    &self.api_key,
                    &slug,
                    params.clone(),
                    text,
                    normalized_entity_id.as_deref(),
                    resolved_account_ref.as_deref(),
                )
                .await
            {
                Ok(result) => return Ok(result),
                Err(err) => v3_errors.push(format!("{slug}: {err}")),
            }
        }

        // 所有候选都失败，构建错误摘要
        let v3_error_summary = if v3_errors.is_empty() {
            "no v3 candidates attempted".to_string()
        } else {
            v3_errors.join(" | ")
        };

        let prime_suffix =
            prime_error.as_deref().map(|msg| format!(" ({msg})")).unwrap_or_default();

        // 根据 NLP 模式返回不同的错误信息
        if text.is_some() {
            anyhow::bail!(
                "Composio v3 NLP execute failed on candidates ({v3_error_summary}){prime_suffix}{}",
                util::build_connected_account_hint(
                    app_hint.as_deref(),
                    normalized_entity_id.as_deref(),
                    resolved_account_ref.as_deref(),
                )
            );
        }

        anyhow::bail!(
            "Composio execute failed on v3 ({v3_error_summary}){prime_suffix}{}",
            util::build_connected_account_hint(
                app_hint.as_deref(),
                normalized_entity_id.as_deref(),
                resolved_account_ref.as_deref(),
            )
        );
    }

    /// 构建 v3 API 的 slug 候选列表
    ///
    /// 根据操作名称生成可能的 slug 候选。优先使用缓存中的映射，
    /// 然后添加通过命名规则推断的候选。自动去重。
    ///
    /// # 参数
    ///
    /// - `action_name`：操作名称
    ///
    /// # 返回值
    ///
    /// 返回去重后的 slug 候选向量，保持缓存候选优先的顺序
    fn build_v3_slug_candidates(&self, action_name: &str) -> Vec<String> {
        let mut candidates = Vec::new();
        // 辅助闭包：添加非空且不重复的候选
        let mut push_candidate = |candidate: String| {
            if !candidate.is_empty() && !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        };

        // 优先使用缓存中的 slug
        if let Some(hit) = self.lookup_cached_action_slug(action_name) {
            push_candidate(hit);
        }

        // 添加通过命名规则推断的候选
        for slug in build_tool_slug_candidates(action_name) {
            push_candidate(slug);
        }

        candidates
    }

    /// 缓存操作名称到 slug 的映射
    ///
    /// 将操作别名和实际 slug 的对应关系存入缓存，加速后续相同操作的执行。
    ///
    /// # 参数
    ///
    /// - `alias`：操作别名或名称
    /// - `slug`：实际的操作 slug
    fn cache_action_slug(&self, alias: &str, slug: &str) {
        // 规范化缓存键
        let Some(key) = normalize_action_cache_key(alias) else {
            return;
        };
        let trimmed_slug = slug.trim();
        // 跳过空 slug
        if trimmed_slug.is_empty() {
            return;
        }
        self.action_slug_cache.write().insert(key, trimmed_slug.to_string());
    }

    /// 查找缓存的操作 slug
    ///
    /// 根据操作名称从缓存查找对应的 slug。
    ///
    /// # 参数
    ///
    /// - `action_name`：操作名称
    ///
    /// # 返回值
    ///
    /// 缓存命中时返回 slug，否则返回 `None`
    fn lookup_cached_action_slug(&self, action_name: &str) -> Option<String> {
        let key = normalize_action_cache_key(action_name)?;
        self.action_slug_cache.read().get(&key).cloned()
    }

    /// 获取 OAuth 连接 URL
    ///
    /// 为指定的应用或认证配置生成 OAuth 授权链接。用户通过该链接完成第三方服务的授权连接。
    /// 使用 Composio v3 API 端点。
    ///
    /// # 参数
    ///
    /// - `app_name`：可选的应用名称，用于查找对应的认证配置
    /// - `auth_config_id`：可选的认证配置 ID，优先于 app_name 使用
    /// - `entity_id`：实体标识符，用于关联连接到特定用户/组织
    ///
    /// # 返回值
    ///
    /// 成功时返回 `ComposioConnectionLink`，包含授权 URL 和相关元数据；
    /// 失败时返回错误（如参数缺失或 API 调用失败）
    ///
    /// # 参数优先级
    ///
    /// 如果提供了 `auth_config_id`，直接使用；
    /// 否则必须提供 `app_name`，通过 API 解析对应的 `auth_config_id`。
    pub async fn get_connection_url(
        &self,
        app_name: Option<&str>,
        auth_config_id: Option<&str>,
        entity_id: &str,
    ) -> anyhow::Result<types::ComposioConnectionLink> {
        // 确定使用的认证配置 ID
        let auth_config_id = match auth_config_id {
            Some(id) => id.to_string(),
            None => {
                // 未提供 auth_config_id 时，需要通过 app_name 解析
                let app = app_name.ok_or_else(|| {
                    anyhow::anyhow!("Missing 'app' or 'auth_config_id' for v3 connect")
                })?;
                self.api.resolve_auth_config_id(&self.client(), &self.api_key, app).await?
            }
        };

        // 调用 v3 API 获取连接 URL
        self.api
            .get_connection_url_v3(&self.client(), &self.api_key, &auth_config_id, entity_id)
            .await
    }

    /// 获取工具的完整模式定义
    ///
    /// 调用 `GET /api/v3/tools/{tool_slug}` 端点获取工具的详细模式，包括输入和输出参数定义。
    /// LLM 需要此模式信息来构造正确的 `params` 参数。
    ///
    /// # 参数
    ///
    /// - `tool_slug`：工具的唯一标识符 slug
    ///
    /// # 返回值
    ///
    /// 成功时返回工具模式的 JSON 值；
    /// 失败时返回 API 调用错误
    pub(crate) async fn get_tool_schema(
        &self,
        tool_slug: &str,
    ) -> anyhow::Result<serde_json::Value> {
        // 规范化 slug
        let slug = normalize_tool_slug(tool_slug);
        self.api.get_tool_schema(&self.client(), &self.api_key, &slug).await
    }

    /// 构建 v3 执行操作的请求体
    ///
    /// 工具方法，用于构建执行操作的 API 请求参数。
    ///
    /// # 参数
    ///
    /// - `tool_slug`：工具 slug
    /// - `params`：操作参数
    /// - `text`：可选的自然语言文本
    /// - `entity_id`：可选的实体 ID
    /// - `connected_account_ref`：可选的已连接账户引用
    ///
    /// # 返回值
    ///
    /// 返回元组 (endpoint_path, request_body)
    pub(crate) fn build_execute_action_v3_request(
        tool_slug: &str,
        params: serde_json::Value,
        text: Option<&str>,
        entity_id: Option<&str>,
        connected_account_ref: Option<&str>,
    ) -> (String, serde_json::Value) {
        api::build_execute_action_v3_request(
            tool_slug,
            params,
            text,
            entity_id,
            connected_account_ref,
        )
    }

    /// 构建 v3 操作列表的查询参数
    ///
    /// 工具方法，用于构建查询操作列表的 URL 查询参数。
    ///
    /// # 参数
    ///
    /// - `app_name`：可选的应用名称过滤器
    ///
    /// # 返回值
    ///
    /// 返回键值对形式的查询参数向量
    pub(crate) fn build_list_actions_v3_query(app_name: Option<&str>) -> Vec<(String, String)> {
        api::build_list_actions_v3_query(app_name)
    }
}
#[cfg(test)]
#[path = "core_tests.rs"]
mod core_tests;
