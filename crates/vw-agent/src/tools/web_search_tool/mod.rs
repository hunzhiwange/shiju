//! 网络搜索工具（多提供方）
//!
//! 通用网络搜索工具，支持 DuckDuckGo、Brave、Serper、Firecrawl、Tavily 等多种提供方。

use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use vw_api_types::tools::ToolResultContentDto;

#[derive(Debug, Clone, Serialize)]
struct SearchResultItem {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snippet: Option<String>,
}

/// 网络搜索工具结构体
///
/// 支持多种搜索引擎提供方：
/// - `duckduckgo`：DuckDuckGo（免费，无需 API 密钥）
/// - `brave`：Brave Search（需要 API 密钥）
/// - `serper`：Serper Google Search（需要 API 密钥）
/// - `google`：Google Search（通过 Serper API，需要 API 密钥）
/// - `bing`：Bing Search（通过 Serper API，需要 API 密钥）
/// - `firecrawl`：Firecrawl（需要 API 密钥和 `firecrawl` 特性）
/// - `tavily`：Tavily（需要 API 密钥）
///
/// # 字段说明
/// - `security`：安全策略，用于权限控制和速率限制
/// - `provider`：搜索引擎提供方名称（小写）
/// - `api_keys`：API 密钥列表（支持多个密钥轮询使用）
/// - `api_url`：自定义 API 端点 URL（可选）
/// - `max_results`：返回结果的最大数量（1-10）
/// - `timeout_secs`：HTTP 请求超时时间（秒）
/// - `user_agent`：HTTP User-Agent 字符串
/// - `key_index`：当前密钥索引（用于轮询）
pub struct WebSearchTool {
    /// 安全策略引用，用于检查执行权限和速率限制
    security: Arc<SecurityPolicy>,
    /// 搜索提供方名称（已转换为小写）
    provider: String,
    /// API 密钥列表（支持逗号分隔的多个密钥，用于负载均衡）
    pub(crate) api_keys: Vec<String>,
    /// 可选的自定义 API URL（覆盖默认端点）
    api_url: Option<String>,
    /// 返回结果的最大数量（范围 1-10）
    max_results: usize,
    /// HTTP 请求超时时间（秒，最小值为 1）
    timeout_secs: u64,
    /// HTTP 请求的 User-Agent 字符串
    user_agent: String,
    /// 当前使用的 API 密钥索引（原子操作，支持并发轮询）
    key_index: Arc<AtomicUsize>,
}

#[derive(Debug, Clone, Deserialize)]
struct Args {
    query: String,
    #[serde(default, alias = "numResults", alias = "num_results", alias = "max_results")]
    num: Option<usize>,
    #[serde(default, alias = "language", alias = "lang")]
    lr: Option<String>,
}

impl WebSearchTool {
    /// 创建新的网络搜索工具实例
    ///
    /// # 参数
    /// - `security`：安全策略，用于权限检查和速率限制
    /// - `provider`：搜索引擎提供方，支持 `duckduckgo`、`brave`、`serper`、`google`、`bing`
    /// - `api_key`：API 密钥（支持逗号分隔的多个密钥，用于轮询负载均衡）
    /// - `api_url`：可选的自定义 API URL（覆盖默认端点）
    /// - `max_results`：返回结果的最大数量（1-10，超出范围自动调整）
    /// - `timeout_secs`：请求超时时间（秒，最小值为 1）
    /// - `user_agent`：HTTP User-Agent 字符串
    ///
    /// # 示例
    /// ```ignore
    /// use std::sync::Arc;
    /// use vibe_window::app::agent::security::SecurityPolicy;
    /// use vibe_window::app::agent::tools::WebSearchTool;
    ///
    /// let security = Arc::new(SecurityPolicy::default());
    /// let tool = WebSearchTool::new(
    ///     security,
    ///     "duckduckgo".to_string(),
    ///     None,
    ///     None,
    ///     5,
    ///     30,
    ///     vw_config_types::tools::DEFAULT_USER_AGENT.to_string(),
    /// );
    /// ```
    pub fn new(
        security: Arc<SecurityPolicy>,
        provider: String,
        api_key: Option<String>,
        api_url: Option<String>,
        max_results: usize,
        timeout_secs: u64,
        user_agent: String,
    ) -> Self {
        // 解析逗号分隔的 API 密钥列表，用于轮询负载均衡
        let api_keys = api_key
            .as_ref()
            .map(|keys| {
                keys.split(',').map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).collect()
            })
            .unwrap_or_default();

        Self {
            security,
            provider: provider.trim().to_lowercase(),
            api_keys,
            api_url,
            max_results: max_results.clamp(1, 10),
            timeout_secs: timeout_secs.max(1),
            user_agent: vw_config_types::tools::resolve_user_agent(&user_agent),
            key_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn with_max_results(&self, max_results: usize) -> Self {
        Self {
            security: self.security.clone(),
            provider: self.provider.clone(),
            api_keys: self.api_keys.clone(),
            api_url: self.api_url.clone(),
            max_results: max_results.clamp(1, 10),
            timeout_secs: self.timeout_secs,
            user_agent: self.user_agent.clone(),
            key_index: self.key_index.clone(),
        }
    }

    fn resolve_serper_endpoint(&self, provider: &str) -> String {
        let default_endpoint = match provider {
            "bing" => "https://bing.serper.dev/search",
            _ => "https://google.serper.dev/search",
        };

        self.api_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                if value.ends_with("/search") {
                    value.to_string()
                } else {
                    format!("{}/search", value.trim_end_matches('/'))
                }
            })
            .unwrap_or_else(|| default_endpoint.to_string())
    }

    /// 获取下一个 API 密钥（轮询策略）
    ///
    /// 使用原子计数器实现轮询负载均衡，确保多个 API 密钥均匀使用。
    /// 当只有一个密钥时始终返回该密钥。
    ///
    /// # 返回值
    /// - `Some(String)`：下一个可用的 API 密钥
    /// - `None`：未配置任何 API 密钥
    pub(crate) fn get_next_api_key(&self) -> Option<String> {
        if self.api_keys.is_empty() {
            return None;
        }
        let idx = self.key_index.fetch_add(1, Ordering::Relaxed) % self.api_keys.len();
        Some(self.api_keys[idx].clone())
    }

    fn provider_label(&self) -> String {
        match self.provider.as_str() {
            "ddg" | "duckduckgo" => "DuckDuckGo".to_string(),
            "brave" => "Brave".to_string(),
            "serper" => "Serper".to_string(),
            "google" => "Google".to_string(),
            "bing" => "Bing".to_string(),
            "firecrawl" => "Firecrawl".to_string(),
            "tavily" => "Tavily".to_string(),
            other => other.to_string(),
        }
    }

    fn parse_formatted_results(output: &str) -> (Option<String>, Vec<SearchResultItem>) {
        let mut lines = output.lines();
        let Some(first_line) = lines.next().map(str::trim) else {
            return (None, Vec::new());
        };

        if first_line.starts_with("No results found for:") {
            return (None, Vec::new());
        }

        let provider = first_line
            .rsplit_once("(via ")
            .and_then(|(_, rest)| rest.strip_suffix(')'))
            .map(ToOwned::to_owned);

        let mut results = Vec::<SearchResultItem>::new();
        let mut current: Option<SearchResultItem> = None;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let is_result_header = trimmed
                .split_once('.')
                .and_then(|(idx, title)| idx.parse::<usize>().ok().map(|_| title.trim()))
                .filter(|title| !title.is_empty());

            if let Some(title) = is_result_header {
                if let Some(existing) = current.take() {
                    results.push(existing);
                }
                current =
                    Some(SearchResultItem { title: title.to_string(), url: None, snippet: None });
                continue;
            }

            let Some(current_item) = current.as_mut() else {
                continue;
            };

            if current_item.url.is_none()
                && (trimmed.starts_with("http://") || trimmed.starts_with("https://"))
            {
                current_item.url = Some(trimmed.to_string());
                continue;
            }

            current_item.snippet = Some(match current_item.snippet.take() {
                Some(existing) if !existing.is_empty() => format!("{existing} {trimmed}"),
                _ => trimmed.to_string(),
            });
        }

        if let Some(existing) = current {
            results.push(existing);
        }

        (provider, results)
    }

    /// 构建 HTTP 客户端
    ///
    /// 创建配置好的 `reqwest::Client` 实例，包含：
    /// - 请求超时设置（非 WASM 目标）
    /// - User-Agent 头
    ///
    /// # 返回值
    /// - `Ok(Client)`：成功构建的 HTTP 客户端
    /// - `Err`：客户端构建失败
    fn build_client(&self) -> anyhow::Result<reqwest::Client> {
        let builder = reqwest::Client::builder();
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.timeout(Duration::from_secs(self.timeout_secs));

        let builder = builder
            .user_agent(self.user_agent.as_str())
            .default_headers(super::http_headers::default_headers());

        builder.build().map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    /// 使用 DuckDuckGo HTML 接口执行网络搜索（免费，无需 API 密钥）
    ///
    /// 通过 DuckDuckGo 的 HTML 搜索页面获取结果，然后解析 HTML 提取
    /// 搜索结果。
    ///
    /// # 参数
    /// - `query`：搜索查询字符串
    ///
    /// # 返回值
    /// - `Ok(String)`：格式化的搜索结果（包含标题、URL 和摘要）
    /// - `Err`：请求失败或解析错误
    ///
    /// # 注意
    /// 此方法不需要 API 密钥，但可能受速率限制影响
    async fn search_duckduckgo(&self, query: &str) -> anyhow::Result<String> {
        let encoded_query = urlencoding::encode(query);
        let search_url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

        let client = self.build_client()?;

        let response = client
            .get(&search_url)
            .header(
                reqwest::header::ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if !response.status().is_success() {
            anyhow::bail!("DuckDuckGo search failed with status: {}", response.status());
        }

        let html = response.text().await.map_err(|e| anyhow::anyhow!(e.to_string()))?;
        self.parse_duckduckgo_results(&html, query)
    }

    /// 解析 DuckDuckGo 搜索的 HTML 响应为格式化结果
    ///
    /// 使用正则表达式从 HTML 中提取：
    /// - 结果链接（`<a class="result__a">`）
    /// - 摘要片段（`<a class="result__snippet">`）
    ///
    /// # 参数
    /// - `html`：DuckDuckGo 返回的 HTML 响应
    /// - `query`：原始搜索查询（用于上下文显示）
    ///
    /// # 返回值
    /// - `Ok(String)`：格式化的搜索结果字符串
    /// - `Err`：正则表达式编译失败
    ///
    /// # 输出格式
    /// ```text
    /// Search results for: <query> (via DuckDuckGo)
    /// 1. <标题>
    ///    <URL>
    ///    <摘要>
    /// 2. ...
    /// ```
    pub(crate) fn parse_duckduckgo_results(
        &self,
        html: &str,
        query: &str,
    ) -> anyhow::Result<String> {
        // 提取结果链接：<a class="result__a" href="...">Title</a>
        let link_regex = Regex::new(
            r#"<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="([^"]+)"[^>]*>([\s\S]*?)</a>"#,
        )?;

        // 提取摘要片段：<a class="result__snippet">...</a>
        let snippet_regex = Regex::new(r#"<a class="result__snippet[^"]*"[^>]*>([\s\S]*?)</a>"#)?;

        let link_matches: Vec<_> =
            link_regex.captures_iter(html).take(self.max_results + 2).collect();

        let snippet_matches: Vec<_> =
            snippet_regex.captures_iter(html).take(self.max_results + 2).collect();

        if link_matches.is_empty() {
            return Ok(format!("No results found for: {}", query));
        }

        let mut lines = vec![format!("Search results for: {} (via DuckDuckGo)", query)];

        let count = link_matches.len().min(self.max_results);

        for i in 0..count {
            let caps = &link_matches[i];
            let url_str = decode_ddg_redirect_url(&caps[1]);
            let title = strip_tags(&caps[2]);

            lines.push(format!("{}. {}", i + 1, title.trim()));
            lines.push(format!("   {}", url_str.trim()));

            // 如果有摘要则添加
            if i < snippet_matches.len() {
                let snippet = strip_tags(&snippet_matches[i][1]);
                let snippet = snippet.trim();
                if !snippet.is_empty() {
                    lines.push(format!("   {}", snippet));
                }
            }
        }

        Ok(lines.join("\n"))
    }

    /// 使用 Brave Search API 执行网络搜索
    ///
    /// 调用 Brave Search API 获取 JSON 格式的搜索结果。
    ///
    /// # 参数
    /// - `query`：搜索查询字符串
    ///
    /// # 返回值
    /// - `Ok(String)`：格式化的搜索结果
    /// - `Err`：API 密钥未配置、请求失败或响应解析错误
    ///
    /// # 配置要求
    /// 需要在配置文件中设置 `[web_search].api_key`
    async fn search_brave(&self, query: &str) -> anyhow::Result<String> {
        let auth_token = self
            .get_next_api_key()
            .ok_or_else(|| anyhow::anyhow!("Brave API key not configured"))?;

        let encoded_query = urlencoding::encode(query);
        let search_url = format!(
            "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
            encoded_query, self.max_results
        );

        let client = self.build_client()?;

        let response = client
            .get(&search_url)
            .header("Accept", "application/json")
            .header("X-Subscription-Token", auth_token)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if !response.status().is_success() {
            anyhow::bail!("Brave search failed with status: {}", response.status());
        }

        let json: serde_json::Value =
            response.json().await.map_err(|e| anyhow::anyhow!(e.to_string()))?;
        self.parse_brave_results(&json, query)
    }

    /// 解析 Brave Search API 的 JSON 响应为格式化结果
    ///
    /// 从 JSON 响应中提取 `web.results` 数组，格式化为可读文本。
    ///
    /// # 参数
    /// - `json`：Brave API 返回的 JSON 响应
    /// - `query`：原始搜索查询（用于上下文显示）
    ///
    /// # 返回值
    /// - `Ok(String)`：格式化的搜索结果字符串
    /// - `Err`：响应格式无效（缺少 `web.results` 字段）
    fn parse_brave_results(&self, json: &serde_json::Value, query: &str) -> anyhow::Result<String> {
        let results = json
            .get("web")
            .and_then(|w| w.get("results"))
            .and_then(|r| r.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid Brave API response"))?;

        if results.is_empty() {
            return Ok(format!("No results found for: {}", query));
        }

        let mut lines = vec![format!("Search results for: {} (via Brave)", query)];

        for (i, result) in results.iter().take(self.max_results).enumerate() {
            let title = result.get("title").and_then(|t| t.as_str()).unwrap_or("No title");
            let url = result.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let description = result.get("description").and_then(|d| d.as_str()).unwrap_or("");

            lines.push(format!("{}. {}", i + 1, title));
            lines.push(format!("   {}", url));
            if !description.is_empty() {
                lines.push(format!("   {}", description));
            }
        }

        Ok(lines.join("\n"))
    }

    async fn search_serper(&self, query: &str, provider: &str) -> anyhow::Result<String> {
        let api_key = self.get_next_api_key().ok_or_else(|| {
            anyhow::anyhow!(
                "web_search provider '{}' requires web_search.api_key in system settings",
                provider
            )
        })?;

        let endpoint = self.resolve_serper_endpoint(provider);
        let client = self.build_client()?;
        let response = client
            .post(&endpoint)
            .header("X-API-KEY", api_key)
            .header(reqwest::header::ACCEPT, "application/json, text/plain, */*")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&json!({
                "q": query,
                "num": self.max_results,
            }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{} search failed: {e}", provider))?;

        let status = response.status();
        let body = response.text().await.map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if !status.is_success() {
            anyhow::bail!("{} search failed with status {}: {}", provider, status.as_u16(), body);
        }

        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Invalid {} response JSON: {e}", provider))?;
        self.parse_serper_results(&parsed, query, provider)
    }

    pub(crate) fn parse_serper_results(
        &self,
        json: &serde_json::Value,
        query: &str,
        provider: &str,
    ) -> anyhow::Result<String> {
        let results = json
            .get("organic")
            .and_then(serde_json::Value::as_array)
            .or_else(|| json.get("results").and_then(serde_json::Value::as_array))
            .ok_or_else(|| {
                anyhow::anyhow!("Invalid {} response: missing results array", provider)
            })?;

        if results.is_empty() {
            return Ok(format!("No results found for: {}", query));
        }

        let provider_label = match provider {
            "bing" => "Bing",
            "google" => "Google",
            _ => "Serper",
        };

        let mut lines = vec![format!("Search results for: {} (via {})", query, provider_label)];

        for (i, result) in results.iter().take(self.max_results).enumerate() {
            let title =
                result.get("title").and_then(serde_json::Value::as_str).unwrap_or("No title");
            let url = result
                .get("link")
                .or_else(|| result.get("url"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let description = result
                .get("snippet")
                .or_else(|| result.get("description"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");

            lines.push(format!("{}. {}", i + 1, title));
            lines.push(format!("   {}", url));
            if !description.trim().is_empty() {
                lines.push(format!("   {}", description.trim()));
            }
        }

        Ok(lines.join("\n"))
    }

    /// 使用 Firecrawl Search API 执行网络搜索
    ///
    /// 调用 Firecrawl 搜索端点获取增强的搜索结果。
    ///
    /// # 参数
    /// - `query`：搜索查询字符串
    ///
    /// # 返回值
    /// - `Ok(String)`：格式化的搜索结果
    /// - `Err`：API 密钥未配置、请求失败或 API 返回错误
    ///
    /// # 配置要求
    /// - 需要启用 `firecrawl` Cargo 特性
    /// - 需要在配置文件中设置 `[web_search].api_key`
    /// - 可通过 `[web_search].api_url` 覆盖默认端点
    ///
    /// # API 端点
    /// - 默认：`https://api.firecrawl.dev/v1/search`
    /// - 自定义：`{api_url}/v1/search`
    #[cfg(feature = "firecrawl")]
    async fn search_firecrawl(&self, query: &str) -> anyhow::Result<String> {
        let auth_token = self.get_next_api_key().ok_or_else(|| {
            anyhow::anyhow!(
                "web_search provider 'firecrawl' requires web_search.api_key in system settings"
            )
        })?;

        let api_url = self
            .api_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("https://api.firecrawl.dev");
        let endpoint = format!("{}/v1/search", api_url.trim_end_matches('/'));
        let client = self.build_client()?;

        let response = client
            .post(endpoint)
            .header(reqwest::header::ACCEPT, "application/json, text/plain, */*")
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {auth_token}"))
            .json(&json!({
                "query": query,
                "limit": self.max_results,
                "timeout": (self.timeout_secs * 1000) as u64,
            }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Firecrawl search failed: {e}"))?;
        let status = response.status();
        let body = response.text().await.map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if !status.is_success() {
            anyhow::bail!("Firecrawl search failed with status {}: {}", status.as_u16(), body);
        }

        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Invalid Firecrawl response JSON: {e}"))?;
        if !parsed.get("success").and_then(serde_json::Value::as_bool).unwrap_or(false) {
            let error =
                parsed.get("error").and_then(serde_json::Value::as_str).unwrap_or("unknown error");
            anyhow::bail!("Firecrawl search failed: {error}");
        }

        let results = parsed
            .get("data")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("Firecrawl response missing data array"))?;

        if results.is_empty() {
            return Ok(format!("No results found for: {}", query));
        }

        let mut lines = vec![format!("Search results for: {} (via Firecrawl)", query)];

        for (i, result) in results.iter().take(self.max_results).enumerate() {
            let title =
                result.get("title").and_then(serde_json::Value::as_str).unwrap_or("No title");
            let url = result.get("url").and_then(serde_json::Value::as_str).unwrap_or("");
            let description =
                result.get("description").and_then(serde_json::Value::as_str).unwrap_or("");

            lines.push(format!("{}. {}", i + 1, title));
            lines.push(format!("   {}", url));
            if !description.trim().is_empty() {
                lines.push(format!("   {}", description.trim()));
            }
        }

        Ok(lines.join("\n"))
    }

    /// Firecrawl 搜索存根（未启用特性时）
    ///
    /// 当 `firecrawl` 特性未启用时，调用此方法将返回错误。
    ///
    /// # 返回值
    /// 始终返回错误，提示需要启用 `firecrawl` 特性
    #[cfg(not(feature = "firecrawl"))]
    #[allow(clippy::unused_async)]
    async fn search_firecrawl(&self, _query: &str) -> anyhow::Result<String> {
        anyhow::bail!("web_search provider 'firecrawl' requires Cargo feature 'firecrawl'")
    }

    /// 使用 Tavily Search API 执行网络搜索
    ///
    /// Tavily 是专为 AI 应用优化的搜索 API，返回高质量、结构化的搜索结果。
    ///
    /// # 参数
    /// - `query`：搜索查询字符串
    ///
    /// # 返回值
    /// - `Ok(String)`：格式化的搜索结果（包含标题、URL 和内容片段）
    /// - `Err`：API 密钥未配置、请求失败或 API 返回错误
    ///
    /// # 配置要求
    /// - 需要在配置文件中设置 `[web_search].api_key`
    /// - 可通过 `[web_search].api_url` 覆盖默认端点
    ///
    /// # API 参数
    /// - `search_depth`: "basic"（基础搜索深度）
    /// - `include_answer`: false（不包含 AI 生成答案）
    /// - `include_raw_content`: false（不包含原始 HTML）
    /// - `include_images`: false（不包含图片结果）
    async fn search_tavily(&self, query: &str) -> anyhow::Result<String> {
        let api_key = self.get_next_api_key().ok_or_else(|| {
            anyhow::anyhow!(
                "web_search provider 'tavily' requires web_search.api_key in system settings"
            )
        })?;

        let api_url = self
            .api_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("https://api.tavily.com");

        let endpoint = format!("{}/search", api_url.trim_end_matches('/'));
        let client = self.build_client()?;

        let response = client
            .post(&endpoint)
            .header(reqwest::header::ACCEPT, "application/json, text/plain, */*")
            .json(&json!({
                "api_key": api_key,
                "query": query,
                "max_results": self.max_results,
                "search_depth": "basic",
                "include_answer": false,
                "include_raw_content": false,
                "include_images": false,
            }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Tavily search failed: {e}"))?;

        let status = response.status();
        let body = response.text().await.map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if !status.is_success() {
            anyhow::bail!("Tavily search failed with status {}: {}", status.as_u16(), body);
        }

        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Invalid Tavily response JSON: {e}"))?;

        // 检查响应中是否包含 API 错误
        if let Some(error) = parsed.get("error").and_then(|e| e.as_str()) {
            anyhow::bail!("Tavily API error: {}", error);
        }

        let results = parsed
            .get("results")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("Tavily response missing results array"))?;

        if results.is_empty() {
            return Ok(format!("No results found for: {}", query));
        }

        let mut lines = vec![format!("Search results for: {} (via Tavily)", query)];

        for (i, result) in results.iter().take(self.max_results).enumerate() {
            let title =
                result.get("title").and_then(serde_json::Value::as_str).unwrap_or("No title");
            let url = result.get("url").and_then(serde_json::Value::as_str).unwrap_or("");
            let content = result.get("content").and_then(serde_json::Value::as_str).unwrap_or("");

            lines.push(format!("{}. {}", i + 1, title));
            lines.push(format!("   {}", url));
            if !content.trim().is_empty() {
                lines.push(format!("   {}", content.trim()));
            }
        }

        Ok(lines.join("\n"))
    }
}

/// 解码 DuckDuckGo 重定向 URL 以提取实际目标 URL
///
/// DuckDuckGo 的搜索结果链接是重定向 URL，格式如：
/// `https://duckduckgo.com/l/?uddg=<encoded_url>&rut=...`
///
/// 此函数从重定向 URL 中提取 `uddg` 参数并解码。
///
/// # 参数
/// - `raw_url`：DuckDuckGo 返回的重定向 URL
///
/// # 返回值
/// - 解码后的目标 URL
/// - 如果解码失败或不是重定向 URL，返回原始 URL
///
/// # 示例
/// ```ignore
/// let redirect = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com";
/// let decoded = decode_ddg_redirect_url(redirect);
/// assert_eq!(decoded, "https://example.com");
/// ```
pub(crate) fn decode_ddg_redirect_url(raw_url: &str) -> String {
    if let Some(index) = raw_url.find("uddg=") {
        let encoded = &raw_url[index + 5..];
        let encoded = encoded.split('&').next().unwrap_or(encoded);
        if let Ok(decoded) = urlencoding::decode(encoded) {
            return decoded.into_owned();
        }
    }

    raw_url.to_string()
}

/// 移除 HTML 标签，仅保留纯文本内容
///
/// 使用正则表达式 `<[^>]+>` 匹配并删除所有 HTML 标签。
///
/// # 参数
/// - `content`：包含 HTML 标签的内容
///
/// # 返回值
/// 不包含 HTML 标签的纯文本字符串
///
/// # 示例
/// ```ignore
/// let html = "<b>Hello</b> <em>World</em>";
/// let text = strip_tags(html);
/// assert_eq!(text, "Hello World");
/// ```
pub(crate) fn strip_tags(content: &str) -> String {
    let re = Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(content, "").to_string()
}

/// Tool trait 实现
///
/// 将 WebSearchTool 注册为代理可调用的工具。
/// 根据配置的提供方自动路由到相应的搜索引擎。
///
/// # 平台兼容性
/// - 原生平台：支持 `Send` trait
/// - WASM 平台：不支持 `Send`（使用 `?Send` 标记）
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for WebSearchTool {
    /// 返回工具名称
    ///
    /// # 返回值
    /// 固定返回 `"web_search_tool"`
    fn name(&self) -> &str {
        "web_search_tool"
    }

    /// 返回工具描述
    ///
    /// # 返回值
    /// 中文描述，说明工具用途：搜索网络信息，返回标题、URL 和描述
    fn description(&self) -> &str {
        "Search the web for information. Returns relevant results with titles, URLs, and descriptions. Use it for current information, news, or research topics."
    }

    /// 返回工具参数 JSON Schema
    ///
    /// # 返回值
    /// JSON Schema 定义，包含一个必填参数：
    /// - `query`（string）：搜索查询词
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query. Be specific for better results."
                },
                "num": {
                    "type": "integer",
                    "description": "Compatibility field for the number of results to return, from 1 to 10."
                },
                "numResults": {
                    "type": "integer",
                    "description": "Compatibility alias for num."
                },
                "lr": {
                    "type": "string",
                    "description": "Optional language restriction hint, currently accepted for input compatibility."
                }
            },
            "required": ["query"]
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            crate::app::agent::tools::WEB_SEARCH_TOOL_ID,
            self.description(),
            self.parameters_schema(),
        )
        .with_display_name(crate::app::agent::tools::WEB_SEARCH_TOOL_ID)
        .with_aliases(vec![
            crate::app::agent::tools::WEB_SEARCH_TOOL_ALIAS.to_string(),
            "websearch".to_string(),
            "web_search".to_string(),
        ])
        .with_read_only(true)
        .with_destructive(false)
        .with_concurrency_safe(true)
        .with_requires_user_interaction(false)
        .with_strict(true)
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let legacy = self.execute(input.clone()).await?;
        let args = serde_json::from_value::<Args>(input).ok();
        let query = args.as_ref().map(|args| args.query.trim().to_string()).unwrap_or_default();
        let requested_num = args.as_ref().and_then(|args| args.num).unwrap_or(self.max_results);
        let requested_lr = args.as_ref().and_then(|args| args.lr.clone()).unwrap_or_default();

        if !legacy.success {
            let mut result = ToolCallResult::from_legacy_result(legacy);
            result.render_hint = Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::WEB_SEARCH_TOOL_ID.to_string()),
                kind: Some("web_search".to_string()),
                summary: Some(if query.is_empty() {
                    "Web search failed".to_string()
                } else {
                    format!("Web search failed for {query}")
                }),
                metadata: json!({
                    "query": query,
                    "provider": self.provider_label(),
                    "requested_num": requested_num,
                    "lr": requested_lr,
                }),
            });
            return Ok(result);
        }

        let raw = legacy.output;
        let (provider, results) = Self::parse_formatted_results(&raw);
        let provider = provider.unwrap_or_else(|| self.provider_label());
        let data = json!({
            "query": query.clone(),
            "provider": provider.clone(),
            "requested_num": requested_num,
            "lr": requested_lr.clone(),
            "results": results,
            "raw": raw.clone(),
        });

        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String(raw),
            content_blocks: vec![ToolResultContentDto::Json {
                value: json!({
                    "query": query.clone(),
                    "provider": provider.clone(),
                    "result_count": data
                        .get("results")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0),
                    "results": data.get("results").cloned().unwrap_or(Value::Array(Vec::new())),
                }),
            }],
            render_hint: Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::WEB_SEARCH_TOOL_ID.to_string()),
                kind: Some("web_search".to_string()),
                summary: Some(
                    if let Some(count) = data.get("results").and_then(Value::as_array).map(Vec::len)
                    {
                        if count == 0 {
                            if query.is_empty() {
                                "No search results".to_string()
                            } else {
                                format!("No results for {query}")
                            }
                        } else if query.is_empty() {
                            format!("Found {count} results")
                        } else {
                            format!("Found {count} results for {query}")
                        }
                    } else {
                        "Web search completed".to_string()
                    },
                ),
                metadata: json!({
                    "provider": provider,
                    "requested_num": requested_num,
                    "lr": requested_lr,
                    "result_count": data
                        .get("results")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0),
                }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    /// 执行网络搜索
    ///
    /// # 参数
    /// - `args`：JSON 对象，必须包含 `query` 字段
    ///
    /// # 返回值
    /// - `Ok(ToolResult { success: true, ... })`：搜索成功，结果在 `output` 字段
    /// - `Ok(ToolResult { success: false, ... })`：权限被拒绝或速率限制
    /// - `Err`：参数错误或搜索失败
    ///
    /// # 安全检查
    /// 1. 检查 `can_act()` - 是否允许执行操作（非只读模式）
    /// 2. 检查 `record_action()` - 是否未超出速率限制
    ///
    /// # 提供方路由
    /// - `duckduckgo` / `ddg` → DuckDuckGo 搜索（免费）
    /// - `brave` → Brave Search API
    /// - `serper` / `google` → Google Search（通过 Serper API）
    /// - `bing` → Bing Search（通过 Serper API）
    /// - `firecrawl` → Firecrawl Search API
    /// - `tavily` → Tavily Search API
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 安全策略检查：是否允许执行操作
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        // 速率限制检查：是否超出操作频率限制
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        // 提取并验证查询参数
        let args: Args = serde_json::from_value(args)
            .map_err(|error| anyhow::anyhow!("Missing or invalid parameters: {error}"))?;
        let query = args.query.as_str();
        let search_tool = args
            .num
            .map(|requested| self.with_max_results(requested))
            .unwrap_or_else(|| self.with_max_results(self.max_results));

        // 检查查询是否为空
        if query.trim().is_empty() {
            anyhow::bail!("Search query cannot be empty");
        }

        // 记录搜索日志
        tracing::info!("Searching web for: {}", query);

        // 根据提供方路由到相应的搜索引擎
        let result = match search_tool.provider.as_str() {
            "duckduckgo" | "ddg" => search_tool.search_duckduckgo(query).await?,
            "brave" => search_tool.search_brave(query).await?,
            "serper" | "google" => {
                search_tool.search_serper(query, search_tool.provider.as_str()).await?
            }
            "bing" => search_tool.search_serper(query, "bing").await?,
            "firecrawl" => search_tool.search_firecrawl(query).await?,
            "tavily" => search_tool.search_tavily(query).await?,
            _ => anyhow::bail!(
                "Unknown search provider: '{}'. Set web_search.provider to 'duckduckgo', 'brave', 'serper', 'google', 'bing', 'firecrawl', or 'tavily' in system settings",
                search_tool.provider
            ),
        };

        Ok(ToolResult { success: true, output: result, error: None })
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
