//! 委托能力策略归一化可信运行时输入，不从模型文本解析能力或权限。

use std::collections::{BTreeMap, BTreeSet};
use config_agent::DelegateCapabilityPolicy;

/// 能力策略的解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityPolicyResolution {
    /// 模型实际可见的规范能力 ID。
    pub visible_ids: BTreeSet<String>,
    /// 需要作为协议默认项注入的规范能力 ID。
    pub default_injected_ids: BTreeSet<String>,
}

impl CapabilityPolicyResolution {
    #[cfg(test)]
    pub fn new<V, D, VS, DS>(visible_ids: V, default_injected_ids: D) -> Self
    where
        V: IntoIterator<Item = VS>,
        D: IntoIterator<Item = DS>,
        VS: AsRef<str>,
        DS: AsRef<str>,
    {
        Self {
            visible_ids: visible_ids
                .into_iter()
                .filter_map(|id| normalize_capability_id(id.as_ref()))
                .collect(),
            default_injected_ids: default_injected_ids
                .into_iter()
                .filter_map(|id| normalize_capability_id(id.as_ref()))
                .collect(),
        }
    }
}

/// 归一化能力 ID，忽略空白和大小写差异。
pub fn normalize_capability_id(id: &str) -> Option<String> {
    let id = id.trim();
    (!id.is_empty()).then(|| id.to_lowercase())
}

/// 用稳定技能 ID 解析委托技能能力。
pub fn resolve_skill_capabilities<A, D, U, H, AS, DS, US, HS>(
    available: A,
    defaults: D,
    trusted_user_selection: U,
    policy: DelegateCapabilityPolicy,
    hard_disabled: H,
) -> CapabilityPolicyResolution
where
    A: IntoIterator<Item = AS>,
    D: IntoIterator<Item = DS>,
    U: IntoIterator<Item = US>,
    H: IntoIterator<Item = HS>,
    AS: AsRef<str>,
    DS: AsRef<str>,
    US: AsRef<str>,
    HS: AsRef<str>,
{
    let aliases = available
        .into_iter()
        .filter_map(|id| normalize_capability_id(id.as_ref()))
        .map(|id| (id.clone(), id))
        .collect();

    resolve_capabilities(aliases, defaults, trusted_user_selection, policy, hard_disabled)
}

/// Resolve the access boundary independently from the first-turn default injection.
pub fn resolve_skill_defaults(
    available: &[String],
    boundary: &[String],
    defaults: &[String],
    policy: DelegateCapabilityPolicy,
) -> CapabilityPolicyResolution {
    let mut resolution = resolve_skill_capabilities(
        available,
        boundary,
        std::iter::empty::<&str>(),
        policy,
        std::iter::empty::<&str>(),
    );
    resolution.default_injected_ids = defaults
        .iter()
        .filter_map(|id| normalize_capability_id(id))
        .filter(|id| resolution.visible_ids.contains(id))
        .collect();
    resolution
}

fn resolve_capabilities<D, U, H, DS, US, HS>(
    aliases: BTreeMap<String, String>,
    defaults: D,
    _trusted_user_selection: U,
    policy: DelegateCapabilityPolicy,
    hard_disabled: H,
) -> CapabilityPolicyResolution
where
    D: IntoIterator<Item = DS>,
    U: IntoIterator<Item = US>,
    H: IntoIterator<Item = HS>,
    DS: AsRef<str>,
    US: AsRef<str>,
    HS: AsRef<str>,
{
    let defaults = resolve_selected_ids(defaults, &aliases);
    let hard_disabled = resolve_selected_ids(hard_disabled, &aliases);
    let mut visible_ids = match policy {
        DelegateCapabilityPolicy::Strict => defaults.clone(),
        DelegateCapabilityPolicy::Unrestricted => aliases.values().cloned().collect(),
    };
    visible_ids.retain(|id| !hard_disabled.contains(id));

    let mut default_injected_ids = defaults;
    default_injected_ids.retain(|id| !hard_disabled.contains(id));

    CapabilityPolicyResolution { visible_ids, default_injected_ids }
}

fn resolve_selected_ids<I, S>(selection: I, aliases: &BTreeMap<String, String>) -> BTreeSet<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    selection
        .into_iter()
        .filter_map(|id| normalize_capability_id(id.as_ref()))
        .filter_map(|id| aliases.get(&id).cloned())
        .collect()
}

#[cfg(test)]
#[path = "capability_policy_tests.rs"]
mod tests;

/// Resolved from trusted project configuration, never from model-generated options.
pub struct NativeAgentDefaults {
    pub effective: std::sync::Arc<agent_defaults::EffectiveDefaults>,
    pub skills_fixed: bool,
    pub allowed_skills: Vec<String>,
    pub skill_policy: DelegateCapabilityPolicy,
    pub mcp_fixed: bool,
    pub permission_fixed: bool,
}

pub async fn native_agent_defaults<H: crate::admission_host::AdmissionHost>(
    config: &agent_config::runtime::Config,
    key: &str,
    explicit_model: Option<&str>,
    workspace: &std::path::Path,
) -> anyhow::Result<Option<NativeAgentDefaults>> {
    use agent_defaults::{DefaultSelection, DefaultsContext, resolve};
    use config_agent_profile::{AgentDefault, AgentModelReference, AgentRuntime};
    let agents = config_agent::merged_agent_configs(&config.agents);
    let agent = agents.get(key).ok_or_else(|| anyhow::anyhow!("unknown agent: {key}"))?;
    if !matches!(agent.runtime, AgentRuntime::Native) {
        return Ok(None);
    }
    let skills_fixed = matches!(agent.defaults.skills, AgentDefault::Fixed(_));
    let mcp_fixed = matches!(agent.defaults.mcp, AgentDefault::Fixed(_));
    let permission_fixed = matches!(agent.defaults.permission, AgentDefault::Fixed(_));
    if !(skills_fixed
        || !agent.allowed_skills.is_empty()
        || agent.skill_policy == DelegateCapabilityPolicy::Strict
        || mcp_fixed
        || permission_fixed
        || matches!(agent.defaults.model, AgentDefault::Fixed(_)))
    {
        return Ok(None);
    }
    let model = explicit_model
        .filter(|value| !value.trim().is_empty() && !value.eq_ignore_ascii_case("auto"))
        .map(|value| {
            let (provider, model) =
                value.split_once('/').ok_or_else(|| anyhow::anyhow!("invalid model reference"))?;
            Ok::<_, anyhow::Error>(AgentModelReference {
                provider: provider.into(),
                model: model.into(),
            })
        })
        .transpose()?;
    let mut context = DefaultsContext {
        permission_ceiling: H::permission_ceiling(key).await?,
        ..Default::default()
    };
    context.skills = H::skills(workspace, config);
    if mcp_fixed {
        context.mcp = H::mcp(workspace, &AgentRuntime::Native).await?;
    }
    let inherited =
        DefaultSelection { skills: Some(agent.allowed_skills.clone()), ..Default::default() };
    let effective =
        resolve(agent, &DefaultSelection { model, ..Default::default() }, &inherited, &context)?;
    for skill in &effective.skills {
        if effective.permission_action("skill", skill) == permission_runtime::next::Action::Deny {
            anyhow::bail!("default skill is denied by permission policy");
        }
    }
    Ok(Some(NativeAgentDefaults {
        effective: std::sync::Arc::new(effective),
        skills_fixed,
        allowed_skills: agent.allowed_skills.clone(),
        skill_policy: agent.skill_policy,
        mcp_fixed,
        permission_fixed,
    }))
}
