//! 验证依赖宿主环境与项目指令读取的系统提示契约。
//! 检查项目指令与身份白名单互不串扰，以及真实宿主环境的提示渲染。

#[test]
fn environment_wraps_missing_model_with_explicit_root() {
    let entries = super::environment(None, Some("  prompt-test-root  "));

    assert_eq!(entries.len(), 1);
    assert!(entries[0].contains("unknown/unknown"));
    assert!(entries[0].contains("Working directory: prompt-test-root"));
}

#[test]
fn system_includes_environment_for_explicit_root() {
    let prompt = super::system(None, Some("prompt-test-root"));

    assert!(prompt.contains("unknown/unknown"));
    assert!(prompt.contains("Working directory: prompt-test-root"));
}

#[test]
fn system_prompt_keeps_project_agents_independent_from_empty_identity_whitelist() {
    let project = tempfile::tempdir().expect("project workspace");
    let project_agents = project.path().join("AGENTS.md");
    std::fs::write(&project_agents, "selected project protocol").expect("write project agents");

    let prompt =
        super::system_with_agent_metadata(None, project.path().to_str(), Some("reviewer"), &[]);
    let project_path = crate::app::agent::util::filesystem::normalize_path(&project_agents)
        .to_string_lossy()
        .to_string();

    assert_eq!(prompt.visible_instruction_paths, vec![project_path]);
    assert_eq!(prompt.instruction_context().len(), 1);
    assert!(prompt.instruction_context()[0].content.contains("selected project protocol"));
}

#[test]
// 初始请求读取显式工作区指令族，但不把身份白名单为空误解为禁用项目指令。
fn system_prompt_initial_request_reads_explicit_workspace_agents_family() {
    let project = tempfile::tempdir().expect("project workspace");
    std::fs::create_dir_all(project.path().join(".vibewindow"))
        .expect("create project config directory");
    std::fs::write(project.path().join("AGENTS.md"), "root protocol").expect("write root agents");
    std::fs::write(project.path().join(".vibewindow/AGENTS.md"), "project protocol")
        .expect("write project config agents");
    std::fs::write(project.path().join("AGENTS.local.md"), "local protocol")
        .expect("write local agents");

    let prompt = super::system_with_metadata(None, project.path().to_str());

    assert_eq!(prompt.visible_instruction_paths.len(), 3);
    let context = prompt
        .instruction_context()
        .iter()
        .map(|resolved| resolved.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(context.contains("root protocol"));
    assert!(context.contains("project protocol"));
    assert!(context.contains("local protocol"));
}

#[test]
fn agents_context_base_system_excludes_instruction_content() {
    let project = tempfile::tempdir().expect("project workspace");
    std::fs::write(project.path().join("AGENTS.md"), "agents instruction content")
        .expect("write project agents");

    let prompt = super::system_with_metadata(None, project.path().to_str());

    // 宿主只验证真实工作区读取；系统消息分离由领域测试覆盖。
    assert_eq!(prompt.instruction_context().len(), 1);
    assert!(prompt.instruction_context()[0].content.contains("agents instruction content"));
}
