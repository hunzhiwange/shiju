//! 通过真实技能工具、包外安装状态和会话恢复验证限定调用。

use crate::config::Config;
use crate::skills::{SkillListingSessionState, load_skills_full_with_config};
use crate::tools::skill::{SkillTool, explicitly_referenced_skill_names};
use crate::tools::traits::Tool;
use crate::tools::{ToolUseContext, scope_tool_use_context};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MANIFEST: &[u8] = include_bytes!("../../../testdata/plugin_compat/t10/plugin.json");
const SKILL: &[u8] = include_bytes!("../../../testdata/plugin_compat/t10/SKILL.md");

struct Fixture {
    _temp: tempfile::TempDir,
    workspace: PathBuf,
    config: Arc<Config>,
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

impl Fixture {
    fn new(restricted: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        let source = temp.path().join("original");
        write(&source.join(".codex-plugin/plugin.json"), MANIFEST);
        let content = if restricted {
            String::from_utf8(SKILL.to_vec())
                .unwrap()
                .replace("argument-hint:", "disable-model-invocation: true\nargument-hint:")
                .into_bytes()
        } else {
            SKILL.to_vec()
        };
        write(&source.join("skills/review/SKILL.md"), &content);
        write(&source.join("skills/review/resources/input.txt"), b"package resource");
        write(&source.join("skills/review/scripts/review.sh"), b"#!/bin/sh\nprintf '%s' \"$1\"\n");
        for market in ["first", "second"] {
            crate::plugins::install_plugin_from_marketplace(
                source.to_str().unwrap(),
                &workspace.join("plugins"),
                market,
            )
            .unwrap();
        }
        let mut config = Config::default();
        config.workspace_dir = workspace.clone();
        config.config_path = temp.path().join("config/vibewindow.json");
        Self { _temp: temp, workspace, config: Arc::new(config) }
    }

    fn tool(&self) -> SkillTool {
        SkillTool::new_with_runtime_config(
            Arc::new(crate::security::SecurityPolicy::default()),
            "t10".into(),
            false,
            self.workspace.clone(),
            self.config.clone(),
        )
    }
}

#[tokio::test]
async fn codex_plugin_compat_t10_invokes_namespaced_skill_with_arguments() {
    let fixture = Fixture::new(false);
    let tool = fixture.tool();
    let context = Arc::new(ToolUseContext::new("t10", None).with_full_access_enabled(true));
    let result = scope_tool_use_context(
        context.clone(),
        tool.call(json!({"skill":"invocation-pack@second:review","args":"src/main.rs --strict"})),
    )
    .await
    .unwrap();
    assert_eq!(result.data["success"], true, "qualified skill must load: {:?}", result.data);
    let text = result.extra_messages[0].content.as_str().unwrap();
    assert!(text.contains("src/main.rs --strict"));
    assert!(text.contains("invocation-pack@second/skills/review"));
    assert!(!text.contains("invocation-pack@first/skills/review"));
    let records = context.take_invoked_skills();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].skill_name, "invocation-pack@second:review");
    assert_eq!(records[0].args.as_deref(), Some("src/main.rs --strict"));
    let root = fixture.workspace.join("plugins/invocation-pack@second");
    assert_eq!(fs::read(root.join("skills/review/SKILL.md")).unwrap(), SKILL);
    assert_eq!(fs::read(root.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
    assert!(!fixture.workspace.join("resources/input.txt").exists());
}

#[tokio::test]
async fn codex_plugin_compat_t10_rejects_invalid_input() {
    let fixture = Fixture::new(false);
    let context = Arc::new(ToolUseContext::new("t10", None).with_full_access_enabled(true));
    let result =
        scope_tool_use_context(context.clone(), fixture.tool().call(json!({"skill":"review"})))
            .await
            .unwrap();
    assert_eq!(
        result.data["success"], false,
        "an unqualified duplicate must not pick the first market"
    );
    assert!(context.take_invoked_skills().is_empty());
    for invalid in ["invocation-pack@missing:review", "invocation-pack@first:../review"] {
        let result =
            scope_tool_use_context(context.clone(), fixture.tool().call(json!({"skill":invalid})))
                .await
                .unwrap();
        assert_eq!(result.data["success"], false);
    }
}

#[tokio::test]
async fn codex_plugin_compat_t10_preserves_isolation() {
    let fixture = Fixture::new(true);
    let skills = load_skills_full_with_config(&fixture.workspace, &fixture.config);
    let selected = explicitly_referenced_skill_names(
        &skills,
        ["Use [@unrelated-label](plugin://invocation-pack@first) with $review"],
    );
    assert!(
        selected.contains("invocation-pack@first:review"),
        "exact plugin identity selects its own skill"
    );
    assert!(!selected.contains("invocation-pack@second:review"));
    let context = Arc::new(
        ToolUseContext::new("t10", None)
            .with_full_access_enabled(true)
            .with_explicitly_requested_skills(selected),
    );
    let result = scope_tool_use_context(
        context.clone(),
        fixture.tool().call(json!({"skill":"invocation-pack@first:review","args":"one"})),
    )
    .await
    .unwrap();
    assert_eq!(result.data["success"], true, "selected plugin may be explicitly invoked");
    let other = scope_tool_use_context(
        context.clone(),
        fixture.tool().call(json!({"skill":"invocation-pack@second:review"})),
    )
    .await
    .unwrap();
    assert_eq!(other.data["success"], false, "selection must not authorize another marketplace");
    let mut state = SkillListingSessionState::default();
    for record in context.take_invoked_skills() {
        state.register_invoked("main", record);
    }
    let state: SkillListingSessionState =
        serde_json::from_value(serde_json::to_value(state).unwrap()).unwrap();
    state.validate_invoked_sources("main", &skills).unwrap();
    let replay = state.restore_invoked("main").unwrap();
    assert_eq!(replay.len(), 1);
    assert!(replay[0].content.contains("invocation-pack@first:review"));
    assert!(replay[0].content.contains("with one"));
    assert!(state.restore_invoked("other-agent").unwrap().is_empty());
    crate::plugins::set_plugin_enabled(
        &fixture.workspace.join("plugins"),
        "invocation-pack@first",
        false,
    )
    .unwrap();
    let current = load_skills_full_with_config(&fixture.workspace, &fixture.config);
    assert!(
        state.validate_invoked_sources("main", &current).is_err(),
        "disabled packages cannot replay"
    );
}

#[tokio::test]
async fn codex_plugin_compat_t10_rejects_ambiguous_namespace_and_keeps_unique_alias() {
    let fixture = Fixture::new(false);
    let context = Arc::new(ToolUseContext::new("t10", None).with_full_access_enabled(true));
    let result = scope_tool_use_context(
        context.clone(),
        fixture.tool().call(json!({"skill":"invocation-pack:review"})),
    )
    .await
    .unwrap();
    assert_eq!(result.data["success"], false);
    assert!(result.data["error"].as_str().unwrap().contains("歧义"));
    assert!(context.take_invoked_skills().is_empty());
    crate::plugins::set_plugin_enabled(
        &fixture.workspace.join("plugins"),
        "invocation-pack@second",
        false,
    )
    .unwrap();
    let result = scope_tool_use_context(
        context.clone(),
        fixture.tool().call(json!({"skill":"invocation-pack:review"})),
    )
    .await
    .unwrap();
    assert_eq!(result.data["success"], true);
    assert_eq!(result.data["skill"], "invocation-pack@first:review");
    assert!(result.model_text().contains("invocation-pack@first:review"));
    assert_eq!(context.take_invoked_skills()[0].skill_name, "invocation-pack@first:review");
}

#[test]
fn codex_plugin_compat_t10_uses_exact_plugin_links_without_display_name_authorization() {
    let fixture = Fixture::new(false);
    let mut skills = load_skills_full_with_config(&fixture.workspace, &fixture.config);
    skills.push(crate::skills::Skill { name: "alias".into(), ..Default::default() });
    for wrong in [
        "Use [$alias](plugin://invocation-pack@first)",
        "Use [@alias](plugin://invocation-pack@first-more)",
        "Use [@alias](plugin://)",
        "Use [@HOME](plugin://invocation-pack@first)",
    ] {
        assert!(explicitly_referenced_skill_names(&skills, [wrong]).is_empty(), "{wrong}");
    }
    assert!(
        explicitly_referenced_skill_names(
            &skills,
            ["Use [@alias](plugin://invocation-pack@first)"]
        )
        .is_empty(),
        "plugin selection alone must keep skill bodies on demand"
    );
    let selected = explicitly_referenced_skill_names(
        &skills,
        ["Use [@alias] ( plugin://invocation-pack@second?browserFamily=chrome ) with $review"],
    );
    assert!(selected.contains("invocation-pack@second:review"));
    assert!(!selected.contains("invocation-pack@first:review"));
    assert!(!selected.contains("alias"));
    assert!(!selected.contains("review"));
    assert!(!selected.contains("invocation-pack:review"));
}

#[tokio::test]
async fn codex_plugin_compat_t10_preserves_delegate_identity_boundaries() {
    let fixture = Fixture::new(false);
    let context = Arc::new(
        ToolUseContext::new("t10", None)
            .with_full_access_enabled(true)
            .with_delegate_skill_policy(
                vw_config_types::agent::DelegateCapabilityPolicy::Strict,
                ["invocation-pack@first:review"],
            )
            .with_explicitly_requested_skills(["invocation-pack@second:review"]),
    );
    let tool = fixture.tool().with_skill_directory_context(&context);
    assert!(tool.description().contains("invocation-pack@first:review"));
    assert!(!tool.description().contains("invocation-pack@second:review"));
    let denied = scope_tool_use_context(
        context.clone(),
        tool.call(json!({"skill":"invocation-pack@second:review"})),
    )
    .await
    .unwrap();
    assert_eq!(denied.data["success"], false);
    assert!(context.take_invoked_skills().is_empty());
    let ambiguous = scope_tool_use_context(
        context.clone(),
        tool.call(json!({"skill":"invocation-pack:review"})),
    )
    .await
    .unwrap();
    assert_eq!(
        ambiguous.data["success"], false,
        "delegation must not hide another market to make an ambiguous alias valid"
    );
    let allowed = scope_tool_use_context(
        context.clone(),
        tool.call(json!({"skill":"invocation-pack@first:review"})),
    )
    .await
    .unwrap();
    assert_eq!(allowed.data["success"], true);
}

#[test]
fn codex_plugin_compat_t10_catalog_and_preloads_preserve_qualified_identity() {
    let fixture = Fixture::new(false);
    let skills = load_skills_full_with_config(&fixture.workspace, &fixture.config);
    let mut state = SkillListingSessionState::default();
    let reminder = state.prepare("main", &skills, true, 100_000).unwrap();
    assert!(reminder.skill_names.contains(&"invocation-pack@first:review".into()));
    assert!(reminder.skill_names.contains(&"invocation-pack@second:review".into()));
    assert!(!reminder.content.contains("Read resources/input.txt"));
    assert!(
        state
            .preload_defaults(
                "main",
                &skills,
                &["invocation-pack:review".into()].into_iter().collect()
            )
            .is_err()
    );
    assert!(state.invoked_records("main").is_empty());
    let selected = explicitly_referenced_skill_names(
        &skills,
        ["Use [@alias](plugin://invocation-pack@first) with $review"],
    );
    state.preload_explicit("main", &skills, &selected).unwrap();
    assert_eq!(state.invoked_records("main")[0].skill_name, "invocation-pack@first:review");
    let mut changed = skills.clone();
    for skill in &mut changed {
        if let Some(source) = &mut skill.plugin_source
            && source.plugin_id == "invocation-pack@first"
        {
            source.plugin_id = "renamed@market".into();
        }
    }
    assert!(
        state.validate_invoked_sources("main", &changed).is_err(),
        "a path cannot authorize a changed identity"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t10_relative_resources_and_scripts_use_skill_directory() {
    let fixture = Fixture::new(false);
    let context = Arc::new(ToolUseContext::new("t10", None).with_full_access_enabled(true));
    let result = scope_tool_use_context(
        context,
        fixture.tool().call(json!({"skill":"invocation-pack@first:review","args":"two words"})),
    )
    .await
    .unwrap();
    assert_eq!(result.data["success"], true);
    let directory =
        PathBuf::from(result.data["base_dir"].as_str().unwrap().strip_prefix("file://").unwrap());
    assert_eq!(fs::read(directory.join("resources/input.txt")).unwrap(), b"package resource");
    let output = std::process::Command::new("/bin/sh")
        .arg("scripts/review.sh")
        .arg("two words")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"two words");
    assert_eq!(fs::read(directory.join("SKILL.md")).unwrap(), SKILL);
    assert_eq!(
        fs::read(directory.join("scripts/review.sh")).unwrap(),
        b"#!/bin/sh\nprintf '%s' \"$1\"\n"
    );
}

#[tokio::test]
async fn codex_plugin_compat_t10_preserves_user_invocation_restriction() {
    let fixture = Fixture::new(false);
    let path = fixture.workspace.join("plugins/invocation-pack@first/skills/review/SKILL.md");
    let restricted = String::from_utf8(SKILL.to_vec())
        .unwrap()
        .replace("argument-hint:", "user-invocable: false\nargument-hint:");
    fs::write(&path, &restricted).unwrap();
    let context = Arc::new(
        ToolUseContext::new("t10", None)
            .with_full_access_enabled(true)
            .with_explicitly_requested_skills(["invocation-pack@first:review"]),
    );
    let result = scope_tool_use_context(
        context.clone(),
        fixture.tool().call(json!({"skill":"invocation-pack@first:review"})),
    )
    .await
    .unwrap();
    assert_eq!(result.data["success"], false);
    assert!(context.take_invoked_skills().is_empty());
    assert_eq!(fs::read_to_string(path).unwrap(), restricted);
}
