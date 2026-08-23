//! 验证技能列表的筛选、排序和展示信息构造行为。
//! 增量列表受预算约束并保持确定排序，尚未发送的技能留待后续轮次。

use super::{Skill, SkillListingSessionState};

fn skill(name: &str, description: &str) -> Skill {
    Skill { name: name.to_string(), description: description.to_string(), ..Skill::default() }
}

#[test]
fn skill_listing_sends_only_budgeted_deltas_per_agent() {
    let mut state = SkillListingSessionState::default();
    let skills = vec![skill("zeta", "Zeta skill"), skill("alpha", "Alpha skill")];

    let first = state.prepare("primary", &skills, true, 20_000).expect("first listing");
    assert!(first.content.contains("<name>alpha</name>"));
    assert!(first.content.contains("<name>zeta</name>"));
    assert!(first.content.find("alpha") < first.content.find("zeta"));
    assert!(first.estimated_tokens <= first.budget_tokens);

    assert!(state.prepare("primary", &skills, true, 20_000).is_none());

    let new_skill = skill("beta", "Beta skill");
    let delta = state
        .prepare("primary", &[skills[0].clone(), new_skill], true, 20_000)
        .expect("new skill delta");
    assert!(delta.content.contains("<name>beta</name>"));
    assert!(!delta.content.contains("<name>zeta</name>"));

    let another_agent =
        state.prepare("reviewer", &skills, true, 20_000).expect("independent agent listing");
    assert!(another_agent.content.contains("<name>alpha</name>"));
    assert!(another_agent.content.contains("<name>zeta</name>"));
}

#[test]
fn skill_listing_truncates_descriptions_and_respects_budget_deterministically() {
    let mut state = SkillListingSessionState::default();
    let description = format!("{}SKILL.md body must not leak", "a".repeat(300));
    let skills = vec![skill("alpha", &description), skill("beta", "short")];

    let first = state.prepare("primary", &skills, true, 25_000).expect("budgeted listing");
    assert!(first.estimated_tokens <= first.budget_tokens);
    assert!(first.content.contains(&"a".repeat(250)));
    assert!(!first.content.contains("SKILL.md body must not leak"));
    assert!(first.content.contains("<system-reminder>"));
    assert!(!first.content.contains("<instructions>"));
    assert!(!first.content.contains("<tools>"));
}
