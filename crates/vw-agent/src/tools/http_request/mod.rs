//! HTTP 请求工具
//!
//! 执行 HTTP API 请求，支持 GET、POST、PUT、DELETE 等方法。
//! 具有可配置的安全策略，包括域名白名单、响应大小限制和超时控制。

use super::traits::{Tool, ToolResult};
use super::url_validation::{
    DomainPolicy, UrlSchemePolicy, normalize_allowed_domains, validate_url,
};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

/// HTTP 请求工具
///
/// 用于执行 HTTP API 请求的工具实现，支持多种 HTTP 方法。
/// 该工具集成了安全策略控制，包括域名白名单、响应大小限制和超时控制，
/// 以确保请求的安全性和可控性。
///
/// # 安全特性
///
/// - 域名白名单：仅允许访问预配置的域名
/// - 响应大小限制：防止过大的响应消耗过多内存
/// - 超时控制：防止请求长时间挂起
/// - 敏感信息脱敏：在日志和输出中自动隐藏敏感头部
///
/// # 支持的 HTTP 方法
///
/// GET、POST、PUT、DELETE、PATCH、HEAD、OPTIONS
pub struct HttpRequestTool {
    /// 安全策略引用，用于控制是否允许执行动作
    security: Arc<SecurityPolicy>,
    /// 允许访问的域名白名单列表
    allowed_domains: Vec<String>,
    /// 响应体的最大大小（字节数），超出将被截断
    max_response_size: usize,
    /// 请求超时时间（秒），0 表示使用默认值 30 秒
    timeout_secs: u64,
    /// HTTP 请求的 User-Agent 标识
    user_agent: String,
}

impl HttpRequestTool {
    /// 创建新的 HTTP 请求工具实例
    ///
    /// # 参数
    ///
    /// - `security`: 安全策略的共享引用，用于动作控制和速率限制
    /// - `allowed_domains`: 允许访问的域名列表，将被规范化处理
    /// - `max_response_size`: 响应体的最大字节数限制
    /// - `timeout_secs`: 请求超时秒数（0 表示使用默认 30 秒）
    /// - `user_agent`: HTTP 请求使用的 User-Agent 字符串
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `HttpRequestTool` 实例
    pub fn new(
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        max_response_size: usize,
        timeout_secs: u64,
        user_agent: String,
    ) -> Self {
        Self {
            security,
            allowed_domains: normalize_allowed_domains(allowed_domains),
            max_response_size,
            timeout_secs,
            user_agent,
        }
    }

    /// 验证 URL 是否符合安全策略
    ///
    /// 检查 URL 的协议是否为 HTTP/HTTPS，以及域名是否在白名单中。
    /// 同时会阻止访问私有/本地主机地址。
    ///
    /// # 参数
    ///
    /// - `raw_url`: 待验证的原始 URL 字符串
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 验证通过的规范化 URL
    /// - `Err`: 验证失败，包含具体的错误信息
    fn validate_url(&self, raw_url: &str) -> anyhow::Result<String> {
        validate_url(
            raw_url,
            &DomainPolicy {
                allowed_domains: &self.allowed_domains,
                blocked_domains: &[],
                allowed_field_name: "http_request.allowed_domains",
                blocked_field_name: None,
                empty_allowed_message: "HTTP request tool is enabled but no allowed_domains are configured. Add http_request.allowed_domains in system settings",
                scheme_policy: UrlSchemePolicy::HttpOrHttps,
                ipv6_error_context: "http_request",
            },
        )
    }

    /// 验证并解析 HTTP 方法
    ///
    /// 将字符串形式的 HTTP 方法转换为 `reqwest::Method` 枚举值。
    /// 方法名称不区分大小写。
    ///
    /// # 参数
    ///
    /// - `method`: HTTP 方法名称字符串（如 "GET"、"POST" 等）
    ///
    /// # 返回值
    ///
    /// - `Ok(reqwest::Method)`: 对应的 HTTP 方法枚举
    /// - `Err`: 不支持的 HTTP 方法，包含错误信息
    fn validate_method(&self, method: &str) -> anyhow::Result<reqwest::Method> {
        match method.to_uppercase().as_str() {
            "GET" => Ok(reqwest::Method::GET),
            "POST" => Ok(reqwest::Method::POST),
            "PUT" => Ok(reqwest::Method::PUT),
            "DELETE" => Ok(reqwest::Method::DELETE),
            "PATCH" => Ok(reqwest::Method::PATCH),
            "HEAD" => Ok(reqwest::Method::HEAD),
            "OPTIONS" => Ok(reqwest::Method::OPTIONS),
            _ => anyhow::bail!(
                "Unsupported HTTP method: {method}. Supported: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS"
            ),
        }
    }

    /// 解析 JSON 格式的请求头为键值对列表
    ///
    /// 从 JSON 对象中提取所有的字符串值头部，忽略非字符串值。
    ///
    /// # 参数
    ///
    /// - `headers`: JSON 格式的请求头对象
    ///
    /// # 返回值
    ///
    /// 返回 `(键, 值)` 元组的向量
    fn parse_headers(&self, headers: &serde_json::Value) -> Vec<(String, String)> {
        let mut result = Vec::new();
        if let Some(obj) = headers.as_object() {
            for (key, value) in obj {
                if let Some(str_val) = value.as_str() {
                    result.push((key.clone(), str_val.to_string()));
                }
            }
        }
        result
    }

    /// 对请求头进行脱敏处理，用于日志显示
    ///
    /// 识别并隐藏敏感的头部值（如 Authorization、Token 等），
    /// 防止敏感信息泄露到日志中。
    ///
    /// # 参数
    ///
    /// - `headers`: 原始请求头列表
    ///
    /// # 返回值
    ///
    /// 返回脱敏后的请求头列表，敏感值被替换为 `"***REDACTED***"`
    fn redact_headers_for_display(headers: &[(String, String)]) -> Vec<(String, String)> {
        headers
            .iter()
            .map(|(key, value)| {
                let lower = key.to_lowercase();
                // 检查是否为敏感头部（包含认证、密钥、令牌等关键字）
                let is_sensitive = lower.contains("authorization")
                    || lower.contains("api-key")
                    || lower.contains("apikey")
                    || lower.contains("token")
                    || lower.contains("secret");
                if is_sensitive {
                    (key.clone(), "***REDACTED***".into())
                } else {
                    (key.clone(), value.clone())
                }
            })
            .collect()
    }

    /// 执行实际的 HTTP 请求
    ///
    /// 构建并发送 HTTP 请求，应用超时、代理等配置。
    /// 在非 WASM 环境下会禁用自动重定向。
    ///
    /// # 参数
    ///
    /// - `url`: 请求的目标 URL
    /// - `method`: HTTP 方法
    /// - `headers`: 请求头列表
    /// - `body`: 可选的请求体内容
    ///
    /// # 返回值
    ///
    /// - `Ok(reqwest::Response)`: 成功收到的响应
    /// - `Err`: 请求失败，包含错误信息
    ///
    /// # 超时处理
    ///
    /// 当 `timeout_secs` 为 0 时，自动使用安全的默认值 30 秒
    async fn execute_request(
        &self,
        url: &str,
        method: reqwest::Method,
        headers: Vec<(String, String)>,
        body: Option<&str>,
    ) -> anyhow::Result<reqwest::Response> {
        // 处理超时配置：0 表示使用安全默认值
        let timeout_secs = if self.timeout_secs == 0 {
            tracing::warn!("http_request: timeout_secs is 0, using safe default of 30s");
            30
        } else {
            self.timeout_secs
        };

        // 构建 HTTP 客户端
        let builder = reqwest::Client::builder();

        // 在非 WASM 目标上设置超时和连接超时
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder
            .timeout(Duration::from_secs(timeout_secs))
            .connect_timeout(Duration::from_secs(10));

        // 设置 User-Agent
        let builder = builder.user_agent(self.user_agent.as_str());

        // 在非 WASM 目标上禁用自动重定向（安全考虑）
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.redirect(reqwest::redirect::Policy::none());

        // 应用运行时代理配置
        let builder =
            crate::app::agent::config::apply_runtime_proxy_to_builder(builder, "tool.http_request");
        let client = builder.build()?;

        // 构建请求
        let mut request = client.request(method, url);

        // 添加请求头
        for (key, value) in headers {
            request = request.header(&key, &value);
        }

        // 添加请求体（如果提供）
        if let Some(body_str) = body {
            request = request.body(body_str.to_string());
        }

        // 发送请求
        Ok(request.send().await?)
    }

    async fn format_response(&self, response: reqwest::Response) -> ToolResult {
        let status = response.status();
        let status_code = status.as_u16();

        let headers_text = response
            .headers()
            .iter()
            .map(|(k, _)| {
                let is_sensitive = k.as_str().to_lowercase().contains("set-cookie");
                if is_sensitive {
                    format!("{}: ***REDACTED***", k.as_str())
                } else {
                    format!("{}: {:?}", k.as_str(), k.as_str())
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let response_text = match response.text().await {
            Ok(text) => self.truncate_response(&text),
            Err(e) => format!("[Failed to read response body: {e}]"),
        };

        let output = format!(
            "Status: {} {}\nResponse Headers: {}\n\nResponse Body:\n{}",
            status_code,
            status.canonical_reason().unwrap_or("Unknown"),
            headers_text,
            response_text
        );

        ToolResult {
            success: status.is_success(),
            output,
            error: if status.is_client_error() || status.is_server_error() {
                Some(format!("HTTP {}", status_code))
            } else {
                None
            },
        }
    }

    /// 截断过长的响应体
    ///
    /// 当响应体超过配置的最大大小时，截断并添加提示信息。
    /// 按字符截断以避免破坏 UTF-8 编码。
    ///
    /// # 参数
    ///
    /// - `text`: 原始响应文本
    ///
    /// # 返回值
    ///
    /// 如果超过大小限制，返回截断后的文本（带提示信息）；
    /// 否则返回原始文本
    fn truncate_response(&self, text: &str) -> String {
        if text.len() > self.max_response_size {
            // 按字符截断，避免破坏多字节字符
            let mut truncated = text.chars().take(self.max_response_size).collect::<String>();
            truncated.push_str("\n\n... [Response truncated due to size limit] ...");
            truncated
        } else {
            text.to_string()
        }
    }
}

/// Tool trait 实现
///
/// 为 `HttpRequestTool` 实现 `Tool` trait，使其可以作为工具被 Agent 调用。
/// 实现中包含了完整的安全检查、URL 验证和错误处理。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for HttpRequestTool {
    /// 返回工具名称
    ///
    /// 工具名称用于在配置和调用中标识此工具
    fn name(&self) -> &str {
        "http_request"
    }

    /// 返回工具的描述信息
    ///
    /// 描述了工具的功能、支持的 HTTP 方法以及安全约束
    fn description(&self) -> &str {
        "向外部 API 发送 HTTP 请求。支持 GET、POST、PUT、DELETE、PATCH、HEAD、OPTIONS 方法。\
        安全约束：仅限白名单域名、禁止本地/私有主机、可配置超时和响应大小限制。"
    }

    /// 返回工具参数的 JSON Schema 定义
    ///
    /// 定义了工具接受的参数结构，包括：
    /// - `url`: 必需，请求的 URL
    /// - `method`: 可选，HTTP 方法，默认为 GET
    /// - `headers`: 可选，HTTP 请求头对象
    /// - `body`: 可选，请求体内容
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "要请求的 HTTP 或 HTTPS URL"
                },
                "method": {
                    "type": "string",
                    "description": "HTTP 方法（GET、POST、PUT、DELETE、PATCH、HEAD、OPTIONS）",
                    "default": "GET"
                },
                "headers": {
                    "type": "object",
                    "description": "可选的 HTTP 头部键值对（例如 {\"Authorization\": \"Bearer token\", \"Content-Type\": \"application/json\"}）",
                    "default": {}
                },
                "body": {
                    "type": "string",
                    "description": "可选的请求体（用于 POST、PUT、PATCH 请求）"
                }
            },
            "required": ["url"]
        })
    }

    /// 执行 HTTP 请求工具
    ///
    /// 这是工具的主要执行入口，完成以下步骤：
    /// 1. 解析和验证参数
    /// 2. 检查安全策略（自主性、速率限制）
    /// 3. 验证 URL 和 HTTP 方法
    /// 4. 执行请求并处理响应
    /// 5. 格式化输出结果
    ///
    /// # 参数
    ///
    /// - `args`: JSON 格式的工具参数，包含 url、method、headers、body
    ///
    /// # 返回值
    ///
    /// 返回 `ToolResult`，包含：
    /// - `success`: 请求是否成功（HTTP 2xx）
    /// - `output`: 格式化的响应信息（状态码、头部、响应体）
    /// - `error`: 错误信息（如果请求失败）
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 解析必需的 URL 参数
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))?;

        // 解析可选参数，提供默认值
        let method_str = args.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
        let headers_val = args.get("headers").cloned().unwrap_or(json!({}));
        let body = args.get("body").and_then(|v| v.as_str());

        // 检查是否允许执行动作（自主性检查）
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        // 检查速率限制
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        // 验证 URL（域名白名单、协议检查等）
        let url = match self.validate_url(url) {
            Ok(v) => v,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                });
            }
        };

        // 验证 HTTP 方法
        let method = match self.validate_method(method_str) {
            Ok(m) => m,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                });
            }
        };

        // 解析请求头
        let request_headers = self.parse_headers(&headers_val);

        // 执行请求并处理响应
        match self.execute_request(&url, method, request_headers, body).await {
            Ok(response) => Ok(self.format_response(response).await),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("HTTP request failed: {e}")),
            }),
        }
    }
}

/// 单元测试模块
///
/// 测试代码位于 `tests/http_request.rs` 文件中，
/// 包含对 HTTP 请求工具各项功能的测试用例。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
