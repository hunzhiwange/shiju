use super::defaults::*;
use serde_json::json;
use vw_config_types::agent::{AgentDefault, AgentDefinitionConfig, AgentModelReference};

#[test]
fn agent_defaults_respects_explicit_and_fixed_precedence() {
    let mut agent = AgentDefinitionConfig::default();
    let inherited = DefaultSelection { skills: Some(vec!["read".into()]), ..Default::default() };
    let mut context = DefaultsContext::default();
    context.skills.insert("read".into(), true);
    let result = resolve(&agent, &DefaultSelection::default(), &inherited, &context).unwrap();
    assert_eq!(result.skills, vec!["read"]);
    agent.defaults.skills = AgentDefault::Fixed(vec![]);
    let result = resolve(&agent, &DefaultSelection::default(), &inherited, &context).unwrap();
    assert!(result.skills.is_empty());
    let model = AgentModelReference { provider: "p".into(), model: "m".into() };
    agent.defaults.model = AgentDefault::Fixed(model.clone());
    let explicit = DefaultSelection {
        model: Some(AgentModelReference { model: "other".into(), ..model.clone() }),
        ..Default::default()
    };
    assert_eq!(resolve(&agent, &explicit, &inherited, &context).unwrap().model, explicit.model);
    agent.provider = "p".into();
    agent.model = "m".into();
    agent.force_model_selection = true;
    assert!(resolve(&agent, &explicit, &inherited, &context).is_err());
    agent.system_prompt = Some("rules".into());
    agent.recommended_prompts = vec!["suggestion".into()];
    let result = resolve(&agent, &DefaultSelection::default(), &inherited, &context).unwrap();
    assert_eq!(result.system_prompt.as_deref(), Some("rules"));
}

#[test]
fn agent_defaults_rejects_disabled_capabilities() {
    let mut agent = AgentDefinitionConfig::default();
    let mut context = DefaultsContext::default();
    agent.defaults.skills = AgentDefault::Fixed(vec!["private".into()]);
    assert!(
        resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
            .is_err()
    );
    context.skills.insert("private".into(), false);
    assert!(
        resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
            .is_err()
    );
    context.skills.insert("private".into(), true);
    agent.defaults.mcp = AgentDefault::Fixed(vec!["missing".into()]);
    assert!(
        resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
            .is_err()
    );
    context.mcp.insert("missing".into(), false);
    assert!(
        resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
            .is_err()
    );
    context.mcp.insert("missing".into(), true);
    context.permission_ceiling = crate::permission::next::from_config(&json!({"bash":"deny"}));
    agent.defaults.permission = AgentDefault::Fixed(json!({"bash":"allow"}));
    let result =
        resolve(&agent, &DefaultSelection::default(), &DefaultSelection::default(), &context)
            .unwrap();
    assert_eq!(result.permission_action("bash", "*"), crate::permission::next::Action::Deny);
}

#[test]
fn agent_defaults_validates_inputs_and_permission_intersection() {
    let mut agent = AgentDefinitionConfig::default();
    let context = DefaultsContext {
        permission_ceiling: crate::permission::next::from_config(
            &json!({"*":"allow", "bash":"ask"}),
        ),
        ..Default::default()
    };
    let inherited =
        DefaultSelection { permission: Some(json!({"*":"allow"})), ..Default::default() };
    let explicit =
        DefaultSelection { permission: Some(json!({"bash":"deny"})), ..Default::default() };
    let result = resolve(&agent, &explicit, &inherited, &context).unwrap();
    assert_eq!(result.permission_action("bash", "echo"), crate::permission::next::Action::Deny);
    let result = resolve(&agent, &DefaultSelection::default(), &inherited, &context).unwrap();
    assert_eq!(result.permission_action("bash", "echo"), crate::permission::next::Action::Ask);
    agent.defaults.permission = AgentDefault::Fixed(json!({"bash":"invalid"}));
    assert!(resolve(&agent, &DefaultSelection::default(), &inherited, &context).is_err());
    agent.defaults.permission = AgentDefault::Inherit;
    agent.force_model_selection = true;
    assert!(resolve(&agent, &DefaultSelection::default(), &inherited, &context).is_err());
    agent.force_model_selection = false;
    agent.enabled = false;
    assert!(resolve(&agent, &DefaultSelection::default(), &inherited, &context).is_err());
}
