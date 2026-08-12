use super::{Skill, SkillListingSessionState};

fn skill(name: &str) -> Skill {
    Skill { name: name.to_string(), description: format!("{name} description"), ..Skill::default() }
}

#[test]
fn skill_listing_requires_enabled_skill_tool() {
    let mut state = SkillListingSessionState::default();
    let skills = vec![skill("docs")];

    assert!(state.prepare("primary", &skills, false, 20_000).is_none());
    let reminder = state
        .prepare("primary", &skills, true, 20_000)
        .expect("skill becomes available after enabling tool");
    assert!(reminder.content.contains("<name>docs</name>"));
}

#[test]
fn skill_listing_keeps_unsent_skills_for_later_budgeted_delta() {
    let mut state = SkillListingSessionState::default();
    let skills = vec![skill("alpha"), skill("beta")];

    assert!(state.prepare("primary", &skills, true, 1_000).is_none());
    let reminder = state
        .prepare("primary", &skills, true, 20_000)
        .expect("skills remain unsent after an insufficient budget");
    assert!(reminder.content.contains("<name>alpha</name>"));
    assert!(reminder.content.contains("<name>beta</name>"));
}

#[test]
fn default_preload_uses_real_skill_source_once_and_restores_after_replay() {
    let root = tempfile::tempdir().expect("skill dir");
    let source = root.path().join("SKILL.md");
    std::fs::write(&source, "# Default\nFULL_DEFAULT_PROTOCOL").expect("write skill");
    let skills = vec![Skill { name: "default".to_string(), location: Some(source), ..Skill::default() }];
    let mut state = SkillListingSessionState::default();
    let defaults = ["default".to_string()].into_iter().collect();

    state.preload_defaults("delegate", &skills, &defaults).expect("preload defaults");
    state.preload_defaults("delegate", &skills, &defaults).expect("deduplicate defaults");
    let replays = state.restore_invoked("delegate").expect("restore defaults");

    assert_eq!(replays.len(), 1);
    assert!(replays[0].content.contains("FULL_DEFAULT_PROTOCOL"));
}
