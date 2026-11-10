//! 原生智能体与委托代理能力解析。
//!
//! 委托策略归一化可信运行时输入；原生默认入口还读取当前租户、项目配置与
//! 技能/MCP 目录，不从模型文本解析能力或权限。

use std::collections::{BTreeMap, BTreeSet};
use vw_config_types::agent::DelegateCapabilityPolicy;

/// 能力策略的解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapabilityPolicyResolution {
    /// 模型实际可见的规范能力 ID。
    pub(crate) visible_ids: BTreeSet<String>,
    /// 需要作为协议默认项注入的规范能力 ID。
    pub(crate) default_injected_ids: BTreeSet<String>,
}

impl CapabilityPolicyResolution {
    #[cfg(test)]
    pub(crate) fn new<V, D, VS, DS>(visible_ids: V, default_injected_ids: D) -> Self
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
pub(crate) fn normalize_capability_id(id: &str) -> Option<String> {
    let id = id.trim();
    (!id.is_empty()).then(|| id.to_lowercase())
}

/// 用稳定技能 ID 解析委托技能能力。
pub(crate) fn resolve_skill_capabilities<A, D, U, H, AS, DS, US, HS>(
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
pub(crate) fn resolve_skill_defaults(
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

/// Resolved from trusted project configuration, never from model-generated options.
pub(crate) struct NativeAgentDefaults {
    pub(crate) effective: std::sync::Arc<crate::agent::defaults::EffectiveDefaults>,
    pub(crate) skills_fixed: bool,
    pub(crate) allowed_skills: Vec<String>,
    pub(crate) skill_policy: DelegateCapabilityPolicy,
    pub(crate) mcp_fixed: bool,
    pub(crate) permission_fixed: bool,
}

pub(crate) async fn native_agent_defaults(
    config: &crate::config::Config,
    key: &str,
    explicit_model: Option<&str>,
    workspace: &std::path::Path,
) -> anyhow::Result<Option<NativeAgentDefaults>> {
    use crate::agent::defaults::{DefaultSelection, DefaultsContext, resolve};
    use vw_config_types::agent::{AgentDefault, AgentModelReference, AgentRuntime};
    let agents = vw_config_types::agent::merged_agent_configs(&config.agents);
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
        permission_ceiling: crate::agent::native_permission_ceiling(key)
            .await
            .map_err(anyhow::Error::msg)?,
        ..Default::default()
    };
    context.skills = crate::skills::load_skills_with_config(workspace, config)
        .into_iter()
        .map(|skill| (skill.invocation_name().to_string(), true))
        .collect();
    if mcp_fixed {
        let mcp =
            crate::tools::mcp_common::load_resolved_runtime_mcp_config_from(workspace).await?;
        context.mcp = mcp
            .mcp_servers
            .iter()
            .map(|server| {
                (
                    crate::tools::mcp_common::server_name(server).to_string(),
                    crate::tools::mcp_common::management_for_server(&mcp, server).enabled,
                )
            })
            .collect();
    }
    let inherited =
        DefaultSelection { skills: Some(agent.allowed_skills.clone()), ..Default::default() };
    let effective =
        resolve(agent, &DefaultSelection { model, ..Default::default() }, &inherited, &context)?;
    for skill in &effective.skills {
        if effective.permission_action("skill", skill) == crate::permission::next::Action::Deny {
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
