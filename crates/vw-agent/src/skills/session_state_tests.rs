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
