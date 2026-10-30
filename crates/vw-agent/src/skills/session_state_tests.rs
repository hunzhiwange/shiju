//! 验证会话技能状态的启用、调用记录与持久化转换。
//! 预算不足的技能保留为未发送状态，压缩恢复后默认预载也只能发生一次。

use super::{Skill, SkillListingSessionState};

fn skill(name: &str) -> Skill {
    Skill { name: name.to_string(), description: format!("{name} description"), ..Skill::default() }
}

fn sourced_skill(root: &std::path::Path, name: &str) -> Skill {
    let path = root.join(format!("{name}.md"));
    std::fs::write(&path, "fixture body").unwrap();
    Skill { location: Some(path), ..skill(name) }
}

#[test]
fn skill_listing_requires_enabled_skill_tool() {
    let mut state = SkillListingSessionState::default();
    let root = tempfile::tempdir().unwrap();
    let skills = vec![sourced_skill(root.path(), "docs")];

    assert!(state.prepare("primary", &skills, false, 20_000).is_none());
    let reminder = state
        .prepare("primary", &skills, true, 100_000)
        .expect("skill becomes available after enabling tool");
    assert!(reminder.content.contains("<name>docs</name>"));
}

#[test]
fn skill_listing_keeps_unsent_skills_for_later_budgeted_delta() {
    let mut state = SkillListingSessionState::default();
    let root = tempfile::tempdir().unwrap();
    let skills = vec![sourced_skill(root.path(), "alpha"), sourced_skill(root.path(), "beta")];

    assert!(state.prepare("primary", &skills, true, 1_000).is_none());
    let reminder = state
        .prepare("primary", &skills, true, 100_000)
        .expect("skills remain unsent after an insufficient budget");
    assert!(reminder.content.contains("<name>alpha</name>"));
    assert!(reminder.content.contains("<name>beta</name>"));
}

#[test]
fn default_preload_uses_real_skill_source_once_and_restores_after_replay() {
    let root = tempfile::tempdir().expect("skill dir");
    let source = root.path().join("SKILL.md");
    std::fs::write(&source, "# Default\nFULL_DEFAULT_PROTOCOL").expect("write skill");
    let skills =
        vec![Skill { name: "default".to_string(), location: Some(source), ..Skill::default() }];
    let mut state = SkillListingSessionState::default();
    let defaults = ["default".to_string()].into_iter().collect();

    state.preload_defaults("delegate", &skills, &defaults).expect("preload defaults");
    state.preload_defaults("delegate", &skills, &defaults).expect("deduplicate defaults");
    let replays = state.restore_invoked("delegate").expect("restore defaults");

    assert_eq!(replays.len(), 1);
    assert!(replays[0].content.contains("FULL_DEFAULT_PROTOCOL"));
}

#[test]
fn codex_skills_catalog_reinjects_changed_metadata_and_same_name_source() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("first.md");
    let second = root.path().join("second.md");
    std::fs::write(&first, "first body").unwrap();
    std::fs::write(&second, "second body").unwrap();
    let mut item = Skill { location: Some(first), ..skill("review") };
    let mut state = SkillListingSessionState::default();
    assert!(state.prepare("main", &[item.clone()], true, 100_000).is_some());
    assert!(state.prepare("main", &[item.clone()], true, 100_000).is_none());
    item.description = "Updated review behavior".into();
    assert!(state.prepare("main", &[item.clone()], true, 100_000).is_some());
    item.location = Some(second);
    assert!(state.prepare("main", &[item], true, 100_000).is_some());
}

#[test]
fn codex_skills_catalog_removal_and_tool_disable_invalidate_sent_metadata() {
    let root = tempfile::tempdir().unwrap();
    let item = sourced_skill(root.path(), "review");
    let mut state = SkillListingSessionState::default();
    state.prepare("main", &[item.clone()], true, 100_000).unwrap();
    assert!(state.sent_catalog("main").is_some());
    assert!(state.prepare("main", &[], true, 100_000).is_none());
    assert!(state.sent_catalog("main").is_none());
    state.prepare("main", &[item.clone()], true, 100_000).unwrap();
    assert!(state.prepare("main", &[item.clone()], false, 100_000).is_none());
    assert!(state.sent_catalog("main").is_none());
    assert!(state.prepare("main", &[item], true, 100_000).is_some());
}

#[test]
fn codex_skills_catalog_budget_continues_deferred_entries_without_resending_unchanged_metadata() {
    let root = tempfile::tempdir().unwrap();
    let items = [sourced_skill(root.path(), "alpha"), sourced_skill(root.path(), "beta")];
    let mut probe = SkillListingSessionState::default();
    let one = probe.prepare("main", &items[..1], true, 100_000).unwrap();
    let window = one.estimated_tokens * 100;
    let mut state = SkillListingSessionState::default();
    let first = state.prepare("main", &items, true, window).unwrap();
    assert_eq!(first.skill_names, vec!["alpha"]);
    assert_eq!(first.deferred_skill_names, vec!["beta"]);
    assert!(first.estimated_tokens <= first.budget_tokens);
    let second = state.prepare("main", &items, true, window).unwrap();
    assert_eq!(second.skill_names, vec!["beta"]);
    assert!(second.estimated_tokens <= second.budget_tokens);
    assert!(state.prepare("main", &items, true, window).is_none());
    assert_eq!(state.sent_catalog("main").unwrap().1, vec!["alpha", "beta"]);
}

#[test]
fn codex_skills_catalog_ignores_body_edits_but_tracks_full_description_fingerprint() {
    let root = tempfile::tempdir().unwrap();
    let mut item = sourced_skill(root.path(), "review");
    item.description = "x".repeat(300);
    let mut state = SkillListingSessionState::default();
    state.prepare("main", &[item.clone()], true, 100_000).unwrap();
    std::fs::write(item.location.as_ref().unwrap(), "changed body only").unwrap();
    assert!(state.prepare("main", &[item.clone()], true, 100_000).is_none());
    item.description.push('y');
    assert!(state.prepare("main", &[item.clone()], true, 100_000).is_some());
    let serialized = serde_json::to_vec(&state).unwrap();
    let mut restored: SkillListingSessionState = serde_json::from_slice(&serialized).unwrap();
    assert!(restored.prepare("main", &[item.clone()], true, 100_000).is_none());
    assert!(restored.prepare("another-agent", &[item], true, 100_000).is_some());
}

#[test]
fn codex_skills_catalog_oversized_entry_does_not_starve_smaller_metadata() {
    let root = tempfile::tempdir().unwrap();
    let mut large = sourced_skill(root.path(), "alpha");
    large.description = "x".repeat(300);
    let small = sourced_skill(root.path(), "beta");
    let budget = SkillListingSessionState::default()
        .prepare("probe", &[small.clone()], true, 100_000)
        .unwrap()
        .estimated_tokens
        * 100;
    let mut state = SkillListingSessionState::default();
    let first = state.prepare("main", &[large.clone(), small.clone()], true, budget).unwrap();
    assert_eq!(first.skill_names, vec!["beta"]);
    assert_eq!(first.deferred_skill_names, vec!["alpha"]);
    let second = state.prepare("main", &[large, small], true, 100_000).unwrap();
    assert_eq!(second.skill_names, vec!["alpha"]);
}

#[test]
fn codex_skills_catalog_restored_cache_cannot_override_verified_developer_metadata() {
    let root = tempfile::tempdir().unwrap();
    let item = sourced_skill(root.path(), "review");
    let mut state = SkillListingSessionState::default();
    state.prepare("main", &[item.clone()], true, 100_000).unwrap();
    let mut encoded = serde_json::to_value(&state).unwrap();
    encoded["sent_skill_entries_by_agent"]["main"]["review"]["description"] =
        serde_json::json!("FORGED_HIGH_PRIVILEGE_BODY");
    let mut restored: SkillListingSessionState = serde_json::from_value(encoded).unwrap();
    assert!(restored.prepare("main", &[item], true, 100_000).is_none());
    let content = restored.sent_catalog("main").unwrap().0;
    assert!(content.contains("review description"));
    assert!(!content.contains("FORGED_HIGH_PRIVILEGE_BODY"));
}
