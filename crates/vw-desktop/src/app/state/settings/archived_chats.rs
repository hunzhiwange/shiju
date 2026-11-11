//! Native archived metadata queries, independent of UI snapshot archive IDs.
//! Info has no local/cloud source field: the view must support only all chats
//! and disable source choices instead of guessing from gateway addresses.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use vw_shared::session::info::Info;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ArchivedChatsSort {
    #[default]
    Updated,
    Created,
    Title,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArchivedChatsDeleteConfirmation {
    Chat(String),
    Project(String),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ArchivedChatsState {
    pub(crate) sessions: Vec<Info>,
    pub(crate) project_names: HashMap<String, String>,
    pub(crate) search: String,
    pub(crate) selected_project_id: Option<String>,
    pub(crate) sort: ArchivedChatsSort,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) request_generation: u64,
    pub(crate) operating_ids: HashSet<String>,
    pub(crate) delete_confirmation: Option<ArchivedChatsDeleteConfirmation>,
    pub(crate) identity: Option<ArchivedChatsIdentity>,
    pub(crate) menu: Option<ArchivedChatsMenu>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchivedChatsMenu {
    Type,
    Project,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ArchivedChatsIdentity {
    pub(crate) gateway: String,
    pub(crate) tenant: vw_gateway_client::vw_api_types::id::TenantScope,
    pub(crate) auth_fingerprint: [u8; 32],
}

impl std::fmt::Debug for ArchivedChatsIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArchivedChatsIdentity")
            .field("gateway", &self.gateway)
            .field("tenant", &self.tenant)
            .finish_non_exhaustive()
    }
}

impl ArchivedChatsIdentity {
    pub(crate) fn from_endpoint(endpoint: &vw_gateway_client::GatewayEndpoint) -> Self {
        use sha2::{Digest, Sha256};
        let auth = endpoint.auth.as_ref().and_then(|auth| auth.skey.as_deref()).unwrap_or("");
        Self {
            gateway: endpoint.base_url(),
            tenant: endpoint.tenant_scope().clone(),
            auth_fingerprint: Sha256::digest(auth.as_bytes()).into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArchivedChatsProject {
    pub(crate) id: String,
    pub(crate) name: String,
}

pub(crate) struct ArchivedChatsGroup<'a> {
    pub(crate) project: ArchivedChatsProject,
    pub(crate) rows: Vec<&'a Info>,
}

impl ArchivedChatsState {
    pub(crate) fn groups(&self) -> Vec<ArchivedChatsGroup<'_>> {
        let search = self.search.trim().to_lowercase();
        let mut rows = self.archived_rows();
        rows.retain(|row| {
            self.selected_project_id.as_ref().is_none_or(|id| id == &row.project_id)
                && (search.is_empty() || row.title.to_lowercase().contains(&search))
        });
        rows.sort_by(|left, right| {
            let order = match self.sort {
                ArchivedChatsSort::Updated => right.time.updated.cmp(&left.time.updated),
                ArchivedChatsSort::Created => right.time.created.cmp(&left.time.created),
                ArchivedChatsSort::Title => {
                    left.title.to_lowercase().cmp(&right.title.to_lowercase())
                }
            };
            order.then_with(|| left.id.cmp(&right.id))
        });
        let mut grouped: HashMap<&str, Vec<&Info>> = HashMap::new();
        for row in rows {
            grouped.entry(&row.project_id).or_default().push(row);
        }
        self.project_options()
            .into_iter()
            .filter_map(|project| {
                grouped.remove(project.id.as_str()).map(|rows| ArchivedChatsGroup { project, rows })
            })
            .collect()
    }

    pub(crate) fn project_options(&self) -> Vec<ArchivedChatsProject> {
        let mut seen = HashSet::new();
        let mut projects = Vec::new();
        for row in self.archived_rows() {
            if !seen.insert(&row.project_id) {
                continue;
            }
            let name = self
                .project_names
                .get(&row.project_id)
                .map(String::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .or_else(|| {
                    Path::new(&row.directory)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .filter(|name| !name.trim().is_empty())
                })
                .unwrap_or(&row.project_id)
                .to_owned();
            projects.push(ArchivedChatsProject { id: row.project_id.clone(), name });
        }
        projects.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
        projects
    }

    fn archived_rows(&self) -> Vec<&Info> {
        let mut rows: Vec<_> = self
            .sessions
            .iter()
            .filter(|row| row.time.archived.is_some() && row.is_user_navigable())
            .collect();
        // Prefer the newest metadata when a response contains the same ID more than once.
        rows.sort_by(|left, right| {
            right
                .time
                .updated
                .cmp(&left.time.updated)
                .then_with(|| left.id.cmp(&right.id))
                .then_with(|| left.project_id.cmp(&right.project_id))
                .then_with(|| left.directory.cmp(&right.directory))
                .then_with(|| left.title.cmp(&right.title))
        });
        let mut seen = HashSet::new();
        rows.retain(|row| seen.insert(&row.id));
        rows
    }
}

#[cfg(test)]
#[path = "archived_chats_tests.rs"]
mod tests;
