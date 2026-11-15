//! Reviewed Pi thinking levels and their exact catalog and protocol bindings.

use super::preset_pi::{PiPresetCompatValue, PiPresetProtocol};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::LazyLock};

pub(super) const FORCE_ADAPTIVE_THINKING: &str = "forceAdaptiveThinking";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PiThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

/// A null value disables a level. A missing key leaves that level to Pi.
pub type PiThinkingLevelMap = BTreeMap<PiThinkingLevel, Option<String>>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    profiles: BTreeMap<String, PiThinkingLevelMap>,
    bindings: Vec<Binding>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Binding {
    catalog_key: String,
    api: PiPresetProtocol,
    profile_id: String,
    model_compat: Option<BindingCompat>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BindingCompat {
    force_adaptive_thinking: bool,
}

static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
    let catalog: Catalog = serde_json::from_str(include_str!("presets/pi_thinking.json"))
        .expect("valid built-in Pi thinking profiles");
    assert!(catalog.profiles.values().all(|map| !map.is_empty()));
    catalog
});

pub(super) struct ResolvedPiThinkingProfile {
    pub map: PiThinkingLevelMap,
    pub force_adaptive_thinking: bool,
}

pub(super) fn profile_map(profile_id: &str) -> PiThinkingLevelMap {
    CATALOG.profiles.get(profile_id).expect("known built-in Pi thinking profile").clone()
}

pub(super) fn resolve(
    catalog_key: &str,
    api: PiPresetProtocol,
    adaptive_compat: Option<&PiPresetCompatValue>,
) -> Option<ResolvedPiThinkingProfile> {
    let binding = CATALOG
        .bindings
        .iter()
        .find(|binding| binding.catalog_key == catalog_key && binding.api == api)?;
    let force_adaptive_thinking =
        binding.model_compat.as_ref().is_some_and(|compat| compat.force_adaptive_thinking);
    // The reviewed binding supplies the flag unless the deployment explicitly conflicts.
    if force_adaptive_thinking
        && adaptive_compat.is_some_and(|value| *value != PiPresetCompatValue::Flag(true))
    {
        return None;
    }
    Some(ResolvedPiThinkingProfile {
        map: profile_map(&binding.profile_id),
        force_adaptive_thinking,
    })
}
