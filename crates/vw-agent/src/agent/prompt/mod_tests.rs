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
    }
}

#[test]
fn section_names_are_stable() {
    assert_eq!(IdentitySection.name(), "identity");
    assert_eq!(ToolsSection.name(), "tools");
    assert_eq!(ActionSection.name(), "action");
    assert_eq!(SafetySection.name(), "safety");
    assert_eq!(SkillsSection.name(), "skills");
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

    assert!(native.contains("respond naturally"));
    assert!(dispatcher.contains("<tool_call>"));
    assert_ne!(native, dispatcher);
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
