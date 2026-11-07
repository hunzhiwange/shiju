use std::collections::BTreeSet;
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        env_clean::{EnvCleanRequest, EnvCleanSelection},
        env_diagnose::{EnvDiagnostic, EnvSource},
        env_restore::{EnvRestoreItem, EnvRestoreRequest},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Clean,
    Restore,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operation {
    Diagnose,
    Write(Action),
}

#[derive(Clone, Default)]
pub(crate) struct Form {
    pub entries: Vec<EnvDiagnostic>,
    pub selected: BTreeSet<usize>,
    pub backup_id: String,
    pub restored: Vec<EnvRestoreItem>,
    pub confirmation: Option<Action>,
    pub busy: Option<Operation>,
    pub error: Option<&'static str>,
}

pub(crate) const MAX_SELECTIONS: usize = 256;
pub(crate) const SCROLL_ID: &str = "provider-environment-scroll";

pub(crate) fn available(app: AppId) -> bool {
    cfg!(target_os = "macos")
        && matches!(app, AppId::Claude | AppId::Codex | AppId::Gemini | AppId::GrokBuild)
}

pub(crate) fn cleanup_selection(entry: &EnvDiagnostic) -> Option<&EnvCleanSelection> {
    if entry.cleanup_available && matches!(entry.source, EnvSource::UserShell { .. }) {
        entry.cleanup.as_ref()
    } else {
        None
    }
}

impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderEnvironmentForm([redacted])")
    }
}

impl Form {
    pub fn record_backup(&mut self, backup_id: String) {
        self.backup_id = backup_id;
        self.restored.clear();
    }

    pub fn writing(&self) -> bool {
        matches!(self.busy, Some(Operation::Write(_)))
    }

    pub fn toggle(&mut self, index: usize, selected: bool) {
        if self.entries.get(index).and_then(cleanup_selection).is_some() {
            if selected {
                self.selected.insert(index);
            } else {
                self.selected.remove(&index);
            }
        }
    }

    pub fn can_clean(&self) -> bool {
        (1..=MAX_SELECTIONS).contains(&self.selected.len())
    }

    pub fn clean_request(&self, host: &str) -> Option<EnvCleanRequest> {
        if !self.can_clean() {
            return None;
        }
        let selections = self
            .selected
            .iter()
            .map(|index| self.entries.get(*index).and_then(cleanup_selection).cloned())
            .collect::<Option<Vec<_>>>()?;
        Some(EnvCleanRequest { target_host_id: host.into(), selections })
    }
}

pub(crate) fn restore_request(host: &str, backup_id: &str) -> Option<EnvRestoreRequest> {
    let backup_id = backup_id.trim();
    if backup_id.is_empty() {
        return None;
    }
    Some(EnvRestoreRequest { target_host_id: host.into(), backup_id: backup_id.into() })
}
