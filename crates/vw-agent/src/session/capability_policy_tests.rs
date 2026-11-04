//! 验证技能能力策略只解析配置模式，不允许模型自行扩权。

use super::capability_policy::{CapabilityPolicyResolution, resolve_skill_capabilities};
use vw_config_types::agent::DelegateCapabilityPolicy;

#[test]
// 模型只能在预先配置的能力范围内选择，不能借输出内容自行扩大权限。
fn skill_capability_policy_resolves_both_modes_without_model_self_authorization() {
    let skills = ["review", "rustcodegraph", "disabled"];
    let strict_skills = resolve_skill_capabilities(
        &skills,
        [" review "],
        ["rustcodegraph"],
        DelegateCapabilityPolicy::Strict,
        ["disabled"],
    );
    assert_eq!(strict_skills, CapabilityPolicyResolution::new(["review"], ["review"]));

    let unrestricted_skills = resolve_skill_capabilities(
        &skills,
        std::iter::empty::<&str>(),
        std::iter::empty::<&str>(),
        DelegateCapabilityPolicy::Unrestricted,
        ["disabled"],
    );
    assert_eq!(
        unrestricted_skills,
        CapabilityPolicyResolution::new(["review", "rustcodegraph"], std::iter::empty::<&str>(),)
    );
}

#[test]
fn codex_skills_defaults_disabled_ids_stay_outside_strict_and_unrestricted_capabilities() {
    for policy in [DelegateCapabilityPolicy::Strict, DelegateCapabilityPolicy::Unrestricted] {
        let resolution = resolve_skill_capabilities(
            ["review", "disabled"],
            [" REVIEW ", "disabled"],
            ["disabled"],
            policy,
            ["disabled"],
        );
        assert_eq!(resolution, CapabilityPolicyResolution::new(["review"], ["review"]));
    }
    let resolution = resolve_skill_capabilities(
        ["review"],
        std::iter::empty::<&str>(),
        ["review"],
        DelegateCapabilityPolicy::Strict,
        std::iter::empty::<&str>(),
    );
    assert_eq!(
        resolution,
        CapabilityPolicyResolution::new(std::iter::empty::<&str>(), std::iter::empty::<&str>())
    );
}

#[test]
fn agent_native_defaults_mcp_selection_reaches_registered_tools_with_session_isolation() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    let workspace = tempfile::tempdir().unwrap();
    let script = workspace.path().join("fixture.py");
    std::fs::write(&script, include_str!("native_defaults_mcp_fixture.py")).unwrap();
    std::fs::write(
        workspace.path().join(".vwacprc.json"),
        serde_json::to_vec(&serde_json::json!({
            "mcpServers": [
                {"name":"selected", "command":"python3", "args":[script]},
                {"name":"unselected", "command":"python3", "args":[script]},
                {"name":"disabled", "command":"missing-disabled-command", "enabled":false}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace.path().to_path_buf();
    assert!(
        runtime
            .block_on(super::capability_policy::native_agent_defaults(
                &config,
                "main",
                None,
                workspace.path()
            ))
            .unwrap()
            .is_none()
    );
    for (key, ids, expected) in [("main", vec!["selected"], true), ("reviewer", vec![], false)] {
        let agent = serde_json::from_value(serde_json::json!({
            "provider":"", "model":"", "defaults":{"mcp":{"mode":"fixed","value":ids}}
        }))
        .unwrap();
        config.agents.insert(key.into(), agent);
        let defaults = runtime
            .block_on(super::capability_policy::native_agent_defaults(
                &config,
                key,
                None,
                workspace.path(),
            ))
            .unwrap()
            .unwrap();
        let context = crate::tools::ToolUseContext::new(
            format!("native-{key}"),
            Some(workspace.path().to_string_lossy().to_string()),
        )
        .with_native_mcp_selection(defaults.effective.mcp.iter().cloned().collect());
        let ctx = crate::tools::ToolRuntimeContext::new(
            format!("native-{key}"),
            Some(workspace.path().to_string_lossy().to_string()),
        )
        .with_tool_use_context(context);
        let specs = crate::tools::tool_specs_for_context(&ctx);
        let id = crate::tools::mcp_tool::mcp_tool_id("selected", "Echo Tool").unwrap();
        assert_eq!(specs.iter().any(|spec| spec.id == id), expected);
        assert!(
            !specs.iter().any(|spec| spec.id.starts_with("mcp__unselected")
                || spec.id.starts_with("mcp__disabled"))
        );
        if expected {
            let result =
                crate::tools::execute_tool_call(&id, r#"{"message":"native"}"#, &ctx).unwrap();
            assert!(result.is_success());
            assert!(result.model_result.to_string().contains("echo:native"));
        }
    }
}

#[test]
fn agent_native_defaults_do_not_inject_disabled_skills() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    let workspace = tempfile::tempdir().unwrap();
    let dir = workspace.path().join("skills/disabled");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        "---\nname: disabled\ndescription: Disabled fixture\n---\nMUST_NOT_INJECT\n",
    )
    .unwrap();
    std::fs::write(crate::skills::local_skill_disabled_marker_path(&dir), "").unwrap();
    let mut config = crate::config::Config::default();
    config.agents.insert(
        "main".into(),
        serde_json::from_value(serde_json::json!({
            "provider":"", "model":"", "defaults":{"skills":{"mode":"fixed","value":["disabled"]}}
        }))
        .unwrap(),
    );
    assert!(
        runtime
            .block_on(super::capability_policy::native_agent_defaults(
                &config,
                "main",
                None,
                workspace.path()
            ))
            .is_err()
    );
}

#[test]
fn agent_native_defaults_mcp_respects_tenant_and_project_safety_scope() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    let workspace = tempfile::tempdir().unwrap();
    let other_workspace = tempfile::tempdir().unwrap();
    runtime
        .block_on(crate::config::update(serde_json::json!({
            "mcpServers":[{"name":"owner-only", "command":"missing-owner-fixture"}]
        })))
        .unwrap();
    let mut config = crate::config::Config::default();
    config.agents.insert(
        "main".into(),
        serde_json::from_value(serde_json::json!({
            "provider":"", "model":"", "defaults":{"mcp":{"mode":"fixed","value":["owner-only"]}}
        }))
        .unwrap(),
    );
    let defaults = runtime
        .block_on(super::capability_policy::native_agent_defaults(
            &config,
            "main",
            None,
            workspace.path(),
        ))
        .unwrap()
        .unwrap();
    assert_eq!(defaults.effective.mcp, vec!["owner-only"]);
    runtime
        .block_on(crate::config::update_project(
            workspace.path(),
            serde_json::json!({
                "mcpServerOverrides":[{"name":"owner-only", "enabled":false}]
            }),
        ))
        .unwrap();
    let error = match runtime.block_on(super::capability_policy::native_agent_defaults(
        &config,
        "main",
        None,
        workspace.path(),
    )) {
        Err(error) => error,
        Ok(_) => panic!("project-disabled MCP must reject admission"),
    };
    assert!(error.to_string().contains("MCP capability is disabled"));
    let tenant = vw_api_types::id::TenantScope::new("native-platform", "other-company").unwrap();
    let result = runtime.block_on(crate::session::ui_store::with_tenant_scope(tenant, async {
        super::capability_policy::native_agent_defaults(
            &config,
            "main",
            None,
            other_workspace.path(),
        )
        .await
    }));
    match result {
        Err(error) => assert!(error.to_string().contains("unknown or out-of-scope MCP capability")),
        Ok(_) => panic!("another tenant must not resolve the owner's global MCP server"),
    }
}

#[test]
fn agent_config_cleanup_strict_skill_boundary_survives_fixed_selection() {
    use crate::agent::defaults::{DefaultSelection, DefaultsContext, resolve};
    use vw_config_types::agent::{AgentDefault, AgentDefinitionConfig};
    let mut agent = AgentDefinitionConfig::default();
    agent.skill_policy = DelegateCapabilityPolicy::Strict;
    agent.allowed_skills = vec!["allowed".into()];
    agent.defaults.skills = AgentDefault::Fixed(vec!["outside".into()]);
    let context = DefaultsContext {
        skills: [("allowed".into(), true), ("outside".into(), true)].into(),
        ..Default::default()
    };
    assert!(
        resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
            .is_err()
    );
    agent.defaults.skills = AgentDefault::Fixed(vec![]);
    assert!(
        resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
            .unwrap()
            .skills
            .is_empty()
    );
    agent.skill_policy = DelegateCapabilityPolicy::Unrestricted;
    agent.defaults.skills = AgentDefault::Fixed(vec!["outside".into()]);
    assert_eq!(
        resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
            .unwrap()
            .skills,
        vec!["outside"]
    );
    for available in [None, Some(false)] {
        let mut context = DefaultsContext::default();
        if let Some(enabled) = available {
            context.skills.insert("outside".into(), enabled);
        }
        assert!(
            resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
                .is_err()
        );
    }
}

#[test]
fn agent_config_cleanup_native_skill_inheritance_uses_allowed_source() {
    use vw_config_types::agent::AgentDefault;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    let workspace = tempfile::tempdir().unwrap();
    let dir = workspace.path().join("skills/allowed");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), "---\nname: allowed\ndescription: fixture\n---\n")
        .unwrap();
    let mut config = crate::config::Config::default();
    config.agents.insert(
        "main".into(),
        serde_json::from_value(
            serde_json::json!({"provider":"", "model":"", "allowed_skills":["allowed"], "skill_policy":"strict"}),
        )
        .unwrap(),
    );
    for (selection, expected) in [
        (AgentDefault::Inherit, vec!["allowed"]),
        (AgentDefault::Fixed(vec![]), vec![]),
        (AgentDefault::Fixed(vec!["allowed".into()]), vec!["allowed"]),
    ] {
        config.agents.get_mut("main").unwrap().defaults.skills = selection;
        let result = runtime
            .block_on(super::capability_policy::native_agent_defaults(
                &config,
                "main",
                None,
                workspace.path(),
            ))
            .unwrap()
            .unwrap();
        assert_eq!(result.effective.skills, expected);
    }
    config.agents.get_mut("main").unwrap().defaults.skills =
        AgentDefault::Fixed(vec!["missing".into()]);
    assert!(
        runtime
            .block_on(super::capability_policy::native_agent_defaults(
                &config,
                "main",
                None,
                workspace.path()
            ))
            .is_err()
    );
}

#[test]
fn agent_config_cleanup_fixed_empty_keeps_visible_strict_boundary() {
    let available = vec!["allowed".into(), "outside".into()];
    let boundary = vec!["allowed".into()];
    for defaults in [vec![], vec!["allowed".into()]] {
        let result = super::capability_policy::resolve_skill_defaults(
            &available,
            &boundary,
            &defaults,
            DelegateCapabilityPolicy::Strict,
        );
        assert_eq!(result.visible_ids, ["allowed".to_string()].into());
        assert_eq!(result.default_injected_ids, defaults.into_iter().collect());
    }
}
