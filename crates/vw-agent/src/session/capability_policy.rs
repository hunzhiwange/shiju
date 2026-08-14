//! 委托代理能力的纯策略解析。
//!
//! 该模块只把可信运行时输入归一化为实际可见能力和默认注入能力；不读取
//! Gateway、工具调用参数或模型生成的文本。

use crate::app::agent::tools::ToolSpec;
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

/// 用真实工具规格的稳定 ID 和别名解析委托工具能力。
pub(crate) fn resolve_tool_capabilities<D, U, H, DS, US, HS>(
    available: &[ToolSpec],
    defaults: D,
    trusted_user_selection: U,
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
    let mut aliases = BTreeMap::new();
    for spec in available {
        let Some(id) = normalize_capability_id(&spec.id) else {
            continue;
        };
        aliases.insert(id.clone(), id.clone());
        for alias in &spec.aliases {
            if let Some(alias) = normalize_capability_id(alias) {
                aliases.entry(alias).or_insert_with(|| id.clone());
            }
        }
    }

    resolve_capabilities(aliases, defaults, trusted_user_selection, policy, hard_disabled)
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

fn resolve_capabilities<D, U, H, DS, US, HS>(
    aliases: BTreeMap<String, String>,
    defaults: D,
    trusted_user_selection: U,
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
    let user_selection = resolve_selected_ids(trusted_user_selection, &aliases);
    let hard_disabled = resolve_selected_ids(hard_disabled, &aliases);
    let mut visible_ids = match policy {
        DelegateCapabilityPolicy::Strict => defaults.clone(),
        DelegateCapabilityPolicy::UserOverride => {
            defaults.union(&user_selection).cloned().collect()
        }
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
