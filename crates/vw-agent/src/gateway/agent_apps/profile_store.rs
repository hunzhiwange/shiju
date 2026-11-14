//! Tenant-shared entities and host/group selection in one atomic settings record.
//! Callers resolve project identity and authorize the execution host before entry.

use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use vw_agent_apps::project_profiles::snapshot::{SnapshotReadFailure, SnapshotScope};
use vw_api_types::app_providers::mode_state::ProviderModeStatus;
use vw_api_types::project_profiles::ProjectProfileSkillRef;
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

    /// One synchronous executor job owns the entire read/apply/save/select sequence.
    /// Invoke from a blocking worker, after project/host/domain authorization.
    pub(crate) fn with_operation<T>(
        &self,
        tenant: &TenantScope,
        work: impl FnOnce(&ProfileOperation<'_>) -> Result<T>,
    ) -> Result<T> {
        let _guard = super::profile_operations::lock(tenant, &self.execution_host_id)?;
        work(&ProfileOperation { store: self, tenant, _worker: std::marker::PhantomData })
    }
}

/// Only constructed while the outer operation lock is held. Never send this
/// borrowed context to another job or retain it across an await.
pub(crate) struct ProfileOperation<'a> {
    store: &'a ProjectProfileStore,
    tenant: &'a TenantScope,
    _worker: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl ProfileOperation<'_> {
    /// Called on the authorized executor with domain readers for the same scope.
    /// Creation only inserts an entity; existing host/group pointers are preserved.
    pub(crate) fn create_from_current<P, M, S, R>(
        &self,
        name: &str,
        scope: &SnapshotScope<'_>,
        read_provider: P,
        read_mcp: M,
        read_skills: S,
        read_prompt: R,
    ) -> Result<ProjectProfile>
    where
        P: FnOnce(&SnapshotScope<'_>) -> Result<ProviderModeStatus, SnapshotReadFailure>,
        M: FnOnce(&SnapshotScope<'_>) -> Result<Vec<String>, SnapshotReadFailure>,
        S: FnOnce(&SnapshotScope<'_>) -> Result<Vec<ProjectProfileSkillRef>, SnapshotReadFailure>,
        R: FnOnce(&SnapshotScope<'_>) -> Result<Option<String>, SnapshotReadFailure>,
    {
        ensure!(scope.project.tenant_scope() == self.tenant, "profile operation tenant mismatch");
        ensure!(scope.target_host_id == self.store.execution_host_id, "profile host requires executor dispatch");
        vw_agent_apps::project_profiles::create::create_from_current(
            name, scope, read_provider, read_mcp, read_skills, read_prompt,
            |profile| self.save(scope.project.tenant_scope(), profile, None),
        ).map_err(|error| anyhow::anyhow!("{error}"))
    }

    /// Capture on the authorized executor, then merge under the storage transaction.
    /// Callers must authorize the project and all four domain reads beforehand.
    pub(crate) fn resnapshot<P, M, S, R>(
        &self,
        id: &ProjectProfileId,
        expected_version: u64,
        scope: &SnapshotScope<'_>,
        read_provider: P,
        read_mcp: M,
        read_skills: S,
        read_prompt: R,
    ) -> Result<ProjectProfile>
    where
        P: FnOnce(&SnapshotScope<'_>) -> Result<ProviderModeStatus, SnapshotReadFailure>,
        M: FnOnce(&SnapshotScope<'_>) -> Result<Vec<String>, SnapshotReadFailure>,
        S: FnOnce(&SnapshotScope<'_>) -> Result<Vec<ProjectProfileSkillRef>, SnapshotReadFailure>,
        R: FnOnce(&SnapshotScope<'_>) -> Result<Option<String>, SnapshotReadFailure>,
    {
        ensure!(scope.project.tenant_scope() == self.tenant, "profile operation tenant mismatch");
        ensure!(scope.target_host_id == self.store.execution_host_id, "profile host requires executor dispatch");
        // Reject corrupt storage, missing identities and stale requests before reads.
        let previous = self.get(scope.project.tenant_scope(), id)?
            .context("project profile not found")?;
        ensure!(previous.version == expected_version, "project profile version conflict");
        let snapshot = vw_agent_apps::project_profiles::snapshot::snapshot_current(
            scope, read_provider, read_mcp, read_skills, read_prompt,
        )?;
        // Native reads never run while holding the settings transaction/record lock.
        self.mutate(scope.project.tenant_scope(), |state| {
            let profile = state.profiles.get_mut(&id.0).context("project profile not found")?;
            vw_agent_apps::project_profiles::resnapshot::merge_snapshot(
                profile, &snapshot, scope.group, expected_version, chrono::Utc::now().timestamp(),
            )?;
            Ok(profile.clone())
        })
    }

    /// Validate the target before autosaving the previous selection. Does not apply
    /// configuration or move the pointer. The operation owner must serialize the
    /// entire prepare/apply/select sequence for this authorized host/project/group.
    pub(crate) fn prepare_switch<P, M, S, R>(
        &self,
        target_id: &ProjectProfileId,
        scope: &SnapshotScope<'_>,
        read_provider: P,
        read_mcp: M,
        read_skills: S,
        read_prompt: R,
    ) -> Result<vw_agent_apps::project_profiles::autosave::PreparedProfileSwitch>
    where
        P: FnOnce(&SnapshotScope<'_>) -> Result<ProviderModeStatus, SnapshotReadFailure>,
        M: FnOnce(&SnapshotScope<'_>) -> Result<Vec<String>, SnapshotReadFailure>,
        S: FnOnce(&SnapshotScope<'_>) -> Result<Vec<ProjectProfileSkillRef>, SnapshotReadFailure>,
        R: FnOnce(&SnapshotScope<'_>) -> Result<Option<String>, SnapshotReadFailure>,
    {
        ensure!(scope.project.tenant_scope() == self.tenant, "profile operation tenant mismatch");
        ensure!(scope.target_host_id == self.store.execution_host_id, "profile host requires executor dispatch");
        // Decoding the complete stored state rejects malformed target payloads
        // before any native reads or automatic writes can happen.
        let target = self.get(scope.project.tenant_scope(), target_id)?
            .context("target project profile not found")?;
        let current = self.current(scope.project, scope.target_host_id, scope.group)?;
        vw_agent_apps::project_profiles::autosave::prepare_switch(
            target, current.as_ref(), scope.group, |previous| -> Result<()> {
                let profile = self.get(scope.project.tenant_scope(), previous)?
                    .context("previous project profile not found")?;
                self.resnapshot(previous, profile.version, scope,
                    read_provider, read_mcp, read_skills, read_prompt)?;
                Ok(())
            },
        )
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

    /// Delete a tenant-shared entity and every selection referring to it atomically.
    /// Callers must authorize shared-entity deletion, including other host selections.
    pub(crate) fn delete(
        &self,
        tenant: &TenantScope,
        id: &ProjectProfileId,
        expected_version: u64,
    ) -> Result<()> {
        ensure!(!id.0.trim().is_empty(), "empty project profile id");
        self.mutate(tenant, |state| {
            let profile = state.profiles.get(&id.0).context("project profile not found")?;
            ensure!(profile.version == expected_version, "project profile version conflict");
            state.profiles.remove(&id.0);
            state.selections.retain(|selection| &selection.profile_id != id);
            Ok(())
        })
    }

    /// Rename the stored entity under the same transaction as the version check.
    /// Loading here avoids overwriting a concurrently updated snapshot.
    pub(crate) fn rename(
        &self,
        tenant: &TenantScope,
        id: &ProjectProfileId,
        name: &str,
        expected_version: u64,
    ) -> Result<ProjectProfile> {
        self.mutate(tenant, |state| {
            let profile = state.profiles.get_mut(&id.0).context("project profile not found")?;
            vw_agent_apps::project_profiles::rename::rename(
                profile, name, expected_version, chrono::Utc::now().timestamp(),
            )?;
            Ok(profile.clone())
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
        ensure!(target_host_id == self.store.execution_host_id, "profile host requires executor dispatch");
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

    /// Clear only this authorized project's current group marker, including a
    /// dangling reference. An absent marker succeeds; entities and native
    /// configuration remain unchanged. Another project's slot is never released.
    pub(crate) fn clear(
        &self,
        project: &ProjectScope,
        target_host_id: &str,
        group: ProjectProfileScope,
    ) -> Result<()> {
        ensure!(target_host_id == self.store.execution_host_id, "profile host requires executor dispatch");
        self.mutate(project.tenant_scope(), |state| {
            for selection in state.selections.iter().filter(|selection| {
                selection.target_host_id == target_host_id && selection.group == group
            }) {
                // Validate every matching record before removing any, even if
                // persisted state contains duplicate host/group selections.
                ensure!(selection.project_id == project.project_id().as_ref(), "profile slot owned by another project");
            }
            state.selections.retain(|selection| {
                selection.target_host_id != target_host_id || selection.group != group
            });
            Ok(())
        })
    }

    pub(crate) fn current(
        &self,
        project: &ProjectScope,
        target_host_id: &str,
        group: ProjectProfileScope,
    ) -> Result<Option<ProjectProfileId>> {
        ensure!(target_host_id == self.store.execution_host_id, "profile host requires executor dispatch");
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
        ensure!(tenant == self.tenant, "profile operation tenant mismatch");
        decode(self.store.settings.get(tenant, NAMESPACE, KEY)?.as_deref())
    }

    fn mutate<T>(
        &self,
        tenant: &TenantScope,
        update: impl FnOnce(&mut ProfileState) -> Result<T>,
    ) -> Result<T> {
        ensure!(tenant == self.tenant, "profile operation tenant mismatch");
        self.store.settings.update_with_preimages(tenant, NAMESPACE, KEY, &[], |raw| {
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
