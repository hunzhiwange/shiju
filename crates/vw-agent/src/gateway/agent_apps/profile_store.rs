//! Tenant-shared entities and host/group selection in one atomic settings record.
//! Callers resolve project identity and authorize the execution host before entry.

use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use vw_api_types::id::{ProjectScope, TenantScope};
use vw_api_types::project_profiles::{ProjectProfile, ProjectProfileId, ProjectProfileScope};

use crate::global_database::GlobalDatabase;
use crate::gateway::settings_store::GatewaySettingsStore;

const NAMESPACE: &str = "project_profiles";
const KEY: &str = "state_v1";

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileState {
    profiles: BTreeMap<String, ProjectProfile>,
    selections: Vec<ProfileSelection>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileSelection {
    pub(crate) target_host_id: String,
    pub(crate) group: ProjectProfileScope,
    pub(crate) project_id: String,
    pub(crate) profile_id: ProjectProfileId,
}

#[derive(Clone)]
pub(crate) struct ProjectProfileStore {
    settings: GatewaySettingsStore,
    execution_host_id: String,
}

impl ProjectProfileStore {
    /// `execution_host_id` must come from executor identity, never a request body.
    pub(crate) fn new(database: GlobalDatabase, execution_host_id: String) -> Result<Self> {
        ensure!(!execution_host_id.trim().is_empty(), "empty execution host id");
        Ok(Self {
            settings: GatewaySettingsStore::new(database)?,
            execution_host_id,
        })
    }

    /// Stable ID ordering is independent of database provider or insertion order.
    pub(crate) fn list(&self, tenant: &TenantScope) -> Result<Vec<ProjectProfile>> {
        Ok(self.read(tenant)?.profiles.into_values().collect())
    }

    pub(crate) fn get(
        &self,
        tenant: &TenantScope,
        id: &ProjectProfileId,
    ) -> Result<Option<ProjectProfile>> {
        Ok(self.read(tenant)?.profiles.remove(&id.0))
    }

    /// None creates a new identity; updates require the exact prior version.
    /// The owner supplies the version token, so clients cannot reset it.
    pub(crate) fn save(
        &self,
        tenant: &TenantScope,
        mut profile: ProjectProfile,
        expected_version: Option<u64>,
    ) -> Result<ProjectProfile> {
        ensure!(!profile.id.0.trim().is_empty(), "empty project profile id");
        profile.name = profile.name.trim().to_owned();
        ensure!(!profile.name.is_empty(), "empty project profile name");
        self.mutate(tenant, |state| {
            match (state.profiles.get(&profile.id.0), expected_version) {
                (None, None) => profile.version = 1,
                (Some(previous), Some(expected)) if previous.version == expected => {
                    profile.version = expected.checked_add(1).context("profile version exhausted")?;
                }
                _ => anyhow::bail!("project profile version conflict"),
            }
            state.profiles.insert(profile.id.0.clone(), profile.clone());
            Ok(profile)
        })
    }

    /// Selection records ownership of a host/group slot, not successful application.
    /// A different project must explicitly release its slot before reassignment.
    pub(crate) fn select(
        &self,
        project: &ProjectScope,
        target_host_id: &str,
        group: ProjectProfileScope,
        id: &ProjectProfileId,
        expected_version: u64,
    ) -> Result<()> {
        ensure!(target_host_id == self.execution_host_id, "profile host requires executor dispatch");
        self.mutate(project.tenant_scope(), |state| {
            let profile = state.profiles.get(&id.0).context("project profile not found")?;
            ensure!(profile.version == expected_version, "project profile version conflict");
            if let Some(selection) = state.selections.iter_mut().find(|selection| {
                selection.target_host_id == target_host_id && selection.group == group
            }) {
                ensure!(selection.project_id == project.project_id().as_ref(), "profile slot owned by another project");
                selection.profile_id = id.clone();
            } else {
                state.selections.push(ProfileSelection {
                    target_host_id: target_host_id.to_owned(),
                    group,
                    project_id: project.project_id().as_ref().to_owned(),
                    profile_id: id.clone(),
                });
            }
            Ok(())
        })
    }

    pub(crate) fn current(
        &self,
        project: &ProjectScope,
        target_host_id: &str,
        group: ProjectProfileScope,
    ) -> Result<Option<ProjectProfileId>> {
        ensure!(target_host_id == self.execution_host_id, "profile host requires executor dispatch");
        let state = self.read(project.tenant_scope())?;
        let selection = state.selections.iter().find(|selection| {
            selection.target_host_id == target_host_id && selection.group == group
        });
        match selection {
            Some(selection) => {
                ensure!(selection.project_id == project.project_id().as_ref(), "profile slot owned by another project");
                ensure!(state.profiles.contains_key(&selection.profile_id.0), "dangling project profile selection");
                Ok(Some(selection.profile_id.clone()))
            }
            None => Ok(None),
        }
    }

    fn read(&self, tenant: &TenantScope) -> Result<ProfileState> {
        decode(self.settings.get(tenant, NAMESPACE, KEY)?.as_deref())
    }

    fn mutate<T>(
        &self,
        tenant: &TenantScope,
        update: impl FnOnce(&mut ProfileState) -> Result<T>,
    ) -> Result<T> {
        self.settings.update_with_preimages(tenant, NAMESPACE, KEY, &[], |raw| {
            let mut state = decode(raw)?;
            let result = update(&mut state)?;
            Ok((result, Some(serde_json::to_string(&state)?)))
        })
    }
}

fn decode(raw: Option<&str>) -> Result<ProfileState> {
    match raw {
        Some(raw) => serde_json::from_str(raw).context("invalid persisted project profile state"),
        None => Ok(ProfileState::default()),
    }
}
