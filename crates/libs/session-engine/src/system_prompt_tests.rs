//! Local prompt capability, mode, and explicit path contracts.

use super::*;

#[test]
fn unified_prompt_omits_unavailable_capability_instructions() {
    let prompt = render_unified_prompt("provider/model", "/workspace", &["file_read".into()], "");

    assert!(prompt.contains("## Files and Search"));
    assert!(prompt.contains("the final response must mention the primary output"));
    assert!(prompt.contains("Markdown inline code"));
    assert!(!prompt.contains("file_edit"));
    assert!(!prompt.contains("file_write"));
    assert!(!prompt.contains("`glob`"));
    assert!(!prompt.contains("`grep`"));
    assert!(!prompt.contains("## Skills"));
    assert!(!prompt.contains("## Task Tracking"));
    assert!(!prompt.contains("## Subagents"));
    assert!(!prompt.contains("Assess independent workstreams"));
    assert!(!prompt.contains("bounded work"));
    assert!(!prompt.contains("parallelism has clear benefit"));
    assert!(!prompt.contains("Proactively start subagents"));
    assert!(!prompt.contains("delegation-heavy tasks directly"));
    assert!(!prompt.contains("## Background Tasks"));
    assert!(!prompt.contains("## Current Runtime Mode"));
}

#[test]
fn system_prompt_uses_registered_thread_tools_for_visible_tasks() {
    let prompt = render_unified_prompt(
        "provider/model",
        "/workspace",
        &["create_thread".into(), "AgentTool".into()],
        "",
    );

    assert!(prompt.contains("## Visible Task Sessions"));
    assert!(prompt.contains("`create_thread`"));
    assert!(prompt.contains("user-visible independent task"));
    assert!(prompt.contains("hidden subtask"));
    assert!(prompt.contains("record the thread ID"));
    assert!(prompt.contains("available task tools"));
    assert!(prompt.contains("Do not take over the target task locally"));
    assert!(prompt.contains("plan-run"));

    let legacy_prompt = render_unified_prompt(
        "provider/model",
        "/workspace",
        &["SessionTool".into(), "AgentTool".into()],
        "",
    );
    assert!(!legacy_prompt.contains("## Visible Task Sessions"));

    let no_thread_tools =
        render_unified_prompt("provider/model", "/workspace", &["AgentTool".into()], "");
    assert!(!no_thread_tools.contains("## Visible Task Sessions"));
}

#[test]
fn unified_prompt_injects_only_enabled_runtime_modes() {
    let goal_prompt = render_unified_prompt_with_modes(
        "provider/model",
        "/workspace",
        &[],
        RuntimePromptModes { goal: true, workflow: false },
        "",
    );
    assert!(goal_prompt.contains("## Current Runtime Mode"));
    assert!(goal_prompt.contains("Goal mode is enabled"));
    assert!(!goal_prompt.contains("Workflow mode is enabled"));

    let workflow_prompt = render_unified_prompt_with_modes(
        "provider/model",
        "/workspace",
        &[],
        RuntimePromptModes { goal: true, workflow: true },
        "",
    );
    assert!(workflow_prompt.contains("Goal mode is enabled"));
    assert!(workflow_prompt.contains("Workflow mode is enabled"));
    assert!(!workflow_prompt.contains("{{runtime_modes}}"));
}

#[test]
fn resolve_cwd_prefers_non_empty_root_and_trims_it() {
    assert_eq!(resolve_cwd(Some("  /tmp/vw-work  ")), "/tmp/vw-work");
}
