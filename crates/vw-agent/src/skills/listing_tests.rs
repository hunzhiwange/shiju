//! 验证技能列表的筛选、排序和展示信息构造行为。
//! 增量列表受预算约束并保持确定排序，尚未发送的技能留待后续轮次。

use super::{Skill, SkillListingSessionState};

fn skill(root: &std::path::Path, name: &str, description: &str) -> Skill {
    let source = root.join(format!("{name}.md"));
    std::fs::write(&source, "fixture body").unwrap();
    Skill {
        name: name.to_string(),
        description: description.to_string(),
        location: Some(source),
        ..Skill::default()
    }
}

#[test]
fn skill_listing_sends_only_budgeted_deltas_per_agent() {
    let root = tempfile::tempdir().unwrap();
    let mut state = SkillListingSessionState::default();
    let skills =
        vec![skill(root.path(), "zeta", "Zeta skill"), skill(root.path(), "alpha", "Alpha skill")];

    let first = state.prepare("primary", &skills, true, 100_000).expect("first listing");
    assert!(first.content.contains("<name>alpha</name>"));
    assert!(first.content.contains("<name>zeta</name>"));
    assert!(first.content.find("alpha") < first.content.find("zeta"));
    assert!(first.estimated_tokens <= first.budget_tokens);

    assert!(state.prepare("primary", &skills, true, 100_000).is_none());

    let new_skill = skill(root.path(), "beta", "Beta skill");
    let delta = state
        .prepare("primary", &[skills[0].clone(), new_skill], true, 100_000)
        .expect("new skill delta");
    assert!(delta.content.contains("<name>beta</name>"));
    assert!(!delta.content.contains("<name>zeta</name>"));

    let another_agent =
        state.prepare("reviewer", &skills, true, 100_000).expect("independent agent listing");
    assert!(another_agent.content.contains("<name>alpha</name>"));
    assert!(another_agent.content.contains("<name>zeta</name>"));
}

#[test]
fn skill_listing_truncates_descriptions_and_respects_budget_deterministically() {
    let root = tempfile::tempdir().unwrap();
    let mut state = SkillListingSessionState::default();
    let description = format!("{}SKILL.md body must not leak", "a".repeat(300));
    let skills =
        vec![skill(root.path(), "alpha", &description), skill(root.path(), "beta", "short")];

    let first = state.prepare("primary", &skills, true, 25_000).expect("budgeted listing");
    assert!(first.estimated_tokens <= first.budget_tokens);
    assert!(first.content.contains(&"a".repeat(250)));
    assert!(!first.content.contains("SKILL.md body must not leak"));
    assert!(first.content.contains("<skills_instructions>"));
    assert!(!first.content.contains("<instructions>"));
    assert!(!first.content.contains("<tools>"));
}

#[test]
fn codex_skills_catalog_includes_canonical_source_and_usage_without_body() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("SKILL.md");
    std::fs::write(&path, "body must not be opened by the listing").unwrap();
    let item = Skill {
        name: "review".into(),
        description: "Review changes".into(),
        location: Some(path.clone()),
        prompts: vec!["PRIVATE_BODY_MARKER".into()],
        ..Default::default()
    };
    let reminder =
        SkillListingSessionState::default().prepare("main", &[item], true, 100_000).unwrap();
    assert!(reminder.content.contains(path.canonicalize().unwrap().to_str().unwrap()));
    assert!(reminder.content.contains("explicitly names"));
    assert!(reminder.content.contains("description matches"));
    assert!(reminder.content.contains("relative resources"));
    assert!(!reminder.content.contains("PRIVATE_BODY_MARKER"));
}

#[test]
fn codex_skills_catalog_preserves_source_priority_and_omits_unverified_or_disabled_items() {
    let root = tempfile::tempdir().unwrap();
    let first = skill(root.path(), "review", "Preferred source");
    let mut duplicate = first.clone();
    duplicate.description = "Lower priority source".into();
    let mut disabled = skill(root.path(), "disabled", "Disabled entry");
    disabled.disable_model_invocation = true;
    let missing = Skill {
        name: "missing".into(),
        description: "Unverified entry".into(),
        ..Default::default()
    };
    let mut state = SkillListingSessionState::default();
    let reminder =
        state.prepare("main", &[first, duplicate, disabled, missing], true, 100_000).unwrap();
    assert_eq!(reminder.skill_names, vec!["review"]);
    assert!(reminder.content.contains("Preferred source"));
    assert!(!reminder.content.contains("Lower priority"));
    assert!(!reminder.content.contains("Unverified entry"));
    assert!(!reminder.content.contains("Disabled entry"));
}

#[test]
fn codex_skills_catalog_does_not_reopen_body_when_preparing_metadata() {
    let root = tempfile::tempdir().unwrap();
    let item = skill(root.path(), "review", "Known metadata");
    std::fs::write(item.location.as_ref().unwrap(), [0xff, 0xfe]).unwrap();
    let reminder =
        SkillListingSessionState::default().prepare("main", &[item], true, 100_000).unwrap();
    assert!(reminder.content.contains("Known metadata"));
}
