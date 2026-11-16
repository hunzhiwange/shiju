//! Context preflight uses the library protocol directly.

#[test]
fn context_preflight_runs_before_next_llm_step_after_real_input_usage() {
    let mut messages = vec![serde_json::json!({"role": "system", "content": "system"})];
    for index in 0..9 {
        let call_id = format!("call-{index}");
        messages.push(serde_json::json!({
            "role": "assistant",
            "tool_calls": [{"id": call_id, "type": "function"}]
        }));
        messages.push(serde_json::json!({
            "role": "tool",
            "tool_call_id": call_id,
            "content": if index == 0 { "old output ".repeat(80) } else { "recent".to_string() }
        }));
    }
    messages.push(serde_json::json!({"role": "user", "content": "latest question"}));

    let result = super::prepare_context_before_llm_step(
        &mut messages,
        &["system".to_string()],
        &[],
        &provider::ModelLimit {
            context: 500,
            input: Some(500),
            output: 0,
        },
        Some(490),
        false,
    )
    .expect("the next LLM step should report whether internal compaction is required");

    assert!(result.needs_internal_compaction);
    assert!(messages.iter().any(|message| message["role"] == "system"));
    assert!(messages.iter().any(|message| message["role"] == "user"));
    assert_ne!(
        messages
            .iter()
            .find(|message| message["tool_call_id"] == "call-0")
            .and_then(|message| message["content"].as_str()),
        Some(
            "old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output old output "
        )
    );
}

#[test]
fn codex_skills_defaults_strict_policy_restricts_catalog_and_preloads_once() {
    let workspace = tempfile::tempdir().expect("skill workspace");
    let default_path = workspace.path().join("default/SKILL.md");
    let user_path = workspace.path().join("user/SKILL.md");
    std::fs::create_dir_all(default_path.parent().expect("default parent")).expect("default dir");
    std::fs::create_dir_all(user_path.parent().expect("user parent")).expect("user dir");
    std::fs::write(&default_path, "# Default\nDEFAULT_SKILL_BODY").expect("default skill");
    std::fs::write(&user_path, "# User\nUSER_SKILL_BODY").expect("user skill");
    let skills = vec![
        agent_skills::Skill {
            name: "default".to_string(),
            location: Some(default_path),
            ..Default::default()
        },
        agent_skills::Skill {
            name: "user".to_string(),
            location: Some(user_path),
            ..Default::default()
        },
    ];
    let options = serde_json::json!({
        "delegate_skill_policy": "strict",
        "delegate_default_allowed_skills": ["default"],
        "delegate_trusted_user_selected_skills": ["user"],
    });
    let resolution = super::delegated_skill_policy_resolution(&skills, &options)
        .expect("delegated policy should resolve");
    assert_eq!(resolution.visible_ids, ["default"].into_iter().map(String::from).collect());

    let mut protocol = super::DesktopSessionProtocol::new(skills, "delegate", 20_000);
    protocol.preload_skills(&resolution.default_injected_ids).expect("preload default");
    let history = vec![serde_json::json!({"role": "user", "content": "work"})];
    let first = protocol.assemble(&history, &[], true).expect("first protocol");
    let second = protocol.assemble(&history, &[], true).expect("second protocol");
    let default_count = |messages: &[serde_json::Value]| {
        messages
            .iter()
            .filter(|message| {
                message["content"]
                    .as_str()
                    .is_some_and(|content| content.contains("DEFAULT_SKILL_BODY"))
            })
            .count()
    };
    assert_eq!(default_count(&first), 1);
    assert_eq!(default_count(&second), 1);
    assert!(!first.iter().any(|message| {
        message["content"].as_str().is_some_and(|content| content.contains("USER_SKILL_BODY"))
    }));
}

#[test]
fn codex_skills_defaults_missing_source_is_an_error() {
    let mut protocol = super::DesktopSessionProtocol::new(Vec::new(), "main", 20_000);
    let error = protocol
        .preload_skills(&["missing-default".to_string()].into_iter().collect())
        .expect_err("missing configured default must not silently succeed");
    assert!(error.contains("missing-default"));
    let item = agent_skills::Skill { name: "unlocated-default".into(), ..Default::default() };
    let mut protocol = super::DesktopSessionProtocol::new(vec![item], "main", 20_000);
    let error = protocol
        .preload_skills(&["unlocated-default".into()].into_iter().collect())
        .expect_err("discovered defaults need a real source");
    assert!(error.contains("unlocated-default"));
}

#[test]
fn codex_skills_catalog_protocol_retains_metadata_and_user_body_without_changing_history() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("SKILL.md");
    std::fs::write(&source, "# Review\nVERIFIED_REPLAY_BODY").unwrap();
    let item = agent_skills::Skill {
        name: "review".into(),
        description: "Review metadata".into(),
        location: Some(source.clone()),
        ..Default::default()
    };
    let mut protocol = super::DesktopSessionProtocol::new(vec![item], "main", 100_000);
    let record =
        agent_skills::invoked::InvokedSkillRecord::new("review", &source, None).unwrap();
    protocol.skill_state.register_invoked("main", record);
    let history = vec![serde_json::json!({"role":"user","content":"keep working"})];
    let original = history.clone();
    for _ in 0..2 {
        let assembled = protocol.assemble(&history, &[], true).unwrap();
        assert!(assembled.messages.iter().any(|m| m["role"] == "developer"
            && m["content"].as_str().is_some_and(|s| s.contains("Review metadata"))));
        assert!(assembled.messages.iter().any(|m| m["role"] == "user"
            && m["content"].as_str().is_some_and(|s| s.contains("VERIFIED_REPLAY_BODY"))));
        assert!(
            assembled
                .message_sources
                .iter()
                .any(|s| s["form"] == "skill_catalog"
                    && s["entries"] == serde_json::json!(["review"]))
        );
        assert_eq!(history, original);
    }
    protocol.skills.clear();
    let error =
        protocol.assemble(&history, &[], true).expect_err("revoked sources must stop replay");
    assert!(error.contains("review"));
}
