use super::*;
use config_agent::DelegateAgentConfig;

#[test]
fn forced_child_model_overrides_parent_selection() {
    let agent = DelegateAgentConfig {
        provider: "token-plan".to_string(),
        model: "qwen3.7-plus".to_string(),
        force_model_selection: true,
        ..DelegateAgentConfig::default()
    };

    assert_eq!(
        delegated_model_override(&agent, Some("wrong-provider/qwen3.7-plus".to_string())),
        Some("token-plan/qwen3.7-plus".to_string())
    );
}

#[test]
fn incomplete_forced_child_binding_is_ignored() {
    let agent = DelegateAgentConfig {
        provider: "token-plan".to_string(),
        force_model_selection: true,
        ..DelegateAgentConfig::default()
    };

    assert_eq!(
        delegated_model_override(&agent, Some("token-plan/qwen3.7-plus".to_string())),
        Some("token-plan/qwen3.7-plus".to_string())
    );
}

#[test]
fn child_soft_preference_does_not_bypass_profile_fit() {
    let agent = DelegateAgentConfig {
        provider: "token-plan".to_string(),
        model: "qwen3.7-plus".to_string(),
        ..DelegateAgentConfig::default()
    };

    assert_eq!(delegated_model_override(&agent, None), None);
}

// 历史整理保留角色与工具结果，只把最后一个用户输入提升为本轮查询。

#[tokio::test]
async fn resolve_main_request_overrides_preserves_registered_tool_selection() {
    let overrides = resolve_delegate_policy(
        "main".to_string(),
        config_agent::AgentDefinitionConfig::default(),
        None,
        Some(vec!["file_read".to_string(), "memory_store".to_string(), "schedule".to_string()]),
        Some("provider/model".to_string()),
    )
    .expect("main overrides");

    assert_eq!(overrides.agent.as_deref(), Some("main"));
    assert_eq!(
        overrides.allowed_tools,
        Some(vec!["file_read".to_string(), "memory_store".to_string(), "schedule".to_string(),])
    );
}
