//! Native defaults integration coverage in the host.
use config_agent::DelegateCapabilityPolicy;

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
    let tenant = vw_api_transport::TenantScope::new("native-platform", "other-company").unwrap();
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
fn agent_config_cleanup_native_skill_inheritance_uses_allowed_source() {
    use config_agent_profile::AgentDefault;
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
