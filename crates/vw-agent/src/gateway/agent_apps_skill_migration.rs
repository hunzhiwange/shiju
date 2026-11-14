//! Category commit through the existing project/host skill settings owner.
use super::*;
use std::{collections::BTreeMap, path::Path};
use vw_agent_apps::{
    data_management::{
        preview::Decision,
        skills::{SkillMigration, SkillsPreview},
        source::{SourceScope, SourceSnapshot},
    },
    skills::bulk_sync::SkillSyncTarget,
};

impl SkillInstallsStore {
    /// Call only after authorizing migration, resource reads and every application
    /// destination for this principal/project/host. Preview alone grants no writes.
    pub fn preview_migration(
        &self,
        scope: &SourceScope,
        source: &SourceSnapshot,
        root: &Path,
        resources: &Path,
    ) -> Result<SkillsPreview> {
        let state = self.get(scope.project(), scope.host_id(), root)?;
        let mut existing = state.migrated.clone();
        let revision = migration_revision(&state)?;
        // Existing installations without migration provenance still conflict.
        // Never treat them as Added and overwrite without a replace decision.
        for skill in &state.skills {
            if !existing.contains_key(&skill.directory) {
                existing.insert(
                    skill.directory.clone(),
                    vw_agent_apps::data_management::skills::MigratedSkill {
                        id: skill.directory.clone(),
                        directory: skill.directory.clone(),
                        name: skill.name.clone(),
                        description: skill.description.clone(),
                        repository: state.repo_sources.get(&skill.directory).cloned(),
                        readme_url: None,
                        installed_at: 0,
                        updated_at: 0,
                        content_hash: String::new(),
                        enabled: BTreeMap::new(),
                    },
                );
            }
        }
        for detail in
            vw_agent_apps::skills::inventory::get_all_installed(&state.ssot_root, &state.skills)?
        {
            if let Some(record) = existing.get_mut(&detail.skill.directory) {
                record.content_hash =
                    detail.content_hash.context("installed skill resource missing")?;
                record.name = detail.skill.name;
                record.description = detail.skill.description;
            }
        }
        Ok(SkillsPreview::prepare(scope, source, revision, resources, &existing)?)
    }

    pub fn migrate_skills(
        &self,
        scope: &SourceScope,
        source: &SourceSnapshot,
        root: &Path,
        snapshot_root: &Path,
        plan: &SkillsPreview,
        decisions: &[(uuid::Uuid, Decision)],
        targets: &BTreeMap<String, SkillSyncTarget>,
    ) -> Result<SkillInstallsState> {
        let key = Self::key(scope.project(), scope.host_id())?;
        let root = std::fs::canonicalize(root)?;
        let mut operation = None;
        let committed = self.0.update_with_preimages(
            scope.project().tenant_scope(),
            Self::NAMESPACE,
            &key,
            &[],
            |payload| {
                let mut state = Self::decode(payload, &root)?;
                let selected =
                    plan.selected(scope, source, &migration_revision(&state)?, decisions)?;
                if selected.is_empty() {
                    return Ok((state, None));
                }
                operation = Some(SkillMigration::prepare(
                    &root,
                    plan,
                    &selected,
                    &state.skills,
                    &state.copy_bindings,
                    targets,
                )?);
                let change = operation.as_mut().context("missing skill migration")?;
                // Recheck filesystem observations after the batch has acquired its locks.
                plan.selected(scope, source, &migration_revision(&state)?, decisions)?;
                let receipt = capture_migration(snapshot_root, scope, &state, &selected)?;
                // Keep the durable receipt in the same owner transaction as the mutation.
                state.migration_snapshot = Some(receipt);
                for record in &selected {
                    ensure!(
                        !state
                            .migrated
                            .values()
                            .any(|old| old.id == record.id && old.directory != record.directory),
                        "skill source identity conflict"
                    );
                    state.migrated.insert(record.directory.clone(), (**record).clone());
                    match &record.repository {
                        Some(repo) => {
                            state.repo_sources.insert(record.directory.clone(), repo.clone());
                        }
                        None => {
                            state.repo_sources.remove(&record.directory);
                        }
                    }
                    state.update_metadata.remove(&record.directory);
                }
                state.skills = change.skills.clone();
                state.copy_bindings = change.bindings.clone();
                state.version = state.version.checked_add(1).context("skill version exhausted")?;
                let json = serde_json::to_string(&state)?;
                change.publish()?;
                Ok((state, Some(json)))
            },
        );
        match committed {
            Ok(state) => {
                if let Some(change) = operation {
                    change.finish().context(
                        "skill migration committed; quarantine cleanup requires recovery",
                    )?;
                }
                Ok(state)
            }
            Err(error) => {
                if let Some(change) = operation.as_mut() {
                    change.rollback().context("skill migration rollback requires recovery")?;
                }
                Err(error)
            }
        }
    }
}

fn migration_revision(state: &SkillInstallsState) -> Result<String> {
    use sha2::{Digest, Sha256};
    let inventory =
        vw_agent_apps::skills::inventory::get_all_installed(&state.ssot_root, &state.skills)?;
    let mut content = Vec::new();
    for detail in inventory {
        content.push((detail.skill.directory, detail.content_hash));
    }
    let mut projections = Vec::new();
    for (key, binding) in &state.copy_bindings {
        projections.push((
            key,
            vw_agent_apps::skills::copy_deploy::binding_fingerprint(
                &binding.destination,
                binding.link_target.as_deref(),
            )?,
        ));
    }
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&(state, content, projections))?)))
}

fn capture_migration(
    destination: &Path,
    scope: &SourceScope,
    state: &SkillInstallsState,
    selected: &[&vw_agent_apps::data_management::skills::MigratedSkill],
) -> Result<vw_agent_apps::data_management::safety_snapshot::SafetySnapshotReceipt> {
    use std::collections::BTreeSet;
    use vw_agent_apps::data_management::safety_snapshot::*;
    let mut paths = BTreeSet::new();
    for record in selected {
        if state.skills.iter().any(|s| s.directory == record.directory) {
            paths.insert(state.ssot_root.join(&record.directory));
        }
        for binding in state.copy_bindings.values().filter(|b| b.directory == record.directory) {
            paths.insert(binding.destination.clone());
        }
    }
    let bytes = vw_agent_apps::data_management::skills::capture_skill_resources(
        paths,
        &serde_json::to_vec(state)?,
    )?;
    let target = SnapshotTarget {
        category: SnapshotCategory::Skill,
        object_id: "installed-category".into(),
    };
    let image = OwnerPreimage {
        target: target.clone(),
        state: Some(PresentPreimage { revision: migration_revision(state)?, bytes }),
    };
    Ok(create_safety_snapshot(
        destination,
        &SafetySnapshotScope {
            project: scope.project().clone(),
            principal_id: scope.principal_id().to_owned(),
            host_id: scope.host_id().to_owned(),
        },
        &[target],
        || Ok(vec![image]),
    )?)
}
