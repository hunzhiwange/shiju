//! Project-scoped skill repository configuration in the existing settings store.

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use vw_agent_apps::skills::repos::{SkillRepo, SkillRepos};
use vw_api_types::id::ProjectScope;

use super::settings_store::GatewaySettingsStore;
use crate::global_database::GlobalDatabase;

pub(super) const NAMESPACE: &str = "agent_apps.skills.repos";
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillReposState {
    pub schema_version: u32,
    pub version: u64,
    pub repos: Vec<SkillRepo>,
}

impl Default for SkillReposState {
    fn default() -> Self {
        Self { schema_version: SCHEMA_VERSION, version: 0, repos: Vec::new() }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("skill repository configuration version conflict")]
pub struct SkillReposVersionConflict;

/// All operations require a trusted scope; mutations additionally require the
/// caller's repository-management authorization. This adapter does not grant it.
pub struct SkillReposStore(GatewaySettingsStore);

impl SkillReposStore {
    pub fn new(database: GlobalDatabase) -> Result<Self> {
        Ok(Self(GatewaySettingsStore::new(database)?))
    }

    /// Callers must supply a trusted, authorized project scope, never a request header alone.
    /// Repository references are project configuration, independent of installation hosts.
    pub fn get(&self, scope: &ProjectScope) -> Result<SkillReposState> {
        let key = config_key(scope)?;
        let payload = self.0.get(scope.tenant_scope(), NAMESPACE, &key)?;
        decode(payload.as_deref())
    }

    /// The caller must authorize repository management in this project.
    pub fn save(
        &self,
        scope: &ProjectScope,
        expected_version: u64,
        repo: SkillRepo,
    ) -> Result<SkillReposState> {
        self.update(scope, expected_version, |repos| {
            repos.save(repo)?;
            Ok(())
        })
    }

    pub fn set_enabled(
        &self,
        scope: &ProjectScope,
        expected_version: u64,
        owner: &str,
        name: &str,
        enabled: bool,
    ) -> Result<SkillReposState> {
        self.update(scope, expected_version, |repos| {
            repos.set_enabled(owner, name, enabled)?;
            Ok(())
        })
    }

    /// Remove configuration only; installed skill entities and bindings remain untouched.
    pub fn remove(
        &self,
        scope: &ProjectScope,
        expected_version: u64,
        owner: &str,
        name: &str,
    ) -> Result<SkillReposState> {
        self.update(scope, expected_version, |repos| {
            repos.remove(owner, name)?;
            Ok(())
        })
    }

    fn update(
        &self,
        scope: &ProjectScope,
        expected_version: u64,
        change: impl FnOnce(&mut SkillRepos) -> Result<()>,
    ) -> Result<SkillReposState> {
        let key = config_key(scope)?;
        self.0.update_with_preimages(scope.tenant_scope(), NAMESPACE, &key, &[], |payload| {
            let mut state = decode(payload)?;
            if state.version != expected_version {
                bail!(SkillReposVersionConflict);
            }
            let mut repos = SkillRepos::from_saved(state.repos.clone())?;
            change(&mut repos)?;
            if repos.entries() == state.repos.as_slice() {
                return Ok((state, None));
            }
            state.version =
                state.version.checked_add(1).context("skill repository version exhausted")?;
            state.repos = repos.entries().to_vec();
            let replacement = serde_json::to_string(&state)?;
            // This local candidate is returned only after the settings transaction commits.
            Ok((state, Some(replacement)))
        })
    }
}

pub(super) fn config_key(scope: &ProjectScope) -> Result<String> {
    Ok(serde_json::to_string(scope.project_id())?)
}

fn decode(payload: Option<&str>) -> Result<SkillReposState> {
    let mut state: SkillReposState = payload
        .map(|json| serde_json::from_str(json).context("invalid saved skill repositories"))
        .transpose()?
        .unwrap_or_default();
    ensure!(state.schema_version == SCHEMA_VERSION, "unsupported skill repository schema version");
    state.repos = SkillRepos::from_saved(state.repos)?.entries().to_vec();
    Ok(state)
}

/// Local installation metadata shares the existing tenant settings owner.
/// This service executes on the selected host; callers must authorize the
/// project, host, SSOT root and source before invoking it. It does not dispatch
/// remote requests or reinterpret remote paths on the gateway.
#[cfg(target_os = "macos")]
#[derive(Clone)]
pub struct SkillInstallsStore(GatewaySettingsStore);

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillInstallsState {
    pub schema_version: u32,
    pub version: u64,
    pub ssot_root: std::path::PathBuf,
    #[serde(default = "default_storage_mode")]
    pub storage_mode: vw_agent_apps::skills::link_deploy::SyncMethod,
    #[serde(default)]
    pub copy_bindings: std::collections::BTreeMap<String, vw_agent_apps::skills::copy_deploy::CopyBinding>,
    pub skills: Vec<vw_agent_apps::skills::local_install::LocalSkill>,
    #[serde(default)]
    pub migration_snapshot: Option<vw_agent_apps::data_management::safety_snapshot::SafetySnapshotReceipt>,
    #[serde(default)]
    pub migrated: std::collections::BTreeMap<String, vw_agent_apps::data_management::skills::MigratedSkill>,
    #[serde(default)]
    pub update_metadata: std::collections::BTreeMap<String, vw_agent_apps::skills::update_apply::SkillUpdateMetadata>,
    #[serde(default)]
    pub repo_sources: std::collections::BTreeMap<String, vw_agent_apps::skills::repo_install::RepoSkillSource>,
    #[serde(default)]
    pub install_targets: Vec<vw_agent_apps::skills::bulk_sync::SkillInstallTarget>,
}

#[cfg(target_os = "macos")]
impl SkillInstallsStore {
    const NAMESPACE: &'static str = "agent_apps.skills.installed";

    pub fn new(database: GlobalDatabase) -> Result<Self> {
        Ok(Self(GatewaySettingsStore::new(database)?))
    }

    pub fn get(
        &self,
        scope: &ProjectScope,
        target_host_id: &str,
        ssot: &std::path::Path,
    ) -> Result<SkillInstallsState> {
        let key = Self::key(scope, target_host_id)?;
        ensure!(ssot.is_absolute(), "skill storage root must be absolute");
        let root = std::fs::canonicalize(ssot)?;
        let payload = self.0.get(scope.tenant_scope(), Self::NAMESPACE, &key)?;
        Self::decode(payload.as_deref(), &root)
    }

    /// Read filesystem truth only after the same scoped storage lookup as installation.
    pub fn inventory(
        &self,
        scope: &ProjectScope,
        target_host_id: &str,
        ssot: &std::path::Path,
    ) -> Result<Vec<vw_agent_apps::skills::inventory::InstalledSkillDetail>> {
        let state = self.get(scope, target_host_id, ssot)?;
        Ok(vw_agent_apps::skills::inventory::get_all_installed(&state.ssot_root, &state.skills)?)
    }

    /// Read-only service entry, independent of desktop refresh scheduling.
    pub async fn check_updates_report(
        &self,
        scope: &ProjectScope,
        target_host_id: &str,
        ssot: &std::path::Path,
        client: &reqwest::Client,
    ) -> Result<vw_agent_apps::skills::update_check::SkillUpdateCheckResult> {
        let state = self.get(scope, target_host_id, ssot)?;
        Ok(vw_agent_apps::skills::update_check::check_updates_report(
            client, &state.ssot_root, &state.skills, &state.repo_sources,
        ).await)
    }

    /// A changed version, saved record or existing directory is a conflict.
    /// No hooks or application projection run as part of local installation.
    #[allow(clippy::too_many_arguments)]
    pub fn install_local(
        &self,
        scope: &ProjectScope,
        target_host_id: &str,
        expected_version: u64,
        ssot: &std::path::Path,
        source: &std::path::Path,
        directory: &str,
    ) -> Result<SkillInstallsState> {
        self.install_local_with_source(
            scope, target_host_id, expected_version, ssot, source, directory, source,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn install_local_with_source(
        &self,
        scope: &ProjectScope,
        target_host_id: &str,
        expected_version: u64,
        ssot: &std::path::Path,
        source: &std::path::Path,
        directory: &str,
        provenance: &std::path::Path,
    ) -> Result<SkillInstallsState> {
        use vw_agent_apps::skills::local_install::LocalInstall;

        let key = Self::key(scope, target_host_id)?;
        ensure!(ssot.is_absolute(), "skill storage root must be absolute");
        let root = std::fs::canonicalize(ssot)?;
        // The physical-root lock spans the database transaction and rollback.
        let installation = LocalInstall::prepare(&root, source, directory)?;
        self.commit_install(scope, &key, expected_version, &root, installation, None, Some(provenance))
    }

    /// Executes only on the authorized host, using that host's network policy.
    #[allow(clippy::too_many_arguments)]
    pub async fn install_repo(
        &self,
        scope: &ProjectScope,
        target_host_id: &str,
        expected_version: u64,
        ssot: &std::path::Path,
        client: &reqwest::Client,
        repo: &SkillRepo,
        selection: &str,
        directory: &str,
    ) -> Result<SkillInstallsState> {
        let key = Self::key(scope, target_host_id)?;
        ensure!(ssot.is_absolute(), "skill storage root must be absolute");
        let root = std::fs::canonicalize(ssot)?;
        let prepared = vw_agent_apps::skills::repo_install::prepare_repo_install(
            client, repo, selection, &root, directory,
        ).await?;
        self.commit_install(scope, &key, expected_version, &root,
            prepared.installation, Some(prepared.source), None)
    }

    #[allow(clippy::too_many_arguments)]
    fn commit_install(
        &self,
        scope: &ProjectScope,
        key: &str,
        expected_version: u64,
        root: &std::path::Path,
        mut installation: vw_agent_apps::skills::local_install::LocalInstall,
        repo_source: Option<vw_agent_apps::skills::repo_install::RepoSkillSource>,
        provenance: Option<&std::path::Path>,
    ) -> Result<SkillInstallsState> {
        let directory = installation.skill().directory.clone();
        let result = self.0.update_with_preimages(
            scope.tenant_scope(), Self::NAMESPACE, &key, &[], |payload| {
                let mut state = Self::decode(payload, &root)?;
                ensure!(state.version == expected_version, "skill installation version conflict");
                ensure!(
                    !state.skills.iter().any(|skill| skill.directory.eq_ignore_ascii_case(&directory)),
                    "skill installation identity conflict"
                );
                state.version = state.version.checked_add(1)
                    .context("skill installation version exhausted")?;
                let mut skill = installation.skill().clone();
                if let Some(source) = provenance {
                    skill.source = std::fs::canonicalize(source)?;
                }
                if let Some(source) = &repo_source {
                    // Temporary extraction paths must never become persisted origins.
                    skill.source = root.join(&directory);
                    state.repo_sources.insert(directory.clone(), source.clone());
                }
                state.skills.push(skill);
                state.skills.sort_by(|left, right| left.directory.cmp(&right.directory));
                let replacement = serde_json::to_string(&state)?;
                installation.publish()?;
                Ok((state, Some(replacement)))
            },
        );
        match result {
            Ok(state) => {
                installation.finish();
                Ok(state)
            }
            Err(error) => {
                if installation.rollback().is_err() {
                    // Do not hide partial publication or erase another writer's content.
                    return Err(error.context("skill installation directory rollback requires recovery"));
                }
                Err(error)
            }
        }
    }

    /// Import exactly one explicit skill/app selection, without deploying it.
    /// Repeated imports of the same content identity leave settings unchanged.
    #[allow(clippy::too_many_arguments)]
    pub fn import_unmanaged(
        &self, scope: &ProjectScope, target_host_id: &str, expected_version: u64,
        home: &std::path::Path, ssot: &std::path::Path,
        targets: &[vw_agent_apps::skills::app_paths::AppSkillsPath],
        selection: &vw_agent_apps::skills::unmanaged_import::UnmanagedSelection,
    ) -> Result<SkillInstallsState> {
        use vw_agent_apps::skills::unmanaged_import::{prepare_import, PreparedImport};
        let state = self.get(scope, target_host_id, ssot)?;
        ensure!(state.version == expected_version, "skill installation version conflict");
        match prepare_import(home, &state.ssot_root, &state.skills, targets, selection)? {
            PreparedImport::Existing(_) => Ok(state),
            PreparedImport::New(installation) => {
                let key = Self::key(scope, target_host_id)?;
                self.commit_install(scope, &key, expected_version, &state.ssot_root,
                    installation, None, None)
            }
        }
    }

    /// Import on the authorized execution host. Every item commits separately;
    /// conflicts and failures never roll back an earlier successful item.
    pub fn install_from_zip(
        &self,
        scope: &ProjectScope,
        target_host_id: &str,
        ssot: &std::path::Path,
        archive: &std::path::Path,
    ) -> Result<vw_agent_apps::skills::zip_install::ZipInstallResult> {
        use vw_agent_apps::skills::zip_install::{
            ZipImport, ZipInstallResult, ZipSkippedSkill, ZipFailedSkill,
        };
        // Validate scope, host and storage before reading the authorized archive.
        self.get(scope, target_host_id, ssot)?;
        let import = ZipImport::prepare(archive)?;
        let mut result = ZipInstallResult::default();
        for skill in &import.skills {
            if skill.validate().is_err() {
                result.failed.push(ZipFailedSkill {
                    directory: skill.directory.clone(), reason: "invalid skill directory".into(),
                });
                continue;
            }
            // Refresh after each commit; a failed item does not advance our version.
            let attempt = (|| -> Result<Option<vw_agent_apps::skills::local_install::LocalSkill>> {
                let state = self.get(scope, target_host_id, ssot)?;
                let saved = state.skills.iter().find(|saved| {
                    saved.directory.eq_ignore_ascii_case(&skill.directory)
                });
                if let Some(saved) = saved {
                    result.skipped.push(ZipSkippedSkill {
                        directory: skill.directory.clone(), existing_id: saved.directory.clone(),
                        name: saved.name.clone(),
                    });
                    return Ok(None);
                }
                match std::fs::symlink_metadata(ssot.join(&skill.directory)) {
                    Ok(_) => {
                        result.skipped.push(ZipSkippedSkill {
                            directory: skill.directory.clone(), existing_id: skill.directory.clone(),
                            name: skill.directory.clone(),
                        });
                        return Ok(None);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
                let installed = self.install_local_with_source(
                    scope, target_host_id, state.version, ssot, &skill.source, &skill.directory, archive,
                )?;
                Ok(installed.skills.into_iter().find(|saved| saved.directory == skill.directory))
            })();
            match attempt {
                Ok(Some(installed)) => result.installed.push(installed),
                Ok(None) => {}
                Err(_) => result.failed.push(ZipFailedSkill {
                    directory: skill.directory.clone(),
                    reason: "skill installation failed; inspect storage state before retrying".into(),
                }),
            }
        }
        Ok(result)
    }

    /// Runs on the authorized installation host. The caller must resolve and
    /// authorize the app instance and target path before calling this adapter.
    #[allow(clippy::too_many_arguments)]
    pub fn enable_copy(
        &self, scope: &ProjectScope, target_host_id: &str, expected_version: u64,
        ssot: &std::path::Path, installation_id: &str,
        target: &vw_agent_apps::skills::app_paths::AppSkillsPath, directory: &str,
    ) -> Result<SkillInstallsState> {
        self.enable_with_method(scope, target_host_id, expected_version, ssot, installation_id,
            target, directory, vw_agent_apps::skills::link_deploy::SyncMethod::Copy)
    }

    /// The binding records the actual link/copy mode; authorization matches enable_copy.
    #[allow(clippy::too_many_arguments)]
    pub fn enable_with_method(
        &self, scope: &ProjectScope, target_host_id: &str, expected_version: u64,
        ssot: &std::path::Path, installation_id: &str,
        target: &vw_agent_apps::skills::app_paths::AppSkillsPath, directory: &str,
        method: vw_agent_apps::skills::link_deploy::SyncMethod,
    ) -> Result<SkillInstallsState> {
        ensure!(!installation_id.trim().is_empty(), "invalid application installation");
        let key = Self::key(scope, target_host_id)?;
        let root = std::fs::canonicalize(ssot)?;
        let binding_key = serde_json::to_string(&(installation_id, target.app_id, directory))?;
        let mut deployment = None;
        let result = self.0.update_with_preimages(scope.tenant_scope(), Self::NAMESPACE, &key, &[], |payload| {
            let mut state = Self::decode(payload, &root)?;
            ensure!(state.version == expected_version, "skill installation version conflict");
            ensure!(state.skills.iter().any(|skill| skill.directory == directory), "skill is not installed in SSOT");
            deployment = Some(vw_agent_apps::skills::link_deploy::prepare_deploy(
                &root, target, directory, state.copy_bindings.get(&binding_key), method,
            )?);
            let copy = deployment.as_mut().context("missing copy deployment")?;
            state.copy_bindings.insert(binding_key.clone(), copy.binding().clone());
            state.version = state.version.checked_add(1).context("skill installation version exhausted")?;
            let replacement = serde_json::to_string(&state)?;
            copy.publish()?;
            Ok((state, Some(replacement)))
        });
        match result {
            Ok(state) => {
                if let Some(copy) = deployment { copy.finish(); }
                Ok(state)
            }
            Err(error) => {
                if let Some(copy) = deployment.as_mut() {
                    if copy.rollback().is_err() { return Err(error.context("skill copy rollback requires recovery")); }
                }
                Err(error)
            }
        }
    }

    /// Update only the exact persisted repository/branch and binding snapshot.
    /// This service must execute on the authorized host, like install_repo.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_skill(
        &self, scope: &ProjectScope, host: &str, expected_version: u64,
        ssot: &std::path::Path, client: &reqwest::Client, directory: &str,
        expected_hash: &str,
    ) -> Result<SkillInstallsState> {
        self.update_skill_at(scope, host, expected_version, ssot, client, directory,
            expected_hash, "https://codeload.github.com", &|| Ok(())).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn update_skill_at(
        &self, scope: &ProjectScope, host: &str, expected_version: u64,
        ssot: &std::path::Path, client: &reqwest::Client, directory: &str,
        expected_hash: &str, archives: &str,
        before_commit: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<SkillInstallsState> {
        use vw_agent_apps::skills::update_apply::{download_update_at, SkillUpdate, SkillUpdateMetadata};
        let snapshot = self.get(scope, host, ssot)?;
        ensure!(snapshot.version == expected_version, "skill installation version conflict");
        let saved = snapshot.skills.iter().find(|s| s.directory == directory)
            .context("skill is not installed")?;
        let source = snapshot.repo_sources.get(directory).context("skill has no repository source")?;
        let (_archive, tree) = download_update_at(client, source, archives).await?;
        before_commit()?;
        let key = Self::key(scope, host)?;
        let mut update = None;
        let result = self.0.update_with_preimages(scope.tenant_scope(), Self::NAMESPACE, &key, &[], |payload| {
            let mut state = Self::decode(payload, &snapshot.ssot_root)?;
            // A concurrent uninstall or rebind must never be resurrected.
            ensure!(state == snapshot, "skill update snapshot conflict");
            update = Some(SkillUpdate::prepare(&state.ssot_root, &tree, saved,
                expected_hash, &state.copy_bindings)?);
            let prepared = update.as_mut().context("missing staged update")?;
            let entry = state.skills.iter_mut().find(|s| s.directory == directory)
                .context("skill was uninstalled")?;
            *entry = prepared.skill.clone();
            state.update_metadata.insert(directory.to_owned(), SkillUpdateMetadata {
                content_hash: prepared.content_hash.clone(),
                updated_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs(),
            });
            for (key, binding) in prepared.bindings() {
                state.copy_bindings.insert(key.to_owned(), binding.clone());
            }
            state.version = state.version.checked_add(1).context("skill installation version exhausted")?;
            let replacement = serde_json::to_string(&state)?;
            prepared.publish()?;
            Ok((state, Some(replacement)))
        });
        match result {
            Ok(state) => { if let Some(update) = update { update.finish(); } Ok(state) }
            Err(error) => {
                if let Some(update) = update.as_mut() {
                    if update.rollback().is_err() { return Err(error.context("skill update rollback requires recovery")); }
                }
                Err(error)
            }
        }
    }

    /// Disable only the saved deployment for this authorized installation.
    /// Preserved/conflicting paths retain their binding and do not advance state.
    #[allow(clippy::too_many_arguments)]
    pub fn disable_deploy(
        &self, scope: &ProjectScope, target_host_id: &str, expected_version: u64,
        ssot: &std::path::Path, installation_id: &str,
        target: &vw_agent_apps::skills::app_paths::AppSkillsPath, directory: &str,
    ) -> Result<SkillInstallsState> {
        ensure!(!installation_id.trim().is_empty(), "invalid application installation");
        let key = Self::key(scope, target_host_id)?;
        let root = std::fs::canonicalize(ssot)?;
        let binding_key = serde_json::to_string(&(installation_id, target.app_id, directory))?;
        let mut deployment = None;
        let result = self.0.update_with_preimages(scope.tenant_scope(), Self::NAMESPACE, &key, &[], |payload| {
            let mut state = Self::decode(payload, &root)?;
            ensure!(state.version == expected_version, "skill installation version conflict");
            deployment = Some(vw_agent_apps::skills::disable_deploy::DisableDeploy::prepare(
                &root, target, directory, state.copy_bindings.get(&binding_key),
            )?);
            state.copy_bindings.remove(&binding_key);
            state.version = state.version.checked_add(1).context("skill installation version exhausted")?;
            let replacement = serde_json::to_string(&state)?;
            deployment.as_mut().context("missing skill disable deployment")?.publish()?;
            Ok((state, Some(replacement)))
        });
        match result {
            Ok(state) => {
                if let Some(deployment) = deployment { deployment.finish()?; }
                Ok(state)
            }
            Err(error) => {
                if let Some(deployment) = deployment.as_mut() {
                    if deployment.rollback().is_err() {
                        return Err(error.context("skill disable rollback requires recovery"));
                    }
                }
                Err(error)
            }
        }
    }

    /// Backup and remove one installed skill on this authorized execution host.
    /// The caller authorizes the backup root and each resolved application target.
    /// Unresolved or externally modified projections are reported as preserved.
    #[allow(clippy::too_many_arguments)]
    pub fn uninstall_skill(
        &self, scope: &ProjectScope, host: &str, expected_version: u64,
        ssot: &std::path::Path, backup_root: &std::path::Path, directory: &str,
        targets: &std::collections::BTreeMap<String, vw_agent_apps::skills::app_paths::AppSkillsPath>,
    ) -> Result<(SkillInstallsState, vw_agent_apps::skills::uninstall::SkillUninstallResult)> {
        let key = Self::key(scope, host)?;
        ensure!(ssot.is_absolute(), "skill storage root must be absolute");
        let root = std::fs::canonicalize(ssot)?;
        let mut uninstall = None;
        let result = self.0.update_with_preimages(scope.tenant_scope(), Self::NAMESPACE, &key, &[], |payload| {
            let mut state = Self::decode(payload, &root)?;
            ensure!(state.version == expected_version, "skill installation version conflict");
            let saved = state.skills.iter().find(|skill| skill.directory == directory)
                .context("skill is not installed")?;
            uninstall = Some(vw_agent_apps::skills::uninstall::SkillUninstall::prepare(
                &root, backup_root, saved, &state.copy_bindings, targets,
            )?);
            state.skills.retain(|skill| skill.directory != directory);
            state.copy_bindings.retain(|_, binding| binding.directory != directory);
            state.update_metadata.remove(directory);
            state.repo_sources.remove(directory);
            state.version = state.version.checked_add(1).context("skill installation version exhausted")?;
            let replacement = serde_json::to_string(&state)?;
            uninstall.as_mut().context("missing skill uninstall")?.publish()?;
            Ok((state, Some(replacement)))
        });
        match result {
            Ok(state) => {
                let result = uninstall.context("missing committed skill uninstall")?.finish()?;
                Ok((state, result))
            }
            Err(error) => {
                if let Some(uninstall) = uninstall.as_mut() {
                    if uninstall.rollback().is_err() {
                        return Err(error.context("skill uninstall rollback requires recovery"));
                    }
                }
                Err(error)
            }
        }
    }

    /// Restore only caller-authorized host targets; backup paths never grant access.
    #[allow(clippy::too_many_arguments)]
    pub fn restore_skill_backup(
        &self, scope: &ProjectScope, host: &str, expected_version: u64,
        ssot: &std::path::Path, backup_root: &std::path::Path, backup_id: &str,
        targets: &std::collections::BTreeMap<String, vw_agent_apps::skills::app_paths::AppSkillsPath>,
    ) -> Result<(SkillInstallsState, vw_agent_apps::skills::backup_restore::SkillRestoreResult)> {
        let key = Self::key(scope, host)?;
        ensure!(ssot.is_absolute(), "skill storage root must be absolute");
        let root = std::fs::canonicalize(ssot)?;
        let mut restore = None;
        let result = self.0.update_with_preimages(scope.tenant_scope(), Self::NAMESPACE, &key, &[], |payload| {
            let mut state = Self::decode(payload, &root)?;
            ensure!(state.version == expected_version, "skill installation version conflict");
            restore = Some(vw_agent_apps::skills::backup_restore::SkillRestore::prepare(
                &root, backup_root, backup_id, targets,
            )?);
            let operation = restore.as_mut().context("missing skill restore")?;
            ensure!(!state.skills.iter().any(|skill| skill.directory.eq_ignore_ascii_case(&operation.skill().directory)),
                "skill is already installed");
            ensure!(!state.copy_bindings.values().any(|binding| binding.directory == operation.skill().directory),
                "skill still has saved bindings");
            for (key, target) in targets {
                let (installation, app, directory): (String, vw_api_types::agent_apps::AppId, String) =
                    serde_json::from_str(key).context("invalid skill restore binding identity")?;
                vw_agent_apps::skills::bulk_sync::SkillInstallTarget {
                    installation_id: installation, app_id: app,
                }.validate()?;
                ensure!(app == target.app_id && directory == operation.skill().directory,
                    "skill restore binding identity mismatch");
                ensure!(!state.copy_bindings.contains_key(key), "skill restore binding conflict");
            }
            state.version = state.version.checked_add(1).context("skill installation version exhausted")?;
            let applied = operation.publish()?;
            state.skills.push(applied.skill.clone());
            state.skills.sort_by(|left, right| left.directory.cmp(&right.directory));
            state.copy_bindings.extend(applied.bindings.clone());
            let replacement = serde_json::to_string(&state)?;
            Ok(((state, applied), Some(replacement)))
        });
        match result {
            Ok(result) => {
                restore.context("missing committed skill restore")?.finish();
                Ok(result)
            }
            Err(error) => {
                if let Some(restore) = restore.as_mut() {
                    if restore.rollback().is_err() {
                        return Err(error.context("skill restore rollback requires recovery"));
                    }
                }
                Err(error)
            }
        }
    }

    fn key(scope: &ProjectScope, target_host_id: &str) -> Result<String> {
        ensure!(
            !target_host_id.trim().is_empty() && target_host_id.len() <= 255
                && !target_host_id.chars().any(char::is_control),
            "invalid skill execution host"
        );
        Ok(serde_json::to_string(&(scope.project_id(), target_host_id))?)
    }

    fn decode(payload: Option<&str>, root: &std::path::Path) -> Result<SkillInstallsState> {
        use vw_agent_apps::skills::local_install::validate_directory;

        let state: SkillInstallsState = match payload {
            Some(payload) => serde_json::from_str(payload).context("invalid installed skill state")?,
            None => SkillInstallsState {
                schema_version: 1, version: 0, ssot_root: root.to_owned(), skills: Vec::new(), migration_snapshot: None, migrated: Default::default(),
                storage_mode: default_storage_mode(),
                copy_bindings: std::collections::BTreeMap::new(),
                update_metadata: std::collections::BTreeMap::new(),
                repo_sources: std::collections::BTreeMap::new(),
                install_targets: Vec::new(),
            },
        };
        vw_agent_apps::skills::bulk_sync::validate_targets(&state.install_targets)?;
        ensure!(state.schema_version == 1, "unsupported installed skill schema version");
        ensure!(state.ssot_root == root, "skill storage root changed; explicit migration required");
        let mut names = std::collections::BTreeSet::new();
        for skill in &state.skills {
            validate_directory(&skill.directory)?;
            ensure!(names.insert(skill.directory.to_ascii_lowercase()), "duplicate installed skill");
            ensure!(!skill.name.trim().is_empty(), "invalid installed skill name");
            ensure!(skill.source.is_absolute(), "invalid installed skill source");
        }
        for binding in state.copy_bindings.values() {
            validate_directory(&binding.directory)?;
            ensure!(binding.destination.is_absolute()
                && binding.destination.file_name().and_then(|name| name.to_str()) == Some(binding.directory.as_str()),
                "invalid saved skill copy destination");
            if let Some(source) = &binding.link_target {
                ensure!(source == &root.join(&binding.directory), "invalid saved skill link target");
            }
            ensure!(binding.fingerprint.len() == 64 && binding.fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "invalid saved skill copy fingerprint");
            ensure!(state.skills.iter().any(|skill| skill.directory == binding.directory), "orphan skill copy binding");
        }
        for (directory, metadata) in &state.update_metadata {
            validate_directory(directory)?;
            ensure!(state.skills.iter().any(|skill| &skill.directory == directory),
                "orphan skill update metadata");
            ensure!(metadata.content_hash.len() == 64
                && metadata.content_hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "invalid saved skill update hash");
        }
        for (directory, source) in &state.repo_sources {
            validate_directory(directory)?;
            ensure!(state.skills.iter().any(|skill| &skill.directory == directory), "orphan skill repository source");
            vw_agent_apps::skills::repos::validate_repo_ref(&source.owner, &source.name, &source.branch)?;
            ensure!(!source.branch.is_empty() && !source.branch.eq_ignore_ascii_case("HEAD"), "unresolved installed skill branch");
        }
        Ok(state)
    }
}

// Skill jobs use the same tenant settings owner and project/host key as installs.
#[cfg(target_os = "macos")]
static SKILL_EXECUTION_EPOCH: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| uuid::Uuid::new_v4().to_string());

#[cfg(target_os = "macos")]
impl SkillInstallsStore {
    const JOBS_NAMESPACE: &'static str = "agent_apps.skills.jobs";

    /// Caller must authorize this project and execution host just as for install_repo.
    /// A snapshot is sufficient to resume observation after a transport disconnect.
    pub fn job_snapshot(
        &self, scope: &ProjectScope, host: &str, task_id: &vw_api_types::id::TaskId,
    ) -> Result<vw_agent_apps::skills::jobs::SkillJob> {
        self.update_jobs(scope, host, |jobs| {
            jobs.iter().find(|job| &job.snapshot.task_id == task_id)
                .map(|job| job.snapshot.clone()).context("skill task not found")
        })
    }

    /// Uses the same project/host storage boundary as job_snapshot.
    /// Cancellation is acknowledged only after the worker releases staged files.
    pub fn cancel_job(
        &self, scope: &ProjectScope, host: &str, task_id: &vw_api_types::id::TaskId,
    ) -> Result<vw_agent_apps::skills::jobs::SkillJob> {
        self.update_jobs(scope, host, |jobs| {
            let job = jobs.iter_mut().find(|job| &job.snapshot.task_id == task_id)
                .context("skill task not found")?;
            Ok(vw_agent_apps::skills::job_cancel::request_cancel(job))
        })
    }

    /// Ownership is checked inside the same transaction as cancellation.
    /// Missing, legacy and differently bound jobs are indistinguishable.
    pub fn owned_job(
        &self, scope: &ProjectScope, host: &str, id: &vw_api_types::id::TaskId,
        principal: &str, root: &std::path::Path, cancel: bool,
    ) -> Result<Option<vw_agent_apps::skills::jobs::SkillJob>> {
        let canonical_root = std::fs::canonicalize(root).ok();
        self.update_jobs(scope, host, |jobs| {
            let Some(job) = jobs.iter_mut().find(|job| &job.snapshot.task_id == id
                && job.owner_principal.as_deref() == Some(principal)
                && job.resource_root.as_deref().is_some_and(|saved| {
                    saved == root || canonical_root.as_ref().is_some_and(|root| {
                        std::fs::canonicalize(saved).is_ok_and(|saved| &saved == root)
                    })
                })) else { return Ok(None); };
            Ok(Some(if cancel { vw_agent_apps::skills::job_cancel::request_cancel(job) }
                else { job.snapshot.clone() }))
        })
    }

    fn check_job_cancel(
        &self, scope: &ProjectScope, host: &str, id: &vw_api_types::id::TaskId,
    ) -> std::io::Result<()> {
        self.update_jobs(scope, host, |jobs| {
            let job = jobs.iter().find(|job| &job.snapshot.task_id == id)
                .context("skill task not found")?;
            ensure!(!job.cancel_requested, "skill task cancellation requested");
            Ok(())
        }).map_err(|_| std::io::Error::other("skill task stopped before commit"))
    }

    fn update_jobs<T>(
        &self, scope: &ProjectScope, host: &str,
        change: impl FnOnce(&mut Vec<vw_agent_apps::skills::jobs::SavedSkillJob>) -> Result<T>,
    ) -> Result<T> {
        let key = Self::key(scope, host)?;
        self.0.update_with_preimages(scope.tenant_scope(), Self::JOBS_NAMESPACE, &key, &[], |payload| {
            let mut jobs: Vec<vw_agent_apps::skills::jobs::SavedSkillJob> = payload
                .map(serde_json::from_str).transpose()?.unwrap_or_default();
            for job in &mut jobs {
                job.interrupt_previous_execution(SKILL_EXECUTION_EPOCH.as_str());
            }
            let result = change(&mut jobs)?;
            Ok((result, Some(serde_json::to_string(&jobs)?)))
        })
    }

    /// Return the persisted ID before downloading. Paths and network client are
    /// resolved and authorized on this execution host, never on a remote client.
    #[allow(clippy::too_many_arguments)]
    pub fn start_repo_install(
        &self, scope: ProjectScope, host: String, expected_version: u64,
        ssot: std::path::PathBuf, client: reqwest::Client, repo: SkillRepo,
        selection: String, directory: String, principal: String,
    ) -> Result<vw_agent_apps::skills::jobs::SkillJob> {
        self.start_repo_install_at(scope, host, expected_version, ssot, client, repo,
            selection, directory, principal, "https://codeload.github.com".into())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start_repo_install_at(&self, scope: ProjectScope, host: String,
        expected_version: u64, ssot: std::path::PathBuf, client: reqwest::Client, repo: SkillRepo,
        selection: String, directory: String, principal: String, archives: String,
    ) -> Result<vw_agent_apps::skills::jobs::SkillJob> {
        use vw_agent_apps::skills::jobs::{SavedSkillJob, SkillJobPhase, MAX_RETAINED_JOBS};
        // Reject invalid executor/storage/version before persisting a task or networking.
        let runtime = tokio::runtime::Handle::try_current()
            .context("skill task requires an execution runtime")?;
        let state = self.get(&scope, &host, &ssot)?;
        ensure!(state.version == expected_version, "skill installation version conflict");
        vw_agent_apps::skills::repos::validate_repo_ref(&repo.owner, &repo.name, &repo.branch)?;
        vw_agent_apps::skills::local_install::validate_directory(&directory)?;
        ensure!(!selection.is_empty() && !selection.split('/').any(|part| {
            part.is_empty() || matches!(part, "." | "..") || part.contains(['\\', ':'])
                || part.chars().any(char::is_control)
        }), "invalid repository skill selection");
        let job = self.update_jobs(&scope, &host, |jobs| {
            if jobs.len() >= MAX_RETAINED_JOBS {
                let oldest = jobs.iter().position(|job| job.snapshot.phase.is_terminal())
                    .context("skill task capacity reached")?;
                jobs.remove(oldest);
            }
            let mut job = SavedSkillJob::new(SKILL_EXECUTION_EPOCH.to_string());
            job.owner_principal = Some(principal.clone());
            job.resource_root = Some(ssot.clone());
            let snapshot = job.snapshot.clone();
            jobs.push(job);
            Ok(snapshot)
        })?;
        let store = self.clone();
        let id = job.task_id.clone();
        let guard = SkillJobExit {
            store: store.clone(), scope: scope.clone(), host: host.clone(), id: id.clone(),
        };
        runtime.spawn(async move {
            let _guard = guard;
            let outcome = async {
                let check = || store.check_job_cancel(&scope, &host, &id);
                let prepare = vw_agent_apps::skills::repo_install::prepare_checked(
                    &client, &repo, &selection, &state.ssot_root, &directory,
                    &archives, &check,
                );
                // Dropping preparation closes the response and its temporary files.
                let prepared = tokio::select! {
                    result = prepare => result?,
                    stopped = async {
                        loop {
                            if let Err(error) = check() {
                                break Err::<(), std::io::Error>(error);
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        }
                    } => { stopped?; anyhow::bail!("skill task stopped before commit"); }
                };
                store.set_job_phase(&scope, &host, &id, SkillJobPhase::Staged, None)?;
                store.set_job_phase(&scope, &host, &id, SkillJobPhase::Committing, None)?;
                let key = Self::key(&scope, &host)?;
                store.commit_install(&scope, &key, expected_version, &state.ssot_root,
                    prepared.installation, Some(prepared.source), None)?;
                store.set_job_phase(&scope, &host, &id, SkillJobPhase::Completed, Some(directory))?;
                Ok::<(), anyhow::Error>(())
            }.await;
            if outcome.is_err() {
                // Persist no raw filesystem/network errors or private source payloads.
                let _ = store.set_job_phase(&scope, &host, &id, SkillJobPhase::Failed, None);
            }
        });
        Ok(job)
    }

    fn set_job_phase(
        &self, scope: &ProjectScope, host: &str, id: &vw_api_types::id::TaskId,
        phase: vw_agent_apps::skills::jobs::SkillJobPhase, directory: Option<String>,
    ) -> Result<()> {
        use vw_agent_apps::skills::jobs::SkillJobPhase;
        self.update_jobs(scope, host, |jobs| {
            let job = jobs.iter_mut().find(|job| &job.snapshot.task_id == id)
                .context("skill task not found")?;
            ensure!(!job.snapshot.phase.is_terminal(), "skill task is already terminal");
            // Serialized with cancel_job: either cancellation wins before commit,
            // or the commit boundary wins and cancellation becomes observational.
            if matches!(phase, SkillJobPhase::Staged | SkillJobPhase::Committing) {
                ensure!(!job.cancel_requested, "skill task cancellation requested");
            }
            let phase = if job.cancel_requested && phase == SkillJobPhase::Failed
                && job.snapshot.phase != SkillJobPhase::Committing {
                SkillJobPhase::Interrupted
            } else { phase };
            job.snapshot.requires_reconciliation = matches!(phase, SkillJobPhase::Failed | SkillJobPhase::Interrupted)
                && job.snapshot.phase == SkillJobPhase::Committing;
            job.snapshot.phase = phase;
            job.snapshot.installed_directory = directory;
            Ok(())
        })
    }
}

/// Dropping a worker (including panic or runtime shutdown) never leaves a live
/// task snapshot behind. Completed snapshots are protected by set_job_phase.
#[cfg(target_os = "macos")]
struct SkillJobExit {
    store: SkillInstallsStore,
    scope: ProjectScope,
    host: String,
    id: vw_api_types::id::TaskId,
}

#[cfg(target_os = "macos")]
impl Drop for SkillJobExit {
    fn drop(&mut self) {
        let _ = self.store.set_job_phase(&self.scope, &self.host, &self.id,
            vw_agent_apps::skills::jobs::SkillJobPhase::Interrupted, None);
    }
}

#[cfg(target_os = "macos")]
impl SkillInstallsStore {
    /// Synchronous local/archive work still receives a durable task identity.
    /// Authorization and optimistic-version preflight must precede this call.
    pub fn run_file_install<T>(
        &self, scope: &ProjectScope, host: &str, root: &std::path::Path, principal: &str,
        install: impl FnOnce() -> Result<T>,
    ) -> Result<(vw_agent_apps::skills::jobs::SkillJob, T)> {
        use vw_agent_apps::skills::jobs::{SavedSkillJob, SkillJobPhase, MAX_RETAINED_JOBS};
        let job = self.update_jobs(scope, host, |jobs| {
            if jobs.len() >= MAX_RETAINED_JOBS {
                let oldest = jobs.iter().position(|job| job.snapshot.phase.is_terminal())
                    .context("skill task capacity reached")?;
                jobs.remove(oldest);
            }
            let mut job = SavedSkillJob::new(SKILL_EXECUTION_EPOCH.to_string());
            job.owner_principal = Some(principal.to_owned());
            job.resource_root = Some(root.to_owned());
            let snapshot = job.snapshot.clone();
            jobs.push(job);
            Ok(snapshot)
        })?;
        let _guard = SkillJobExit {
            store: self.clone(), scope: scope.clone(), host: host.into(), id: job.task_id.clone(),
        };
        self.set_job_phase(scope, host, &job.task_id, SkillJobPhase::Committing, None)?;
        match install() {
            Ok(result) => {
                self.set_job_phase(scope, host, &job.task_id, SkillJobPhase::Completed, None)?;
                Ok((self.job_snapshot(scope, host, &job.task_id)?, result))
            }
            Err(error) => {
                let _ = self.set_job_phase(scope, host, &job.task_id, SkillJobPhase::Failed, None);
                Err(error)
            }
        }
    }
}

#[cfg(target_os = "macos")]
impl SkillInstallsStore {
    /// Preferences are project/host settings. They grant no filesystem authority;
    /// callers must resolve and authorize the selected installations before use.
    pub fn save_install_targets(
        &self, scope: &ProjectScope, host: &str, expected_version: u64,
        ssot: &std::path::Path,
        targets: Vec<vw_agent_apps::skills::bulk_sync::SkillInstallTarget>,
    ) -> Result<SkillInstallsState> {
        vw_agent_apps::skills::bulk_sync::validate_targets(&targets)?;
        let snapshot = self.get(scope, host, ssot)?;
        let key = Self::key(scope, host)?;
        self.0.update_with_preimages(scope.tenant_scope(), Self::NAMESPACE, &key, &[], |payload| {
            let mut state = Self::decode(payload, &snapshot.ssot_root)?;
            ensure!(state.version == expected_version, "skill installation version conflict");
            if state.install_targets == targets { return Ok((state, None)); }
            state.install_targets = targets;
            state.version = state.version.checked_add(1).context("skill installation version exhausted")?;
            let replacement = serde_json::to_string(&state)?;
            Ok((state, Some(replacement)))
        })
    }

    /// Explicit targets only: never infer authorization from saved preferences.
    /// Each skill/app pair uses its own transaction. An item failure does not
    /// discard earlier outcomes or stop later applications and skills.
    #[allow(clippy::too_many_arguments)]
    pub fn bulk_toggle(
        &self, scope: &ProjectScope, host: &str, ssot: &std::path::Path,
        directories: &[String], targets: &[vw_agent_apps::skills::bulk_sync::SkillSyncTarget],
        enabled: bool, method: vw_agent_apps::skills::link_deploy::SyncMethod,
    ) -> Result<vw_agent_apps::skills::bulk_sync::BulkSyncResult> {
        use vw_agent_apps::skills::bulk_sync::{BulkSyncResult, SkillSyncOutcome, SkillSyncStatus};
        self.get(scope, host, ssot)?;
        let mut result = BulkSyncResult::default();
        for directory in directories {
            for target in targets {
                let attempt = (|| -> Result<()> {
                    target.identity().validate()?;
                    let state = self.get(scope, host, ssot)?;
                    if enabled {
                        self.enable_with_method(scope, host, state.version, ssot,
                            &target.installation_id, &target.path, directory, method)?;
                    } else {
                        self.disable_deploy(scope, host, state.version, ssot,
                            &target.installation_id, &target.path, directory)?;
                    }
                    Ok(())
                })();
                result.items.push(SkillSyncOutcome {
                    target: target.identity(), directory: directory.clone(),
                    status: if attempt.is_err() { SkillSyncStatus::Failed }
                        else if enabled { SkillSyncStatus::Enabled } else { SkillSyncStatus::Disabled },
                });
            }
        }
        Ok(result)
    }

    /// Install once, then publish to the first selected app and the remaining
    /// apps in order. The committed installation survives projection failures.
    #[allow(clippy::too_many_arguments)]
    pub fn install_local_to_targets(
        &self, scope: &ProjectScope, host: &str, expected_version: u64,
        ssot: &std::path::Path, source: &std::path::Path, directory: &str,
        targets: &[vw_agent_apps::skills::bulk_sync::SkillSyncTarget],
        method: vw_agent_apps::skills::link_deploy::SyncMethod,
    ) -> Result<vw_agent_apps::skills::bulk_sync::BulkSyncResult> {
        ensure!(!targets.is_empty(), "select at least one skill install target");
        let state = self.install_local(scope, host, expected_version, ssot, source, directory)?;
        Ok(self.distribute_installed(scope, host, ssot, &state, directory, targets, method))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn install_repo_to_targets(
        &self, scope: &ProjectScope, host: &str, expected_version: u64,
        ssot: &std::path::Path, client: &reqwest::Client, repo: &SkillRepo,
        selection: &str, directory: &str,
        targets: &[vw_agent_apps::skills::bulk_sync::SkillSyncTarget],
        method: vw_agent_apps::skills::link_deploy::SyncMethod,
    ) -> Result<vw_agent_apps::skills::bulk_sync::BulkSyncResult> {
        ensure!(!targets.is_empty(), "select at least one skill install target");
        let state = self.install_repo(scope, host, expected_version, ssot, client, repo,
            selection, directory).await?;
        Ok(self.distribute_installed(scope, host, ssot, &state, directory, targets, method))
    }

    #[allow(clippy::too_many_arguments)]
    fn distribute_installed(
        &self, scope: &ProjectScope, host: &str, ssot: &std::path::Path,
        state: &SkillInstallsState, directory: &str,
        targets: &[vw_agent_apps::skills::bulk_sync::SkillSyncTarget],
        method: vw_agent_apps::skills::link_deploy::SyncMethod,
    ) -> vw_agent_apps::skills::bulk_sync::BulkSyncResult {
        use vw_agent_apps::skills::bulk_sync::{BulkSyncResult, SkillSyncOutcome, SkillSyncStatus};
        let mut result = BulkSyncResult {
            installed: state.skills.iter().filter(|skill| skill.directory == directory).cloned().collect(),
            items: Vec::new(),
        };
        // Even a storage read failure after installation is an app failure,
        // rather than an error that hides the committed SSOT installation.
        for target in targets {
            match self.bulk_toggle(scope, host, ssot, &[directory.to_owned()],
                std::slice::from_ref(target), true, method) {
                Ok(attempt) => result.items.extend(attempt.items),
                Err(_) => result.items.push(SkillSyncOutcome {
                    target: target.identity(), directory: directory.to_owned(), status: SkillSyncStatus::Failed,
                }),
            }
        }
        result
    }

    /// Reproject saved bindings for the authorized installations. Native Pi/OMP
    /// directories are intentionally excluded from automatic resynchronization.
    /// No directory scanning or removal of unrecorded copies/links occurs.
    pub fn resync_all_apps(
        &self, scope: &ProjectScope, host: &str, ssot: &std::path::Path,
        targets: &[vw_agent_apps::skills::bulk_sync::SkillSyncTarget],
    ) -> Result<vw_agent_apps::skills::bulk_sync::BulkSyncResult> {
        use vw_agent_apps::skills::{app_paths::SkillDistribution,
            bulk_sync::{BulkSyncResult, SkillSyncOutcome, SkillSyncStatus}, link_deploy::SyncMethod};
        let snapshot = self.get(scope, host, ssot)?;
        let mut result = BulkSyncResult::default();
        for target in targets {
            if target.path.distribution != SkillDistribution::ManagedBinding { continue; }
            for (key, binding) in &snapshot.copy_bindings {
                let (installation, app, directory): (String, vw_api_types::agent_apps::AppId, String) =
                    match serde_json::from_str(key) {
                        Ok(identity) => identity,
                        Err(_) => continue, // Invalid identities grant no cleanup authority.
                    };
                if installation != target.installation_id || app != target.path.app_id { continue; }
                let mut status = SkillSyncStatus::Enabled;
                let attempt = (|| -> Result<()> {
                    target.identity().validate()?;
                    ensure!(directory == binding.directory, "invalid saved skill binding identity");
                    let state = self.get(scope, host, ssot)?;
                    // A concurrent update changes the snapshot, so preserve it.
                    ensure!(state.copy_bindings.get(key) == Some(binding), "skill binding changed during resync");
                    let missing = match std::fs::symlink_metadata(state.ssot_root.join(&directory)) {
                        Ok(_) => false,
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
                        Err(error) => return Err(error.into()),
                    };
                    if missing {
                        ensure!(binding.link_target.is_some(), "orphan skill copy preserved");
                        status = SkillSyncStatus::Disabled;
                        self.disable_deploy(scope, host, state.version, ssot,
                            &installation, &target.path, &directory)?;
                    } else {
                        self.enable_with_method(scope, host, state.version, ssot,
                            &installation, &target.path, &directory, SyncMethod::Auto)?;
                    }
                    Ok(())
                })();
                result.items.push(SkillSyncOutcome {
                    target: target.identity(), directory,
                    status: if attempt.is_err() { SkillSyncStatus::Failed } else { status },
                });
            }
        }
        Ok(result)
    }
}

#[cfg(target_os = "macos")]
impl SkillInstallsStore {
    /// Persist identity before network work, retaining terminal status for reconnects.
    #[allow(clippy::too_many_arguments)]
    pub fn start_skill_update(&self, scope: ProjectScope, host: String,
        expected_version: u64, ssot: std::path::PathBuf, client: reqwest::Client,
        directory: String, expected_hash: String,
    ) -> Result<vw_agent_apps::skills::jobs::SkillJob> {
        self.start_skill_update_at(scope, host, expected_version, ssot, client, directory,
            expected_hash, "https://codeload.github.com".into(), None)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start_skill_update_at(&self, scope: ProjectScope, host: String,
        expected_version: u64, ssot: std::path::PathBuf, client: reqwest::Client,
        directory: String, expected_hash: String, archives: String, principal: Option<String>,
    ) -> Result<vw_agent_apps::skills::jobs::SkillJob> {
        use vw_agent_apps::skills::jobs::{SavedSkillJob, SkillJobPhase, MAX_RETAINED_JOBS};
        let runtime = tokio::runtime::Handle::try_current()?;
        let state = self.get(&scope, &host, &ssot)?;
        ensure!(state.version == expected_version, "skill installation version conflict");
        vw_agent_apps::skills::local_install::validate_directory(&directory)?;
        ensure!(state.skills.iter().any(|s| s.directory == directory)
            && state.repo_sources.contains_key(&directory), "skill has no installed repository source");
        ensure!(vw_agent_apps::skills::inventory::compute_dir_hash(&ssot.join(&directory))? == expected_hash,
            "skill content conflict");
        let job = self.update_jobs(&scope, &host, |jobs| {
            if jobs.len() >= MAX_RETAINED_JOBS {
                let oldest = jobs.iter().position(|j| j.snapshot.phase.is_terminal())
                    .context("skill task capacity reached")?;
                jobs.remove(oldest);
            }
            let mut job = SavedSkillJob::new(SKILL_EXECUTION_EPOCH.to_string());
            job.owner_principal = principal.clone();
            job.resource_root = Some(ssot.clone());
            let snapshot = job.snapshot.clone();
            jobs.push(job);
            Ok(snapshot)
        })?;
        let store = self.clone();
        let id = job.task_id.clone();
        let guard = SkillJobExit { store: store.clone(), scope: scope.clone(), host: host.clone(), id: id.clone() };
        runtime.spawn(async move {
            let _guard = guard;
            let result = async {
                // Stay cancellable throughout network preparation. The durable
                // phase transition serializes cancellation with publication.
                let before_commit = || store.set_job_phase(&scope, &host, &id, SkillJobPhase::Committing, None);
                let work = store.update_skill_at(&scope, &host, expected_version, &ssot, &client,
                    &directory, &expected_hash, &archives, &before_commit);
                let state = tokio::select! {
                    result = work => result?,
                    stopped = async {
                        loop {
                            if let Err(error) = store.check_job_cancel(&scope, &host, &id) {
                                break Err::<(), std::io::Error>(error);
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        }
                    } => { stopped?; anyhow::bail!("skill task stopped before commit"); }
                };
                store.update_jobs(&scope, &host, |jobs| {
                    let job = jobs.iter_mut().find(|job| job.snapshot.task_id == id)
                        .context("skill task not found")?;
                    job.snapshot.phase = SkillJobPhase::Completed;
                    job.snapshot.installed_directory = Some(directory.clone());
                    job.snapshot.update_version = Some(state.version);
                    job.snapshot.content_hash = Some(state.update_metadata.get(&directory)
                        .context("missing update metadata")?.content_hash.clone());
                    Ok(())
                })
            }.await;
            if result.is_err() {
                let _ = store.set_job_phase(&scope, &host, &id, SkillJobPhase::Failed, None);
            }
        });
        Ok(job)
    }
}

#[cfg(target_os = "macos")]
impl SkillInstallsStore {
    /// Caller authorizes project/host, host HOME/runtime root and every resolved
    /// application target. This operation never creates roots or imports old databases.
    #[allow(clippy::too_many_arguments)]
    pub fn migrate_storage(
        &self, scope: &ProjectScope, host: &str, expected_version: u64,
        ssot: &std::path::Path, destination: &std::path::Path,
        home: &std::path::Path, runtime_root: &std::path::Path,
        targets: &[vw_agent_apps::skills::bulk_sync::SkillSyncTarget],
    ) -> Result<vw_agent_apps::skills::storage_move::StorageMoveResult> {
        use vw_agent_apps::skills::storage_move::StorageMove;
        ensure!(home.is_absolute() && runtime_root.is_absolute() && destination.is_absolute(),
            "skill migration roots must be absolute");
        // Reject links in the selected destination and both allowed host roots.
        let allowed = [runtime_root.to_owned(), home.join(".agents/skills")];
        ensure!(allowed.iter().any(|path| path == destination), "unsupported skill storage destination");
        vw_agent_apps::skills::storage_move::validate_destination(home, destination)
            .map_err(|_| anyhow::anyhow!("unsafe skill migration destination"))?;
        let destination = std::fs::canonicalize(destination)?;
        let snapshot = self.get(scope, host, ssot)?;
        ensure!(snapshot.version == expected_version, "skill installation version conflict");
        let mut roots = Vec::new();
        for target in targets {
            target.identity().validate()?;
            target.path.require_distribution()?;
            roots.push(target.path.directory.clone());
        }
        // Every recorded binding needs a resolved host-authorized target. Do not
        // turn a saved filesystem path into new write authority.
        for (key, binding) in &snapshot.copy_bindings {
            let (installation, app, directory): (String, vw_api_types::agent_apps::AppId, String) =
                serde_json::from_str(key)?;
            ensure!(directory == binding.directory && targets.iter().any(|target|
                target.installation_id == installation && target.path.app_id == app
                    && target.path.directory.join(&directory) == binding.destination),
                "skill migration requires every bound application target");
        }
        let names = snapshot.skills.iter().map(|skill| skill.directory.clone()).collect::<Vec<_>>();
        let mut migration = StorageMove::prepare(&snapshot.ssot_root, &destination, &names, &roots)?;
        if !migration.result.errors.is_empty() {
            migration.rollback().context("skill migration copy rollback requires recovery")?;
            migration.result.migrated.clear();
            return Ok(migration.result.clone());
        }
        if snapshot.ssot_root == destination { return Ok(migration.result.clone()); }
        let key = Self::key(scope, host)?;
        let committed = self.0.update_with_preimages(scope.tenant_scope(), Self::NAMESPACE, &key, &[], |payload| {
            let mut state = Self::decode(payload, &snapshot.ssot_root)?;
            ensure!(state.version == expected_version, "skill installation version conflict");
            for binding in state.copy_bindings.values_mut() {
                *binding = migration.retarget(&snapshot.ssot_root, binding)?;
            }
            for skill in &mut state.skills {
                if skill.source == snapshot.ssot_root.join(&skill.directory) {
                    skill.source = destination.join(&skill.directory);
                }
            }
            state.ssot_root = destination.clone();
            state.version = state.version.checked_add(1).context("skill installation version exhausted")?;
            Ok(((), Some(serde_json::to_string(&state)?)))
        });
        if let Err(error) = committed {
            migration.rollback().context("skill migration rollback requires recovery; old root retained")?;
            return Err(error);
        }
        Ok(migration.finish())
    }
}

#[cfg(target_os = "macos")]
fn default_storage_mode() -> vw_agent_apps::skills::link_deploy::SyncMethod {
    vw_agent_apps::skills::link_deploy::SyncMethod::Auto
}

#[cfg(target_os = "macos")]
impl SkillInstallsStore {
    /// Saved roots grant no authority: only the two host-authorized choices are accepted.
    pub fn storage_state(&self, scope: &ProjectScope, host: &str,
        runtime: &std::path::Path, home: &std::path::Path,
    ) -> Result<SkillInstallsState> {
        let payload = self.0.get(scope.tenant_scope(), Self::NAMESPACE, &Self::key(scope, host)?)?;
        let root = match &payload {
            Some(payload) => serde_json::from_str::<SkillInstallsState>(payload)?.ssot_root,
            None => std::fs::canonicalize(runtime)?,
        };
        let allowed = [runtime.to_owned(), home.join(".agents/skills")];
        ensure!(allowed.iter().any(|candidate| std::fs::canonicalize(candidate).ok().as_ref() == Some(&root)),
            "unsupported saved skill storage root");
        vw_agent_apps::skills::storage_move::validate_destination(home, &root)?;
        Self::decode(payload.as_deref(), &root)
    }

    pub fn save_storage_mode(&self, scope: &ProjectScope, host: &str, expected: u64,
        root: &std::path::Path, mode: vw_agent_apps::skills::link_deploy::SyncMethod,
    ) -> Result<SkillInstallsState> {
        self.0.update_with_preimages(scope.tenant_scope(), Self::NAMESPACE, &Self::key(scope, host)?, &[], |payload| {
            let mut state = Self::decode(payload, root)?;
            ensure!(state.version == expected, "skill installation version conflict");
            if state.storage_mode == mode { return Ok((state, None)); }
            state.storage_mode = mode;
            state.version = state.version.checked_add(1).context("skill installation version exhausted")?;
            let json = serde_json::to_string(&state)?;
            Ok((state, Some(json)))
        })
    }
}

#[cfg(target_os = "macos")]
#[path = "agent_apps_skill_migration.rs"]
mod migration;
