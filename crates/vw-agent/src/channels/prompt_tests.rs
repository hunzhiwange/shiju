use super::*;
use crate::app::agent::memory::{Memory, MemoryCategory, MemoryEntry};
use crate::app::agent::tools::{Tool, ToolResult};
use serde_json::{Value, json};

struct NamedTool(&'static str);

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.0
    }

    fn description(&self) -> &str {
        "unit test tool"
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "ok".to_string(), error: None })
    }
}

#[derive(Clone)]
struct FakeMemory {
    entries: Vec<MemoryEntry>,
    fail: bool,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Memory for FakeMemory {
    fn name(&self) -> &str {
        "fake"
    }

    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        if self.fail {
            anyhow::bail!("recall failed");
        }
        Ok(self.entries.clone())
    }

    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(self.entries.clone())
    }

    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(self.entries.len())
    }

    async fn health_check(&self) -> bool {
        !self.fail
    }
}

fn memory_entry(key: &str, content: &str, score: Option<f64>) -> MemoryEntry {
    MemoryEntry {
        id: key.to_string(),
        key: key.to_string(),
        content: content.to_string(),
        category: MemoryCategory::Core,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        session_id: None,
        score,
    }
}

#[test]
fn channel_system_prompt_keeps_cli_plain_when_no_context() {
    assert_eq!(build_channel_system_prompt("base", "cli", "", false), "base");
}

#[test]
fn channel_system_prompt_adds_delivery_visibility_and_delivery_context() {
    let prompt = build_channel_system_prompt("base", "telegram", "chat-1", false);

    assert!(prompt.starts_with("base\n\nWhen responding on Telegram:"));
    assert!(prompt.contains("Do not reveal raw tool names"));
    assert!(prompt.contains("channel=telegram"));
    assert!(prompt.contains(r#""to":"chat-1""#));
}

#[test]
fn channel_system_prompt_can_allow_explicit_execution_details() {
    let prompt = build_channel_system_prompt("", "discord", "", true);

    assert!(prompt.contains("the user explicitly requested command/tool details"));
    assert!(!prompt.contains("Channel context:"));
}

#[test]
fn channel_system_prompt_uses_visibility_as_root_for_unknown_non_cli_channel() {
    let prompt = build_channel_system_prompt("", "custom", "", false);

    assert!(prompt.starts_with("Execution visibility: run tools/functions in the background"));
    assert!(!prompt.starts_with("\n\n"));
    assert!(!prompt.contains("Channel context:"));
}

#[test]
fn channel_system_prompt_uses_delivery_instructions_when_base_is_empty() {
    let prompt = build_channel_system_prompt("", "whatsapp", "", false);

    assert!(prompt.starts_with("When responding on WhatsApp:"));
    assert!(prompt.contains("Do not reveal raw tool names"));
}

#[test]
fn runtime_tool_visibility_sorts_filters_and_uses_native_protocol_text() {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(NamedTool("zeta")),
        Box::new(NamedTool("alpha")),
        Box::new(NamedTool("beta")),
    ];
    let prompt = build_runtime_tool_visibility_prompt(&tools, &["alpha".to_string()], true);

    assert!(prompt.contains("- Allowed tools (2):"));
    assert!(!prompt.contains("  - `alpha`"));
    assert!(prompt.find("`beta`").unwrap() < prompt.find("`zeta`").unwrap());
    assert!(prompt.contains("Excluded by runtime policy: alpha"));
    assert!(prompt.contains("native provider function-calling"));
    assert!(prompt.contains("Do not emit `<tool_call>` XML tags"));
}

#[test]
fn runtime_tool_visibility_reports_no_tools_and_xml_protocol() {
    let tools: Vec<Box<dyn Tool>> = Vec::new();
    let prompt = build_runtime_tool_visibility_prompt(&tools, &[], false);

    assert!(prompt.contains("- Allowed tools: (none)"));
    assert!(prompt.contains("Excluded by runtime policy: (none)"));
    assert!(prompt.contains("Tool Use Protocol"));
    assert!(prompt.contains("<tool_call>"));
}

#[test]
fn runtime_tool_visibility_xml_protocol_lists_sorted_tools_and_exclusions() {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(NamedTool("zeta")),
        Box::new(NamedTool("alpha")),
        Box::new(NamedTool("beta")),
    ];
    let prompt = build_runtime_tool_visibility_prompt(
        &tools,
        &["zeta".to_string(), "alpha".to_string()],
        false,
    );

    assert!(prompt.contains("- Allowed tools (1):"));
    assert!(prompt.contains("  - `beta`"));
    assert!(prompt.contains("Excluded by runtime policy: alpha, zeta"));
    assert!(prompt.contains("Tool Use Protocol"));
    assert!(prompt.contains("<tool_call>"));
}

#[test]
fn memory_context_skip_predicate_handles_special_keys_and_size() {
    assert!(should_skip_memory_context_entry(" assistant_resp_42 ", "short"));
    assert!(should_skip_memory_context_entry("Chat_History", "short"));
    assert!(should_skip_memory_context_entry("normal", &"x".repeat(MEMORY_CONTEXT_MAX_CHARS + 1)));
    assert!(!should_skip_memory_context_entry("normal", "short"));
}

#[tokio::test]
async fn memory_context_filters_scores_special_keys_and_truncates_long_entries() {
    let long_content = "a".repeat(MEMORY_CONTEXT_ENTRY_MAX_CHARS + 20);
    let mem = FakeMemory {
        entries: vec![
            memory_entry("keep", "useful", Some(0.9)),
            memory_entry("low", "hidden", Some(0.1)),
            memory_entry("assistant_resp_1", "hidden", Some(1.0)),
            memory_entry("chat_history", "hidden", Some(1.0)),
            memory_entry("long", &long_content, None),
        ],
        fail: false,
    };

    let context = build_memory_context(&mem, "query", 0.5).await;

    assert!(context.starts_with("[Memory context]\n"));
    assert!(context.contains("- keep: useful\n"));
    assert!(context.contains("- long: "));
    assert!(context.contains("..."));
    assert!(!context.contains("hidden"));
    assert!(!context.contains("- low:"));
    assert!(context.ends_with('\n'));
}

#[tokio::test]
async fn memory_context_is_empty_when_recall_fails_or_filters_everything() {
    let failing = FakeMemory { entries: Vec::new(), fail: true };
    assert_eq!(build_memory_context(&failing, "query", 0.0).await, "");

    let filtered =
        FakeMemory { entries: vec![memory_entry("low", "hidden", Some(0.1))], fail: false };
    assert_eq!(build_memory_context(&filtered, "query", 0.9).await, "");
}

#[tokio::test]
async fn memory_context_limits_included_entries_to_maximum() {
    let mem = FakeMemory {
        entries: vec![
            memory_entry("one", "1", None),
            memory_entry("two", "2", None),
            memory_entry("three", "3", None),
            memory_entry("four", "4", None),
            memory_entry("five", "5", None),
        ],
        fail: false,
    };

    let context = build_memory_context(&mem, "query", 0.0).await;

    assert!(context.contains("- one: 1\n"));
    assert!(context.contains("- four: 4\n"));
    assert!(!context.contains("- five: 5\n"));
}

#[tokio::test]
async fn memory_context_stops_when_total_character_budget_would_be_exceeded() {
    let long_key = "k".repeat(1_980);
    let mem = FakeMemory {
        entries: vec![
            memory_entry(&format!("{long_key}-1"), "short", None),
            memory_entry(&format!("{long_key}-2"), "short", None),
            memory_entry("tail", "should not fit", None),
        ],
        fail: false,
    };

    let context = build_memory_context(&mem, "query", 0.0).await;

    assert!(context.contains(&format!("- {long_key}-1: short\n")));
    assert!(context.contains(&format!("- {long_key}-2: short\n")));
    assert!(!context.contains("- tail: should not fit\n"));
}

#[test]
fn inject_workspace_file_handles_present_missing_empty_and_truncated_files() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("AGENTS.md"), "abcdef").unwrap();
    std::fs::write(temp.path().join("EMPTY.md"), "   \n").unwrap();

    let mut prompt = String::new();
    inject_workspace_file(&mut prompt, temp.path(), "AGENTS.md", 3);
    inject_workspace_file(&mut prompt, temp.path(), "EMPTY.md", 3);
    inject_workspace_file(&mut prompt, temp.path(), "MISSING.md", 3);

    assert!(prompt.contains("### AGENTS.md"));
    assert!(prompt.contains("abc"));
    assert!(prompt.contains("truncated at 3 chars"));
    assert!(!prompt.contains("### EMPTY.md"));
    assert!(prompt.contains("[File not found: MISSING.md]"));
}

#[test]
fn workspace_identity_context_loads_openclaw_bootstrap_files() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("AGENTS.md"), "agent notes").unwrap();
    std::fs::write(temp.path().join("BOOTSTRAP.md"), "boot notes").unwrap();

    let prompt = build_workspace_identity_context(temp.path(), None, Some(200));

    assert!(prompt.starts_with("## Project Context"));
    assert!(prompt.contains("ALREADY injected"));
    assert!(prompt.contains("agent notes"));
    assert!(prompt.contains("boot notes"));
    assert!(prompt.contains("[File not found: SOUL.md]"));
}

#[test]
fn load_openclaw_bootstrap_files_skips_missing_bootstrap_marker() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("AGENTS.md"), "agent notes").unwrap();

    let mut prompt = String::new();
    load_openclaw_bootstrap_files(&mut prompt, temp.path(), 200);

    assert!(prompt.contains("agent notes"));
    assert!(prompt.contains("[File not found: SOUL.md]"));
    assert!(!prompt.contains("### BOOTSTRAP.md"));
}

#[test]
fn workspace_identity_context_accepts_identity_config_and_default_limit() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("AGENTS.md"), "agent notes").unwrap();
    let identity_config = crate::app::agent::config::IdentityConfig {
        format: "aieos".to_string(),
        aieos_path: Some("identity.json".to_string()),
        aieos_inline: None,
    };

    let prompt = build_workspace_identity_context(temp.path(), Some(&identity_config), None);

    assert!(prompt.starts_with("## Project Context"));
    assert!(prompt.contains("agent notes"));
}

#[test]
fn system_prompt_includes_tools_workspace_runtime_and_mode_specific_task_text() {
    let temp = tempfile::tempdir().unwrap();
    let prompt = build_system_prompt_with_mode(
        temp.path(),
        "model-x",
        &[("read", "Read files")],
        &[],
        None,
        Some(20),
        true,
        crate::app::agent::config::SkillsPromptInjectionMode::Compact,
    );

    assert!(prompt.contains("## Tools"));
    assert!(prompt.contains("- **read**: Read files"));
    assert!(prompt.contains("respond naturally"));
    assert!(prompt.contains("Working directory:"));
    assert!(prompt.contains("Model: model-x"));
    assert!(prompt.contains("## Channel Capabilities"));
}

#[test]
fn system_prompt_default_wrapper_uses_xml_task_text() {
    let temp = tempfile::tempdir().unwrap();
    let prompt = build_system_prompt(temp.path(), "model-y", &[], &[], None, Some(10));

    assert!(prompt.contains("Instead: emit actual <tool_call> tags"));
    assert!(prompt.contains("Model: model-y"));
}

#[test]
fn system_prompt_with_compact_skill_mode_includes_skills_without_tools_section() {
    let temp = tempfile::tempdir().unwrap();
    let skill = crate::app::agent::skills::Skill {
        name: "do-task-plans".to_string(),
        description: "executes task plans".to_string(),
        version: "1.0.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: Vec::new(),
        prompts: vec!["follow the written task plan".to_string()],
        location: Some(temp.path().join("skills/do-task-plans/SKILL.md")),
    };
    let prompt = build_system_prompt_with_mode(
        temp.path(),
        "model-z",
        &[],
        &[skill],
        None,
        Some(50),
        false,
        crate::app::agent::config::SkillsPromptInjectionMode::Compact,
    );

    assert!(!prompt.contains("## Tools"));
    assert!(prompt.contains("## Available Skills"));
    assert!(prompt.contains("do-task-plans"));
    assert!(prompt.contains("skills/do-task-plans/SKILL.md"));
}
