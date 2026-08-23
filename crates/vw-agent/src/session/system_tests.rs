//! 验证系统提示环境、模型适配与指令拼装的稳定契约。

use super::*;
use serde_json::json;

fn test_model(api_id: &str, provider_id: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": api_id,
        "providerID": provider_id,
        "api": {
            "id": api_id,
            "url": "http://localhost",
            "adapter": "openai-compatible"
        },
        "name": api_id,
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}

#[test]
fn yes_no_en_is_stable() {
    assert_eq!(yes_no_en(true), "Yes");
    assert_eq!(yes_no_en(false), "No");
}

#[test]
fn environment_text_contains_core_fields() {
    let text = environment_text("gpt-5", "openai", "/tmp/work", true, "no extra dirs");
    assert!(text.contains("gpt-5"));
    assert!(text.contains("openai/gpt-5"));
    assert!(text.contains("/tmp/work"));
    assert!(text.contains("Yes"));
    assert!(text.contains(platform()));
}

#[test]
fn provider_returns_unified_prompt_for_all_models() {
    let models = [
        ("gpt-5-mini", "openai"),
        ("gpt-4.1", "openai"),
        ("o3-mini", "openai"),
        ("gemini-2.5-pro", "google"),
        ("claude-sonnet-4", "anthropic"),
        ("Trinity-v1", "local"),
        ("qwen-plus", "qwen"),
        ("gpt-5-claude-gemini-Trinity", "mixed"),
        ("gpt-4-claude-gemini-Trinity", "mixed"),
        ("unknown-model", "custom"),
    ];

    for (api_id, provider_id) in models {
        let prompts = provider(&test_model(api_id, provider_id));
        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].contains(UNIFIED_PROMPT_MARKER));
        assert!(prompts[0].contains(&format!("{provider_id}/{api_id}")));
        assert!(!prompts[0].contains("{{"));
    }
}

#[test]
fn instructions_render_unified_prompt_defaults() {
    let instructions = instructions();

    assert!(instructions.contains(UNIFIED_PROMPT_MARKER));
    assert!(instructions.contains("`unknown`"));
    assert!(!instructions.contains("{{"));
    assert!(!instructions.starts_with('\n'));
    assert!(!instructions.ends_with('\n'));
}

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
    let prompt = render_unified_prompt("deepseek/deepseek-v4-flash", "/workspace", &tools, true);

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
    assert!(
        prompt.contains("## RustCodeGraph")
            && prompt.contains("explore")
            && prompt.contains("impact")
            && prompt.contains("affected")
            && prompt.contains("`sync` operation once")
            && prompt.contains("watched automatically"),
        "unified prompt must preserve the indexed Rust navigation workflow"
    );
    assert!(template.lines().count() <= 80, "unified prompt template should remain compact");
    assert!(template.contains("{{model_id}}"));
    assert!(template.contains("{{working_directory}}"));
    assert!(template.contains("{{runtime_capabilities}}"));
    assert!(template.contains("{{runtime_modes}}"));
    assert!(prompt.contains("deepseek/deepseek-v4-flash"));
    assert!(prompt.contains("/workspace"));
    assert!(prompt.contains("## Subagents"));
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
fn unified_prompt_omits_unavailable_capability_instructions() {
    let prompt = render_unified_prompt("provider/model", "/workspace", &["file_read".into()], true);

    assert!(prompt.contains("## Files and Search"));
    assert!(prompt.contains("the final response must mention the primary output"));
    assert!(prompt.contains("Markdown inline code"));
    assert!(!prompt.contains("file_edit"));
    assert!(!prompt.contains("file_write"));
    assert!(!prompt.contains("`glob`"));
    assert!(!prompt.contains("`grep`"));
    assert!(!prompt.contains("## Skills"));
    assert!(!prompt.contains("## RustCodeGraph"));
    assert!(!prompt.contains("## Task Tracking"));
    assert!(!prompt.contains("## Subagents"));
    assert!(!prompt.contains("## Background Tasks"));
    assert!(!prompt.contains("## Current Runtime Mode"));
}

#[test]
fn unified_prompt_injects_only_enabled_runtime_modes() {
    let goal_prompt = render_unified_prompt_with_modes(
        "provider/model",
        "/workspace",
        &[],
        false,
        RuntimePromptModes { goal: true, workflow: false },
    );
    assert!(goal_prompt.contains("## Current Runtime Mode"));
    assert!(goal_prompt.contains("Goal mode is enabled"));
    assert!(!goal_prompt.contains("Workflow mode is enabled"));

    let workflow_prompt = render_unified_prompt_with_modes(
        "provider/model",
        "/workspace",
        &[],
        false,
        RuntimePromptModes { goal: true, workflow: true },
    );
    assert!(workflow_prompt.contains("Goal mode is enabled"));
    assert!(workflow_prompt.contains("Workflow mode is enabled"));
    assert!(!workflow_prompt.contains("{{runtime_modes}}"));
}

#[test]
fn resolve_cwd_prefers_non_empty_root_and_trims_it() {
    assert_eq!(resolve_cwd(Some("  /tmp/vw-work  ")), "/tmp/vw-work");
}

#[test]
fn environment_text_preserves_structure_for_non_git_workspace() {
    let text = environment_text("model", "provider", "/workspace", false, "");
    let lines = text.lines().collect::<Vec<_>>();

    assert_eq!(lines.len(), 11);
    assert_eq!(lines[0], "You are using the model model. Exact model ID: provider/model");
    assert_eq!(lines[1], "Here is some information about the runtime environment:");
    assert_eq!(lines[2], "<env>");
    assert_eq!(lines[3], "  Working directory: /workspace");
    assert_eq!(lines[4], "  Is Git repository: No");
    assert_eq!(lines[5], format!("  Platform: {}", platform()));
    assert!(lines[6].starts_with("  Today's date: "));
    assert_eq!(lines[7], "</env>");
    assert_eq!(lines[8], "<directories>");
    assert_eq!(lines[9], "  ");
    assert_eq!(lines[10], "</directories>");
}

#[test]
fn today_date_string_uses_documented_four_component_format() {
    let date = today_date_string();
    let components = date.split_whitespace().collect::<Vec<_>>();

    assert_eq!(components.len(), 4);
    assert_eq!(components[0].len(), 3);
    assert_eq!(components[1].len(), 3);
    assert_eq!(components[3].len(), 4);
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
