//! 测试代理模块门面对核心类型和兼容入口的导出契约。

use super::*;
use crate::app::agent::permission::next::Action;
use serde_json::{Value, json};

fn config(mode: &str, hidden: bool) -> DelegateAgentConfig {
    DelegateAgentConfig { mode: mode.to_string(), hidden, ..DelegateAgentConfig::default() }
}

fn resolved(mode: &str, hidden: bool) -> ResolvedAgentDefinition {
    ResolvedAgentDefinition { definition: config(mode, hidden), permission: Vec::new() }
}

fn state(entries: &[(&str, ResolvedAgentDefinition)], default_agent: Option<&str>) -> State {
    State {
        agents: entries.iter().map(|(key, value)| ((*key).to_string(), value.clone())).collect(),
        default_agent: default_agent.map(str::to_string),
    }
}

#[test]
fn info_serialization_flattens_and_round_trips_definition() {
    let info = Info {
        key: "review".into(),
        definition: DelegateAgentConfig {
            provider: "openai".into(),
            model: "gpt-5".into(),
            description: Some("Reviews changes".into()),
            ..DelegateAgentConfig::default()
        },
    };

    let value = serde_json::to_value(&info).unwrap();

    assert_eq!(value["key"], "review");
    assert_eq!(value["provider"], "openai");
    assert_eq!(value["model"], "gpt-5");
    assert!(value.get("definition").is_none());

    let decoded: Info = serde_json::from_value(value).unwrap();
    assert_eq!(decoded.key, "review");
    assert_eq!(decoded.definition.provider, "openai");
    assert_eq!(decoded.definition.model, "gpt-5");
    assert_eq!(decoded.definition.description.as_deref(), Some("Reviews changes"));
}

#[test]
fn primary_visible_excludes_hidden_and_subagents() {
    assert!(is_primary_visible(&config("primary", false)));
    assert!(is_primary_visible(&config("all", false)));
    assert!(!is_primary_visible(&config("subagent", false)));
    assert!(!is_primary_visible(&config("primary", true)));
}

#[test]
fn choose_default_agent_validates_configured_default() {
    let missing = state(&[("main", resolved("primary", false))], Some("ghost"));
    assert_eq!(choose_default_agent(&missing), Err("default agent \"ghost\" not found".into()));

    let subagent = state(&[("worker", resolved("subagent", false))], Some("worker"));
    assert_eq!(
        choose_default_agent(&subagent),
        Err("default agent \"worker\" is a subagent".into())
    );

    let hidden = state(&[("main", resolved("primary", true))], Some("main"));
    assert_eq!(choose_default_agent(&hidden), Err("default agent \"main\" is hidden".into()));
}

#[test]
fn choose_default_agent_uses_configured_then_builtin_priority() {
    let configured = state(
        &[("main", resolved("primary", false)), ("custom", resolved("all", false))],
        Some("custom"),
    );
    assert_eq!(choose_default_agent(&configured).unwrap(), "custom");

    let main =
        state(&[("build", resolved("primary", false)), ("main", resolved("primary", false))], None);
    assert_eq!(choose_default_agent(&main).unwrap(), "main");

    let main_only = state(&[("main", resolved("primary", false))], None);
    assert_eq!(choose_default_agent(&main_only).unwrap(), "main");
}

#[test]
fn choose_default_agent_falls_back_alphabetically_or_errors() {
    let fallback =
        state(&[("zeta", resolved("all", false)), ("alpha", resolved("primary", false))], None);
    assert_eq!(choose_default_agent(&fallback).unwrap(), "alpha");

    let none = state(
        &[("hidden", resolved("primary", true)), ("worker", resolved("subagent", false))],
        None,
    );
    assert_eq!(choose_default_agent(&none), Err("no primary visible agent found".into()));
}

#[test]
fn normalize_agent_mode_accepts_known_aliases() {
    assert_eq!(normalize_agent_mode(" primary "), "primary");
    assert_eq!(normalize_agent_mode("SUB_AGENT"), "subagent");
    assert_eq!(normalize_agent_mode("sub-agent"), "subagent");
    assert_eq!(normalize_agent_mode("unknown"), "all");
}

#[test]
fn merge_json_value_deep_merges_objects_and_replaces_scalars() {
    let mut target = json!({
        "edit": {"*": "deny", "src/*.rs": "allow"},
        "read": "ask"
    });
    let source = json!({
        "edit": {"tests/*.rs": "allow"},
        "read": {"*": "allow"},
        "bash": "deny"
    });

    merge_json_value(&mut target, &source);

    assert_eq!(target["edit"]["*"], "deny");
    assert_eq!(target["edit"]["src/*.rs"], "allow");
    assert_eq!(target["edit"]["tests/*.rs"], "allow");
    assert_eq!(target["read"]["*"], "allow");
    assert_eq!(target["bash"], "deny");

    merge_json_value(&mut target["bash"], &Value::Bool(true));
    assert_eq!(target["bash"], true);
}

#[test]
fn load_agent_configs_merges_configured_agents_with_builtins() {
    let cfg = json!({
        "agents": {
            "custom": {
                "provider": "openai",
                "model": "gpt-5",
                "mode": "sub_agent"
            },
            "main": {
                "provider": "anthropic",
                "model": "claude",
                "mode": "primary"
            }
        }
    });

    let agents = load_agent_configs(&cfg);

    for key in ["build", "plan", "compaction", "title", "summary"] {
        assert!(!agents.contains_key(key), "{key} must not be loaded");
    }
    assert_eq!(agents["custom"].provider, "openai");
    assert_eq!(agents["custom"].mode, "sub_agent");
    assert_eq!(agents["main"].label.as_deref(), Some("Main"));
    assert!(agents["main"].builtin);

    let fallback = load_agent_configs(&json!({"agents": "not an object"}));
    assert!(fallback.contains_key("main"));
    assert_eq!(fallback.len(), 5);
}

#[test]
fn load_agent_configs_discards_malformed_configured_map() {
    let agents = load_agent_configs(&json!({
        "agents": {
            "incomplete": {
                "provider": "openai"
            }
        }
    }));

    assert!(!agents.contains_key("incomplete"));
    assert!(agents.contains_key("main"));
    assert_eq!(agents.len(), 5);
}

#[test]
fn load_agent_configs_uses_builtins_when_agents_key_is_missing() {
    let agents = load_agent_configs(&json!({"default_agent": "main"}));

    assert!(agents.contains_key("main"));
    assert_eq!(agents.len(), 5);
    assert!(agents.values().all(|agent| agent.builtin));
}

#[test]
fn resolve_model_ref_handles_direct_split_and_missing_values() {
    let direct = DelegateAgentConfig {
        provider: "openai".into(),
        model: "gpt-5".into(),
        ..DelegateAgentConfig::default()
    };
    let direct_ref = resolve_model_ref(&direct).unwrap();
    assert_eq!(direct_ref.provider_id, "openai");
    assert_eq!(direct_ref.model_id, "gpt-5");

    let split = DelegateAgentConfig {
        model: "anthropic/claude-sonnet".into(),
        ..DelegateAgentConfig::default()
    };
    let split_ref = resolve_model_ref(&split).unwrap();
    assert_eq!(split_ref.provider_id, "anthropic");
    assert_eq!(split_ref.model_id, "claude-sonnet");

    let missing_model =
        DelegateAgentConfig { provider: "openai".into(), ..DelegateAgentConfig::default() };
    assert!(resolve_model_ref(&missing_model).is_none());

    let unsplit = DelegateAgentConfig { model: "gpt-5".into(), ..DelegateAgentConfig::default() };
    assert!(resolve_model_ref(&unsplit).is_none());
}

#[test]
fn resolve_model_ref_trims_components_and_prefers_explicit_provider() {
    let explicit = DelegateAgentConfig {
        provider: " openai ".into(),
        model: " anthropic/claude/sonnet ".into(),
        ..DelegateAgentConfig::default()
    };
    let explicit_ref = resolve_model_ref(&explicit).unwrap();
    assert_eq!(explicit_ref.provider_id, "openai");
    assert_eq!(explicit_ref.model_id, "anthropic/claude/sonnet");

    let combined = DelegateAgentConfig {
        model: " anthropic/claude/sonnet ".into(),
        ..DelegateAgentConfig::default()
    };
    let combined_ref = resolve_model_ref(&combined).unwrap();
    assert_eq!(combined_ref.provider_id, "anthropic");
    assert_eq!(combined_ref.model_id, "claude/sonnet");
}

#[test]
fn build_resolved_agent_applies_runtime_defaults() {
    let defaults = permission_next::from_config(&json!({"read": "allow"}));
    let user = permission_next::from_config(&json!({"question": "deny"}));

    let mut role_config = DelegateAgentConfig {
        mode: "sub-agent".into(),
        permission: json!({"edit": {"*": "deny"}}),
        ..DelegateAgentConfig::default()
    };
    let coder = build_resolved_agent("coder", role_config.clone(), &defaults, &user);
    assert_eq!(coder.definition.mode, "subagent");
    assert!(coder.permission.iter().any(|rule| rule.permission == "read"));
    assert!(coder.permission.iter().any(|rule| rule.permission == "question"));

    role_config.system_prompt = Some("custom".into());
    let explore_keeps_custom =
        build_resolved_agent("explore", role_config, &Vec::new(), &Vec::new());
    assert_eq!(explore_keeps_custom.definition.system_prompt.as_deref(), Some("custom"));

    for key in ["explore"] {
        let resolved =
            build_resolved_agent(key, DelegateAgentConfig::default(), &Vec::new(), &Vec::new());
        assert!(
            resolved.definition.system_prompt.as_ref().is_some_and(|prompt| !prompt.is_empty())
        );
    }
}

#[test]
fn public_delegate_agents_resolve_dedicated_role_prompts() {
    let expectations = [
        ("researcher", "资料研究", "禁止写入工作区", "外部一手资料"),
        ("coder", "代码实现", "可以修改工作区", "相邻实现和测试"),
        ("reviewer", "代码审查", "禁止修改工作区", "文件、行号、触发条件和影响"),
        ("explore", "代码探索", "禁止修改工作区", "路径、符号、调用关系"),
    ];

    for (key, role, workspace_boundary, responsibility) in expectations {
        let resolved =
            build_resolved_agent(key, DelegateAgentConfig::default(), &Vec::new(), &Vec::new());
        let prompt = resolved
            .definition
            .system_prompt
            .as_deref()
            .unwrap_or_else(|| panic!("{key} should resolve a dedicated prompt"));

        for expected in [
            role,
            workspace_boundary,
            responsibility,
            "有界子任务",
            "不得创建子代理",
            "不得伪造验证结果",
        ] {
            assert!(prompt.contains(expected), "{key} prompt should contain {expected}: {prompt}");
        }
    }
}

#[test]
fn build_resolved_agent_orders_permission_layers() {
    let defaults = permission_next::from_config(&json!({"shared": "allow"}));
    let user = permission_next::from_config(&json!({"shared": "ask"}));
    let resolved = build_resolved_agent(
        "custom",
        DelegateAgentConfig {
            permission: json!({"shared": "deny"}),
            ..DelegateAgentConfig::default()
        },
        &defaults,
        &user,
    );

    let actions = resolved
        .permission
        .iter()
        .filter(|rule| rule.permission == "shared")
        .map(|rule| rule.action)
        .collect::<Vec<_>>();
    assert_eq!(actions, vec![Action::Allow, Action::Deny, Action::Ask]);
}

#[test]
fn build_resolved_agent_allows_selected_skills() {
    let resolved = build_resolved_agent(
        "main",
        DelegateAgentConfig {
            allowed_skills: vec!["shiju".to_string()],
            permission: json!({"skill": {"*": "deny"}}),
            ..DelegateAgentConfig::default()
        },
        &Vec::new(),
        &Vec::new(),
    );

    assert_eq!(
        permission_next::evaluate("skill", "shiju", &[resolved.permission.clone()]).action,
        Action::Allow
    );
    assert_eq!(
        permission_next::evaluate("skill", "other", &[resolved.permission]).action,
        Action::Deny
    );
}

#[test]
fn build_resolved_agent_keeps_global_skill_deny_authoritative() {
    let user = permission_next::from_config(&json!({"skill": {"*": "deny"}}));
    let resolved = build_resolved_agent(
        "main",
        DelegateAgentConfig {
            allowed_skills: vec!["shiju".to_string()],
            ..DelegateAgentConfig::default()
        },
        &Vec::new(),
        &user,
    );

    assert_eq!(
        permission_next::evaluate("skill", "shiju", &[resolved.permission]).action,
        Action::Deny
    );
}

#[test]
fn build_resolved_agent_preserves_custom_builtin_prompts() {
    for key in ["researcher", "coder", "reviewer", "explore"] {
        let resolved = build_resolved_agent(
            key,
            DelegateAgentConfig {
                system_prompt: Some("custom prompt".into()),
                ..DelegateAgentConfig::default()
            },
            &Vec::new(),
            &Vec::new(),
        );

        assert_eq!(resolved.definition.system_prompt.as_deref(), Some("custom prompt"));
    }
}

#[test]
fn build_resolved_agent_keeps_explicit_tool_output_deny() {
    let trunc = truncate_glob();
    let resolved = build_resolved_agent(
        "main",
        DelegateAgentConfig {
            permission: json!({"external_directory": {trunc.clone(): "deny"}}),
            ..DelegateAgentConfig::default()
        },
        &Vec::new(),
        &Vec::new(),
    );

    assert!(resolved.permission.iter().any(|rule| {
        rule.permission == "external_directory"
            && rule.pattern == trunc
            && rule.action == Action::Deny
    }));
}

#[test]
fn load_state_skips_disabled_agents_and_preserves_default_name() {
    let cfg = json!({
        "default_agent": "custom",
        "permission": {
            "question": "ask"
        },
        "agents": {
            "custom": {
                "provider": "openai",
                "model": "gpt-5",
                "mode": "primary"
            },
            "disabled": {
                "enabled": false,
                "provider": "openai",
                "model": "gpt-5"
            }
        }
    });
    let configured_agents = load_agent_configs(&cfg);
    let user = permission_next::from_config(cfg.get("permission").unwrap_or(&Value::Null));
    let defaults = permission_next::from_config(&json!({"read": "allow"}));
    let agents = configured_agents
        .into_iter()
        .filter(|(_, config)| config.enabled)
        .map(|(key, config)| {
            let resolved = build_resolved_agent(&key, config, &defaults, &user);
            (key, resolved)
        })
        .collect::<std::collections::HashMap<_, _>>();

    let state = State {
        agents,
        default_agent: cfg.get("default_agent").and_then(Value::as_str).map(str::to_string),
    };

    assert_eq!(choose_default_agent(&state).unwrap(), "custom");
    assert!(state.agents.contains_key("custom"));
    assert!(!state.agents.contains_key("disabled"));
    assert!(
        state.agents["custom"]
            .permission
            .iter()
            .any(|rule| { rule.permission == "question" && rule.action == Action::Ask })
    );
}

#[test]
fn prompt_and_truncate_glob_are_non_empty() {
    assert!(!generate_prompt().trim().is_empty());
    let glob = truncate_glob();
    assert!(glob.ends_with("tool-output/*"));
}
