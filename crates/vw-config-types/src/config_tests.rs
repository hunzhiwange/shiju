//! 验证顶层配置的默认值、兼容字段、模型查找及敏感信息脱敏契约。
//! 用例区分持久化兼容、运行时解析优先级和调试输出三类外部接口。

// 空配置必须形成可启动的最小提供方集合，并将所有可选子系统保持为空。
#[test]
fn config_defaults_promote_shiju_as_primary_provider() {
    let config = super::Config::default();
    assert!(
        config.workspace_dir.ends_with(
            std::path::PathBuf::from(crate::paths::HOME_CONFIG_DIR_NAME).join("workspace")
        )
    );
    assert!(config.config_path.ends_with(
        std::path::PathBuf::from(crate::paths::HOME_CONFIG_DIR_NAME).join("shiju.json")
    ));
    assert_eq!(config.default_provider.as_deref(), Some("shiju"));
    assert_eq!(config.default_model.as_deref(), Some("shiju/deepseek-v4.1-flash"));
    assert_eq!(config.default_temperature, 0.7);
    assert!(config.enabled_providers.is_empty());
    assert_eq!(config.popular_providers, ["shiju"]);
    assert_eq!(
        config.providers.get("shiju"),
        Some(&serde_json::json!({
            "api": "https://router.shiju.cloud/v1",
            "env": ["SHIJU_API_KEY"],
            "adapter": "openai-compatible",
            "models": {
                "deepseek-v4.1-flash": {
                    "id": "deepseek-v4.1-flash",
                    "status": "active"
                }
            }
        }))
    );
    assert!(config.model_providers.is_empty());
    assert!(config.agents.is_empty());
    assert!(config.model_routes.is_empty());
    assert!(config.embedding_routes.is_empty());
    assert!(config.acp.is_empty());
    assert!(config.mcp_servers.is_empty());
    assert!(config.small_model.is_none());
    assert!(config.api_key.is_none());
    assert!(config.api_url.is_none());
    assert!(config.provider_api.is_none());
    assert!(config.model_support_vision.is_none());
}

// 已移除的代理段既不能重新序列化，也不能在读取旧文件时绕回当前模型。
#[test]
fn config_serialization_omits_removed_agent_section() {
    let serialized = serde_json::to_value(super::Config::default()).expect("serialize config");

    assert!(serialized.get("agent").is_none());
    assert!(serialized.get("agents").is_some());
}

#[test]
fn config_deserialization_does_not_route_removed_agent_section() {
    let config: super::Config = serde_json::from_value(serde_json::json!({
        "agent": {"max_iterations": 1}
    }))
    .expect("deserialize config with ignored removed section");

    assert!(config.agents.is_empty());
    assert!(serde_json::to_value(config).expect("serialize config").get("agent").is_none());
}

#[test]
fn config_deserialization_preserves_explicit_provider_and_model() {
    let config: super::Config = serde_json::from_value(serde_json::json!({
        "default_provider": "openrouter",
        "default_model": "openrouter/explicit-model"
    }))
    .unwrap();

    assert_eq!(config.default_provider.as_deref(), Some("openrouter"));
    assert_eq!(config.default_model.as_deref(), Some("openrouter/explicit-model"));
}

#[test]
fn global_database_path_uses_shared_database_directory() {
    let home = std::path::Path::new("/tmp/vibewindow-home");
    assert_eq!(
        crate::paths::global_database_path(home),
        crate::paths::home_config_dir(home).join("database/vibe-window.sqlite3")
    );
    assert_eq!(
        crate::paths::global_database_config_path(home),
        crate::paths::home_config_dir(home).join("database/config.json")
    );
}

// 推理级别接受受控别名，提供方专属设置始终优先于运行时兼容字段。
#[test]
fn reasoning_level_normalization_accepts_known_values() {
    // All valid values
    assert_eq!(
        super::Config::normalize_reasoning_level_override(Some("minimal"), "test"),
        Some("minimal".to_string())
    );
    assert_eq!(
        super::Config::normalize_reasoning_level_override(Some("low"), "test"),
        Some("low".to_string())
    );
    assert_eq!(
        super::Config::normalize_reasoning_level_override(Some("medium"), "test"),
        Some("medium".to_string())
    );
    assert_eq!(
        super::Config::normalize_reasoning_level_override(Some("high"), "test"),
        Some("high".to_string())
    );
    assert_eq!(
        super::Config::normalize_reasoning_level_override(Some("xhigh"), "test"),
        Some("xhigh".to_string())
    );

    // Case insensitivity
    assert_eq!(
        super::Config::normalize_reasoning_level_override(Some("X-High"), "test"),
        Some("xhigh".to_string())
    );
    assert_eq!(
        super::Config::normalize_reasoning_level_override(Some("MEDIUM"), "test"),
        Some("medium".to_string())
    );
    assert_eq!(
        super::Config::normalize_reasoning_level_override(Some("Minimal"), "test"),
        Some("minimal".to_string())
    );

    // Hyphen and underscore normalization
    assert_eq!(
        super::Config::normalize_reasoning_level_override(Some("x-high"), "test"),
        Some("xhigh".to_string())
    );
    assert_eq!(
        super::Config::normalize_reasoning_level_override(Some("x_high"), "test"),
        Some("xhigh".to_string())
    );

    // None and empty
    assert_eq!(super::Config::normalize_reasoning_level_override(None, "test"), None);
    assert_eq!(super::Config::normalize_reasoning_level_override(Some(""), "test"), None);
    assert_eq!(super::Config::normalize_reasoning_level_override(Some("  "), "test"), None);

    // Invalid values
    assert_eq!(super::Config::normalize_reasoning_level_override(Some("invalid"), "test"), None);
    assert_eq!(super::Config::normalize_reasoning_level_override(Some("veryhigh"), "test"), None);
}

#[test]
fn provider_reasoning_level_takes_precedence_over_runtime_alias() {
    let mut config = super::Config::default();

    // provider set, runtime also set → provider wins
    config.provider.reasoning_level = Some("medium".into());
    config.runtime.reasoning_level = Some("high".into());
    assert_eq!(config.effective_provider_reasoning_level(), Some("medium".to_string()));

    // provider cleared, runtime set → runtime used as fallback
    config.provider.reasoning_level = None;
    assert_eq!(config.effective_provider_reasoning_level(), Some("high".to_string()));

    // both set to the same value
    config.provider.reasoning_level = Some("high".into());
    config.runtime.reasoning_level = Some("high".into());
    assert_eq!(config.effective_provider_reasoning_level(), Some("high".to_string()));

    // both None → None
    config.provider.reasoning_level = None;
    config.runtime.reasoning_level = None;
    assert_eq!(config.effective_provider_reasoning_level(), None);

    // provider set, runtime None → provider wins
    config.provider.reasoning_level = Some("low".into());
    config.runtime.reasoning_level = None;
    assert_eq!(config.effective_provider_reasoning_level(), Some("low".to_string()));
}

// 模型提供方名称按不区分大小写查找，但返回的仍是原始配置条目。
#[test]
fn lookup_model_provider_profile_finds_by_case_insensitive_name() {
    let mut config = super::Config::default();
    let profile = super::ModelProviderConfig::default();
    config.model_providers.insert("GPT-4".to_string(), profile.clone());

    // Exact match
    let result = config.lookup_model_provider_profile("GPT-4");
    assert!(result.is_some());
    assert_eq!(result.unwrap().0, "GPT-4");

    // Case-insensitive match
    let result = config.lookup_model_provider_profile("gpt-4");
    assert!(result.is_some());
    assert_eq!(result.unwrap().0, "GPT-4");

    // Mixed case
    let result = config.lookup_model_provider_profile("Gpt-4");
    assert!(result.is_some());
    assert_eq!(result.unwrap().0, "GPT-4");

    // No match → None
    let result = config.lookup_model_provider_profile("nonexistent");
    assert!(result.is_none());

    // Empty needle → None
    let result = config.lookup_model_provider_profile("");
    assert!(result.is_none());

    // Whitespace needle → None (trimmed to empty)
    let result = config.lookup_model_provider_profile("  ");
    assert!(result.is_none());

    // Multiple providers, match correct one
    config.model_providers.insert("Claude".to_string(), profile);
    let result = config.lookup_model_provider_profile("claude");
    assert!(result.is_some());
    assert_eq!(result.unwrap().0, "Claude");
}

#[test]
fn acp_agent_config_default_is_empty() {
    let cfg = super::AcpAgentConfig::default();
    assert_eq!(cfg.command, "");
    assert!(cfg.args.is_empty());
    assert!(cfg.env.is_empty());
}

// 调试输出只展示结构与标识，密钥、令牌及提供方原始载荷不得泄漏。
#[test]
fn config_debug_impl_redacts_sensitive_sections() {
    let config = super::Config::default();
    let debug_str = format!("{:?}", config);

    // Verify key fields appear in output
    assert!(debug_str.contains("workspace_dir"));
    assert!(debug_str.contains("config_path"));
    assert!(debug_str.contains("default_provider"));
    assert!(debug_str.contains("default_model"));
    assert!(debug_str.contains("default_temperature"));
    assert!(debug_str.contains("sensitive_sections"));
    assert!(debug_str.contains("***REDACTED***"));
    assert!(debug_str.contains("enabled_channels_count"));
    assert!(debug_str.contains("acp_count"));
    assert!(debug_str.ends_with(".. }"));
}

#[test]
fn config_debug_impl_shows_provider_and_agent_ids() {
    let mut config = super::Config::default();
    config.model_providers.insert("gpt-4".to_string(), super::ModelProviderConfig::default());
    config.model_providers.insert("claude".to_string(), super::ModelProviderConfig::default());
    config.agents.insert("agent-a".to_string(), super::DelegateAgentConfig::default());
    config.agents.insert("agent-b".to_string(), super::DelegateAgentConfig::default());

    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("gpt-4"));
    assert!(debug_str.contains("claude"));
    assert!(debug_str.contains("agent-a"));
    assert!(debug_str.contains("agent-b"));
}

#[test]
fn config_debug_impl_shows_channel_count() {
    let mut config = super::Config::default();
    config.channels_config.cli = true;
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("cli_channel_enabled: true"));
    assert!(debug_str.contains("enabled_channels_count"));
}

#[test]
fn config_debug_impl_shows_model_route_counts() {
    let mut config = super::Config::default();
    config.model_routes.push(super::ModelRouteConfig {
        hint: "reasoning".into(),
        provider: "openrouter".into(),
        model: "claude-opus".into(),
        max_tokens: None,
        api_key: None,
    });
    config.model_routes.push(super::ModelRouteConfig {
        hint: "fast".into(),
        provider: "groq".into(),
        model: "llama".into(),
        max_tokens: None,
        api_key: None,
    });
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("model_routes_count: 2"));
}

#[test]
fn config_debug_impl_shows_embedding_route_counts() {
    let mut config = super::Config::default();
    config.embedding_routes.push(super::EmbeddingRouteConfig {
        hint: "semantic".into(),
        provider: "openai".into(),
        model: "text-embedding-3".into(),
        dimensions: None,
        api_key: None,
    });
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("embedding_routes_count: 1"));
}

#[test]
fn config_debug_impl_shows_acp_count() {
    let mut config = super::Config::default();
    config.acp.insert("agent-1".to_string(), super::AcpAgentConfig::default());
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("acp_count: 1"));
}

// MCP 的历史外部字段名继续保留，以兼容现有网关与配置文件。
#[test]
fn config_serializes_mcp_servers_as_acp_field_name() {
    let config = super::Config {
        mcp_servers: vec![serde_json::json!({
            "name": "local",
            "type": "stdio",
            "command": "npx"
        })],
        ..Default::default()
    };

    let value = serde_json::to_value(config).expect("serialize config");

    assert_eq!(value["mcpServers"][0]["name"], "local");
    assert!(value.get("mcp_servers").is_none());
}

#[test]
fn config_serializes_mcp_server_overrides_as_global_field_name() {
    let config = super::Config {
        mcp_server_overrides: vec![serde_json::json!({
            "name": "plugin-mcp",
            "enabled": false
        })],
        ..Default::default()
    };

    let value = serde_json::to_value(config).expect("serialize config");

    assert_eq!(value["mcpServerOverrides"][0]["name"], "plugin-mcp");
    assert_eq!(value["mcpServerOverrides"][0]["enabled"], false);
    assert!(value.get("mcp_server_overrides").is_none());
}

#[test]
fn config_debug_impl_shows_api_key_and_url_configured_flags() {
    let config = super::Config {
        api_key: Some("sk-abc".to_string()),
        api_url: Some("https://api.example.com".to_string()),
        ..Default::default()
    };
    let debug_str = format!("{:?}", config);
    // The Debug impl shows whether they're configured, not the values
    assert!(debug_str.contains("api_key_configured: true"));
    assert!(debug_str.contains("api_url_configured: true"));

    let config2 = super::Config::default();
    let debug_str2 = format!("{:?}", config2);
    assert!(debug_str2.contains("api_key_configured: false"));
    assert!(debug_str2.contains("api_url_configured: false"));
}

#[test]
fn config_debug_impl_shows_providers_count() {
    let mut config = super::Config::default();
    config.providers.insert("openai".to_string(), serde_json::json!({"key": "value"}));
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("providers_count: 1"));
}
