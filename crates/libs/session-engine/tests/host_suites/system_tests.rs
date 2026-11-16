//! Real filesystem/provider probes and host-injected orchestration policy contracts.

use super::*;

#[test]
fn unified_prompt_declares_instruction_priority_and_general_assistant_role() {
    let template = PROMPT_UNIFIED;
    let tools = [
        "file_read",
        "file_edit",
        "file_write",
        "apply_patch",
        "glob",
        "grep",
        "bash",
        "skill",
        "TodoRead",
        "TodoWrite",
        "AgentTool",
        "process",
        "batch",
        "AskUserQuestion",
        "WebSearch",
        "Browser",
        "list_mcp_resources",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let prompt = render_unified_prompt("deepseek/deepseek-v4-flash", "/workspace", &tools);

    assert!(prompt.contains("## Instruction Priority"));
    assert!(prompt.contains("AGENTS.md"));
    assert!(prompt.contains("coding mode") && prompt.contains("work mode"));
    assert!(prompt.contains("## Tool Usage"));
    assert!(
        prompt.contains("Where the protocols do not apply"),
        "default tools must be framed as fallback when project protocol is silent"
    );
    assert!(
        prompt.contains("Skill Tool")
            && prompt.contains("<system-reminder>")
            && prompt.contains("do not guess paths or preload instructions"),
        "skills must be listed as metadata and loaded only by a successful Skill Tool call"
    );
    assert!(
        prompt.contains("file_read")
            && prompt.contains("apply_patch")
            && prompt.contains("TodoWrite"),
        "unified prompt must describe the built-in tool routing contract"
    );
    assert!(template.lines().count() <= 80, "unified prompt template should remain compact");
    assert!(template.contains("{{model_id}}"));
    assert!(template.contains("{{working_directory}}"));
    assert!(template.contains("{{runtime_capabilities}}"));
    assert!(template.contains("{{runtime_modes}}"));
    assert!(prompt.contains("deepseek/deepseek-v4-flash"));
    assert!(prompt.contains("/workspace"));
    assert!(prompt.contains("## Subagents"));
    assert!(prompt.contains("Assess independent workstreams"));
    assert!(prompt.contains("bounded work"));
    assert!(prompt.contains("parallelism has clear benefit"));
    assert!(prompt.contains("Proactively delegate"));
    assert!(prompt.contains("handle tightly coupled work directly"));
    assert!(prompt.contains("## Background Tasks"));
    assert!(prompt.contains("## Task Tracking"));
    assert!(prompt.contains("## Network and Browser"));
    assert!(prompt.contains("## MCP Resources"));
    assert!(prompt.contains("By default, `old_string` must be unique"));
    assert!(prompt.contains("the final response must mention the primary output"));
    assert!(prompt.contains("Markdown inline code"));
    assert!(prompt.contains(":line[:column]"));
    assert!(prompt.contains("Do not use `file://`"));
    assert!(!prompt.contains("{{"));
    assert!(!prompt.contains("<preloaded_skill_instructions>"));
    assert!(prompt.contains("## Autonomous Agent Discipline"));
    assert!(prompt.contains("## Structured Workflow"));
    assert!(
        prompt.contains("you must actually initiate the tool call"),
        "agent loop must require acting on stated next steps"
    );
    assert!(
        prompt.contains("Do not assume code behavior"),
        "weaker models must be told not to invent codebase facts"
    );
    assert!(!prompt.contains("最好的编码智能体"));
    assert!(!prompt.contains("软件工程任务的交互式 CLI 工具"));
}

#[test]
fn detect_git_from_dir_walks_up_to_parent_repo() {
    let dir = tempfile::tempdir().expect("temp dir");
    let nested = dir.path().join("a/b/c");
    std::fs::create_dir_all(&nested).unwrap();

    assert!(!detect_git_from_dir(nested.to_str().unwrap()));
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    assert!(detect_git_from_dir(nested.to_str().unwrap()));
    assert!(!detect_git_from_dir("   "));
}

#[test]
fn detect_git_from_dir_accepts_trimmed_path_and_git_file_marker() {
    let dir = tempfile::tempdir().expect("temp dir");
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested).expect("nested dir");
    std::fs::write(dir.path().join(".git"), "gitdir: /tmp/worktree-git-dir").expect("git marker");

    let padded = format!("  {}  ", nested.display());
    assert!(detect_git_from_dir(&padded));
}

#[test]
fn resolve_is_git_falls_back_to_directory_detection_without_project_context() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cwd = dir.path().to_string_lossy();

    assert!(!resolve_is_git(&cwd));
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    assert!(resolve_is_git(&cwd));
}

#[tokio::test]
// 模型引用缺失或畸形时仍生成基础环境，不能让提示构建因可选模型信息整体失败。
async fn environment_from_ref_handles_missing_and_malformed_model_refs() {
    let root = "/tmp/vw-agent-env-test";

    let unknown = environment_from_ref(None, Some(root)).await;
    assert!(unknown.contains("unknown/unknown"));
    assert!(unknown.contains(root));

    let malformed = environment_from_ref(Some("not-a-model-ref"), Some(root)).await;
    assert!(malformed.contains("unknown/not-a-model-ref"));

    let missing_provider = environment_from_ref(Some("/orphan-model"), Some(root)).await;
    assert!(missing_provider.contains("Exact model ID: unknown//orphan-model"));
}

#[tokio::test]
async fn environment_from_ref_preserves_parsed_ids_when_model_lookup_fails() {
    let root = "/tmp/vw-agent-env-missing-model";

    let text = environment_from_ref(Some("missing-provider/missing-model"), Some(root)).await;

    assert!(text.contains("Exact model ID: missing-provider/missing-model"));
    assert!(text.contains(root));
}

#[test]
fn agcollab_prompt_matches_runtime_capabilities() {
    let prompt = render_unified_prompt("provider/model", "/workspace", &["AgentTool".into()]);
    for expected in [
        "send_message",
        "followup_task",
        "list_agents",
        "fork_turns",
        "colleague data",
        "shared directory",
    ] {
        assert!(prompt.contains(expected), "missing {expected}");
    }
    let denied = render_unified_prompt("provider/model", "/workspace", &[]);
    assert!(!denied.contains("followup_task"));
}
