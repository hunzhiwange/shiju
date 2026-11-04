//! Shared new-session defaults. Callers supply only trusted explicit selections and
//! capability catalogs already restricted to the current tenant/project scope.
use crate::permission::next::{self, Action, Ruleset};
use anyhow::{Result, bail};
use serde_json::Value;
use std::collections::HashMap;
use vw_config_types::agent::{AgentDefault, AgentDefinitionConfig, AgentModelReference};

#[derive(Default)]
pub struct DefaultSelection {
    pub model: Option<AgentModelReference>,
    pub permission: Option<Value>,
    pub skills: Option<Vec<String>>,
    pub mcp: Option<Vec<String>>,
}

/// Scoped stable IDs mapped to availability (false means hard disabled).
/// An omitted ID is unknown or outside the authorized scope, never inherited.
#[derive(Default)]
pub struct DefaultsContext {
    pub skills: HashMap<String, bool>,
    pub mcp: HashMap<String, bool>,
    pub permission_ceiling: Ruleset,
}

pub struct EffectiveDefaults {
    pub model: Option<AgentModelReference>,
    pub skills: Vec<String>,
    pub mcp: Vec<String>,
    pub system_prompt: Option<String>,
    permission: Ruleset,
    permission_ceiling: Ruleset,
}

impl EffectiveDefaults {
    /// Evaluate both policies separately: a default can never override the ceiling.
    pub fn permission_action(&self, permission: &str, pattern: &str) -> Action {
        self.permission_action_with_aliases(permission, &[], pattern)
    }

    pub(crate) fn permission_action_with_aliases(
        &self,
        permission: &str,
        aliases: &[String],
        pattern: &str,
    ) -> Action {
        // Reuse the registered tool identities while retaining rule ordering.
        let evaluate = |rules: &Ruleset| {
            let rules = rules
                .iter()
                .cloned()
                .map(|mut rule| {
                    if aliases.contains(&rule.permission) {
                        rule.permission = permission.to_string();
                    }
                    rule
                })
                .collect::<Ruleset>();
            next::evaluate(permission, pattern, &[rules]).action
        };
        let selected = evaluate(&self.permission);
        let ceiling = evaluate(&self.permission_ceiling);
        match (selected, ceiling) {
            (Action::Deny, _) | (_, Action::Deny) => Action::Deny,
            (Action::Ask, _) | (_, Action::Ask) => Action::Ask,
            (Action::Allow, Action::Allow) => Action::Allow,
        }
    }
}

fn select<T: Clone>(
    explicit: &Option<T>,
    fixed: &AgentDefault<T>,
    inherited: &Option<T>,
) -> Option<T> {
    explicit.clone().or_else(|| match fixed {
        AgentDefault::Fixed(value) => Some(value.clone()),
        AgentDefault::Inherit => inherited.clone(),
    })
}

fn validate_ids(ids: &[String], catalog: &HashMap<String, bool>, kind: &str) -> Result<()> {
    for id in ids {
        match catalog.get(id) {
            Some(true) if !id.trim().is_empty() => {}
            Some(false) => bail!("{kind} capability is disabled"),
            _ => bail!("unknown or out-of-scope {kind} capability"),
        }
    }
    Ok(())
}

fn permission_rules(value: &Value) -> Result<Ruleset> {
    fn action(value: &Value) -> bool {
        matches!(value.as_str(), Some("allow" | "ask" | "deny"))
    }
    let valid = match value {
        Value::Null => true,
        Value::Object(entries) => entries.values().all(|value| {
            action(value) || value.as_object().is_some_and(|patterns| patterns.values().all(action))
        }),
        _ => false,
    };
    if !valid {
        bail!("invalid agent permission rules");
    }
    Ok(next::from_config(value))
}

pub fn resolve(
    agent: &AgentDefinitionConfig,
    explicit: &DefaultSelection,
    inherited: &DefaultSelection,
    context: &DefaultsContext,
) -> Result<EffectiveDefaults> {
    if !agent.enabled {
        bail!("agent is disabled");
    }
    let mut model = select(&explicit.model, &agent.defaults.model, &inherited.model);
    if agent.force_model_selection {
        if !agent.has_complete_model_binding() {
            bail!("forced agent model binding is incomplete");
        }
        let forced =
            AgentModelReference { provider: agent.provider.clone(), model: agent.model.clone() };
        if explicit.model.as_ref().is_some_and(|model| model != &forced) {
            bail!("explicit model conflicts with forced agent model binding");
        }
        model = Some(forced);
    }
    if model
        .as_ref()
        .is_some_and(|model| model.provider.trim().is_empty() || model.model.trim().is_empty())
    {
        bail!("agent model reference is incomplete");
    }
    let skills =
        select(&explicit.skills, &agent.defaults.skills, &inherited.skills).unwrap_or_default();
    let mcp = select(&explicit.mcp, &agent.defaults.mcp, &inherited.mcp).unwrap_or_default();
    validate_ids(&skills, &context.skills, "skill")?;
    if agent.skill_policy == vw_config_types::agent::DelegateCapabilityPolicy::Strict
        && skills.iter().any(|id| !agent.allowed_skills.contains(id))
    {
        bail!("default skill is outside the strict agent skill boundary");
    }

    validate_ids(&mcp, &context.mcp, "MCP")?;
    let permission =
        select(&explicit.permission, &agent.defaults.permission, &inherited.permission)
            .unwrap_or(Value::Null);
    Ok(EffectiveDefaults {
        model,
        skills,
        mcp,
        system_prompt: agent.system_prompt.clone(),
        permission: permission_rules(&permission)?,
        permission_ceiling: context.permission_ceiling.clone(),
    })
}
