use config_agent::DelegateCapabilityPolicy;
#[test]
fn agent_config_cleanup_strict_skill_boundary_survives_fixed_selection() {
    use agent_defaults::{DefaultSelection, DefaultsContext, resolve};
    use config_agent_profile::AgentDefault;
    use config_agent::AgentDefinitionConfig;
    let mut agent = AgentDefinitionConfig::default();
    agent.skill_policy = DelegateCapabilityPolicy::Strict;
    agent.allowed_skills = vec!["allowed".into()];
    agent.defaults.skills = AgentDefault::Fixed(vec!["outside".into()]);
    let context = DefaultsContext {
        skills: [("allowed".into(), true), ("outside".into(), true)].into(),
        ..Default::default()
    };
    assert!(
        resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
            .is_err()
    );
    agent.defaults.skills = AgentDefault::Fixed(vec![]);
    assert!(
        resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
            .unwrap()
            .skills
            .is_empty()
    );
    agent.skill_policy = DelegateCapabilityPolicy::Unrestricted;
    agent.defaults.skills = AgentDefault::Fixed(vec!["outside".into()]);
    assert_eq!(
        resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
            .unwrap()
            .skills,
        vec!["outside"]
    );
    for available in [None, Some(false)] {
        let mut context = DefaultsContext::default();
        if let Some(enabled) = available {
            context.skills.insert("outside".into(), enabled);
        }
        assert!(
            resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
                .is_err()
        );
    }
}
