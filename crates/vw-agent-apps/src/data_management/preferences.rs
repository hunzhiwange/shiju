//! Explicit CC device-settings import. Never discover or open the user's HOME.
//! SQLite settings/common snippets are not device preferences and are not accepted.
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_api_types::agent_apps::{AppHostPreferences, AppId, AppUserPreferences};

use crate::apps::preferences::{HostPlatform, default_user_preferences, normalize_preferences};
use super::{preview::{Decision, Difference, PreviewError}, source::SourceScope};

/// Host-private preview; never serialize source values or target preimages.
pub struct PreferencesPreview {
    scope: SourceScope,
    source_digest: [u8; 32],
    before: Option<String>,
    after: AppUserPreferences,
    difference: Difference,
}

impl PreferencesPreview {
    pub fn difference(&self) -> Difference { self.difference }

    /// Recheck the exact source and target under the owner's commit boundary.
    /// The caller must independently authorize migration and capture a rollback
    /// preimage before applying the returned value. None means no write.
    pub fn resolve(
        &self, scope: &SourceScope, source: &[u8], current: Option<&str>,
        decision: Option<Decision>,
    ) -> Result<Option<&AppUserPreferences>, PreviewError> {
        if scope != &self.scope { return Err(PreviewError::Forbidden); }
        let digest: [u8; 32] = Sha256::digest(source).into();
        if digest != self.source_digest || current != self.before.as_deref() {
            return Err(PreviewError::Stale);
        }
        if decision == Some(Decision::Skip) || self.difference == Difference::Identical {
            return Ok(None);
        }
        if self.difference == Difference::Different && decision != Some(Decision::Replace) {
            return Err(PreviewError::UnresolvedConflict);
        }
        Ok(Some(&self.after))
    }
}

/// Input is an explicitly selected CC settings.json, not an arbitrary SQL row.
/// Only appOrder and visibleApps have equivalent target semantics. All other
/// fields (including paths, launch, proxy and permissions) are ignored entirely.
pub fn prepare(
    scope: &SourceScope, source: &[u8], existing: Option<&str>,
) -> Result<PreferencesPreview, PreviewError> {
    if source.len() > 1024 * 1024 { return Err(PreviewError::InvalidCandidate); }
    let value: Value = serde_json::from_slice(source).map_err(|_| PreviewError::InvalidCandidate)?;
    let fields = value.as_object().ok_or(PreviewError::InvalidCandidate)?;
    let previous: AppUserPreferences = existing.map(serde_json::from_str).transpose()
        .map_err(|_| PreviewError::InvalidCandidate)?.unwrap_or_else(default_user_preferences);
    let mut user = previous.clone();
    let mut supported = false;
    if let Some(order) = fields.get("appOrder").filter(|v| !v.is_null()) {
        user.app_order = serde_json::from_value(order.clone()).map_err(|_| PreviewError::InvalidCandidate)?;
        // Unknown application identifiers have no target semantics; reject rather
        // than silently dropping malformed/path-bearing entries.
        for id in &user.app_order {
            serde_json::from_value::<AppId>(Value::String(id.clone()))
                .map_err(|_| PreviewError::InvalidCandidate)?;
        }
        supported = true;
    }
    if let Some(visible) = fields.get("visibleApps").filter(|v| !v.is_null()) {
        let visible = visible.as_object().ok_or(PreviewError::InvalidCandidate)?;
        let mut seen = std::collections::HashSet::new();
        for (key, value) in visible {
            let key = match key.as_str() { "claudeDesktop" | "claude_desktop" => "claude-desktop", other => other };
            let app: AppId = serde_json::from_value(Value::String(key.into()))
                .map_err(|_| PreviewError::InvalidCandidate)?;
            if !seen.insert(app) { return Err(PreviewError::InvalidCandidate); }
            let enabled = value.as_bool().ok_or(PreviewError::InvalidCandidate)?;
            user.visible_apps.retain(|id| *id != app);
            if enabled { user.visible_apps.push(app); }
        }
        supported = true;
    }
    if !supported { return Err(PreviewError::UnsupportedCategory); }
    let host = AppHostPreferences::default();
    let after = normalize_preferences(user, host.clone(), &host, HostPlatform::MacOs)
        .map_err(|_| PreviewError::InvalidCandidate)?.user;
    let before_normalized = normalize_preferences(previous, host.clone(), &host, HostPlatform::MacOs)
        .map_err(|_| PreviewError::InvalidCandidate)?.user;
    let difference = if existing.is_none() { Difference::Added }
        else if after == before_normalized { Difference::Identical }
        else { Difference::Different };
    Ok(PreferencesPreview { scope: scope.clone(), source_digest: Sha256::digest(source).into(),
        before: existing.map(str::to_owned), after, difference })
}
