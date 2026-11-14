//! Receiving projection owned by data management. Payloads/preimages are never logged.
use std::collections::BTreeMap;
use anyhow::{Result, ensure};
use serde_json::Value;
use vw_agent_apps::data_management::shared_snapshot::{SharedSnapshot, DATA_COMPAT_VERSION};
use crate::{global_database::{GlobalDatabase, config::DatabaseProvider}, gateway::settings_store::GatewaySettingsStore};
use super::shared_snapshot::{SnapshotScope, SnapshotSelection};

struct Change { namespace: String, key: String, before: String, after: String }
/// Opaque, executor-local preview. It contains secrets in preimages: do not serialize it.
pub struct SnapshotImport {
    scope: (String, String, String),
    changes: Vec<Change>,
}
impl SnapshotImport {
    pub fn changed_records(&self) -> usize { self.changes.iter().filter(|c| c.before != c.after).count() }
}

/// Selection is trusted local authorization, never deserialized from config.json.
/// Existing device records are required for connection-bearing objects. Missing local
/// setup is an error; receiving metadata must never manufacture working credentials.
pub fn preview_shared_snapshot(database: &GlobalDatabase, scope: SnapshotScope<'_>,
    selected: &SnapshotSelection, bytes: &[u8]) -> Result<SnapshotImport> {
    ensure!(!scope.principal.trim().is_empty() && !scope.host.trim().is_empty(), "authorized receiving scope required");
    ensure!(bytes.len() as u64 <= vw_agent_apps::device_sync::artifact::MAX_ARTIFACT_SIZE, "snapshot exceeds limit");
    let incoming: SharedSnapshot = serde_json::from_slice(bytes).map_err(|_| anyhow::anyhow!("invalid shared snapshot schema"))?;
    ensure!(incoming.data_compat_version == DATA_COMPAT_VERSION, "unsupported shared snapshot version");
    let mut rows: BTreeMap<(String, String), (String, Value)> = BTreeMap::new();
    let mut edit = |namespace: &str, key: String, update: &mut dyn FnMut(&mut Value) -> Result<()>| -> Result<()> {
        let entry = rows.entry((namespace.to_owned(), key.clone()));
        let (_, value) = match entry {
            std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::btree_map::Entry::Vacant(e) => {
                let raw = GatewaySettingsStore::read_existing(database, scope.project.tenant_scope(), namespace, &key)?
                    .ok_or_else(|| anyhow::anyhow!("receiving object requires local setup"))?;
                let value = serde_json::from_str(&raw).map_err(|_| anyhow::anyhow!("invalid receiving record"))?;
                e.insert((raw, value))
            }
        };
        update(value)
    };
    let mut identities = std::collections::BTreeSet::new();
    for p in incoming.providers {
        ensure!(identities.insert(("provider", format!("{:?}:{}",p.app_id,p.id))), "duplicate provider");
        ensure!(selected.providers.contains(&(p.app_id,p.id.clone())), "provider outside selection");
        edit("app_providers.records", format!("{}:{}",serde_json::to_string(&p.app_id)?,p.id), &mut |v| {
            let basic = vw_agent_apps::providers::basic::read(v).map_err(|_| anyhow::anyhow!("invalid receiving provider"))?;
            let mut fields = basic;
            fields.name = p.name.clone();
            let mut updated = vw_agent_apps::providers::basic::apply(p.app_id,v,&fields,&vw_api_types::app_providers::create::ProviderSecretWrite::Keep)
                .map_err(|_| anyhow::anyhow!("invalid shared provider metadata"))?;
            // Native protocol/model projections have app-specific formats. Do not
            // change them without the corresponding local editor adapter.
            ensure!(updated.get("protocol").cloned().unwrap_or(Value::Null) == serde_json::to_value(p.protocol)?, "provider protocol requires local configuration");
            ensure!(updated.get("model").cloned().unwrap_or(Value::Null) == serde_json::to_value(&p.model)?, "provider model requires local configuration");
            std::mem::swap(v,&mut updated); Ok(())
        })?;
    }
    for m in incoming.mcp {
        ensure!(identities.insert(("mcp",m.id.clone())) && selected.mcp.contains(&m.id), "duplicate or unselected MCP");
        edit(&format!("agent_apps.mcp.catalog.v1.project.{}",serde_json::to_string(scope.project.project_id())?), m.id.clone(), &mut |v| {
            ensure!(v["record"]["record"]["id"] == m.id, "receiving MCP identity mismatch");
            ensure!(v["record"]["scope"] == serde_json::to_value(vw_api_types::mcp_management::McpCatalogScope::Project { scope: scope.project.clone() })?, "receiving MCP scope mismatch");
            ensure!(!m.name.trim().is_empty() && m.name.len() <= 128, "invalid MCP name");
            let previous = v["record"].clone();
            v["record"]["name"] = m.name.clone().into();
            v["record"]["description"] = serde_json::to_value(&m.description)?;
            v["record"]["tags"] = serde_json::to_value(&m.tags)?;
            if previous != v["record"] { increment(&mut v["record"]["record"]["revision"])?; }
            Ok(())
        })?;
    }
    for p in incoming.prompts {
        ensure!(identities.insert(("prompt",format!("{:?}:{}",p.app_id,p.id))) && selected.prompts.contains(&(p.app_id,p.id.clone())), "duplicate or unselected prompt");
        edit("agent_apps.prompts.v1",serde_json::to_string(&(scope.project.project_id(),scope.host,p.app_id))?, &mut |v| {
            let mut catalog: super::prompts_store::PromptCatalog = serde_json::from_value(v.clone()).map_err(|_| anyhow::anyhow!("invalid receiving prompt catalog"))?;
            ensure!(catalog.active_id.as_deref() != Some(p.id.as_str()), "deactivate receiving prompt before import");
            ensure!(!p.name.trim().is_empty() && p.name.len() <= 128 && p.content.len() <= 1024*1024, "invalid shared prompt");
            let entry = catalog.entries.iter_mut().find(|e| e.id == p.id).ok_or_else(|| anyhow::anyhow!("receiving prompt requires local setup"))?;
            if entry.name != p.name || entry.description != p.description || entry.content != p.content {
                entry.name=p.name.clone(); entry.description=p.description.clone(); entry.content=p.content.clone();
                catalog.revision=catalog.revision.checked_add(1).ok_or_else(|| anyhow::anyhow!("prompt revision exhausted"))?;
            }
            *v=serde_json::to_value(catalog)?; Ok(())
        })?;
    }
    if let Some(user) = incoming.preferences {
        ensure!(selected.preferences, "preferences outside selection");
        let normalized=vw_agent_apps::apps::preferences::normalize_preferences(user,Default::default(),&Default::default(),vw_agent_apps::apps::preferences::HostPlatform::MacOs)?.user;
        edit("agent_apps_user",scope.principal.to_owned(), &mut |v| { *v=serde_json::to_value(&normalized)?; Ok(()) })?;
    }
    for r in incoming.repositories {
        ensure!(identities.insert(("repository",format!("{}/{}",r.owner,r.name))) && selected.repositories.contains(&(r.owner.clone(),r.name.clone())), "duplicate or unselected repository");
        vw_agent_apps::skills::repos::validate_repo_ref(&r.owner,&r.name,&r.branch)?;
        edit("agent_apps.skills.repos",serde_json::to_string(scope.project.project_id())?, &mut |v| {
            let mut state: crate::gateway::agent_apps_skills_store::SkillReposState=serde_json::from_value(v.clone())?;
            ensure!(state.schema_version==1,"unsupported repository schema");
            let repo=state.repos.iter_mut().find(|e| e.owner==r.owner && e.name==r.name).ok_or_else(||anyhow::anyhow!("repository requires local setup"))?;
            if repo.branch!=r.branch { repo.branch=r.branch.clone(); state.version=state.version.checked_add(1).ok_or_else(||anyhow::anyhow!("repository revision exhausted"))?; }
            *v=serde_json::to_value(state)?; Ok(())
        })?;
    }
    for skill in &incoming.skills {
        ensure!(identities.insert(("skill",skill.id.clone())) && selected.skills.contains(&skill.directory), "duplicate or unselected skill");
        ensure!(skill.id == skill.directory && skill.sha256.len() == 64 && skill.sha256.bytes().all(|b| b.is_ascii_hexdigit()), "invalid shared skill identity");
        // This entry imports configuration only; archive bytes and installation
        // state remain owned by the resource transaction, never by remote JSON.
    }
    for p in incoming.profiles {
        ensure!(identities.insert(("profile",p.id.clone())) && selected.profiles.contains(&p.id), "duplicate or unselected profile");
        edit("project_profiles","state_v1".into(), &mut |v| {
            let slot=v.get_mut("profiles").and_then(|v| v.get_mut(&p.id)).ok_or_else(||anyhow::anyhow!("profile requires local setup"))?;
            let mut profile: vw_api_types::project_profiles::ProjectProfile=serde_json::from_value(slot.clone())?;
            let before=profile.clone();
            ensure!(!p.name.trim().is_empty() && p.name.len()<=128,"invalid profile name");
            profile.name=p.name.clone();
            let mut groups=std::collections::BTreeSet::new();
            for group in &p.groups {
                let app=vw_api_types::project_profiles::ProjectProfileScope::for_app(group.app_id).ok_or_else(||anyhow::anyhow!("unsupported profile app"))?;
                ensure!(groups.insert(format!("{:?}",app)),"duplicate profile group");
                ensure!(group.provider.as_ref().is_none_or(|id| selected.providers.contains(&(group.app_id,id.clone()))),"unmapped profile provider");
                ensure!(group.mcp.as_ref().is_none_or(|ids| ids.iter().all(|id|selected.mcp.contains(id))),"unmapped profile MCP");
                ensure!(group.prompt.as_ref().is_none_or(|id|id.is_empty() || selected.prompts.contains(&(group.app_id,id.clone()))),"unmapped profile prompt");
                let skills=group.skills.as_ref().map(|dirs| dirs.iter().map(|dir| {
                    ensure!(selected.skills.contains(dir),"unmapped profile skill");
                    let mut matches=selected.profile_skill_directories.iter().filter(|(_,d)| *d==dir);
                    let (id,_)=matches.next().ok_or_else(||anyhow::anyhow!("local skill mapping required"))?;
                    ensure!(matches.next().is_none(),"ambiguous local skill mapping");
                    Ok(vw_api_types::project_profiles::ProjectProfileSkillRef { installation_id:id.clone() })
                }).collect::<Result<Vec<_>>>()).transpose()?;
                *profile.payload.providers.get_mut(app)=group.provider.clone();
                *profile.payload.mcp.get_mut(app)=group.mcp.clone();
                *profile.payload.prompts.get_mut(app)=group.prompt.clone();
                *profile.payload.skills.get_mut(app)=skills;
            }
            if before!=profile { profile.version=profile.version.checked_add(1).ok_or_else(||anyhow::anyhow!("profile revision exhausted"))?; }
            *slot=serde_json::to_value(profile)?; Ok(())
        })?;
    }
    let changes=rows.into_iter().map(|((namespace,key),(before,value))| Ok(Change { namespace,key,before,after:serde_json::to_string(&value)? })).collect::<Result<Vec<_>>>()?;
    Ok(SnapshotImport { scope: scope_key(scope)?, changes })
}
fn increment(value: &mut Value) -> Result<()> {
    *value=value.as_u64().and_then(|v|v.checked_add(1)).ok_or_else(||anyhow::anyhow!("record revision exhausted"))?.into(); Ok(())
}
fn scope_key(scope: SnapshotScope<'_>) -> Result<(String,String,String)> {
    Ok((serde_json::to_string(scope.project)?,scope.principal.into(),scope.host.into()))
}

/// Apply only a locally prepared preview after explicit conflict decisions. One
/// database transaction rechecks every preimage before any write. No remote SQL,
/// native activation, credential resolution or filesystem operation is performed.
pub fn apply_shared_snapshot(database: &GlobalDatabase, scope: SnapshotScope<'_>, preview: SnapshotImport) -> Result<usize> {
    ensure!(preview.scope==scope_key(scope)?, "receiving scope changed");
    let tenant=scope.project.tenant_scope();
    let changed=preview.changed_records();
    match database.provider() {
        DatabaseProvider::Sqlite => database.with_sqlite_transaction(rusqlite::TransactionBehavior::Immediate, |tx| {
            for c in &preview.changes {
                let current: String=tx.query_row("SELECT payload_json FROM gateway_settings WHERE platform_id=?1 AND company_id=?2 AND namespace=?3 AND config_key=?4",rusqlite::params![tenant.platform_id().as_ref(),tenant.company_id().as_ref(),c.namespace,c.key],|r|r.get(0))?;
                ensure!(current==c.before,"receiving configuration changed; preview again");
            }
            for c in &preview.changes { if c.before!=c.after {
                tx.execute("UPDATE gateway_settings SET payload_json=?5 WHERE platform_id=?1 AND company_id=?2 AND namespace=?3 AND config_key=?4",rusqlite::params![tenant.platform_id().as_ref(),tenant.company_id().as_ref(),c.namespace,c.key,c.after])?;
            }} Ok(())
        })?,
        DatabaseProvider::Postgres => {
            let mut conn=database.connect_postgres()?;
            let mut tx=conn.transaction()?;
            for c in &preview.changes {
                let row=tx.query_one("SELECT payload_json FROM gateway_settings WHERE platform_id=$1 AND company_id=$2 AND namespace=$3 AND config_key=$4 FOR UPDATE",&[&tenant.platform_id().as_ref(),&tenant.company_id().as_ref(),&c.namespace,&c.key])?;
                ensure!(row.get::<_,String>(0)==c.before,"receiving configuration changed; preview again");
            }
            for c in &preview.changes { if c.before!=c.after {
                tx.execute("UPDATE gateway_settings SET payload_json=$5 WHERE platform_id=$1 AND company_id=$2 AND namespace=$3 AND config_key=$4",&[&tenant.platform_id().as_ref(),&tenant.company_id().as_ref(),&c.namespace,&c.key,&c.after])?;
            }} tx.commit()?;
        }
    }
    Ok(changed)
}

/// Complete configuration preimages for the exact receiving rows. Native files
/// are untouched by this import; activation is explicitly a separate operation.
impl SnapshotImport {
    pub(crate) fn recovery_preimages(&self) -> Vec<vw_agent_apps::data_management::safety_snapshot::OwnerPreimage> {
        use vw_agent_apps::data_management::safety_snapshot::*;
        use sha2::{Digest, Sha256};
        self.changes.iter().map(|c| OwnerPreimage {
            target: SnapshotTarget { category: match c.namespace.as_str() {
                "app_providers.records" => SnapshotCategory::Provider,
                "agent_apps.prompts.v1" => SnapshotCategory::Prompt,
                "agent_apps_user" => SnapshotCategory::Preference,
                "project_profiles" => SnapshotCategory::Profile,
                "agent_apps.skills.repos" | "agent_apps.skills.installed" => SnapshotCategory::Skill,
                _ => SnapshotCategory::Mcp,
            }, object_id: serde_json::to_string(&(&c.namespace, &c.key)).expect("string tuple") },
            state: Some(PresentPreimage { revision: hex::encode(Sha256::digest(c.before.as_bytes())),
                bytes: c.before.as_bytes().to_vec() }),
        }).collect()
    }
}

impl SnapshotImport {
    /// Trusted installation-owner delta participates in the same preimage CAS
    /// transaction as the portable configuration, never a remote namespace/key.
    pub(crate) fn attach_skill_state(&mut self, key: String, before: String, after: String) {
        self.changes.push(Change { namespace: "agent_apps.skills.installed".into(), key, before, after });
    }
}
