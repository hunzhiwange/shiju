//! 测试系统提示的工作区注入、分段顺序与可选内容处理。
//! 临时工作区和伪提示段用于验证缺失文件、超长内容、Git 状态与段落失败。

use super::*;
use crate::app::agent::tools::{Tool, ToolResult};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

struct TestTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for TestTool {
    fn name(&self) -> &str {
        "inspect"
    }

    fn description(&self) -> &str {
        "Inspect workspace"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "ok".into(), error: None })
    }
}

struct AgentToolStub;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for AgentToolStub {
    fn name(&self) -> &str {
        "AgentTool"
    }

    fn description(&self) -> &str {
        "Delegate work"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "ok".into(), error: None })
    }
}

struct StaticSection {
    name: &'static str,
    body: &'static str,
}

impl PromptSection for StaticSection {
    fn name(&self) -> &str {
        self.name
    }

    fn build(&self, _ctx: &PromptContext<'_>) -> Result<String> {
        Ok(self.body.to_string())
    }
}

fn workspace(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("{name}_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn empty_context<'a>(workspace_dir: &'a Path, tools: &'a [Box<dyn Tool>]) -> PromptContext<'a> {
    PromptContext {
        workspace_dir,
        model_name: "test-model",
        tools,
        skills: &[],
        skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode::Full,
        identity_config: None,
        dispatcher_instructions: "",
        timezone: chrono_tz::Asia::Shanghai,
    }
}

#[test]
fn section_names_are_stable() {
    assert_eq!(IdentitySection.name(), "identity");
    assert_eq!(ToolsSection.name(), "tools");
    assert_eq!(ActionSection.name(), "action");
    assert_eq!(SafetySection.name(), "safety");
    assert_eq!(WorkspaceSection.name(), "workspace");
    assert_eq!(RuntimeSection.name(), "runtime");
    assert_eq!(DateTimeSection.name(), "datetime");
    assert_eq!(ChannelMediaSection.name(), "channel_media");
}

#[test]
fn builder_skips_empty_sections_and_trims_trailing_whitespace() {
    let tools: Vec<Box<dyn Tool>> = Vec::new();
    let ctx = empty_context(Path::new("/tmp"), &tools);
    let builder = SystemPromptBuilder::default()
        .add_section(Box::new(StaticSection { name: "blank", body: "   \n" }))
        .add_section(Box::new(StaticSection { name: "alpha", body: "alpha\n\n" }))
        .add_section(Box::new(StaticSection { name: "beta", body: "beta  " }));

    let prompt = builder.build(&ctx).unwrap();

    assert_eq!(prompt, "alpha\n\nbeta\n\n");
}

#[test]
fn builder_propagates_section_errors() {
    struct FailingSection;

    impl PromptSection for FailingSection {
        fn name(&self) -> &str {
            "failing"
        }

        fn build(&self, _ctx: &PromptContext<'_>) -> Result<String> {
            anyhow::bail!("section failed")
        }
    }

    let tools: Vec<Box<dyn Tool>> = Vec::new();
    let ctx = empty_context(Path::new("/tmp"), &tools);
    let error = SystemPromptBuilder::default()
        .add_section(Box::new(FailingSection))
        .build(&ctx)
        .unwrap_err();

    assert_eq!(error.to_string(), "section failed");
}

#[test]
fn tools_section_returns_empty_without_tools_and_renders_dispatcher_instructions() {
    let empty_tools: Vec<Box<dyn Tool>> = Vec::new();
    let empty_ctx = empty_context(Path::new("/tmp"), &empty_tools);
    assert_eq!(ToolsSection.build(&empty_ctx).unwrap(), "");

    let tools: Vec<Box<dyn Tool>> = vec![Box::new(TestTool)];
    let ctx = PromptContext {
        dispatcher_instructions: "Use XML tools.",
        ..empty_context(Path::new("/tmp"), &tools)
    };
    let rendered = ToolsSection.build(&ctx).unwrap();

    assert!(rendered.contains("## Tools"));
    assert!(rendered.contains("inspect"));
    assert!(rendered.contains("Inspect workspace"));
    assert!(rendered.contains("Use XML tools."));
}

#[test]
fn tools_section_omits_dispatcher_instructions_when_blank() {
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(TestTool)];
    let ctx = empty_context(Path::new("/tmp"), &tools);
    let rendered = ToolsSection.build(&ctx).unwrap();

    assert!(rendered.contains("Parameters: `"));
    assert!(rendered.contains("\"path\""));
    assert!(!rendered.ends_with('\n'));
    assert!(!rendered.contains("Use XML tools."));
}

#[test]
fn action_section_switches_between_native_and_dispatcher_modes() {
    let tools: Vec<Box<dyn Tool>> = Vec::new();
    let native_ctx = empty_context(Path::new("/tmp"), &tools);
    let dispatcher_ctx = PromptContext {
        dispatcher_instructions: "tool tags",
        ..empty_context(Path::new("/tmp"), &tools)
    };

    let native = ActionSection.build(&native_ctx).unwrap();
    let dispatcher = ActionSection.build(&dispatcher_ctx).unwrap();

    for prompt in [&native, &dispatcher] {
        assert!(prompt.contains("act immediately"));
        assert!(prompt.contains("Investigate only enough"));
        assert!(prompt.contains("Keep reasoning internal"));
        assert!(prompt.contains("Ask one concise question only"));
        assert!(prompt.contains("report the outcome and essential verification concisely"));
    }
    assert!(native.contains("call it before giving an explanation"));
    assert!(dispatcher.contains("<tool_call>"));
    assert!(dispatcher.contains("output instructions instead of acting"));
    assert_ne!(native, dispatcher);
}

#[test]
fn action_section_adds_default_multi_agent_policy_only_when_agent_tool_available() {
    for dispatcher_instructions in ["", "tool tags"] {
        let with_agent_tool: Vec<Box<dyn Tool>> = vec![Box::new(TestTool), Box::new(AgentToolStub)];
        let with_agent_ctx = PromptContext {
            dispatcher_instructions,
            ..empty_context(Path::new("/tmp"), &with_agent_tool)
        };
        let action = ActionSection.build(&with_agent_ctx).unwrap();
        for expected in [
            "not genuinely simple and single-step",
            "Complex, multi-part, exploratory, implementation, review",
            "independent workstreams",
            "When uncertain whether a task is simple, delegate",
            "Never delegate the whole task you were assigned",
            "clearly narrower, non-overlapping subtask",
            "same turn",
            "ordering dependencies",
            "overlapping writes",
            "verify evidence and synthesize",
        ] {
            assert!(
                action.contains(expected),
                "missing '{expected}' with dispatcher '{dispatcher_instructions}': {action}"
            );
        }

        let without_agent_tool: Vec<Box<dyn Tool>> = vec![Box::new(TestTool)];
        let without_agent_ctx = PromptContext {
            dispatcher_instructions,
            ..empty_context(Path::new("/tmp"), &without_agent_tool)
        };
        let action = ActionSection.build(&without_agent_ctx).unwrap();
        assert!(!action.contains("not genuinely simple and single-step"));
        assert!(!action.contains("When uncertain whether a task is simple, delegate"));
    }
}

#[test]
fn action_section_teaches_event_driven_subagent_wait_and_synthesis() {
    for dispatcher_instructions in ["", "tool tags"] {
        let with_agent_tool: Vec<Box<dyn Tool>> = vec![Box::new(TestTool), Box::new(AgentToolStub)];
        let with_agent_ctx = PromptContext {
            dispatcher_instructions,
            ..empty_context(Path::new("/tmp"), &with_agent_tool)
        };
        let action = ActionSection.build(&with_agent_ctx).unwrap();
        for expected in [
            "`launch` or `spawn`",
            "Continue useful parent work",
            "`run`",
            "`status`",
            "`wait`",
            "terminal state",
            "timeout",
            "still running",
            "`stop`",
            "Do not claim",
            "verify",
            "synthesize",
        ] {
            assert!(
                action.contains(expected),
                "missing '{expected}' with dispatcher '{dispatcher_instructions}': {action}"
            );
        }

        let without_agent_tool: Vec<Box<dyn Tool>> = vec![Box::new(TestTool)];
        let without_agent_ctx = PromptContext {
            dispatcher_instructions,
            ..empty_context(Path::new("/tmp"), &without_agent_tool)
        };
        let action = ActionSection.build(&without_agent_ctx).unwrap();
        assert!(!action.contains("Continue useful parent work"));
        assert!(!action.contains("still running"));
    }
}

#[test]
fn base_and_task_plan_prompts_share_recursive_orchestration_policy() {
    let temp = tempfile::tempdir().unwrap();
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(TestTool), Box::new(AgentToolStub)];
    let base =
        ActionSection.build(&empty_context(temp.path(), &tools)).expect("build base action prompt");
    let channel = crate::channels::build_system_prompt_with_mode(
        temp.path(),
        "model-x",
        &[("inspect", "Inspect workspace"), ("AgentTool", "Delegate work")],
        &[],
        None,
        None,
        true,
        crate::app::agent::config::SkillsPromptInjectionMode::Compact,
        chrono_tz::Asia::Shanghai,
    );
    let planning = crate::task_plan_run::build_planning_prompt("workflow-1", "实现递归智能体编排");
    let execution = crate::task_plan_run::build_execution_prompt(
        "实现递归智能体编排",
        4,
        &[3],
        temp.path(),
        &temp.path().join("4-current-task.md"),
        "# plan.md\n保持外层编号任务",
        "# 当前任务\n在任务边界内实现",
    );
    let continuation = crate::task_plan_run::build_goal_continuation_prompt(
        "实现递归智能体编排",
        temp.path(),
        &[],
        "# plan.md",
        "缺少验收证据",
        &crate::task_plan_run::GoalTaskFilesState::Pending("任务仍存在".to_string()),
        "",
    );

    for (name, prompt) in [
        ("base", base.as_str()),
        ("channel", channel.as_str()),
        ("planning", planning.as_str()),
        ("execution", execution.as_str()),
        ("goal continuation", continuation.as_str()),
    ] {
        for expected in [
            "Identify internal workstreams, ordering dependencies, and write sets",
            "Launch independent read-only or non-overlapping work",
            "Run dependency-ordered or overlapping writes serially",
            "Retry only safely retryable failures, with a finite attempt limit",
            "`status`",
            "`wait`",
            "`stop`",
            "verify evidence and synthesize required results",
            "delegation depth and parallelism limits",
        ] {
            assert!(prompt.contains(expected), "missing '{expected}' in {name} prompt:\n{prompt}");
        }
    }

    assert!(!execution.contains("不要重新拆分任务"));
    for expected in
        ["不得改写外层编号任务、批次依赖或任务文件所有权", "可在当前任务边界内自主拆分内部工作流"]
    {
        assert!(
            execution.contains(expected),
            "missing execution boundary '{expected}':\n{execution}"
        );
    }

    let workflow_context = crate::app::agent::tools::ToolRuntimeContext::new(
        "workflow-subtask-session",
        Some(temp.path().to_string_lossy().to_string()),
    );
    let agent_tool = crate::app::agent::tools::tool_specs_for_context(&workflow_context)
        .into_iter()
        .find(|spec| spec.id == "AgentTool")
        .expect("default workflow/goal session toolset should include AgentTool");
    for action in ["launch", "run", "status", "wait", "stop"] {
        assert!(
            agent_tool.input_schema["properties"]["action"]["enum"]
                .as_array()
                .expect("AgentTool action enum")
                .iter()
                .any(|value| value == action),
            "AgentTool should expose {action}"
        );
    }
}

#[test]
fn safety_runtime_datetime_and_channel_sections_render_expected_markers() {
    let tools: Vec<Box<dyn Tool>> = Vec::new();
    let ctx = empty_context(Path::new("/tmp"), &tools);

    assert!(SafetySection.build(&ctx).unwrap().contains("Prefer `trash` over `rm`"));

    let runtime = RuntimeSection.build(&ctx).unwrap();
    assert!(runtime.contains("## Runtime"));
    assert!(runtime.contains("Model: test-model"));
    assert!(runtime.contains(std::env::consts::OS));

    let datetime = DateTimeSection.build(&ctx).unwrap();
    assert!(datetime.starts_with("## Current Date & Time"));
    assert!(datetime.contains('('));
    assert!(datetime.ends_with(')'));

    let channel = ChannelMediaSection.build(&ctx).unwrap();
    assert!(channel.contains("[Voice]"));
    assert!(channel.contains("[IMAGE:<path>]"));
    assert!(channel.contains("[Document: <name>]"));
}

#[test]
fn workspace_section_lists_visible_entries_and_git_status() {
    let workspace = workspace("vibewindow_prompt_workspace");
    std::fs::create_dir_all(workspace.join(".git")).unwrap();
    std::fs::create_dir_all(workspace.join(".github")).unwrap();
    std::fs::create_dir_all(workspace.join("src")).unwrap();
    std::fs::write(workspace.join("README.md"), "readme").unwrap();
    std::fs::write(workspace.join(".secret"), "hidden").unwrap();

    let tools: Vec<Box<dyn Tool>> = Vec::new();
    let ctx = empty_context(&workspace, &tools);
    let rendered = WorkspaceSection.build(&ctx).unwrap();

    assert!(rendered.contains("(Git Repository)"));
    assert!(rendered.contains(".github/"));
    assert!(rendered.contains("src/"));
    assert!(rendered.contains("README.md"));
    assert!(!rendered.contains(".secret"));

    let _ = std::fs::remove_dir_all(workspace);
}

#[test]
fn workspace_section_limits_large_directory_listing() {
    let workspace = workspace("vibewindow_prompt_many_files");
    for i in 0..55 {
        std::fs::write(workspace.join(format!("file_{i:02}.txt")), "x").unwrap();
    }

    let tools: Vec<Box<dyn Tool>> = Vec::new();
    let ctx = empty_context(&workspace, &tools);
    let rendered = WorkspaceSection.build(&ctx).unwrap();

    assert!(rendered.contains("... (more files hidden)"));

    let _ = std::fs::remove_dir_all(workspace);
}

#[test]
fn workspace_section_handles_unreadable_directory() {
    let missing =
        std::env::temp_dir().join(format!("vibewindow_prompt_missing_{}", uuid::Uuid::new_v4()));
    let tools: Vec<Box<dyn Tool>> = Vec::new();
    let ctx = empty_context(&missing, &tools);
    let rendered = WorkspaceSection.build(&ctx).unwrap();

    assert!(rendered.contains(&format!("Working directory: `{}`", missing.display())));
    assert!(rendered.contains("<directories>\n</directories>"));
}

#[test]
fn is_git_repo_detects_parent_repository() {
    let repo_workspace = workspace("vibewindow_prompt_git_parent");
    let nested = repo_workspace.join("a/b/c");
    std::fs::create_dir_all(repo_workspace.join(".git")).unwrap();
    std::fs::create_dir_all(&nested).unwrap();

    assert!(is_git_repo(&nested));

    let outside = workspace("vibewindow_prompt_not_git");
    assert!(!is_git_repo(&outside));

    let _ = std::fs::remove_dir_all(repo_workspace);
    let _ = std::fs::remove_dir_all(outside);
}

#[test]
fn inject_workspace_file_handles_missing_empty_present_and_truncated_files() {
    let workspace = workspace("vibewindow_prompt_inject");
    let mut prompt = String::new();

    inject_workspace_file(&mut prompt, &workspace, "OPTIONAL.md", 20, true);
    assert!(prompt.is_empty());

    inject_workspace_file(&mut prompt, &workspace, "REQUIRED.md", 20, false);
    assert!(prompt.contains("[File not found: REQUIRED.md]"));

    std::fs::write(workspace.join("EMPTY.md"), "   \n").unwrap();
    let before_empty = prompt.clone();
    inject_workspace_file(&mut prompt, &workspace, "EMPTY.md", 20, false);
    assert_eq!(prompt, before_empty);

    std::fs::write(workspace.join("SHORT.md"), "short content").unwrap();
    inject_workspace_file(&mut prompt, &workspace, "SHORT.md", 20, false);
    assert!(prompt.contains("### SHORT.md"));
    assert!(prompt.contains("short content"));

    std::fs::write(workspace.join("LONG.md"), "abcdefghijklmnopqrstuvwxyz").unwrap();
    inject_workspace_file(&mut prompt, &workspace, "LONG.md", 12, false);
    assert!(prompt.contains("### LONG.md"));
    assert!(prompt.contains("truncated at 12 chars"));

    let _ = std::fs::remove_dir_all(workspace);
}

#[test]
fn identity_section_includes_missing_markers_for_absent_identity_files() {
    let workspace = workspace("vibewindow_prompt_identity_missing");
    std::fs::write(workspace.join("AGENTS.md"), "project rules").unwrap();

    let tools: Vec<Box<dyn Tool>> = Vec::new();
    let ctx = empty_context(&workspace, &tools);
    let rendered = IdentitySection.build(&ctx).unwrap();

    assert!(rendered.contains("## Project Context"));
    assert!(rendered.contains("project rules"));
    assert!(rendered.contains("[File not found: SOUL.md]"));

    let _ = std::fs::remove_dir_all(workspace);
}

#[test]
fn identity_section_truncates_large_identity_files() {
    let workspace = workspace("vibewindow_prompt_identity_large");
    let content = "a".repeat(BOOTSTRAP_MAX_CHARS + 100);
    std::fs::write(workspace.join("AGENTS.md"), content).unwrap();

    let tools: Vec<Box<dyn Tool>> = Vec::new();
    let ctx = empty_context(&workspace, &tools);
    let rendered = IdentitySection.build(&ctx).unwrap();

    assert!(rendered.contains("### AGENTS.md"));
    assert!(rendered.contains("truncated at 20000 chars"));
    assert!(rendered.contains("use `read` for full file"));

    let _ = std::fs::remove_dir_all(workspace);
}

#[test]
fn skills_section_is_empty_when_no_skills_are_available() {
    let tools: Vec<Box<dyn Tool>> = Vec::new();
    let ctx = empty_context(Path::new("/tmp"), &tools);

    assert_eq!(SkillsSection.build(&ctx).unwrap(), "");
}
