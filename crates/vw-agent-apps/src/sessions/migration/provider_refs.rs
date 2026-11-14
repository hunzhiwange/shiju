//! Executor-only trusted legacy template migration. No provider store or completion marker.
use crate::providers::live::{
    commit::{AppState, PendingTarget, ProviderTransactions},
    record_publication::{ProviderRecordChange, ProviderRecordParticipant},
    stage::{FileChange, WholeFile},
    write_guard::{AppWriteGuard, FilePrivacy, WriteError},
};
use std::{collections::BTreeSet, ffi::OsStr};
use toml_edit::{DocumentMut, Item};
use vw_api_types::{agent_apps::AppId, id::ProjectScope};

const LEGACY_IDS: &[&str] = &[
    "ccswitch",
    "aicodemirror",
    "aicoding",
    "aigocode",
    "aihubmix",
    "ark_agentplan",
    "bailian",
    "bailing",
    "byteplus",
    "claudecn",
    "compshare",
    "compshare_coding",
    "crazyrouter",
    "ctok",
    "cubence",
    "deepseek",
    "dmxapi",
    "doubaoseed",
    "eflowcode",
    "etok",
    "kimi",
    "lemondata",
    "longcat",
    "micu",
    "minimax",
    "minimax_en",
    "modelscope",
    "novita",
    "nvidia",
    "openrouter",
    "packycode",
    "patewayai",
    "pipellm",
    "qianfan_coding",
    "relaxycode",
    "rightcode",
    "runapi",
    "shengsuanyun",
    "siliconflow",
    "siliconflow_en",
    "sssaicode",
    "stepfun",
    "stepfun_en",
    "therouter",
    "xiaomi_mimo",
    "xiaomi_mimo_token_plan",
    "zhipu_glm",
    "zhipu_glm_en",
];

fn known(id: &str) -> bool {
    LEGACY_IDS.iter().any(|known| known.eq_ignore_ascii_case(id))
}
fn selector(item: Option<&Item>) -> Option<&str> {
    item.and_then(Item::as_str).map(str::trim).filter(|id| !id.is_empty())
}

/// Collect from a saved provider loaded by the authorized configuration owner,
/// including templates already normalized to custom. Never accept request IDs as
/// evidence of trust; callers union these sets across scoped nonofficial rows.
pub fn collect_source_model_provider_ids(
    provider_id: &str,
    config: &str,
    official_or_oauth: bool,
) -> Result<BTreeSet<String>, WriteError> {
    let mut ids = BTreeSet::new();
    if official_or_oauth {
        return Ok(ids);
    }
    if known(provider_id.trim()) {
        ids.insert(provider_id.trim().to_owned());
    }
    if config.trim().is_empty() {
        return Ok(ids);
    }
    let doc = config.parse::<DocumentMut>().map_err(|_| WriteError::InvalidResource)?;
    let mut consider = |id: Option<&str>| {
        if let Some(id) = id
            && known(id)
            && doc
                .get("model_providers")
                .and_then(|p| p.get(id))
                .is_some_and(|p| p.as_table_like().is_some())
        {
            ids.insert(id.to_owned());
        }
    };
    consider(selector(doc.get("model_provider")));
    if let Some(profiles) = doc.get("profiles").and_then(Item::as_table_like) {
        for (_, profile) in profiles.iter() {
            consider(selector(profile.get("model_provider")));
        }
    }
    if let Some(id @ ("custom" | "ccswitch")) = selector(doc.get("model_provider"))
        && let Some(name) = doc
            .get("model_providers")
            .and_then(|p| p.get(id))
            .and_then(|p| p.get("name"))
            .and_then(Item::as_str)
            .map(str::trim)
    {
        let normalized = match name {
            "E-FlowCode" => Some("eflowcode"),
            "PIPELLM" => Some("pipellm"),
            name => LEGACY_IDS.iter().copied().find(|id| id.eq_ignore_ascii_case(name)),
        };
        if let Some(id) = normalized {
            ids.insert(id.to_owned());
        }
    }
    Ok(ids)
}

/// Contains native configuration and possibly credentials. Never serialize or log.
/// Source IDs are suitable for the scoped rollout/state_db migration primitives;
/// they do not authorize filesystem access or grant lifecycle exclusion.
pub struct TemplateMigration {
    before: String,
    after: String,
    source_ids: BTreeSet<String>,
}
impl TemplateMigration {
    pub fn config(&self) -> &str {
        &self.after
    }
    pub fn source_ids(&self) -> &BTreeSet<String> {
        &self.source_ids
    }

    /// The authorized executor retains native-writer lifecycle exclusion throughout
    /// configuration publication and subsequent history maintenance. This guard is
    /// only for this publication; drop it before rollout/state_db acquire their own.
    /// Saved template changes must be supplied by the existing record owner, with
    /// its conditional participant. No independent provider persistence is created.
    #[allow(clippy::too_many_arguments)]
    pub fn publish(
        &self,
        transactions: &ProviderTransactions,
        scope: &ProjectScope,
        guard: &mut AppWriteGuard,
        expected: &AppState,
        record: Option<(&ProviderRecordChange, &dyn ProviderRecordParticipant)>,
        validate: impl Fn() -> Result<(), WriteError>,
    ) -> Result<(), WriteError> {
        if guard.app() != AppId::Codex {
            return Err(WriteError::InvalidResource);
        }
        let patch =
            WholeFile::new(Some(self.before.as_bytes()), Some(self.after.as_bytes().to_vec()));
        let changes = [FileChange {
            name: OsStr::new("config.toml"),
            privacy: FilePrivacy::Private,
            patch: &patch,
        }];
        match record {
            Some((change, participant)) => transactions.commit_project_record(
                scope,
                guard,
                expected,
                &changes,
                PendingTarget::default(),
                change,
                participant,
                validate,
            ),
            None => transactions.commit_project_checked(
                scope,
                guard,
                expected,
                false,
                &changes,
                PendingTarget::default(),
                validate,
            ),
        }
    }
}

/// Only executor-loaded nonofficial templates qualify. Unknown custom selectors
/// survive unchanged. Conflicting legacy tables are rejected instead of silently
/// routing profiles to a different endpoint. OAuth/official rows never qualify.
pub fn plan(
    config: &str,
    official_or_oauth: bool,
) -> Result<Option<TemplateMigration>, WriteError> {
    if official_or_oauth || config.trim().is_empty() {
        return Ok(None);
    }
    let mut doc = config.parse::<DocumentMut>().map_err(|_| WriteError::InvalidResource)?;
    let mut ids = BTreeSet::new();
    let mut consider = |id: Option<&str>| {
        if let Some(id) = id
            && known(id)
            && doc
                .get("model_providers")
                .and_then(|p| p.get(id))
                .is_some_and(|p| p.as_table_like().is_some())
        {
            ids.insert(id.to_owned());
        }
    };
    consider(selector(doc.get("model_provider")));
    if let Some(profiles) = doc.get("profiles").and_then(Item::as_table_like) {
        for (_, profile) in profiles.iter() {
            consider(selector(profile.get("model_provider")));
        }
    }
    if ids.is_empty() {
        return Ok(None);
    }
    let providers = doc
        .get_mut("model_providers")
        .and_then(Item::as_table_like_mut)
        .ok_or(WriteError::InvalidResource)?;
    let first = ids.iter().next().ok_or(WriteError::InvalidResource)?;
    let table = providers.get(first).cloned().ok_or(WriteError::InvalidResource)?;
    // Compare values rather than formatting, preserving the selected table bytes.
    let canonical = |item: &Item| -> Result<toml::Value, WriteError> {
        let mut d = DocumentMut::new();
        d.insert("provider", item.clone());
        toml::from_str(&d.to_string()).map_err(|_| WriteError::InvalidResource)
    };
    let expected = canonical(&table)?;
    for id in &ids {
        if canonical(providers.get(id).ok_or(WriteError::InvalidResource)?)? != expected {
            return Err(WriteError::Conflict);
        }
    }
    if let Some(custom) = providers.get("custom") {
        if canonical(custom)? != expected {
            return Err(WriteError::Conflict);
        }
    } else {
        providers.insert("custom", table);
    }
    for id in &ids {
        providers.remove(id);
    }
    if selector(doc.get("model_provider")).is_some_and(|id| ids.contains(id)) {
        doc["model_provider"] = toml_edit::value("custom");
    }
    if let Some(profiles) = doc.get_mut("profiles").and_then(Item::as_table_like_mut) {
        for (_, profile) in profiles.iter_mut() {
            if selector(profile.get("model_provider")).is_some_and(|id| ids.contains(id)) {
                let profile = profile.as_table_like_mut().ok_or(WriteError::InvalidResource)?;
                profile.insert("model_provider", toml_edit::value("custom"));
            }
        }
    }
    Ok(Some(TemplateMigration {
        before: config.to_owned(),
        after: doc.to_string(),
        source_ids: ids,
    }))
}
