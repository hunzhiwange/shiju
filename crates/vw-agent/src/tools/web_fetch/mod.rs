//! 网页抓取工具
//!
//! 本模块提供网页内容抓取与格式转换功能，专为 LLM（大语言模型）消费场景优化。
//! 支持多种内容提供方，可根据部署需求灵活选择：
//!
//! # 支持的提供方
//!
//! - **fast_html2md**: 使用 reqwest 执行 HTTP 请求，将 HTML 转换为 Markdown 格式
//! - **nanohtml2text**: 使用 reqwest 执行 HTTP 请求，将 HTML 转换为纯文本格式
//! - **firecrawl**: 使用 Firecrawl 云服务或自托管 API 执行抓取
//! - **tavily**: 使用 Tavily Extract API 执行抓取
//!
//! # 安全特性
//!
//! - URL 验证：支持域名白名单/黑名单机制，防止 SSRF 攻击
//! - 响应大小限制：防止内存溢出
//! - 超时控制：避免长时间阻塞
//! - 安全策略集成：与全局安全策略联动
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use crate::app::agent::tools::web_fetch::WebFetchTool;
//! use std::sync::Arc;
//!
//! let tool = WebFetchTool::new(
//!     security_policy,
//!     "fast_html2md".to_string(),
//!     None,
//!     None,
//!     vec!["example.com".to_string()],
//!     vec![],
//!     1024 * 1024,
//!     30,
//!     "Mozilla/5.0".to_string(),
//! );
//! ```

use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use super::url_validation::{
    DomainPolicy, UrlSchemePolicy, normalize_allowed_domains, validate_url,
};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use pulldown_cmark::{Event, Parser};
use reqwest::header::ACCEPT;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use vw_api_types::tools::ToolResultContentDto;

/// 默认请求超时时间（秒）
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 最大允许的请求超时时间（秒），防止用户配置过长超时
const MAX_TIMEOUT_SECS: u64 = 120;

/// web_fetch 工具的命令行参数结构
///
/// 该结构体定义了调用 web_fetch 工具时所需的参数，通过 serde 从 JSON 反序列化而来。
#[derive(Debug, Clone, Deserialize)]
struct Args {
    /// 要抓取的目标 URL（必填）
    #[serde(alias = "href")]
    url: String,

    /// 输出格式，默认为 Markdown
    #[serde(default)]
    format: Format,

    /// 可选的请求超时时间（秒），最大不超过 MAX_TIMEOUT_SECS
    #[serde(default, alias = "timeout_secs", alias = "timeoutSeconds")]
    timeout: Option<u64>,

    /// Claude 兼容抓取提示词，当前仅用于兼容输入表面。
    #[serde(default, rename = "prompt", alias = "instructions")]
    _prompt: Option<String>,
}

/// 网页内容输出格式枚举
///
/// 定义了从网页抓取内容后可转换为的目标格式。
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum Format {
    /// 纯文本格式，移除所有 HTML 标签和 Markdown 格式
    Text,

    /// Markdown 格式（默认），保留结构化内容
    #[default]
    Markdown,

    /// 原始 HTML 格式，不进行任何转换
    Html,
}

/// 网页抓取工具
///
/// 提供网页内容抓取、格式转换和安全控制功能。支持多种内容提供方，
/// 可根据部署需求选择本地处理或第三方 API 服务。
///
/// # 提供方说明
///
/// - `fast_html2md`: 本地 HTML 转 Markdown，需要启用 `web-fetch-html2md` 特性
/// - `nanohtml2text`: 本地 HTML 转纯文本，需要启用 `web-fetch-plaintext` 特性
/// - `firecrawl`: 使用 Firecrawl API 服务，需要配置 API 密钥和启用 `firecrawl` 特性
/// - `tavily`: 使用 Tavily Extract API，需要配置 API 密钥
///
/// # 安全特性
///
/// - 域名白名单/黑名单控制
/// - URL scheme 限制（仅 HTTP/HTTPS）
/// - 响应大小限制
/// - 请求超时控制
/// - 与全局安全策略联动
pub struct WebFetchTool {
    /// 安全策略引用，用于权限检查和速率限制
    security: Arc<SecurityPolicy>,

    /// 内容提供方名称（fast_html2md/nanohtml2text/firecrawl/tavily）
    provider: String,

    /// API 密钥列表，支持多密钥轮询以实现负载均衡
    api_keys: Vec<String>,

    /// 自定义 API 端点 URL（可选）
    api_url: Option<String>,

    /// 允许抓取的域名白名单（为空表示拒绝所有）
    allowed_domains: Vec<String>,

    /// 禁止抓取的域名黑名单
    blocked_domains: Vec<String>,

    /// 最大响应体大小（字节），超过将被截断
    max_response_size: usize,

    /// 默认请求超时时间（秒）
    timeout_secs: u64,

    /// HTTP User-Agent 字符串
    user_agent: String,

    /// 当前轮询的 API 密钥索引（原子计数器，支持多线程安全）
    key_index: Arc<AtomicUsize>,
}

impl WebFetchTool {
    /// 创建新的 WebFetchTool 实例
    ///
    /// 初始化网页抓取工具，配置提供方、安全策略和各种运行参数。
    ///
    /// # 参数
    ///
    /// * `security` - 安全策略引用，用于权限和速率限制检查
    /// * `provider` - 提供方名称（fast_html2md/nanohtml2text/firecrawl/tavily）
    /// * `api_key` - API 密钥，支持逗号分隔的多密钥配置（用于轮询负载均衡）
    /// * `api_url` - 可选的自定义 API 端点 URL
    /// * `allowed_domains` - 允许抓取的域名白名单（为空表示拒绝所有）
    /// * `blocked_domains` - 禁止抓取的域名黑名单
    /// * `max_response_size` - 最大响应体大小（字节）
    /// * `timeout_secs` - 默认请求超时时间（秒）
    /// * `user_agent` - HTTP User-Agent 字符串
    ///
    /// # 返回值
    ///
    /// 返回配置好的 WebFetchTool 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let tool = WebFetchTool::new(
    ///     security_policy,
    ///     "fast_html2md".to_string(),
    ///     None,
    ///     None,
    ///     vec!["example.com".to_string()],
    ///     vec![],
    ///     1024 * 1024,  // 1MB
    ///     30,
    ///     "MyBot/1.0".to_string(),
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        security: Arc<SecurityPolicy>,
        provider: String,
        api_key: Option<String>,
        api_url: Option<String>,
        allowed_domains: Vec<String>,
        blocked_domains: Vec<String>,
        max_response_size: usize,
        timeout_secs: u64,
        user_agent: String,
    ) -> Self {
        // 规范化提供方名称：去空白、转小写
        let provider = provider.trim().to_lowercase();

        // 解析逗号分隔的 API 密钥，支持多密钥轮询
        // 例如："key1,key2,key3" -> ["key1", "key2", "key3"]
        let api_keys = api_key
            .as_ref()
            .map(|keys| {
                keys.split(',').map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).collect()
            })
            .unwrap_or_default();

        Self {
            security,
            // 如果提供方为空，默认使用 fast_html2md
            provider: if provider.is_empty() { "fast_html2md".to_string() } else { provider },
            api_keys,
            api_url,
            // 规范化域名列表（统一格式、处理通配符等）
            allowed_domains: normalize_allowed_domains(allowed_domains),
            blocked_domains: normalize_allowed_domains(blocked_domains),
            max_response_size,
            timeout_secs,
            user_agent: vw_config_types::tools::resolve_user_agent(&user_agent),
            // 初始化 API 密钥轮询索引为 0
            key_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 使用轮询策略获取下一个 API 密钥
    ///
    /// 当配置了多个 API 密钥时，使用轮询（Round-Robin）方式选择，
    /// 实现负载均衡和容错。
    ///
    /// # 返回值
    ///
    /// - `Some(String)`: 返回下一个可用的 API 密钥
    /// - `None`: 如果未配置任何 API 密钥
    fn get_next_api_key(&self) -> Option<String> {
        if self.api_keys.is_empty() {
            return None;
        }
        // 原子递增索引并取模，确保线程安全的轮询
        let idx = self.key_index.fetch_add(1, Ordering::Relaxed) % self.api_keys.len();
        Some(self.api_keys[idx].clone())
    }

    /// 根据安全策略验证 URL
    ///
    /// 检查 URL 是否符合安全策略要求，包括：
    /// - 域名白名单/黑名单检查
    /// - URL scheme 限制（仅允许 HTTP/HTTPS）
    /// - 其他 SSRF 防护措施
    ///
    /// # 参数
    ///
    /// * `raw_url` - 待验证的原始 URL 字符串
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 验证通过，返回规范化后的 URL
    /// - `Err`: 验证失败，返回错误信息
    fn validate_url(&self, raw_url: &str) -> anyhow::Result<String> {
        validate_url(
            raw_url,
            &DomainPolicy {
                allowed_domains: &self.allowed_domains,
                blocked_domains: &self.blocked_domains,
                allowed_field_name: "web_fetch.allowed_domains",
                blocked_field_name: Some("web_fetch.blocked_domains"),
                empty_allowed_message: "web_fetch tool is enabled but no allowed_domains are configured. Add web_fetch.allowed_domains in system settings",
                scheme_policy: UrlSchemePolicy::HttpOrHttps,
                ipv6_error_context: "web_fetch",
            },
        )
    }

    /// 截断过长的响应内容
    ///
    /// 当响应内容超过配置的最大大小时，进行截断并添加提示信息。
    ///
    /// # 参数
    ///
    /// * `text` - 原始响应文本
    ///
    /// # 返回值
    ///
    /// 如果超过大小限制，返回截断后的文本；否则返回原文本
    fn truncate_response(&self, text: &str) -> String {
        if text.len() > self.max_response_size {
            // 按字符截取而非字节，避免截断多字节字符
            let mut truncated = text.chars().take(self.max_response_size).collect::<String>();
            truncated.push_str("\n\n... [Response truncated due to size limit] ...");
            truncated
        } else {
            text.to_string()
        }
    }

    /// 计算实际请求超时时间
    ///
    /// 综合考虑用户请求的超时时间、默认超时时间和最大超时限制，
    /// 确定最终的超时时间。
    ///
    /// # 参数
    ///
    /// * `requested` - 用户请求的超时时间（可选）
    ///
    /// # 返回值
    ///
    /// 返回不超过 MAX_TIMEOUT_SECS 的有效超时时间
    fn effective_timeout_secs(&self, requested: Option<u64>) -> u64 {
        let base = requested.unwrap_or(self.timeout_secs);
        if base == 0 {
            // 防止配置为 0 导致立即超时，使用安全默认值
            tracing::warn!("web_fetch: timeout_secs is 0, using safe default of 30s");
            DEFAULT_TIMEOUT_SECS
        } else {
            // 确保不超过最大限制
            base.min(MAX_TIMEOUT_SECS)
        }
    }

    /// 将 HTML 内容转换为目标输出格式
    ///
    /// 根据配置的提供方，将 HTML 转换为 Markdown 或纯文本。
    ///
    /// # 参数
    ///
    /// * `body` - HTML 内容字符串
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 转换成功，返回目标格式的内容
    /// - `Err`: 转换失败或特性未启用
    ///
    /// # 错误
    ///
    /// - 如果对应的 Cargo 特性未启用，返回错误提示
    /// - 如果提供方未知，返回错误提示
    #[allow(unused_variables)]
    fn convert_html_to_output(&self, body: &str) -> anyhow::Result<String> {
        match self.provider.as_str() {
            // fast_html2md 提供方：HTML -> Markdown
            "fast_html2md" => {
                #[cfg(feature = "web-fetch-html2md")]
                {
                    Ok(html2md::parse_html(body))
                }
                #[cfg(not(feature = "web-fetch-html2md"))]
                {
                    anyhow::bail!(
                        "web_fetch provider 'fast_html2md' requires Cargo feature 'web-fetch-html2md'"
                    );
                }
            }
            // nanohtml2text 提供方：HTML -> 纯文本
            "nanohtml2text" => {
                #[cfg(feature = "web-fetch-plaintext")]
                {
                    Ok(nanohtml2text::html2text(body))
                }
                #[cfg(not(feature = "web-fetch-plaintext"))]
                {
                    anyhow::bail!(
                        "web_fetch provider 'nanohtml2text' requires Cargo feature 'web-fetch-plaintext'"
                    );
                }
            }
            // 未知提供方
            _ => anyhow::bail!(
                "Unknown web_fetch provider: '{}'. Set web_fetch.provider to 'fast_html2md', 'nanohtml2text', 'firecrawl', or 'tavily' in system settings",
                self.provider
            ),
        }
    }

    /// 构建配置好的 HTTP 客户端
    ///
    /// 创建带有超时、User-Agent 和代理配置的 reqwest 客户端。
    ///
    /// # 参数
    ///
    /// * `requested_timeout` - 可选的用户请求超时时间
    ///
    /// # 返回值
    ///
    /// 返回配置好的 reqwest::Client 实例
    fn build_http_client(&self, requested_timeout: Option<u64>) -> anyhow::Result<reqwest::Client> {
        let builder = reqwest::Client::builder();

        // 非 WASM 目标平台配置超时和连接参数
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder
            .timeout(Duration::from_secs(self.effective_timeout_secs(requested_timeout)))
            .connect_timeout(Duration::from_secs(10)) // 连接超时 10 秒
            .redirect(reqwest::redirect::Policy::none()); // 禁用自动重定向，手动处理

        // 设置 User-Agent
        let builder = builder
            .user_agent(self.user_agent.as_str())
            .default_headers(super::http_headers::default_headers());

        // 应用运行时代理配置（如果有）
        let builder =
            crate::app::agent::config::apply_runtime_proxy_to_builder(builder, "tool.web_fetch");
        Ok(builder.build()?)
    }

    /// 根据输出格式选择合适的 Accept 请求头
    ///
    /// 不同的输出格式偏好不同的内容类型，通过 Accept 头告知服务器。
    ///
    /// # 参数
    ///
    /// * `format` - 目标输出格式
    ///
    /// # 返回值
    ///
    /// 返回适合的 Accept 头字符串
    fn select_accept_header(format: Format) -> &'static str {
        match format {
            // Markdown 输出仍优先请求浏览器常见的 HTML，再在本地转换
            Format::Markdown => {
                "text/html,application/xhtml+xml,application/xml;q=0.9,text/markdown;q=0.8,text/plain;q=0.7,*/*;q=0.5"
            }
            // 纯文本格式：优先接受纯文本
            Format::Text => "text/plain;q=1.0, text/markdown;q=0.9, text/html;q=0.8, */*;q=0.1",
            // HTML 格式：优先接受 HTML
            Format::Html => {
                "text/html;q=1.0, application/xhtml+xml;q=0.9, text/plain;q=0.8, text/markdown;q=0.7, */*;q=0.1"
            }
        }
    }

    /// 将 Markdown 转换为纯文本
    ///
    /// 移除 Markdown 格式标记，提取纯文本内容。
    ///
    /// # 参数
    ///
    /// * `markdown` - Markdown 格式的内容
    ///
    /// # 返回值
    ///
    /// 返回去除格式标记后的纯文本
    fn markdown_to_text(markdown: &str) -> String {
        let mut out = String::new();
        // 使用 pulldown-cmark 解析 Markdown 事件流
        for event in Parser::new(markdown) {
            match event {
                // 提取文本和代码内容
                Event::Text(v) | Event::Code(v) => out.push_str(&v),
                // 处理换行
                Event::SoftBreak | Event::HardBreak => out.push('\n'),
                // 忽略其他格式标记（标题、列表、链接等）
                _ => {}
            }
        }
        out.trim().to_string()
    }

    /// 将 HTTP 响应转换为目标格式
    ///
    /// 根据内容类型和目标格式，对响应体进行必要的转换。
    ///
    /// # 参数
    ///
    /// * `body` - 响应体内容
    /// * `content_type` - Content-Type 头的值
    /// * `format` - 目标输出格式
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 转换成功
    /// - `Err`: 转换失败
    fn convert_http_output(
        &self,
        body: &str,
        content_type: &str,
        format: Format,
    ) -> anyhow::Result<String> {
        // 判断内容是否为 HTML
        let is_html =
            content_type.contains("text/html") || content_type.contains("application/xhtml+xml");

        // 非 HTML 内容直接返回
        if !is_html {
            return Ok(body.to_string());
        }

        // 根据目标格式转换 HTML
        match format {
            Format::Html => Ok(body.to_string()),
            Format::Markdown => self.convert_html_to_output(body),
            Format::Text => {
                // HTML -> Markdown -> Text
                let markdown = self.convert_html_to_output(body)?;
                Ok(Self::markdown_to_text(&markdown))
            }
        }
    }

    /// 使用 HTTP 提供方抓取网页内容
    ///
    /// 通过 reqwest 直接执行 HTTP 请求，获取网页内容并转换为指定格式。
    /// 支持 fast_html2md 和 nanohtml2text 两种本地处理提供方。
    ///
    /// # 参数
    ///
    /// * `url` - 目标 URL
    /// * `format` - 目标输出格式
    /// * `timeout_secs` - 可选的请求超时时间（秒）
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 抓取成功，返回转换后的内容
    /// - `Err`: 请求失败或转换失败
    ///
    /// # 重定向处理
    ///
    /// 当遇到重定向响应时，返回重定向目标 URL 而非内容，
    /// 允许调用方决定是否跟踪重定向。
    async fn fetch_with_http_provider(
        &self,
        url: &str,
        format: Format,
        timeout_secs: Option<u64>,
    ) -> anyhow::Result<String> {
        // 构建配置好的 HTTP 客户端
        let client = self.build_http_client(timeout_secs)?;

        // 发送 GET 请求，带有 Accept 和 Accept-Language 头
        let response =
            client.get(url).header(ACCEPT, Self::select_accept_header(format)).send().await?;

        // 处理重定向响应
        if response.status().is_redirection() {
            // 提取 Location 头
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| anyhow::anyhow!("Redirect response missing Location header"))?;

            // 解析重定向目标 URL（支持相对路径）
            let redirected_url = reqwest::Url::parse(url)
                .and_then(|base| base.join(location))
                .or_else(|_| reqwest::Url::parse(location))
                .map_err(|e| anyhow::anyhow!("Invalid redirect Location header: {e}"))?
                .to_string();

            // 使用相同的安全策略验证重定向目标
            self.validate_url(&redirected_url)?;
            return Ok(redirected_url);
        }

        let status = response.status();
        // 检查 HTTP 状态码
        if !status.is_success() {
            anyhow::bail!(
                "HTTP {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            );
        }

        // 提取 Content-Type 头
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        // 读取响应体
        let body = response.text().await?;

        // 检查内容类型是否支持
        if content_type.contains("text/plain")
            || content_type.contains("text/markdown")
            || content_type.contains("application/json")
            || content_type.contains("text/html")
            || content_type.contains("application/xhtml+xml")
            || content_type.is_empty()
        {
            return self.convert_http_output(&body, &content_type, format);
        }

        // 不支持的内容类型
        anyhow::bail!(
            "Unsupported content type: {content_type}. web_fetch supports text/html, application/xhtml+xml, text/plain, text/markdown, and application/json."
        )
    }

    /// 使用 Firecrawl API 抓取网页内容
    ///
    /// 调用 Firecrawl 云服务或自托管 API 执行网页抓取，
    /// 返回 Markdown 格式的内容。
    ///
    /// # 参数
    ///
    /// * `url` - 目标 URL
    /// * `timeout_secs` - 可选的请求超时时间（秒）
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 抓取成功，返回 Markdown 内容
    /// - `Err`: API 调用失败或响应解析失败
    ///
    /// # 配置要求
    ///
    /// 需要在配置文件中设置 `[web_fetch].api_key`
    ///
    /// # API 端点
    ///
    /// 默认使用 `https://api.firecrawl.dev/v1/scrape`
    /// 可通过 `api_url` 参数覆盖
    #[cfg(feature = "firecrawl")]
    async fn fetch_with_firecrawl(
        &self,
        url: &str,
        timeout_secs: Option<u64>,
    ) -> anyhow::Result<String> {
        // 获取 API 密钥（支持轮询）
        let auth_token = self.get_next_api_key().ok_or_else(|| {
            anyhow::anyhow!(
                "web_fetch provider 'firecrawl' requires web_fetch.api_key in system settings"
            )
        })?;

        // 确定 API 端点（使用自定义 URL 或默认值）
        let api_url = self
            .api_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("https://api.firecrawl.dev");
        let endpoint = format!("{}/v1/scrape", api_url.trim_end_matches('/'));

        // 构建并发送请求
        let response = self
            .build_http_client(timeout_secs)?
            .post(endpoint)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {auth_token}"))
            .json(&json!({
                "url": url,
                "formats": ["markdown"],  // 请求 Markdown 格式
                "onlyMainContent": true,   // 只提取主要内容
                "timeout": (self.effective_timeout_secs(timeout_secs) * 1000) as u64  // 转换为毫秒
            }))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        // 检查 HTTP 状态
        if !status.is_success() {
            anyhow::bail!("Firecrawl scrape failed with status {}: {}", status.as_u16(), body);
        }

        // 解析 JSON 响应
        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Invalid Firecrawl response JSON: {e}"))?;

        // 检查 success 字段
        if !parsed.get("success").and_then(serde_json::Value::as_bool).unwrap_or(false) {
            let error =
                parsed.get("error").and_then(serde_json::Value::as_str).unwrap_or("unknown error");
            anyhow::bail!("Firecrawl scrape failed: {error}");
        }

        // 提取内容：优先 Markdown，其次 HTML，最后原始 HTML
        let data = parsed
            .get("data")
            .ok_or_else(|| anyhow::anyhow!("Firecrawl response missing data field"))?;
        let output = data
            .get("markdown")
            .and_then(serde_json::Value::as_str)
            .or_else(|| data.get("html").and_then(serde_json::Value::as_str))
            .or_else(|| data.get("rawHtml").and_then(serde_json::Value::as_str))
            .unwrap_or("")
            .to_string();

        // 检查内容是否为空
        if output.trim().is_empty() {
            anyhow::bail!("Firecrawl returned empty content");
        }

        Ok(output)
    }

    /// Firecrawl 提供方的存根实现（特性未启用时）
    ///
    /// 当 `firecrawl` 特性未启用时，返回错误提示。
    #[cfg(not(feature = "firecrawl"))]
    #[allow(clippy::unused_async)]
    async fn fetch_with_firecrawl(
        &self,
        _url: &str,
        _timeout_secs: Option<u64>,
    ) -> anyhow::Result<String> {
        anyhow::bail!("web_fetch provider 'firecrawl' requires Cargo feature 'firecrawl'")
    }

    /// 使用 Tavily Extract API 抓取网页内容
    ///
    /// 调用 Tavily Extract API 执行网页抓取，返回提取的内容。
    /// Tavily 专注于为 AI 应用优化的网页提取。
    ///
    /// # 参数
    ///
    /// * `url` - 目标 URL
    /// * `timeout_secs` - 可选的请求超时时间（秒）
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 抓取成功，返回提取的内容
    /// - `Err`: API 调用失败或响应解析失败
    ///
    /// # 配置要求
    ///
    /// 需要在配置文件中设置 `[web_fetch].api_key`
    ///
    /// # API 端点
    ///
    /// 默认使用 `https://api.tavily.com/extract`
    /// 可通过 `api_url` 参数覆盖
    async fn fetch_with_tavily(
        &self,
        url: &str,
        timeout_secs: Option<u64>,
    ) -> anyhow::Result<String> {
        // 获取 API 密钥（支持轮询）
        let api_key = self.get_next_api_key().ok_or_else(|| {
            anyhow::anyhow!(
                "web_fetch provider 'tavily' requires web_fetch.api_key in system settings"
            )
        })?;

        // 确定 API 端点（使用自定义 URL 或默认值）
        let api_url = self
            .api_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("https://api.tavily.com");

        let endpoint = format!("{}/extract", api_url.trim_end_matches('/'));

        // 构建并发送请求
        let response = self
            .build_http_client(timeout_secs)?
            .post(endpoint)
            .json(&json!({
                "api_key": api_key,
                "urls": [url]  // Tavily 支持批量请求，这里只请求单个 URL
            }))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        // 检查 HTTP 状态
        if !status.is_success() {
            anyhow::bail!("Tavily extract failed with status {}: {}", status.as_u16(), body);
        }

        // 解析 JSON 响应
        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Invalid Tavily response JSON: {e}"))?;

        // 检查 API 错误
        if let Some(error) = parsed.get("error").and_then(|e| e.as_str()) {
            anyhow::bail!("Tavily API error: {}", error);
        }

        // 提取结果数组
        let results = parsed
            .get("results")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("Tavily response missing results array"))?;

        // 检查是否有结果
        if results.is_empty() {
            anyhow::bail!("Tavily returned no results for URL: {}", url);
        }

        // 提取第一个结果的内容（优先 raw_content，其次 content）
        let result = &results[0];
        let output = result
            .get("raw_content")
            .and_then(serde_json::Value::as_str)
            .or_else(|| result.get("content").and_then(serde_json::Value::as_str))
            .unwrap_or("");

        // 检查内容是否为空
        if output.trim().is_empty() {
            anyhow::bail!("Tavily returned empty content for URL: {}", url);
        }

        Ok(output.to_string())
    }
}

/// Tool trait 实现
///
/// 为 WebFetchTool 实现 Tool trait，使其可作为 Agent 工具使用。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for WebFetchTool {
    /// 返回工具名称
    ///
    /// 工具名称用于在工具注册表中标识此工具。
    fn name(&self) -> &str {
        "web_fetch"
    }

    /// 返回工具描述
    ///
    /// 从外部文件加载工具的详细描述文本，
    /// 用于向 LLM 说明工具的用途和使用方法。
    fn description(&self) -> &str {
        include_str!("./web_fetch.txt")
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// 定义工具接受的参数结构，用于：
    /// - 参数验证
    /// - 向 LLM 提供参数说明
    /// - 自动生成工具调用文档
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "HTTP or HTTPS URL to fetch"
                },
                "href": {
                    "type": "string",
                    "description": "Compatibility alias for url."
                },
                "prompt": {
                    "type": "string",
                    "description": "Optional extraction prompt. Currently accepted for compatibility; the fetch path still retrieves content by URL."
                },
                "format": {
                    "type": "string",
                    "enum": ["text", "markdown", "html"],
                    "default": "markdown",
                    "description": "Output format"
                },
                "timeout": {
                    "type": "number",
                    "description": "Optional request timeout in seconds, capped at 120"
                },
                "timeout_secs": {
                    "type": "number",
                    "description": "Compatibility alias for timeout."
                }
            },
            "required": ["url"]
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            crate::app::agent::tools::WEB_FETCH_TOOL_ID,
            self.description(),
            self.parameters_schema(),
        )
        .with_display_name(crate::app::agent::tools::WEB_FETCH_TOOL_ID)
        .with_aliases(vec![
            crate::app::agent::tools::WEB_FETCH_TOOL_ALIAS.to_string(),
            "webfetch".to_string(),
        ])
        .with_read_only(true)
        .with_destructive(false)
        .with_concurrency_safe(true)
        .with_requires_user_interaction(false)
        .with_strict(true)
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let legacy = self.execute(input.clone()).await?;

        let args: Args = serde_json::from_value(input)
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;
        let raw_url = args.url.trim();
        let normalized_url = if let Some(rest) = raw_url.strip_prefix("http://") {
            format!("https://{rest}")
        } else {
            raw_url.to_string()
        };
        let url = if normalized_url.is_empty() {
            String::new()
        } else {
            self.validate_url(&normalized_url).unwrap_or(normalized_url)
        };
        let format = match args.format {
            Format::Text => "text",
            Format::Markdown => "markdown",
            Format::Html => "html",
        };

        if !legacy.success {
            let mut result = ToolCallResult::from_legacy_result(legacy);
            result.render_hint = Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::WEB_FETCH_TOOL_ID.to_string()),
                kind: Some("web_fetch".to_string()),
                summary: Some(if url.is_empty() {
                    "Failed to fetch page".to_string()
                } else {
                    format!("Failed to fetch {url}")
                }),
                metadata: json!({
                    "url": url,
                    "provider": self.provider.clone(),
                    "format": format,
                }),
            });
            return Ok(result);
        }

        let content = legacy.output;
        let truncated = content.contains("[Response truncated due to size limit]");
        let preview: String = content.chars().take(400).collect();
        let data = json!({
            "url": url.clone(),
            "provider": self.provider.clone(),
            "format": format,
            "content": content.clone(),
            "truncated": truncated,
            "timeout_secs": self.effective_timeout_secs(args.timeout),
        });

        Ok(ToolCallResult {
            data,
            model_result: Value::String(content.clone()),
            content_blocks: vec![ToolResultContentDto::Json {
                value: json!({
                    "url": url.clone(),
                    "provider": self.provider.clone(),
                    "format": format,
                    "truncated": truncated,
                    "preview": preview,
                }),
            }],
            render_hint: Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::WEB_FETCH_TOOL_ID.to_string()),
                kind: Some("web_fetch".to_string()),
                summary: Some(if url.is_empty() {
                    "Fetched web page".to_string()
                } else {
                    format!("Fetched {url}")
                }),
                metadata: json!({
                    "url": url,
                    "provider": self.provider.clone(),
                    "format": format,
                    "truncated": truncated,
                    "content_chars": content.chars().count(),
                }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    /// 执行工具操作
    ///
    /// 抓取指定 URL 的内容并转换为指定格式。
    ///
    /// # 执行流程
    ///
    /// 1. 解析和验证输入参数
    /// 2. 检查安全策略（权限和速率限制）
    /// 3. 规范化 URL（HTTP -> HTTPS）
    /// 4. 验证 URL（域名白名单/黑名单）
    /// 5. 根据提供方执行抓取
    /// 6. 截断过长的响应
    /// 7. 返回结果
    ///
    /// # 参数
    ///
    /// * `args` - JSON 格式的工具参数，包含 url、format 和 timeout
    ///
    /// # 返回值
    ///
    /// 返回 ToolResult，包含：
    /// - `success`: 操作是否成功
    /// - `output`: 抓取到的内容（成功时）
    /// - `error`: 错误信息（失败时）
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 解析参数
        let args: Args = serde_json::from_value(args)
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;

        // 检查 URL 参数
        let raw_url = args.url.trim();
        if raw_url.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Missing 'url' parameter".to_string()),
            });
        }

        // 检查安全策略：是否允许执行操作
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        // 检查安全策略：速率限制
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        // 规范化 URL：将 HTTP 自动升级为 HTTPS
        let normalized_url = if let Some(rest) = raw_url.strip_prefix("http://") {
            format!("https://{rest}")
        } else {
            raw_url.to_string()
        };

        // 验证 URL（域名白名单/黑名单、scheme 等）
        let url = match self.validate_url(&normalized_url) {
            Ok(v) => v,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                });
            }
        };

        // 根据提供方执行抓取
        let result = match self.provider.as_str() {
            // 本地 HTTP 提供方
            "fast_html2md" | "nanohtml2text" => {
                self.fetch_with_http_provider(&url, args.format, args.timeout).await
            }
            // Firecrawl API
            "firecrawl" => self.fetch_with_firecrawl(&url, args.timeout).await,
            // Tavily API
            "tavily" => self.fetch_with_tavily(&url, args.timeout).await,
            // 未知提供方
            _ => Err(anyhow::anyhow!(
                "Unknown web_fetch provider: '{}'. Set web_fetch.provider to 'fast_html2md', 'nanohtml2text', 'firecrawl', or 'tavily' in system settings",
                self.provider
            )),
        };

        // 处理结果
        match result {
            Ok(output) => Ok(ToolResult {
                success: true,
                output: self.truncate_response(&output),
                error: None,
            }),
            Err(e) => {
                Ok(ToolResult { success: false, output: String::new(), error: Some(e.to_string()) })
            }
        }
    }
}

/// 单元测试模块
///
/// 包含 WebFetchTool 的单元测试和集成测试。
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
