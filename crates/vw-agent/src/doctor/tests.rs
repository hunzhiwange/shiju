//!
//! Doctor 模块测试集
//!
//! 本模块包含 doctor 诊断子系统的所有单元测试，用于验证配置校验、
//! 环境检查、模型探测等功能的行为正确性。
//!
//! # 测试分类
//!
//! - **Provider 验证测试**：验证 Provider 标识符的格式和有效性检查
//! - **诊断项测试**：验证诊断结果项的显示和分类
//! - **模型探测测试**：验证模型探测错误分类逻辑
//! - **配置校验测试**：验证配置语义检查的各个方面
//! - **环境检查测试**：验证环境依赖项的检测
//! - **工具函数测试**：验证辅助解析和格式化函数
//!

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    /// 测试 Provider 验证逻辑对自定义 URL 格式的检查
    ///
    /// # 测试用例
    ///
    /// - 有效的内置 Provider 标识符应该通过验证（如 "openrouter"）
    /// - 有效的自定义 URL 格式应该通过验证（如 "custom:https://..."）
    /// - 带前缀的自定义 Provider 格式应该通过验证（如 "anthropic-custom:https://..."）
    /// - 无效的自定义格式（缺少 URL）应该返回错误
    /// - 未知的 Provider 标识符应该返回错误
    #[test]
    fn provider_validation_checks_custom_url_shape() {
        // 验证内置 Provider 标识符
        assert!(provider_validation_error("openrouter").is_none());
        // 验证标准自定义 URL 格式
        assert!(provider_validation_error("custom:https://example.com").is_none());
        // 验证带前缀的自定义 Provider 格式
        assert!(provider_validation_error("anthropic-custom:https://example.com").is_none());

        // 验证自定义格式缺少 URL 时返回错误
        let invalid_custom = provider_validation_error("custom:").unwrap_or_default();
        assert!(invalid_custom.contains("requires a URL"));

        // 验证未知 Provider 返回错误
        let invalid_unknown = provider_validation_error("totally-fake").unwrap_or_default();
        assert!(invalid_unknown.contains("Unknown provider"));
    }

    /// 测试诊断项的图标显示
    ///
    /// 验证不同严重级别的诊断项能够正确显示对应的图标：
    /// - Ok 级别显示 ✅
    /// - Warn 级别显示 ⚠️
    /// - Error 级别显示 ❌
    #[test]
    fn diag_item_icons() {
        assert_eq!(DiagItem::ok("t", "m").icon(), "✅");
        assert_eq!(DiagItem::warn("t", "m").icon(), "⚠️ ");
        assert_eq!(DiagItem::error("t", "m").icon(), "❌");
    }

    #[test]
    fn diag_item_constructors_preserve_fields() {
        let cases = [
            (DiagItem::ok("config", "valid"), Severity::Ok, "config", "valid"),
            (DiagItem::warn("workspace", "limited"), Severity::Warn, "workspace", "limited"),
            (DiagItem::error("daemon", "unavailable"), Severity::Error, "daemon", "unavailable"),
        ];

        for (item, expected_severity, expected_category, expected_message) in cases {
            assert_eq!(item.severity, expected_severity);
            assert_eq!(item.category, expected_category);
            assert_eq!(item.message, expected_message);
        }
    }

    #[test]
    fn diag_item_into_result_preserves_fields_and_result_icon() {
        let result = DiagItem::warn("daemon", "scheduler component not tracked").into_result();

        assert_eq!(result.severity, Severity::Warn);
        assert_eq!(result.category, "daemon");
        assert_eq!(result.message, "scheduler component not tracked");
        assert_eq!(result.icon(), "⚠️ ");
    }

    #[test]
    fn diag_result_icons_match_all_severities() {
        let cases = [(Severity::Ok, "✅"), (Severity::Warn, "⚠️ "), (Severity::Error, "❌")];

        for (severity, expected_icon) in cases {
            let result = DiagResult { severity, category: String::new(), message: String::new() };

            assert_eq!(result.icon(), expected_icon);
        }
    }

    #[test]
    fn severity_serializes_to_lowercase_names() {
        let cases = [(Severity::Ok, "ok"), (Severity::Warn, "warn"), (Severity::Error, "error")];

        for (severity, expected) in cases {
            assert_eq!(serde_json::to_value(severity).unwrap(), serde_json::json!(expected));
        }
    }

    #[test]
    fn diag_result_serializes_structured_payload() {
        let result = DiagResult {
            severity: Severity::Error,
            category: "workspace".into(),
            message: "path is not writable".into(),
        };

        assert_eq!(
            serde_json::to_value(result).unwrap(),
            serde_json::json!({
                "severity": "error",
                "category": "workspace",
                "message": "path is not writable",
            })
        );
    }

    /// 测试模型探测错误分类 - 不支持的 Provider 标记为跳过
    ///
    /// 当 Provider 不支持实时模型发现时，应将探测结果标记为 Skipped 状态，
    /// 而不是失败或错误状态。
    #[test]
    fn classify_model_probe_error_marks_unsupported_as_skipped() {
        // 模拟不支持模型发现的 Provider 错误消息
        let outcome = classify_model_probe_error(
            "Provider 'copilot' does not support live model discovery yet",
        );
        // 应该标记为 Skipped 而非错误
        assert_eq!(outcome, ModelProbeOutcome::Skipped);
    }

    /// 测试模型探测错误分类 - 认证和套餐问题标记为 AuthOrAccess
    ///
    /// 验证以下场景被正确分类为认证/访问问题：
    /// - 401 未授权错误（API 密钥无效或缺失）
    /// - 429 套餐限制错误（当前套餐不包含请求的模型）
    #[test]
    fn classify_model_probe_error_marks_auth_and_plan_issues() {
        // 测试认证失败（401 错误）
        let auth_outcome = classify_model_probe_error("OpenAI API error (401): unauthorized");
        assert_eq!(auth_outcome, ModelProbeOutcome::AuthOrAccess);

        // 测试套餐限制（429 错误）
        let plan_outcome = classify_model_probe_error(
            "Z.AI API error (429): plan does not include requested model",
        );
        assert_eq!(plan_outcome, ModelProbeOutcome::AuthOrAccess);
    }

    /// 测试配置校验 - 捕获无效的温度值
    ///
    /// 温度值应该在 [0.0, 2.0] 范围内，超出范围的值应被标记为错误。
    /// 此测试验证温度值为 5.0 时会被正确检测并报告为错误。
    #[test]
    fn config_validation_catches_bad_temperature() {
        let mut config = Config::default();
        // 设置超出有效范围的温度值
        config.default_temperature = 5.0;
        let mut items = Vec::new();
        check_config_semantics(&config, &mut items);
        // 查找温度相关的诊断项
        let temp_item = items.iter().find(|i| i.message.contains("temperature"));
        assert!(temp_item.is_some());
        // 应该被标记为错误级别
        assert_eq!(temp_item.unwrap().severity, Severity::Error);
    }

    /// 测试配置校验 - 接受有效的温度值
    ///
    /// 验证在有效范围内的温度值（如 0.7）能够通过配置校验，
    /// 被标记为 Ok 状态。
    #[test]
    fn config_validation_accepts_valid_temperature() {
        let mut config = Config::default();
        // 设置有效范围内的温度值
        config.default_temperature = 0.7;
        let mut items = Vec::new();
        check_config_semantics(&config, &mut items);
        let temp_item = items.iter().find(|i| i.message.contains("temperature"));
        assert!(temp_item.is_some());
        // 应该被标记为 Ok 级别
        assert_eq!(temp_item.unwrap().severity, Severity::Ok);
    }

    /// 测试配置校验 - 警告未配置任何通道
    ///
    /// 当配置中没有配置任何通信通道（如 Telegram、Slack 等）时，
    /// 应该发出警告，因为这可能意味着代理无法接收外部消息。
    #[test]
    fn config_validation_warns_no_channels() {
        let config = Config::default();
        let mut items = Vec::new();
        check_config_semantics(&config, &mut items);
        // 查找通道相关的诊断项
        let ch_item = items.iter().find(|i| i.message.contains("channel"));
        assert!(ch_item.is_some());
        // 应该被标记为警告级别
        assert_eq!(ch_item.unwrap().severity, Severity::Warn);
    }

    /// 测试配置校验 - 捕获未知的默认 Provider
    ///
    /// 当配置中指定的默认 Provider 不是已知的 Provider 标识符时，
    /// 应该被标记为错误，因为这会导致代理无法正常工作。
    #[test]
    fn config_validation_catches_unknown_provider() {
        let mut config = Config::default();
        // 设置一个不存在的 Provider
        config.default_provider = Some("totally-fake".into());
        let mut items = Vec::new();
        check_config_semantics(&config, &mut items);
        let prov_item = items.iter().find(|i| i.message.contains("default provider"));
        assert!(prov_item.is_some());
        // 应该被标记为错误级别
        assert_eq!(prov_item.unwrap().severity, Severity::Error);
    }

    /// 测试配置校验 - 捕获格式错误的自定义 Provider
    ///
    /// 自定义 Provider 必须遵循 "custom:<URL>" 格式，缺少 URL 或格式错误
    /// 的配置应该被标记为错误。
    #[test]
    fn config_validation_catches_malformed_custom_provider() {
        let mut config = Config::default();
        // 设置格式错误的自定义 Provider（缺少 URL）
        config.default_provider = Some("custom:".into());
        let mut items = Vec::new();
        check_config_semantics(&config, &mut items);

        let prov_item = items
            .iter()
            .find(|item| item.message.contains("default provider \"custom:\" is invalid"));
        assert!(prov_item.is_some());
        // 应该被标记为错误级别
        assert_eq!(prov_item.unwrap().severity, Severity::Error);
    }

    /// 测试配置校验 - 接受有效的自定义 Provider
    ///
    /// 验证格式正确的自定义 Provider URL（如 "custom:https://my-api.com"）
    /// 能够通过配置校验，被标记为 Ok 状态。
    #[test]
    fn config_validation_accepts_custom_provider() {
        let mut config = Config::default();
        // 设置格式正确的自定义 Provider
        config.default_provider = Some("custom:https://my-api.com".into());
        let mut items = Vec::new();
        check_config_semantics(&config, &mut items);
        let prov_item = items.iter().find(|i| i.message.contains("is valid"));
        assert!(prov_item.is_some());
        // 应该被标记为 Ok 级别
        assert_eq!(prov_item.unwrap().severity, Severity::Ok);
    }

    /// 测试配置校验 - 警告模型路由配置中模型名称为空
    ///
    /// 模型路由配置中的 model 字段不应为空，空模型名称会导致路由失败，
    /// 应该发出警告。
    #[test]
    fn config_validation_warns_empty_model_route() {
        let mut config = Config::default();
        // 配置一个模型名称为空的路由
        config.model_routes = vec![crate::app::agent::config::ModelRouteConfig {
            hint: "fast".into(),
            provider: "groq".into(),
            model: String::new(), // 空模型名称
            max_tokens: None,
            api_key: None,
        }];
        let mut items = Vec::new();
        check_config_semantics(&config, &mut items);
        let route_item = items.iter().find(|i| i.message.contains("empty model"));
        assert!(route_item.is_some());
        // 应该被标记为警告级别
        assert_eq!(route_item.unwrap().severity, Severity::Warn);
    }

    /// 测试配置校验 - 警告嵌入路由配置中模型名称为空
    ///
    /// 嵌入路由（embedding route）用于向量化/嵌入操作，模型名称为空
    /// 会导致嵌入功能无法使用，应该发出警告。
    #[test]
    fn config_validation_warns_empty_embedding_route_model() {
        let mut config = Config::default();
        // 配置一个模型名称为空的嵌入路由
        config.embedding_routes = vec![crate::app::agent::config::EmbeddingRouteConfig {
            hint: "semantic".into(),
            provider: "openai".into(),
            model: String::new(), // 空模型名称
            dimensions: Some(1536),
            api_key: None,
        }];

        let mut items = Vec::new();
        check_config_semantics(&config, &mut items);
        let route_item = items
            .iter()
            .find(|item| item.message.contains("embedding route \"semantic\" has empty model"));
        assert!(route_item.is_some());
        // 应该被标记为警告级别
        assert_eq!(route_item.unwrap().severity, Severity::Warn);
    }

    /// 测试配置校验 - 警告嵌入路由使用了无效的 Provider
    ///
    /// 嵌入功能需要特定的 Provider 支持（如 OpenAI），使用不支持嵌入
    /// 功能的 Provider（如 Groq）应该发出警告。
    #[test]
    fn config_validation_warns_invalid_embedding_route_provider() {
        let mut config = Config::default();
        // 配置使用不支持嵌入功能的 Provider
        config.embedding_routes = vec![crate::app::agent::config::EmbeddingRouteConfig {
            hint: "semantic".into(),
            provider: "groq".into(), // Groq 不支持嵌入功能
            model: "text-embedding-3-small".into(),
            dimensions: None,
            api_key: None,
        }];

        let mut items = Vec::new();
        check_config_semantics(&config, &mut items);
        let route_item =
            items.iter().find(|item| item.message.contains("uses invalid provider \"groq\""));
        assert!(route_item.is_some());
        // 应该被标记为警告级别
        assert_eq!(route_item.unwrap().severity, Severity::Warn);
    }

    /// 测试配置校验 - 警告嵌入模型引用了不存在的路由 hint
    ///
    /// 当配置中的嵌入模型使用 hint 引用（如 "hint:semantic"）但没有
    /// 对应的嵌入路由配置时，应该发出警告。
    #[test]
    fn config_validation_warns_missing_embedding_hint_target() {
        let mut config = Config::default();
        // 配置使用 hint 引用，但没有对应的嵌入路由
        config.memory.embedding_model = "hint:semantic".into();

        let mut items = Vec::new();
        check_config_semantics(&config, &mut items);
        let route_item = items
            .iter()
            .find(|item| item.message.contains("no matching [[embedding_routes]] entry exists"));
        assert!(route_item.is_some());
        // 应该被标记为警告级别
        assert_eq!(route_item.unwrap().severity, Severity::Warn);
    }

    /// 测试环境检查 - 检测 git 工具是否可用
    ///
    /// 验证环境检查功能能够正确检测 git 工具的可用性。
    /// git 在 CI/CD 和开发环境中通常是必需的工具。
    #[test]
    fn environment_check_finds_git() {
        let mut items = Vec::new();
        check_environment(&mut items);
        let git_item = items.iter().find(|i| i.message.starts_with("git:"));
        // git 应该在任何 CI/开发环境中可用
        assert!(git_item.is_some());
        assert_eq!(git_item.unwrap().severity, Severity::Ok);
    }

    /// 测试 df 命令输出解析 - 使用最后一行数据
    ///
    /// 解析 df 命令输出时，应该使用最后一行的数据（通常是根挂载点），
    /// 而不是标题行或其他挂载点。
    #[test]
    fn parse_df_available_mb_uses_last_data_line() {
        // 模拟 df 命令的输出格式
        let stdout =
            "Filesystem 1M-blocks Used Available Use% Mounted on\n/dev/sda1 1000 500 500 50% /\n";
        // 应该解析出最后一行的可用空间（500 MB）
        assert_eq!(parse_df_available_mb(stdout), Some(500));
    }

    /// 测试显示文本截断功能 - 保持 UTF-8 字符边界
    ///
    /// 截断包含多字节 UTF-8 字符（如 emoji）的字符串时，必须确保
    /// 不会在字符中间截断，避免产生无效的 UTF-8 序列。
    #[test]
    fn truncate_for_display_preserves_utf8_boundaries() {
        // 测试包含 emoji 的字符串截断
        let preview = truncate_for_display("🙂example-alpha-build", 3);
        // 应该正确保留 emoji 完整性，并添加省略号
        assert_eq!(preview, "🙂ex…");
    }

    /// 测试工作区探测路径生成 - 隐藏且唯一
    ///
    /// 工作区探测路径应该满足以下要求：
    /// - 每次调用生成唯一路径（避免并发冲突）
    /// - 文件名以 ".vibewindow_doctor_probe_" 开头（隐藏文件）
    /// - 位于指定的工作区目录下
    #[test]
    fn workspace_probe_path_is_hidden_and_unique() {
        let tmp = TempDir::new().unwrap();
        // 生成两个探测路径
        let first = workspace_probe_path(tmp.path());
        let second = workspace_probe_path(tmp.path());

        // 两次调用应该生成不同的路径（唯一性）
        assert_ne!(first, second);
        // 验证路径以隐藏文件前缀开头
        assert!(
            first
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".vibewindow_doctor_probe_"))
        );
    }

    /// 测试配置校验 - 委托代理按排序顺序报告
    ///
    /// 验证当配置中存在多个委托代理时，诊断结果按照代理名称的
    /// 字母顺序排序输出，确保结果的可预测性和一致性。
    #[test]
    fn config_validation_reports_delegate_agents_in_sorted_order() {
        let mut config = Config::default();
        // 插入两个委托代理，故意以非字母顺序插入
        config.agents.insert(
            "zeta".into(),
            crate::app::agent::config::DelegateAgentConfig {
                label: None,
                description: None,
                builtin: false,
                mode: "all".into(),
                enabled: true,
                allow_as_subagent: false,
                max_parallel_subagents: 4,
                provider: "totally-fake".into(),
                model: "model-z".into(),
                force_model_selection: false,
                system_prompt: None,
                api_key: None,
                temperature: None,
                top_p: None,
                identity_format: None,
                hidden: false,
                max_depth: 3,
                agentic: false,
                allowed_tools: Vec::new(),
                skill_policy: Default::default(),
                allowed_skills: Vec::new(),
                enabled_identity_files: Vec::new(),
                options: std::collections::HashMap::new(),
                permission: serde_json::Value::Null,
                max_iterations: 10,
                compact_context: false,
                max_history_messages: 100,
                parallel_tools: false,
                steps: None,
            },
        );
        config.agents.insert(
            "alpha".into(),
            crate::app::agent::config::DelegateAgentConfig {
                label: None,
                description: None,
                builtin: false,
                mode: "all".into(),
                enabled: true,
                allow_as_subagent: false,
                max_parallel_subagents: 4,
                provider: "totally-fake".into(),
                model: "model-a".into(),
                force_model_selection: false,
                system_prompt: None,
                api_key: None,
                temperature: None,
                top_p: None,
                identity_format: None,
                hidden: false,
                max_depth: 3,
                agentic: false,
                allowed_tools: Vec::new(),
                skill_policy: Default::default(),
                allowed_skills: Vec::new(),
                enabled_identity_files: Vec::new(),
                options: std::collections::HashMap::new(),
                permission: serde_json::Value::Null,
                max_iterations: 10,
                compact_context: false,
                max_history_messages: 100,
                parallel_tools: false,
                steps: None,
            },
        );

        let mut items = Vec::new();
        check_config_semantics(&config, &mut items);

        // 提取所有代理相关的诊断消息
        let agent_messages: Vec<_> = items
            .iter()
            .filter(|item| item.message.starts_with("agent \""))
            .map(|item| item.message.as_str())
            .collect();

        // 验证存在两条代理消息
        assert_eq!(agent_messages.len(), 2);
        // 验证消息按字母顺序排列（alpha 在前，zeta 在后）
        assert!(agent_messages[0].contains("agent \"alpha\""));
        assert!(agent_messages[1].contains("agent \"zeta\""));
    }
}
