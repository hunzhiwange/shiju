//! Receiving resources uses the existing installation state and deployment owner.
use anyhow::{Result, Context, ensure};
use crate::{gateway::{agent_apps_skills_store::{SkillInstallsStore, SkillInstallsState}, settings_store::GatewaySettingsStore}, global_database::GlobalDatabase};
use vw_api_types::id::ProjectScope;
use std::{collections::{BTreeMap, BTreeSet}, path::Path};
use vw_agent_apps::{data_management::{shared_snapshot::SharedSkill, safety_snapshot::*}, device_sync::skill_update::BatchSkillUpdate, skills::update_apply::SkillUpdateMetadata};

pub(crate) struct PreparedSyncSkills {
    pub updates: Vec<BatchSkillUpdate>,
    pub key: String,
    pub before: String,
    pub after: String,
    pub resources: OwnerPreimage,
}
pub(crate) fn prepare_sync_receive(database: &GlobalDatabase, scope: &ProjectScope, host: &str,
        root: &Path, staging: &Path, incoming: &[SharedSkill], local: &[SharedSkill],
    ) -> Result<PreparedSyncSkills> {
        let key = serde_json::to_string(&(scope.project_id(), host))?;
        let before = GatewaySettingsStore::read_existing(database, scope.tenant_scope(), "agent_apps.skills.installed", &key)?.context("receiving skills require saved installation state")?;
        let mut state = SkillInstallsStore::new(database.clone())?.get(scope, host, root)?;
        ensure!(state == serde_json::from_str::<SkillInstallsState>(&before)?, "skill state changed during capture");
        let mut hashes = BTreeMap::new();
        let mut saved = Vec::new();
        let mut paths = BTreeSet::new();
        for skill in incoming {
            ensure!(hashes.insert(skill.directory.clone(), local.iter().find(|s| s.directory == skill.directory)
                .context("skill outside local preview")?.sha256.clone()).is_none(), "duplicate skill");
            saved.push(state.skills.iter().find(|s| s.directory == skill.directory).context("skill requires local installation")?.clone());
            paths.insert(state.ssot_root.join(&skill.directory));
            for binding in state.copy_bindings.values().filter(|b| b.directory == skill.directory) {
                paths.insert(binding.destination.clone());
            }
        }
        let updates = BatchSkillUpdate::prepare_batch(&state.ssot_root, staging, &saved, &hashes, &state.copy_bindings)?;
        // The prepared updates retain every physical root lease through commit.
        let bytes = vw_agent_apps::data_management::skills::capture_skill_resources(paths, before.as_bytes())?;
        use sha2::{Digest, Sha256};
        let resources = OwnerPreimage { target: SnapshotTarget { category: SnapshotCategory::Skill,
            object_id: format!("resources:{key}") }, state: Some(PresentPreimage {
                revision: hex::encode(Sha256::digest(&bytes)), bytes }) };
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
        for update in &updates {
            let expected = incoming.iter().find(|s| s.directory == update.skill.directory).context("unexpected skill update")?;
            ensure!(expected.sha256 == update.content_hash, "skill content differs from manifest");
            *state.skills.iter_mut().find(|s| s.directory == update.skill.directory).context("skill disappeared")? = update.skill.clone();
            state.update_metadata.insert(update.skill.directory.clone(), SkillUpdateMetadata { content_hash: update.content_hash.clone(), updated_at: now });
            for (key, binding) in update.bindings() { state.copy_bindings.insert(key.to_owned(), binding.clone()); }
        }
        state.version = state.version.checked_add(1).context("skill version exhausted")?;
        Ok(PreparedSyncSkills { updates, key, before, after: serde_json::to_string(&state)?, resources })
    }
