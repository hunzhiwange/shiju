//! 验证 Claude 协议入口在代理、技能、压缩与委派策略上的一致性。

use crate::app::agent::config::{DelegateAgentConfig, DelegateCapabilityPolicy};
use crate::app::agent::skills::{InvokedSkillRecord, Skill, SkillListingSessionState};
use crate::app::agent::tools::{Tool, ToolResult};
use crate::channels::prompt::build_entrypoint_protocol_messages;
use crate::session::capability_policy::resolve_skill_capabilities;
use crate::session::child_agent::stream_request;
use crate::session::instruction_discovery::{self, DiscoveryRoots, SourceKind};
use crate::session::processor::llm_messages::assemble_session_protocol_messages;
use async_trait::async_trait;
use serde_json::json;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{InstructionSource, Resolved, instruction_rules};

fn write(path: impl AsRef<Path>, content: &str) -> PathBuf {
    let path = path.as_ref().to_path_buf();
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
    std::fs::write(&path, content).expect("write fixture");
    path
}

fn resolved(path: &Path, content: String, conditions: Vec<String>) -> Resolved {
    let filepath = path.to_string_lossy().to_string();
    Resolved {
        content: format!("Instructions from: {filepath}\n{content}"),
        filepath: filepath.clone(),
        sources: vec![InstructionSource { filepath, parent: None, conditions }],
        diagnostics: Vec::new(),
    }
}

#[test]
fn claude_protocol_parity_covers_agents_skills_compaction_and_entrypoints() {
    let fixture = tempfile::tempdir().expect("temporary project");
    let managed = fixture.path().join("managed");
    let user = fixture.path().join("user");
    let project = fixture.path().join("project");
    let source = project.join("src/lib.rs");

    let managed_agents = write(managed.join("AGENTS.md"), "managed instruction");
    let user_agents = write(user.join("AGENTS.md"), "user instruction");
    let project_agents = write(project.join("AGENTS.md"), "project instruction");
    let project_config_agents =
        write(project.join(".vibewindow/AGENTS.md"), "project configuration instruction");
    let rule = write(
        project.join(".vibewindow/rules/rust.md"),
        "---\npaths: src/**/*.rs\n---\nconditional Rust instruction\n",
    );
    let project_local_agents = write(project.join("AGENTS.local.md"), "project local instruction");
    let nested_agents = write(project.join("src/AGENTS.md"), "nested instruction");
    let nested_local_agents =
        write(project.join("src/AGENTS.local.md"), "nested local instruction");
    write(&source, "pub fn matrix() {}\n");
    write(project.join("CLAUDE.md"), "legacy instruction must not load");

    let sources = instruction_discovery::discover(&DiscoveryRoots {
        managed_config: Some(managed),
        user_config: Some(user),
        current_directory: Some(project.join("src")),
        worktree: Some(project.clone()),
        project_config_enabled: true,
    });
    assert_eq!(
        sources.iter().map(|source| source.filepath.clone()).collect::<Vec<_>>(),
        vec![
            managed_agents.clone(),
            user_agents.clone(),
            project_agents.clone(),
            project_config_agents.clone(),
            rule.clone(),
            project_local_agents.clone(),
            nested_agents.clone(),
            nested_local_agents.clone(),
        ]
    );
    assert!(sources.iter().all(|source| source.filepath.file_name() != Some("CLAUDE.md".as_ref())));

    let mut instructions = sources
        .iter()
        .filter(|source| source.kind != SourceKind::Rule)
        .map(|source| {
            resolved(
                &source.filepath,
                std::fs::read_to_string(&source.filepath).unwrap(),
                Vec::new(),
            )
        })
        .collect::<Vec<_>>();
    let active_rule = instruction_rules::load_if_matches(&rule, &project, &source)
        .expect("matching conditional rule");
    instructions.insert(4, resolved(&rule, active_rule.content, active_rule.conditions));
    assert_eq!(instructions[4].sources[0].conditions, vec!["src/**/*.rs"]);

    let skill_name = format!("neutral-matrix-{}", std::process::id());
    let skill_file = project.join(".vibewindow/skills").join(&skill_name).join("SKILL.md");
    write(
        &skill_file,
        "---\nname: ignored-by-protocol\ndescription: metadata only\nallowed-tools: [read]\nmodel: provider/matrix\neffort: high\n---\nFULL SKILL BODY $ARGUMENTS\n",
    );
    let skill = Skill {
        name: skill_name.clone(),
        description: "A neutral parity fixture.".to_string(),
        location: Some(skill_file.clone()),
        ..Skill::default()
    };
    let mut state = SkillListingSessionState::default();
    let listing = state.prepare("main", &[skill.clone()], true, 20_000).expect("skill listing");
    assert!(listing.content.contains(&skill_name));
    assert!(!listing.content.contains("FULL SKILL BODY"));
    assert!(
        state.register_invoked(
            "main",
            InvokedSkillRecord::new(&skill_name, &skill_file, Some("argument".to_string()))
                .expect("skill record"),
        )
    );
    let replay = state.restore_invoked("main").expect("first replay");
    assert_eq!(replay.len(), 1);
    assert!(replay[0].content.contains("FULL SKILL BODY argument"));

    let history = vec![json!({ "role": "user", "content": "actual user history" })];
    let invoked = replay
        .iter()
        .map(|replay| json!({ "role": "user", "content": replay.content }))
        .collect::<Vec<_>>();
    let desktop = assemble_session_protocol_messages(
        &history,
        &instructions,
        &invoked,
        Some(json!({ "role": "user", "content": listing.content })),
    );
    assert_eq!(desktop.last(), history.last());
    assert_eq!(desktop[0]["role"], "user");
    assert!(
        desktop[0]["content"]
            .as_str()
            .is_some_and(|content| content.contains("FULL SKILL BODY argument"))
    );
    assert!(desktop[1]["content"].as_str().is_some_and(|content| content.contains(&skill_name)));
    assert_eq!(desktop[2]["role"], "system");
    assert!(desktop[2]["content"].as_str().is_some_and(|content| {
        content.contains("managed instruction") && content.contains("nested local instruction")
    }));

    let agent_tools: Vec<Box<dyn crate::app::agent::tools::Tool>> = Vec::new();
    let agent_prompt = crate::app::agent::agent::prompt::SystemPromptBuilder::with_defaults()
        .build(&crate::app::agent::agent::prompt::PromptContext {
            workspace_dir: &project,
            model_name: "provider/matrix",
            tools: &agent_tools,
            skills: &[skill.clone()],
            skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode::Full,
            identity_config: None,
            dispatcher_instructions: "",
            timezone: chrono_tz::Asia::Shanghai,
        })
        .expect("agent system prompt");
    assert!(!agent_prompt.contains("FULL SKILL BODY"));
    assert!(!agent_prompt.contains("<available_skills>"));

    let external =
        build_entrypoint_protocol_messages(&instructions, &[skill.clone()], true, 20_000);
    assert_eq!(external.len(), 2);
    assert!(external[0].content.contains(&skill_name));
    assert_eq!(external[1].role, "system");
    assert!(external[1].content.contains("conditional Rust instruction"));
    assert!(!external.iter().any(|message| message.content.contains("FULL SKILL BODY")));

    let child = stream_request(
        "child-session",
        "reviewer",
        None,
        "inspect the same protocol",
        desktop.clone(),
        Vec::new(),
        Vec::new(),
        Default::default(),
        Vec::new(),
    );
    assert_eq!(&child.messages[..desktop.len()], desktop.as_slice());
    assert_eq!(
        child.messages.last(),
        Some(&json!({ "role": "user", "content": "inspect the same protocol" }))
    );

    let resumed: SkillListingSessionState =
        serde_json::from_str(&serde_json::to_string(&state).expect("serialize state"))
            .expect("resume state");
    let forked = resumed.fork();
    write(
        &skill_file,
        "---\nallowed-tools: [write]\nmodel: provider/matrix-next\neffort: max\n---\nUPDATED SKILL BODY $ARGUMENTS\n",
    );
    let reloaded = forked.restore_invoked("main").expect("fork reload");
    assert!(reloaded[0].content.contains("UPDATED SKILL BODY argument"));
    assert_eq!(reloaded[0].allowed_tools, vec!["write"]);

    let reloaded_agents = Cell::new(false);
    let mut compacted = history.clone();
    let updates = crate::session::compaction::restore_invoked_skills_after_compaction(
        &mut compacted,
        &forked,
        "main",
        || reloaded_agents.set(true),
    )
    .expect("compaction restore");
    assert!(reloaded_agents.get());
    assert!(
        compacted[0]["content"]
            .as_str()
            .is_some_and(|content| content.contains("UPDATED SKILL BODY argument"))
    );
    assert!(
        updates
            .iter()
            .any(|update| update.key == "skill.model" && update.value == "provider/matrix-next")
    );
}

struct DelegateParityTool {
    name: &'static str,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for DelegateParityTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "delegate parity fixture"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: self.name.to_string(), error: None })
    }
}

#[test]
fn delegate_capability_policy_is_consistent_across_agenttool_gateway_and_legacy_adapter() {
    let parent_tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(DelegateParityTool { name: "file_read" }),
        Arc::new(DelegateParityTool { name: "shell" }),
        Arc::new(DelegateParityTool { name: "skill" }),
        Arc::new(DelegateParityTool { name: "AgentTool" }),
    ];
    let defaults = vec!["file_read".to_string(), "skill".to_string()];
    let trusted_skills = vec!["extra".to_string()];

    for (policy, expected_skill_enum) in [
        (DelegateCapabilityPolicy::Strict, Some(json!(["review"]))),
        (DelegateCapabilityPolicy::Unrestricted, None),
    ] {
        let config = DelegateAgentConfig {
            allowed_tools: defaults.clone(),
            skill_policy: policy,
            allowed_skills: vec!["review".to_string()],
            ..DelegateAgentConfig::default()
        };
        let legacy = crate::app::agent::tools::delegated_tools::build_agentic_tools(
            &parent_tools,
            &config,
            &trusted_skills,
        );
        let legacy_names = legacy.iter().map(|tool| tool.name()).collect::<Vec<_>>();
        assert_eq!(legacy_names, vec!["file_read", "skill"]);

        let skill = legacy.iter().find(|tool| tool.name() == "skill").expect("skill tool");
        match expected_skill_enum {
            Some(expected) => {
                assert_eq!(skill.parameters_schema()["properties"]["name"]["enum"], expected)
            }
            None => assert!(skill.parameters_schema()["properties"].get("name").is_none()),
        }
        let modern_skills = resolve_skill_capabilities(
            ["review", "extra", "model-generated"],
            ["review"],
            &trusted_skills,
            policy,
            std::iter::empty::<&str>(),
        );
        assert_eq!(
            modern_skills.visible_ids.contains("model-generated"),
            policy == DelegateCapabilityPolicy::Unrestricted,
        );
    }
}
