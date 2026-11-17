//! 跨通道与任务计划的提示词集成断言，由真实宿主测试入口注册。
use agent_loop::prompt::*;
use tool_types::{Tool, ToolResult};
use async_trait::async_trait;
use std::path::Path;

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

fn empty_context<'a>(workspace_dir: &'a Path, tools: &'a [Box<dyn Tool>]) -> PromptContext<'a> {
    PromptContext {
        workspace_dir,
        model_name: "test-model",
        tools,
        skills: &[],
        skills_prompt_mode: agent_config::schema::SkillsPromptInjectionMode::Full,
        identity_config: None,
        dispatcher_instructions: "",
        timezone: chrono_tz::Asia::Shanghai,
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
        agent_config::schema::SkillsPromptInjectionMode::Compact,
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

    let workflow_context = crate::tools::ToolRuntimeContext::new(
        "workflow-subtask-session",
        Some(temp.path().to_string_lossy().to_string()),
    );
    let agent_tool = crate::tools::tool_specs_for_context(&workflow_context)
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
