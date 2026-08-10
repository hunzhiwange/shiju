#[test]
fn max_steps_text_matches_trimmed_embedded_constraint() {
    let text = super::max_steps_text();

    assert!(!text.is_empty());
    assert_eq!(text, include_str!("prompt/max-steps.txt").trim());
    assert!(text.contains("工具已禁用"));
}

#[test]
fn block_on_runs_future_without_existing_runtime() {
    let value = super::block_on(async { 42 });

    assert_eq!(value, 42);
}

#[test]
fn block_on_uses_current_runtime_handle() {
    let runtime =
        tokio::runtime::Builder::new_current_thread().build().expect("test runtime should build");
    let _guard = runtime.enter();

    let value = super::block_on(async { 42 });

    assert_eq!(value, 42);
}

#[test]
fn environment_wraps_missing_model_with_explicit_root() {
    let entries = super::environment(None, Some("  prompt-test-root  "));

    assert_eq!(entries.len(), 1);
    assert!(entries[0].contains("unknown/unknown"));
    assert!(entries[0].contains("工作目录: prompt-test-root"));
}

#[test]
fn join_non_empty_parts_omits_blank_entries_and_preserves_order() {
    let prompt = super::join_non_empty_parts(vec![
        "environment".to_string(),
        String::new(),
        " \t ".to_string(),
        "instruction".to_string(),
    ]);

    assert_eq!(prompt, "environment\n\ninstruction");
}

#[test]
fn join_non_empty_parts_keeps_non_blank_content_unchanged() {
    let prompt =
        super::join_non_empty_parts(vec![" environment ".to_string(), "instruction\n".to_string()]);

    assert_eq!(prompt, " environment \n\ninstruction\n");
}

#[test]
fn system_includes_environment_for_explicit_root() {
    let prompt = super::system(None, Some("prompt-test-root"));

    assert!(prompt.contains("unknown/unknown"));
    assert!(prompt.contains("工作目录: prompt-test-root"));
}

#[test]
fn system_prompt_keeps_project_agents_independent_from_empty_identity_whitelist() {
    let project = tempfile::tempdir().expect("project workspace");
    let project_agents = project.path().join("AGENTS.md");
    std::fs::write(&project_agents, "selected project protocol").expect("write project agents");

    let prompt = super::system_with_agent_metadata(
        None,
        project.path().to_str(),
        Some("reviewer"),
        &[],
    );
    let project_path = crate::app::agent::util::filesystem::normalize_path(&project_agents)
        .to_string_lossy()
        .to_string();

    assert!(prompt.content.contains("selected project protocol"));
    assert_eq!(prompt.visible_instruction_paths, vec![project_path]);
    assert!(!prompt.base_content.contains("selected project protocol"));
    assert!(
        prompt
            .project_protocol
            .as_deref()
            .is_some_and(|protocol| protocol.contains("selected project protocol"))
    );
}

#[test]
fn runtime_parts_send_project_protocol_after_default_and_custom_instructions() {
    let prompt = super::SystemPrompt {
        content: "base\n\nproject".to_string(),
        visible_instruction_paths: Vec::new(),
        base_content: "base".to_string(),
        project_protocol: Some("project".to_string()),
    };

    assert_eq!(
        prompt.runtime_parts(Some("custom"), false),
        vec!["base", "custom", "project"]
    );

    let parts = prompt.runtime_parts(Some("custom"), true);
    assert_eq!(&parts[..3], ["base", "custom", "project"]);
    assert_eq!(parts.last().map(String::as_str), Some(super::max_steps_text()));
}

#[test]
fn project_protocol_wrapper_is_generic_and_marks_rules_as_mandatory() {
    let protocol = super::format_project_protocol("use any project-specific workflow");

    assert!(protocol.contains("<project_protocol>"));
    assert!(protocol.contains("use any project-specific workflow"));
    assert!(protocol.contains("强制执行规则"));
}

#[test]
fn runtime_parts_keep_project_referenced_skill_instructions_in_every_step() {
    let project = tempfile::tempdir().expect("project workspace");
    let skill_directory = project.path().join("skills/project-navigator");
    std::fs::create_dir_all(&skill_directory).expect("create skill directory");
    std::fs::write(
        skill_directory.join("SKILL.md"),
        "---\nname: project-navigator\ndescription: project navigation\n---\n\nMUST_USE_PROJECT_NAVIGATOR\n",
    )
    .expect("write project skill");
    std::fs::write(
        project.path().join("AGENTS.md"),
        "代码导航必须使用 `./skills/project-navigator/SKILL.md`。",
    )
    .expect("write project protocol");

    let prompt = super::system_with_metadata(None, project.path().to_str());
    let first_step = prompt.runtime_parts(None, false);
    let later_step = prompt.runtime_parts(Some("continue the task"), false);

    for parts in [&first_step, &later_step] {
        let project_protocol = parts.last().expect("project protocol system part");
        assert!(project_protocol.contains("代码导航必须使用"));
        assert!(project_protocol.contains("<preloaded_skill_instructions>"));
        assert!(project_protocol.contains("MUST_USE_PROJECT_NAVIGATOR"));
    }
}
