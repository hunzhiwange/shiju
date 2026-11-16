//! 技能参数、描述序列化及局部值行为；不依赖代理宿主。

use super::{
    Args, available_skill_names, base_dir_url, explicitly_referenced_skill_names, file_url,
    skill_file_id, skill_location, skill_matches_name, skill_prompt_content,
    text_contains_skill_reference, user_explicitly_requests_skill_use,
    user_invocable_skill_names, xml_escape,
};
use agent_skills::Skill;
use serde_json::json;
use std::path::PathBuf;

fn skill(name: &str, location: Option<PathBuf>, prompts: Vec<&str>) -> Skill {
    Skill {
        name: name.to_string(),
        description: format!("{name} description"),
        version: "1.0.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: Vec::new(),
        prompts: prompts.into_iter().map(ToOwned::to_owned).collect(),
        location,
        ..Skill::default()
    }
}

#[test]
fn args_deserialize_skill_and_ignore_unknown_fields() {
    let args: Args = serde_json::from_value(json!({
        "skill": "rustcodegraph",
        "args": "crates/vw-agent",
        "ignored": true,
    }))
    .expect("valid skill arguments should deserialize");

    assert_eq!(args.skill, "rustcodegraph");
    assert_eq!(args.args.as_deref(), Some("crates/vw-agent"));
}

#[test]
fn args_reject_missing_or_non_string_skill() {
    let missing = serde_json::from_value::<Args>(json!({}));
    let non_string = serde_json::from_value::<Args>(json!({ "skill": 239 }));

    assert!(missing.is_err());
    assert!(non_string.is_err());
}

#[test]
fn skill_file_id_uses_parent_for_standard_skill_files() {
    let md = skill("Display Name", Some(PathBuf::from("/tmp/my-skill/SKILL.md")), vec![]);
    let toml = skill("Display Name", Some(PathBuf::from("/tmp/other-skill/SKILL.toml")), vec![]);
    let custom = skill("Display Name", Some(PathBuf::from("/tmp/custom-name.md")), vec![]);
    let missing = skill("Display Name", None, vec![]);

    assert_eq!(skill_file_id(&md).as_deref(), Some("my-skill"));
    assert_eq!(skill_file_id(&toml).as_deref(), Some("other-skill"));
    assert_eq!(skill_file_id(&custom).as_deref(), Some("custom-name"));
    assert_eq!(skill_file_id(&missing), None);
}

#[test]
fn skill_file_id_handles_case_insensitive_standard_names_and_parentless_paths() {
    let uppercase = skill("Display Name", Some(PathBuf::from("/tmp/cased/SKILL.MD")), vec![]);
    let parentless = skill("Display Name", Some(PathBuf::from("/")), vec![]);

    assert_eq!(skill_file_id(&uppercase).as_deref(), Some("cased"));
    assert_eq!(skill_file_id(&parentless), None);
}

#[test]
fn skill_matching_accepts_display_name_or_file_id() {
    let item = skill("Display Name", Some(PathBuf::from("/tmp/my-skill/SKILL.md")), vec![]);

    assert!(skill_matches_name(&item, "Display Name"));
    assert!(skill_matches_name(&item, "my-skill"));
    assert!(!skill_matches_name(&item, "missing"));
}

#[test]
fn explicit_skill_reference_requires_identifier_boundaries() {
    assert!(text_contains_skill_reference(
        "优先使用 ./skills/rustcodegraph/SKILL.md skill",
        "rustcodegraph"
    ));
    assert!(text_contains_skill_reference("请使用 $RustCodeGraph", "rustcodegraph"));
    assert!(!text_contains_skill_reference("rustcodegraph-next", "rustcodegraph"));
    assert!(!text_contains_skill_reference("do not trustcodegraph output", "rustcodegraph"));
}

#[test]
fn repository_protocol_reference_authorizes_display_name_and_file_id() {
    let item = skill(
        "Rust Code Graph",
        Some(PathBuf::from("/workspace/skills/rustcodegraph/SKILL.md")),
        vec![],
    );

    let requested = explicitly_referenced_skill_names(
        &[item],
        ["优先使用 ./skills/rustcodegraph/SKILL.md skill"],
    );

    assert!(requested.contains("rust code graph"));
    assert!(requested.contains("rustcodegraph"));
}

#[test]
fn generic_user_skill_directive_is_explicit_but_negation_is_not() {
    assert!(user_explicitly_requests_skill_use("请先加载其中规定的技能"));
    assert!(user_explicitly_requests_skill_use("使用技能完成代码导航"));
    assert!(user_explicitly_requests_skill_use("Please use a skill for this task"));
    assert!(!user_explicitly_requests_skill_use("不要使用技能"));
    assert!(!user_explicitly_requests_skill_use("为什么加载技能会失败？"));
    assert!(!user_explicitly_requests_skill_use("请使用普通工具"));
}

#[test]
fn generic_user_skill_directive_only_authorizes_user_invocable_skills() {
    let allowed = skill("allowed", Some(PathBuf::from("/skills/allowed/SKILL.md")), vec![]);
    let mut denied = skill("denied", Some(PathBuf::from("/skills/denied/SKILL.md")), vec![]);
    denied.user_invocable = false;

    let names = user_invocable_skill_names(&[allowed, denied]);

    assert!(names.contains("allowed"));
    assert!(!names.contains("denied"));
}

#[test]
fn skill_location_defaults_to_workspace_skills_directory() {
    let workspace = PathBuf::from("/workspace");
    let explicit = skill("explicit", Some(PathBuf::from("/tmp/skill/SKILL.md")), vec![]);
    let implicit = skill("implicit", None, vec![]);

    assert_eq!(skill_location(&explicit, &workspace), PathBuf::from("/tmp/skill/SKILL.md"));
    assert_eq!(
        skill_location(&implicit, &workspace),
        PathBuf::from("/workspace/skills/implicit/SKILL.md")
    );
}

#[test]
fn available_skill_names_includes_sorted_names_and_file_ids() {
    let skills = vec![
        skill("zeta", Some(PathBuf::from("/tmp/alpha/SKILL.md")), vec![]),
        skill("beta", Some(PathBuf::from("/tmp/custom-gamma.md")), vec![]),
    ];

    assert_eq!(available_skill_names(&skills), "alpha, beta, custom-gamma, zeta");
    assert_eq!(available_skill_names(&[]), "无");
}

#[test]
fn skill_prompt_content_trims_empties_and_uses_fallback() {
    let with_prompts = skill("s", None, vec![" first ", "", "\nsecond\n"]);
    let empty = skill("s", None, vec![" ", ""]);

    let with_prompts = skill_prompt_content(&with_prompts);
    assert!(with_prompts.starts_with("<skill_execution_policy>"));
    assert!(with_prompts.contains("不得静默跳过、合并、替代或虚构任何步骤"));
    assert!(with_prompts.contains("如果任一步骤无法执行，必须立即停止"));
    assert!(with_prompts.ends_with(" first \n\n\nsecond\n"));

    let empty = skill_prompt_content(&empty);
    assert!(empty.starts_with("<skill_execution_policy>"));
    assert!(empty.ends_with("该技能没有内联指令。"));
}

#[test]
fn xml_escape_replaces_special_characters() {
    assert_eq!(xml_escape("a&b<c>d\"e'f"), "a&amp;b&lt;c&gt;d&quot;e&apos;f");
}

#[test]
fn file_urls_are_absolute_and_directory_urls_have_trailing_slash() {
    let url = file_url(PathBuf::from("relative-skill/SKILL.md").as_path());
    assert!(url.starts_with("file:///"));
    assert!(url.ends_with("/relative-skill/SKILL.md"));

    assert_eq!(base_dir_url(PathBuf::from("/tmp/skill").as_path()), "file:////tmp/skill/");
    assert_eq!(base_dir_url(PathBuf::from("/tmp/skill/").as_path()), "file:////tmp/skill/");
}

#[test]
fn args_require_a_string_skill() {
    let args: Args = serde_json::from_value(json!({"skill": "review"})).unwrap();
    assert_eq!(args.skill, "review");

    assert!(serde_json::from_value::<Args>(json!({})).is_err());
    assert!(serde_json::from_value::<Args>(json!({"skill": 7})).is_err());
}
