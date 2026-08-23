//! 验证内置工具目录及多模态、浏览器、HTTP 与搜索工具的配置契约。
//! 目录稳定性、动态工具识别和各工具默认安全限制分别形成独立边界。

// 此哨兵保护拆分后的工具配置测试文件仍由父模块编译。
#[test]
fn task_632_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("tools_tests.rs"));
}

// 内置必需工具及别名是代理能力协商契约，顺序和集合都不能静默变化。
#[test]
fn builtin_required_tool_catalog_matches_expected_ids_and_aliases() {
    let expected_primary = [
        "file_read",
        "file_write",
        "ls",
        "glob",
        "grep",
        "rustcodegraph",
        "apply_patch",
        "bash",
        "http_request",
        "screenshot",
        "AgentTool",
        "skill",
        "schedule",
        "memory_forget",
        "memory_recall",
        "memory_store",
        "batch",
        "AskUserQuestion",
        "Browser",
        "BrowserOpen",
        "Config",
        "TodoRead",
        "TodoWrite",
        "WebSearch",
        "file_edit",
        "image_asset_read",
        "Sleep",
        "McpStatus",
    ];
    assert_eq!(super::BUILTIN_REQUIRED_TOOL_IDS, expected_primary);

    for id in expected_primary {
        assert!(super::is_builtin_required_tool_id(id), "primary id should match: {id}");
    }

    for (alias, primary) in [
        ("shell", "bash"),
        ("question", "AskUserQuestion"),
        ("browser", "Browser"),
        ("browser_open", "BrowserOpen"),
        ("todoread", "TodoRead"),
        ("todowrite", "TodoWrite"),
        ("web_search", "WebSearch"),
        ("websearch", "WebSearch"),
        ("web_search_tool", "WebSearch"),
        ("edit", "file_edit"),
        ("sleep", "Sleep"),
        ("mcp_status", "McpStatus"),
    ] {
        assert!(
            super::is_builtin_required_tool_id(alias),
            "alias `{alias}` should match builtin required (primary `{primary}`)"
        );
    }

    assert!(!super::is_builtin_required_tool_id("mcp__rustcodegraph__search"));
    assert!(!super::is_builtin_required_tool_id("web_fetch"));
    assert!(!super::is_builtin_required_tool_id(""));
}

// MCP 动态工具只按受控前缀识别，普通工具名不能被误归类。
#[test]
fn is_mcp_dynamic_tool_id_detects_prefix() {
    assert!(super::is_mcp_dynamic_tool_id("mcp__rustcodegraph__search"));
    assert!(super::is_mcp_dynamic_tool_id("mcp__github__list_issues"));
    assert!(!super::is_mcp_dynamic_tool_id("mcp__"));
    assert!(!super::is_mcp_dynamic_tool_id("mcp__onlyserver"));
    assert!(!super::is_mcp_dynamic_tool_id("mcp"));
    assert!(!super::is_mcp_dynamic_tool_id("file_read"));
    assert!(!super::is_mcp_dynamic_tool_id(""));
    assert!(!super::is_builtin_required_tool_id("mcp__rustcodegraph__search"));
}

// 多模态处理设置资源上限，并在未配置 OCR 时优先复用模型视觉能力。
#[test]
fn multimodal_config_defaults_and_effective_limits_are_safe() {
    let config = super::MultimodalConfig::default();
    assert_eq!(config.max_images, 10);
    assert_eq!(config.max_image_size_mb, 5);
    assert!(config.allow_remote_fetch);
    assert_eq!(config.effective_limits(), (10, 5));

    let parsed: super::MultimodalConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(parsed.max_images, 10);
    assert_eq!(parsed.max_image_size_mb, 5);
    assert!(parsed.allow_remote_fetch);

    let too_small = super::MultimodalConfig {
        max_images: 0,
        max_image_size_mb: 0,
        allow_remote_fetch: false,
        ocr: super::MultimodalOcrConfig::default(),
    };
    assert_eq!(too_small.effective_limits(), (1, 1));

    let too_large = super::MultimodalConfig {
        max_images: 99,
        max_image_size_mb: 99,
        allow_remote_fetch: true,
        ocr: super::MultimodalOcrConfig::default(),
    };
    assert_eq!(too_large.effective_limits(), (16, 20));
}

#[test]
fn multimodal_ocr_config_defaults_to_model_vision_and_round_trips_model_selection() {
    let config = super::MultimodalConfig::default();
    assert!(!config.ocr.enabled);
    assert!(config.ocr.model_supports_images);
    assert_eq!(config.ocr.model, None);

    let parsed: super::MultimodalConfig = serde_json::from_str("{}").unwrap();
    assert!(!parsed.ocr.enabled);
    assert!(parsed.ocr.model_supports_images);
    assert_eq!(parsed.ocr.model, None);

    let selected: super::MultimodalConfig = serde_json::from_value(serde_json::json!({
        "ocr": {
            "enabled": true,
            "model_supports_images": false,
            "model": "openai/gpt-4.1-mini"
        }
    }))
    .unwrap();
    assert!(selected.ocr.enabled);
    assert!(!selected.ocr.model_supports_images);
    assert_eq!(selected.ocr.model.as_deref(), Some("openai/gpt-4.1-mini"));

    let round_tripped: super::MultimodalConfig =
        serde_json::from_value(serde_json::to_value(selected).unwrap()).unwrap();
    assert!(round_tripped.ocr.enabled);
    assert!(!round_tripped.ocr.model_supports_images);
    assert_eq!(round_tripped.ocr.model.as_deref(), Some("openai/gpt-4.1-mini"));
}

// 浏览器与计算机使用的嵌套配置需同时保护禁用默认和显式覆盖。
#[test]
fn browser_computer_use_config_defaults_and_overrides_deserialize() {
    let config = super::BrowserComputerUseConfig::default();
    assert_eq!(config.endpoint, "http://127.0.0.1:8787/v1/actions");
    assert_eq!(config.api_key, None);
    assert_eq!(config.timeout_ms, 15_000);
    assert!(!config.allow_remote_endpoint);
    assert!(config.window_allowlist.is_empty());
    assert_eq!(config.max_coordinate_x, None);
    assert_eq!(config.max_coordinate_y, None);

    let parsed: super::BrowserComputerUseConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(parsed.endpoint, "http://127.0.0.1:8787/v1/actions");
    assert_eq!(parsed.timeout_ms, 15_000);
    assert!(!parsed.allow_remote_endpoint);

    let custom: super::BrowserComputerUseConfig = serde_json::from_value(serde_json::json!({
        "endpoint": "http://localhost:9000/actions",
        "api_key": "token",
        "timeout_ms": 2500,
        "allow_remote_endpoint": true,
        "window_allowlist": ["Chrome", "Cursor"],
        "max_coordinate_x": 1600,
        "max_coordinate_y": 900
    }))
    .unwrap();
    assert_eq!(custom.endpoint, "http://localhost:9000/actions");
    assert_eq!(custom.api_key.as_deref(), Some("token"));
    assert_eq!(custom.timeout_ms, 2500);
    assert!(custom.allow_remote_endpoint);
    assert_eq!(custom.window_allowlist, vec!["Chrome", "Cursor"]);
    assert_eq!(custom.max_coordinate_x, Some(1600));
    assert_eq!(custom.max_coordinate_y, Some(900));
}

#[test]
fn browser_config_defaults_and_nested_computer_use_deserialize() {
    let config = super::BrowserConfig::default();
    assert!(!config.enabled);
    assert!(config.allowed_domains.is_empty());
    assert_eq!(config.browser_open, "default");
    assert_eq!(config.session_name, None);
    assert_eq!(config.backend, "agent_browser");
    assert!(config.native_headless);
    assert_eq!(config.native_webdriver_url, "http://127.0.0.1:9515");
    assert_eq!(config.native_chrome_path, None);
    assert_eq!(config.computer_use.endpoint, "http://127.0.0.1:8787/v1/actions");

    let parsed: super::BrowserConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(parsed.browser_open, "default");
    assert_eq!(parsed.backend, "agent_browser");
    assert!(parsed.native_headless);
    assert_eq!(parsed.computer_use.timeout_ms, 15_000);

    let custom: super::BrowserConfig = serde_json::from_value(serde_json::json!({
        "enabled": true,
        "allowed_domains": ["example.com", "docs.example.com"],
        "browser_open": "new_tab",
        "session_name": "qa",
        "backend": "computer_use",
        "native_headless": false,
        "native_webdriver_url": "http://localhost:4444",
        "native_chrome_path": "/Applications/Chromium.app",
        "computer_use": {
            "allow_remote_endpoint": true,
            "window_allowlist": ["Browser"]
        }
    }))
    .unwrap();
    assert!(custom.enabled);
    assert_eq!(custom.allowed_domains, vec!["example.com", "docs.example.com"]);
    assert_eq!(custom.browser_open, "new_tab");
    assert_eq!(custom.session_name.as_deref(), Some("qa"));
    assert_eq!(custom.backend, "computer_use");
    assert!(!custom.native_headless);
    assert_eq!(custom.native_webdriver_url, "http://localhost:4444");
    assert_eq!(custom.native_chrome_path.as_deref(), Some("/Applications/Chromium.app"));
    assert!(custom.computer_use.allow_remote_endpoint);
    assert_eq!(custom.computer_use.window_allowlist, vec!["Browser"]);
    assert_eq!(custom.computer_use.endpoint, "http://127.0.0.1:8787/v1/actions");
}

// HTTP 请求默认限制网络能力，用户代理和超时等覆盖项必须独立生效。
#[test]
fn http_request_config_defaults_and_overrides_deserialize() {
    let config = super::HttpRequestConfig::default();
    assert!(!config.enabled);
    assert!(config.allowed_domains.is_empty());
    assert_eq!(config.max_response_size, 1_000_000);
    assert_eq!(config.timeout_secs, 30);
    assert_eq!(config.user_agent, super::DEFAULT_USER_AGENT);

    let parsed: super::HttpRequestConfig = serde_json::from_str("{}").unwrap();
    assert!(!parsed.enabled);
    assert!(parsed.allowed_domains.is_empty());
    assert_eq!(parsed.max_response_size, 1_000_000);
    assert_eq!(parsed.timeout_secs, 30);
    assert_eq!(parsed.user_agent, super::DEFAULT_USER_AGENT);

    let custom: super::HttpRequestConfig = serde_json::from_value(serde_json::json!({
        "enabled": true,
        "allowed_domains": ["api.example.com"],
        "max_response_size": 2048,
        "timeout_secs": 7,
        "user_agent": "TestAgent/2.0"
    }))
    .unwrap();
    assert!(custom.enabled);
    assert_eq!(custom.allowed_domains, vec!["api.example.com"]);
    assert_eq!(custom.max_response_size, 2048);
    assert_eq!(custom.timeout_secs, 7);
    assert_eq!(custom.user_agent, "TestAgent/2.0");
}

// 历史默认用户代理可升级到当前值，用户自定义字符串则必须保留。
#[test]
fn user_agent_resolution_upgrades_legacy_default_and_preserves_override() {
    assert_eq!(super::resolve_user_agent(""), super::DEFAULT_USER_AGENT);
    assert_eq!(super::resolve_user_agent(" VibeWindow/1.0 "), super::DEFAULT_USER_AGENT);
    assert_eq!(super::resolve_user_agent(" CustomAgent/2.0 "), "CustomAgent/2.0");
}

// `Default` 与字段级反序列化默认刻意不同，此用例防止二者被误合并。
#[test]
fn web_fetch_config_default_impl_differs_from_field_level_deserialize_defaults() {
    let config = super::WebFetchConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.provider, "fast_html2md");
    assert_eq!(config.api_key, None);
    assert_eq!(config.api_url, None);
    assert_eq!(config.allowed_domains, vec!["*"]);
    assert!(config.blocked_domains.is_empty());
    assert_eq!(config.max_response_size, 500_000);
    assert_eq!(config.timeout_secs, 30);
    assert_eq!(config.user_agent, super::DEFAULT_USER_AGENT);

    let parsed: super::WebFetchConfig = serde_json::from_str("{}").unwrap();
    assert!(!parsed.enabled);
    assert_eq!(parsed.provider, "fast_html2md");
    assert!(parsed.allowed_domains.is_empty());
    assert!(parsed.blocked_domains.is_empty());
    assert_eq!(parsed.max_response_size, 500_000);
    assert_eq!(parsed.timeout_secs, 30);
    assert_eq!(parsed.user_agent, super::DEFAULT_USER_AGENT);

    let custom: super::WebFetchConfig = serde_json::from_value(serde_json::json!({
        "enabled": true,
        "provider": "firecrawl",
        "api_key": "k1,k2",
        "api_url": "https://crawler.example.com",
        "allowed_domains": ["example.com"],
        "blocked_domains": ["private.example.com"],
        "max_response_size": 1024,
        "timeout_secs": 9,
        "user_agent": "Crawler/9.9"
    }))
    .unwrap();
    assert!(custom.enabled);
    assert_eq!(custom.provider, "firecrawl");
    assert_eq!(custom.api_key.as_deref(), Some("k1,k2"));
    assert_eq!(custom.api_url.as_deref(), Some("https://crawler.example.com"));
    assert_eq!(custom.allowed_domains, vec!["example.com"]);
    assert_eq!(custom.blocked_domains, vec!["private.example.com"]);
    assert_eq!(custom.max_response_size, 1024);
    assert_eq!(custom.timeout_secs, 9);
    assert_eq!(custom.user_agent, "Crawler/9.9");
}

// 搜索配置锁定提供方、结果上限与网络参数的缺省和覆盖行为。
#[test]
fn web_search_config_defaults_and_overrides_deserialize() {
    let config = super::WebSearchConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.provider, "duckduckgo");
    assert_eq!(config.api_key, None);
    assert_eq!(config.api_url, None);
    assert_eq!(config.brave_api_key, None);
    assert_eq!(config.max_results, 5);
    assert_eq!(config.timeout_secs, 15);
    assert_eq!(config.user_agent, super::DEFAULT_USER_AGENT);

    let parsed: super::WebSearchConfig = serde_json::from_str("{}").unwrap();
    assert!(!parsed.enabled);
    assert_eq!(parsed.provider, "duckduckgo");
    assert_eq!(parsed.max_results, 5);
    assert_eq!(parsed.timeout_secs, 15);
    assert_eq!(parsed.user_agent, super::DEFAULT_USER_AGENT);

    let custom: super::WebSearchConfig = serde_json::from_value(serde_json::json!({
        "enabled": true,
        "provider": "brave",
        "api_key": "shared-key",
        "api_url": "https://search.example.com",
        "brave_api_key": "brave-key",
        "max_results": 10,
        "timeout_secs": 3,
        "user_agent": "SearchBot/1.2"
    }))
    .unwrap();
    assert!(custom.enabled);
    assert_eq!(custom.provider, "brave");
    assert_eq!(custom.api_key.as_deref(), Some("shared-key"));
    assert_eq!(custom.api_url.as_deref(), Some("https://search.example.com"));
    assert_eq!(custom.brave_api_key.as_deref(), Some("brave-key"));
    assert_eq!(custom.max_results, 10);
    assert_eq!(custom.timeout_secs, 3);
    assert_eq!(custom.user_agent, "SearchBot/1.2");
}
