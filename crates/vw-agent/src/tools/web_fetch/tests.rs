//! WebFetch 工具的单元测试模块
//!
//! 本模块包含 `WebFetchTool` 的全面测试套件，覆盖以下方面：
//! - 工具基本属性（名称、参数 schema）
//! - HTML 到 Markdown/纯文本的转换
//! - URL 验证（允许列表、阻止列表、通配符）
//! - SSRF（服务器端请求伪造）防护
//! - 安全策略执行（只读模式、速率限制）
//! - 响应截断功能
//! - 域名规范化与去重
//! - API 密钥管理（多密钥、轮询选择）
//!
//! 测试组织遵循功能分组，便于维护和理解。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
    use crate::app::agent::tools::url_validation::{is_private_or_local_host, normalize_domain};
    use crate::app::agent::tools::{WEB_FETCH_TOOL_ALIAS, WEB_FETCH_TOOL_ID};

    /// 创建基础测试用的 WebFetchTool 实例
    ///
    /// 使用默认的 `fast_html2md` 提供者和无 API 密钥配置。
    /// 适用于大多数不需要特殊配置的测试场景。
    ///
    /// # 参数
    ///
    /// * `allowed_domains` - 允许访问的域名列表
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `WebFetchTool` 实例
    fn test_tool(allowed_domains: Vec<&str>) -> WebFetchTool {
        test_tool_with_provider(allowed_domains, vec![], "fast_html2md", None, None)
    }

    /// 创建带有阻止列表的测试用 WebFetchTool 实例
    ///
    /// 适用于需要测试域名黑名单功能的场景。
    ///
    /// # 参数
    ///
    /// * `allowed_domains` - 允许访问的域名列表
    /// * `blocked_domains` - 禁止访问的域名列表（黑名单）
    ///
    /// # 返回值
    ///
    /// 返回配置了允许列表和阻止列表的 `WebFetchTool` 实例
    fn test_tool_with_blocklist(
        allowed_domains: Vec<&str>,
        blocked_domains: Vec<&str>,
    ) -> WebFetchTool {
        test_tool_with_provider(allowed_domains, blocked_domains, "fast_html2md", None, None)
    }

    /// 创建完全自定义配置的测试用 WebFetchTool 实例
    ///
    /// 这是测试工具创建的核心函数，允许自定义所有配置参数。
    /// 内部创建一个带有 `Supervised` 自主级别的默认安全策略。
    ///
    /// # 参数
    ///
    /// * `allowed_domains` - 允许访问的域名列表
    /// * `blocked_domains` - 禁止访问的域名列表
    /// * `provider` - HTML 转换提供者名称（如 "fast_html2md"、"nanohtml2text"、"firecrawl"、"tavily"）
    /// * `provider_key` - 可选的 API 密钥（部分提供者需要）
    /// * `api_url` - 可选的自定义 API 端点 URL
    ///
    /// # 返回值
    ///
    /// 返回完全配置好的 `WebFetchTool` 实例，具有以下默认值：
    /// - 最大响应大小：500,000 字节
    /// - 请求超时：30 秒
    /// - User-Agent：默认桌面浏览器兼容值
    fn test_tool_with_provider(
        allowed_domains: Vec<&str>,
        blocked_domains: Vec<&str>,
        provider: &str,
        provider_key: Option<&str>,
        api_url: Option<&str>,
    ) -> WebFetchTool {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            ..SecurityPolicy::default()
        });
        WebFetchTool::new(
            security,
            provider.to_string(),
            provider_key.map(ToOwned::to_owned),
            api_url.map(ToOwned::to_owned),
            allowed_domains.into_iter().map(String::from).collect(),
            blocked_domains.into_iter().map(String::from).collect(),
            500_000,
            30,
            vw_config_types::tools::DEFAULT_USER_AGENT.to_string(),
        )
    }

    /// 测试工具名称是否正确返回 "web_fetch"
    ///
    /// 验证 `Tool` trait 的 `name()` 方法返回预期的工具标识符。
    #[test]
    fn name_is_web_fetch() {
        let tool = test_tool(vec!["example.com"]);
        assert_eq!(tool.name(), "web_fetch");
    }

    /// 测试参数 schema 是否正确要求 URL 字段
    ///
    /// 验证 `parameters_schema()` 返回的 JSON schema 中：
    /// - 包含 `url` 属性定义
    /// - `url` 被标记为必需字段
    #[test]
    fn parameters_schema_requires_url() {
        let tool = test_tool(vec!["example.com"]);
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["url"].is_object());
        assert!(schema["properties"]["href"].is_object());
        assert!(schema["properties"]["prompt"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("url")));
    }

    #[test]
    fn args_accept_href_alias_and_prompt_surface() {
        let args: Args = serde_json::from_value(json!({
            "href": "https://example.com/docs",
            "prompt": "Summarize the page",
            "timeout_secs": 12
        }))
        .unwrap();

        assert_eq!(args.url, "https://example.com/docs");
        assert_eq!(args.timeout, Some(12));
    }

    #[test]
    fn args_apply_defaults_and_camel_case_aliases() {
        let defaults: Args = serde_json::from_value(json!({"url": "https://example.com"})).unwrap();
        assert!(matches!(defaults.format, Format::Markdown));
        assert_eq!(defaults.timeout, None);
        assert!(defaults._prompt.is_none());

        let aliases: Args = serde_json::from_value(json!({
            "url": "https://example.com/docs",
            "format": "html",
            "timeoutSeconds": 17,
            "instructions": "Extract the main content"
        }))
        .unwrap();
        assert!(matches!(aliases.format, Format::Html));
        assert_eq!(aliases.timeout, Some(17));
        assert_eq!(aliases._prompt.as_deref(), Some("Extract the main content"));
    }

    #[test]
    fn args_reject_unknown_format() {
        let result: Result<Args, _> = serde_json::from_value(json!({
            "url": "https://example.com",
            "format": "pdf"
        }));

        assert!(result.is_err());
    }

    #[test]
    fn constructor_normalizes_provider_and_discards_blank_api_keys() {
        let tool = test_tool_with_provider(
            vec!["example.com"],
            vec![],
            "  TaViLy  ",
            Some(" , key-a, , key-b, "),
            Some("https://extract.example.com"),
        );

        assert_eq!(tool.provider, "tavily");
        assert_eq!(tool.api_keys, vec!["key-a", "key-b"]);
        assert_eq!(tool.api_url.as_deref(), Some("https://extract.example.com"));
    }

    /// 测试 WebFetch spec 是否切换到 Claude 风格表面并保留旧别名
    #[test]
    fn spec_uses_claude_surface() {
        let tool = test_tool(vec!["example.com"]);
        let spec = tool.spec();

        assert_eq!(spec.id, WEB_FETCH_TOOL_ID);
        assert!(spec.aliases.iter().any(|alias| alias == WEB_FETCH_TOOL_ALIAS));
    }

    #[test]
    fn select_accept_header_prioritizes_requested_format() {
        let markdown = WebFetchTool::select_accept_header(Format::Markdown);
        let text = WebFetchTool::select_accept_header(Format::Text);
        let html = WebFetchTool::select_accept_header(Format::Html);

        assert!(markdown.starts_with("text/html,application/xhtml+xml"));
        assert!(text.starts_with("text/plain;q=1.0"));
        assert!(html.starts_with("text/html;q=1.0"));
        assert_ne!(markdown, text);
        assert_ne!(text, html);
    }

    #[test]
    fn markdown_to_text_preserves_text_code_and_breaks() {
        assert_eq!(
            WebFetchTool::markdown_to_text("**bold** and `code`  \nnext"),
            "bold and code\nnext"
        );
        assert_eq!(WebFetchTool::markdown_to_text("first\nsecond"), "first\nsecond");
        assert_eq!(WebFetchTool::markdown_to_text("   "), "");
    }

    #[test]
    fn convert_http_output_only_transforms_html_content_types() {
        let tool = test_tool(vec!["example.com"]);
        assert_eq!(
            tool.convert_http_output("{\"ok\":true}", "application/json", Format::Text).unwrap(),
            "{\"ok\":true}"
        );
        assert_eq!(
            tool.convert_http_output(
                "<h1>Title</h1>",
                "application/xhtml+xml; charset=utf-8",
                Format::Html,
            )
            .unwrap(),
            "<h1>Title</h1>"
        );

        let unknown = test_tool_with_provider(vec!["example.com"], vec![], "unknown", None, None);
        let error = unknown
            .convert_http_output("<p>body</p>", "text/html", Format::Markdown)
            .unwrap_err()
            .to_string();
        assert!(error.contains("Unknown web_fetch provider"));
    }

    #[cfg(not(feature = "web-fetch-html2md"))]
    #[test]
    fn fast_html2md_without_feature_returns_configuration_error() {
        let error = test_tool(vec!["example.com"])
            .convert_html_to_output("<p>body</p>")
            .unwrap_err()
            .to_string();

        assert!(error.contains("requires Cargo feature 'web-fetch-html2md'"));
    }

    #[cfg(not(feature = "web-fetch-plaintext"))]
    #[test]
    fn nanohtml2text_without_feature_returns_configuration_error() {
        let tool =
            test_tool_with_provider(vec!["example.com"], vec![], "nanohtml2text", None, None);
        let error = tool.convert_html_to_output("<p>body</p>").unwrap_err().to_string();

        assert!(error.contains("requires Cargo feature 'web-fetch-plaintext'"));
    }

    /// 测试 HTML 到 Markdown 转换是否保留文档结构
    ///
    /// 使用 `fast_html2md` 提供者验证：
    /// - 标题和列表内容被正确转换
    /// - HTML 标签被移除
    /// - 文本内容保持完整
    ///
    /// 仅在启用 `web-fetch-html2md` feature 时运行。
    #[cfg(feature = "web-fetch-html2md")]
    #[test]
    fn html_to_markdown_conversion_preserves_structure() {
        let tool = test_tool(vec!["example.com"]);
        let html = "<html><body><h1>Title</h1><ul><li>Hello</li></ul></body></html>";
        let markdown = tool.convert_html_to_output(html).unwrap();
        assert!(markdown.contains("Title"));
        assert!(markdown.contains("Hello"));
        assert!(!markdown.contains("<h1>"));
    }

    /// 测试 HTML 到纯文本转换是否移除所有 HTML 标签
    ///
    /// 使用 `nanohtml2text` 提供者验证：
    /// - 文本内容被提取
    /// - 所有 HTML 标签（包括嵌套标签）被移除
    ///
    /// 仅在启用 `web-fetch-plaintext` feature 时运行。
    #[cfg(feature = "web-fetch-plaintext")]
    #[test]
    fn html_to_plaintext_conversion_removes_html_tags() {
        let tool =
            test_tool_with_provider(vec!["example.com"], vec![], "nanohtml2text", None, None);
        let html = "<html><body><h1>Title</h1><p>Hello <b>world</b></p></body></html>";
        let text = tool.convert_html_to_output(html).unwrap();
        assert!(text.contains("Title"));
        assert!(text.contains("Hello"));
        assert!(!text.contains("<h1>"));
    }

    /// 测试 URL 验证接受精确域名匹配
    ///
    /// 当允许列表包含 "example.com" 时，
    /// "https://example.com/page" 应该被接受。
    #[test]
    fn validate_accepts_exact_domain() {
        let tool = test_tool(vec!["example.com"]);
        let got = tool.validate_url("https://example.com/page").unwrap();
        assert_eq!(got, "https://example.com/page");
    }

    /// 测试 URL 验证接受允许域名的子域名
    ///
    /// 当允许列表包含 "example.com" 时，
    /// "docs.example.com" 这样的子域名应该被接受。
    #[test]
    fn validate_accepts_subdomain() {
        let tool = test_tool(vec!["example.com"]);
        assert!(tool.validate_url("https://docs.example.com/guide").is_ok());
    }

    /// 测试通配符 "*" 允许所有域名
    ///
    /// 当允许列表为 ["*"] 时，任何公网域名都应该被接受。
    #[test]
    fn validate_accepts_wildcard() {
        let tool = test_tool(vec!["*"]);
        assert!(tool.validate_url("https://news.ycombinator.com").is_ok());
    }

    /// 测试 URL 验证拒绝空字符串
    ///
    /// 空字符串应该返回包含 "empty" 的错误消息。
    #[test]
    fn validate_rejects_empty_url() {
        let tool = test_tool(vec!["example.com"]);
        let err = tool.validate_url("").unwrap_err().to_string();
        assert!(err.contains("empty"));
    }

    /// 测试 URL 验证拒绝仅包含空白字符的字符串
    ///
    /// 仅包含空格/制表符的字符串应该返回包含 "empty" 的错误消息。
    #[test]
    fn validate_rejects_missing_url() {
        let tool = test_tool(vec!["example.com"]);
        let err = tool.validate_url("  ").unwrap_err().to_string();
        assert!(err.contains("empty"));
    }

    /// 测试 URL 验证拒绝非 HTTP(S) 协议
    ///
    /// FTP 等其他协议应该被拒绝，
    /// 错误消息应提示只支持 http:// 或 https://。
    #[test]
    fn validate_rejects_ftp_scheme() {
        let tool = test_tool(vec!["example.com"]);
        let err = tool.validate_url("ftp://example.com").unwrap_err().to_string();
        assert!(err.contains("http://") || err.contains("https://"));
    }

    /// 测试 URL 验证拒绝不在允许列表中的域名
    ///
    /// 当允许列表为 ["example.com"] 时，
    /// 访问 "google.com" 应该返回包含 "allowed_domains" 的错误。
    #[test]
    fn validate_rejects_allowlist_miss() {
        let tool = test_tool(vec!["example.com"]);
        let err = tool.validate_url("https://google.com").unwrap_err().to_string();
        assert!(err.contains("allowed_domains"));
    }

    /// 测试空允许列表时拒绝所有请求
    ///
    /// 当允许列表为空时，任何 URL 都应该被拒绝，
    /// 错误消息应包含 "allowed_domains"。
    #[test]
    fn validate_requires_allowlist() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = WebFetchTool::new(
            security,
            "fast_html2md".into(),
            None,
            None,
            vec![],
            vec![],
            500_000,
            30,
            "test".to_string(),
        );
        let err = tool.validate_url("https://example.com").unwrap_err().to_string();
        assert!(err.contains("allowed_domains"));
    }

    /// 测试 SSRF 防护阻止 localhost
    ///
    /// 即使 "localhost" 在允许列表中，
    /// 访问 localhost:8080 也应该被阻止，
    /// 错误消息应包含 "local/private"。
    #[test]
    fn ssrf_blocks_localhost() {
        let tool = test_tool(vec!["localhost"]);
        let err = tool.validate_url("https://localhost:8080").unwrap_err().to_string();
        assert!(err.contains("local/private"));
    }

    /// 测试 SSRF 防护阻止私有 IPv4 地址
    ///
    /// 即使私有 IP 在允许列表中，
    /// 访问私有 IP 地址也应该被阻止。
    #[test]
    fn ssrf_blocks_private_ipv4() {
        let tool = test_tool(vec!["192.168.1.5"]);
        let err = tool.validate_url("https://192.168.1.5").unwrap_err().to_string();
        assert!(err.contains("local/private"));
    }

    /// 测试 SSRF 防护识别环回地址
    ///
    /// 验证 `is_private_or_local_host` 函数正确识别
    /// 127.0.0.0/8 网段的所有环回地址。
    #[test]
    fn ssrf_blocks_loopback() {
        assert!(is_private_or_local_host("127.0.0.1"));
        assert!(is_private_or_local_host("127.0.0.2"));
    }

    /// 测试 SSRF 防护识别 RFC1918 私有地址
    ///
    /// 验证 `is_private_or_local_host` 函数正确识别
    /// 以下私有地址范围：
    /// - 10.0.0.0/8
    /// - 172.16.0.0/12
    /// - 192.168.0.0/16
    #[test]
    fn ssrf_blocks_rfc1918() {
        assert!(is_private_or_local_host("10.0.0.1"));
        assert!(is_private_or_local_host("172.16.0.1"));
        assert!(is_private_or_local_host("192.168.1.1"));
    }

    /// 测试通配符允许列表仍然阻止私有地址
    ///
    /// 即使使用 "*" 通配符允许所有域名，
    /// 私有/本地地址仍应被 SSRF 防护阻止。
    #[test]
    fn ssrf_wildcard_still_blocks_private() {
        let tool = test_tool(vec!["*"]);
        let err = tool.validate_url("https://localhost:8080").unwrap_err().to_string();
        assert!(err.contains("local/private"));
    }

    /// 测试只读模式阻止执行
    ///
    /// 当安全策略的自主级别为 `ReadOnly` 时，
    /// 执行请求应该失败，错误消息应包含 "read-only"。
    #[tokio::test]
    async fn blocks_readonly_mode() {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });
        let tool = WebFetchTool::new(
            security,
            "fast_html2md".into(),
            None,
            None,
            vec!["example.com".into()],
            vec![],
            500_000,
            30,
            "test".to_string(),
        );
        let result = tool.execute(json!({"url": "https://example.com"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("read-only"));
    }

    /// 测试速率限制阻止执行
    ///
    /// 当安全策略的 `max_actions_per_hour` 为 0 时，
    /// 执行请求应该失败，错误消息应包含 "rate limit"。
    #[tokio::test]
    async fn blocks_rate_limited() {
        let security =
            Arc::new(SecurityPolicy { max_actions_per_hour: 0, ..SecurityPolicy::default() });
        let tool = WebFetchTool::new(
            security,
            "fast_html2md".into(),
            None,
            None,
            vec!["example.com".into()],
            vec![],
            500_000,
            30,
            "test".to_string(),
        );
        let result = tool.execute(json!({"url": "https://example.com"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("rate limit"));
    }

    #[tokio::test]
    async fn call_failure_reports_normalized_url_provider_and_format() {
        let tool = test_tool_with_provider(vec!["example.com"], vec![], "unknown", None, None);
        let result = tool
            .call(json!({
                "url": "http://example.com/path",
                "format": "html"
            }))
            .await
            .unwrap();

        assert_eq!(result.data["success"], false);
        assert!(result.model_result.as_str().unwrap().contains("Unknown web_fetch provider"));
        assert!(!result.telemetry.as_ref().unwrap().success);

        let hint = result.render_hint.unwrap();
        assert_eq!(hint.summary.as_deref(), Some("Failed to fetch https://example.com/path"));
        assert_eq!(hint.metadata["url"], "https://example.com/path");
        assert_eq!(hint.metadata["provider"], "unknown");
        assert_eq!(hint.metadata["format"], "html");
    }

    #[tokio::test]
    async fn call_rejects_invalid_parameter_types_before_network() {
        let error =
            test_tool(vec!["example.com"]).call(json!({"url": 42})).await.unwrap_err().to_string();

        assert!(error.contains("Missing or invalid parameters"));
    }

    /// 测试响应截断在限制内时保持原样
    ///
    /// 当响应文本长度小于最大限制时，
    /// `truncate_response` 应返回原始文本。
    #[test]
    fn truncate_within_limit() {
        let tool = test_tool(vec!["example.com"]);
        let text = "hello world";
        assert_eq!(tool.truncate_response(text), "hello world");
    }

    /// 测试响应截断在超过限制时添加截断标记
    ///
    /// 当响应文本超过最大限制时，
    /// `truncate_response` 应截断文本并添加 "[Response truncated" 标记。
    #[test]
    fn truncate_over_limit() {
        let tool = WebFetchTool::new(
            Arc::new(SecurityPolicy::default()),
            "fast_html2md".into(),
            None,
            None,
            vec!["example.com".into()],
            vec![],
            10,
            30,
            "test".to_string(),
        );
        let text = "hello world this is long";
        let truncated = tool.truncate_response(text);
        assert!(truncated.contains("[Response truncated"));
    }

    #[test]
    fn truncate_response_preserves_unicode_boundaries_and_exact_marker() {
        let mut tool = test_tool(vec!["example.com"]);
        tool.max_response_size = 3;

        assert_eq!(
            tool.truncate_response("你好世界"),
            "你好世\n\n... [Response truncated due to size limit] ..."
        );

        tool.max_response_size = 0;
        assert_eq!(tool.truncate_response(""), "");
        assert_eq!(
            tool.truncate_response("x"),
            "\n\n... [Response truncated due to size limit] ..."
        );
    }

    #[test]
    fn effective_timeout_uses_config_request_and_safe_bounds() {
        let mut tool = test_tool(vec!["example.com"]);
        tool.timeout_secs = 45;

        assert_eq!(tool.effective_timeout_secs(None), 45);
        assert_eq!(tool.effective_timeout_secs(Some(1)), 1);
        assert_eq!(tool.effective_timeout_secs(Some(0)), DEFAULT_TIMEOUT_SECS);
        assert_eq!(tool.effective_timeout_secs(Some(MAX_TIMEOUT_SECS)), MAX_TIMEOUT_SECS);
        assert_eq!(tool.effective_timeout_secs(Some(MAX_TIMEOUT_SECS + 1)), MAX_TIMEOUT_SECS);
    }

    /// 测试域名规范化移除协议前缀并统一小写
    ///
    /// 验证 `normalize_domain` 函数：
    /// - 移除前导/尾随空白
    /// - 移除 HTTPS:// 等协议前缀
    /// - 移除路径部分
    /// - 转换为小写
    #[test]
    fn normalize_domain_strips_scheme_and_case() {
        let got = normalize_domain("  HTTPS://Docs.Example.com/path ").unwrap();
        assert_eq!(got, "docs.example.com");
    }

    /// 测试允许列表域名去重
    ///
    /// 验证 `normalize_allowed_domains` 函数能够：
    /// - 将不同形式的同一域名（大小写、带协议、带路径）识别为相同
    /// - 去除重复项，只保留一个规范化形式
    #[test]
    fn normalize_deduplicates() {
        let got = normalize_allowed_domains(vec![
            "example.com".into(),
            "EXAMPLE.COM".into(),
            "https://example.com/".into(),
        ]);
        assert_eq!(got, vec!["example.com".to_string()]);
    }

    /// 测试阻止列表精确匹配拒绝
    ///
    /// 当 "evil.com" 在阻止列表中时，
    /// 访问 "https://evil.com/page" 应被拒绝，
    /// 错误消息应包含 "blocked_domains"。
    #[test]
    fn blocklist_rejects_exact_match() {
        let tool = test_tool_with_blocklist(vec!["*"], vec!["evil.com"]);
        let err = tool.validate_url("https://evil.com/page").unwrap_err().to_string();
        assert!(err.contains("blocked_domains"));
    }

    /// 测试阻止列表子域名匹配拒绝
    ///
    /// 当 "evil.com" 在阻止列表中时，
    /// 其子域名 "api.evil.com" 也应被阻止。
    #[test]
    fn blocklist_rejects_subdomain() {
        let tool = test_tool_with_blocklist(vec!["*"], vec!["evil.com"]);
        let err = tool.validate_url("https://api.evil.com/v1").unwrap_err().to_string();
        assert!(err.contains("blocked_domains"));
    }

    /// 测试阻止列表优先级高于允许列表
    ///
    /// 当域名同时出现在允许列表和阻止列表时，
    /// 阻止列表应优先生效，请求被拒绝。
    #[test]
    fn blocklist_wins_over_allowlist() {
        let tool = test_tool_with_blocklist(vec!["evil.com"], vec!["evil.com"]);
        let err = tool.validate_url("https://evil.com").unwrap_err().to_string();
        assert!(err.contains("blocked_domains"));
    }

    /// 测试阻止列表不影响非阻止域名
    ///
    /// 当阻止列表包含 "evil.com" 时，
    /// 访问其他域名（如 "example.com"）应该正常通过。
    #[test]
    fn blocklist_allows_non_blocked() {
        let tool = test_tool_with_blocklist(vec!["*"], vec!["evil.com"]);
        assert!(tool.validate_url("https://example.com").is_ok());
    }

    /// 测试 Firecrawl 提供者要求 API 密钥
    ///
    /// 使用 Firecrawl 提供者但未提供 API 密钥时：
    /// - 若启用 `firecrawl` feature，错误应提示需要 `web_fetch.api_key`
    /// - 若未启用 feature，错误应提示需要 `firecrawl` Cargo feature
    #[tokio::test]
    async fn firecrawl_provider_requires_api_key() {
        let tool = test_tool_with_provider(vec!["*"], vec![], "firecrawl", None, None);
        let result = tool.execute(json!({"url": "https://example.com"})).await.unwrap();
        assert!(!result.success);
        let error = result.error.unwrap_or_default();
        if cfg!(feature = "firecrawl") {
            assert!(error.contains("requires web_fetch.api_key"));
        } else {
            assert!(error.contains("requires Cargo feature 'firecrawl'"));
        }
    }

    /// 测试 Tavily 提供者要求 API 密钥
    ///
    /// 使用 Tavily 提供者但未提供 API 密钥时，
    /// 执行应失败，错误消息应提示需要 `web_fetch.api_key`。
    #[tokio::test]
    async fn tavily_provider_requires_api_key() {
        let tool = test_tool_with_provider(vec!["*"], vec![], "tavily", None, None);
        let result = tool.execute(json!({"url": "https://example.com"})).await.unwrap();
        assert!(!result.success);
        let error = result.error.unwrap_or_default();
        assert!(error.contains("requires web_fetch.api_key"));
    }

    /// 测试多个 API 密钥的解析
    ///
    /// 验证逗号分隔的 API 密钥字符串能够正确解析为多个密钥。
    #[test]
    fn test_multiple_api_keys_parsing() {
        let tool =
            test_tool_with_provider(vec!["*"], vec![], "tavily", Some("key1,key2,key3"), None);
        assert_eq!(tool.api_keys.len(), 3);
        assert_eq!(tool.api_keys[0], "key1");
        assert_eq!(tool.api_keys[1], "key2");
        assert_eq!(tool.api_keys[2], "key3");
    }

    /// 测试带空格的多个 API 密钥解析
    ///
    /// 验证包含额外空格的逗号分隔密钥字符串能够正确解析，
    /// 空格应被自动修剪。
    #[test]
    fn test_multiple_api_keys_with_spaces() {
        let tool =
            test_tool_with_provider(vec!["*"], vec![], "tavily", Some("key1, key2 , key3"), None);
        assert_eq!(tool.api_keys.len(), 3);
        assert_eq!(tool.api_keys[0], "key1");
        assert_eq!(tool.api_keys[1], "key2");
        assert_eq!(tool.api_keys[2], "key3");
    }

    /// 测试 API 密钥轮询选择机制
    ///
    /// 验证 `get_next_api_key` 方法：
    /// - 按顺序返回密钥
    /// - 到达末尾后循环回到第一个密钥
    #[test]
    fn test_round_robin_api_key_selection() {
        let tool =
            test_tool_with_provider(vec!["*"], vec![], "tavily", Some("key1,key2,key3"), None);

        assert_eq!(tool.get_next_api_key().unwrap(), "key1");
        assert_eq!(tool.get_next_api_key().unwrap(), "key2");
        assert_eq!(tool.get_next_api_key().unwrap(), "key3");
        assert_eq!(tool.get_next_api_key().unwrap(), "key1"); // 循环回到第一个
    }

    /// 测试无 API 密钥时返回 None
    ///
    /// 当未配置任何 API 密钥时，
    /// `get_next_api_key` 应返回 `None`。
    #[test]
    fn test_empty_api_key_returns_none() {
        let tool = test_tool_with_provider(vec!["*"], vec![], "tavily", None, None);
        assert!(tool.get_next_api_key().is_none());
    }

    /// 测试单个 API 密钥正常工作
    ///
    /// 验证单个密钥配置能够正确存储和获取。
    #[test]
    fn test_single_api_key_works() {
        let tool = test_tool_with_provider(vec!["*"], vec![], "tavily", Some("single-key"), None);
        assert_eq!(tool.api_keys.len(), 1);
        assert_eq!(tool.get_next_api_key().unwrap(), "single-key");
    }
}
