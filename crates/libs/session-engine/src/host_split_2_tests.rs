//! 纯提示词组成契约；已解析指令由端口提供，不验证真实文件读取。
use super::{PromptHost, system_with_agent_metadata, system_with_metadata};

struct Host;

impl PromptHost for Host {
    async fn environment_from_ref(_model: Option<&str>, _root: Option<&str>) -> String {
        "deterministic environment".to_string()
    }

    async fn workspace_resolved(root: &std::path::Path) -> Vec<session_instructions::Resolved> {
        vec![session_instructions::Resolved {
            filepath: agent_util::filesystem::normalize_path(root.join("AGENTS.md"))
                .to_string_lossy()
                .into_owned(),
            content: "agents instruction content".to_string(),
            sources: Vec::new(),
            diagnostics: Vec::new(),
        }]
    }

    async fn system_resolved() -> Vec<session_instructions::Resolved> {
        panic!("explicit workspace must use workspace_resolved")
    }

    fn recursive_policy() -> &'static str {
        ""
    }
}

#[test]
fn system_prompt_runtime_context_renders_model_directory_and_available_capabilities_once() {
    let project = tempfile::tempdir().expect("project workspace");
    let tools = vec![
        "skill".to_string(),
        "TodoRead".to_string(),
        "TodoWrite".to_string(),
        "AgentTool".to_string(),
    ];
    let prompt = system_with_metadata::<Host>(None, project.path().to_str())
        .with_runtime_context(Some("deepseek/deepseek-v4-flash"), project.path().to_str(), &tools)
        .with_runtime_context(Some("ignored/model"), project.path().to_str(), &tools);

    assert_eq!(prompt.content.matches(crate::system_prompt::UNIFIED_PROMPT_MARKER).count(), 1);
    assert!(prompt.content.contains("deepseek/deepseek-v4-flash"));
    assert!(prompt.content.contains(&project.path().display().to_string()));
    assert!(prompt.content.contains("## Skills"));
    assert!(prompt.content.contains("## Task Tracking"));
    assert!(prompt.content.contains("## Subagents"));
    assert!(!prompt.content.contains("ignored/model"));
    assert!(!prompt.content.contains("{{"));
}

#[test]
fn agents_context_base_system_excludes_instruction_content() {
    let project = tempfile::tempdir().expect("project workspace");
    let prompt = system_with_metadata::<Host>(None, project.path().to_str());

    assert!(!prompt.content.contains("agents instruction content"));
    assert!(!prompt.base_content.contains("agents instruction content"));
    assert_eq!(prompt.instruction_context().len(), 1);
    assert_eq!(prompt.instruction_context()[0].content, "agents instruction content");
}

#[test]
fn system_prompt_keeps_project_agents_independent_from_empty_identity_whitelist() {
    let project = tempfile::tempdir().expect("project workspace");
    let prompt = system_with_agent_metadata::<Host>(
        None,
        project.path().to_str(),
        Some("reviewer"),
        &[],
    );
    let project_path = agent_util::filesystem::normalize_path(project.path().join("AGENTS.md"))
        .to_string_lossy()
        .into_owned();

    assert!(!prompt.content.contains("agents instruction content"));
    assert_eq!(prompt.visible_instruction_paths, vec![project_path]);
    assert!(!prompt.base_content.contains("agents instruction content"));
    assert_eq!(prompt.instruction_context().len(), 1);
    assert!(prompt.instruction_context()[0].content.contains("agents instruction content"));
}
