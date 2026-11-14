//! Data-management owner: selected settings from one database snapshot plus locked resources.
use std::{collections::{BTreeMap, BTreeSet}, path::Path};
use anyhow::{Result, ensure};
use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};
use vw_agent_apps::data_management::shared_snapshot::*;
use vw_api_types::{agent_apps::AppId, id::ProjectScope};
use crate::global_database::{GlobalDatabase, config::DatabaseProvider};

/// Trusted executor selection, after authorization of every object and confirmation
/// that selected free text/resources may leave this device. Never deserialize this
/// structure from a remote snapshot. Empty lists select nothing, never everything.
#[derive(Default)]
pub struct SnapshotSelection {
    pub providers: Vec<(AppId, String)>,
    pub mcp: Vec<String>,
    pub prompts: Vec<(AppId, String)>,
    pub preferences: bool,
    pub repositories: Vec<(String, String)>,
    pub skills: Vec<String>,
    pub profiles: Vec<String>,
    /// Existing profile resolver's authorized opaque skill ID -> SSOT directory.
    pub profile_skill_directories: BTreeMap<String, String>,
}

/// Local version material deliberately has neither Debug nor Serialize.
pub struct CapturedSnapshot {
    pub config: SharedSnapshot,
    pub skills: vw_agent_apps::device_sync::archive_pack::PackedSkillsArchive,
    pub source_versions: BTreeMap<(String, String), Option<[u8; 32]>>,
}

#[derive(Clone, Copy)]
pub struct SnapshotScope<'a> {
    pub project: &'a ProjectScope,
    pub principal: &'a str,
    pub host: &'a str,
}

/// macOS only: matches the existing native skill owner support boundary.
#[cfg(target_os = "macos")]
pub fn capture_shared_snapshot(
    database: &GlobalDatabase, scope: SnapshotScope<'_>, selection: &SnapshotSelection,
    ssot: &Path,
) -> Result<CapturedSnapshot> {
    use std::{fs::OpenOptions, os::{fd::AsRawFd, unix::fs::{MetadataExt, OpenOptionsExt}}};
    ensure!(!scope.principal.trim().is_empty() && !scope.host.trim().is_empty(), "snapshot requires authorized executor scope");
    ensure!(ssot.is_absolute(), "snapshot requires a trusted absolute skill root");
    // Same physical-root flock as install/update/uninstall, before database access.
    // No creation, staging or database initialization on this read path.
    let root = OpenOptions::new().read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC).open(ssot)?;
    ensure!(unsafe { libc::flock(root.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) } == 0,
        "skill resources are busy");
    let identity = root.metadata()?;
    let capture = |read: &mut dyn FnMut(&str, &str) -> Result<Option<String>>| -> Result<CapturedSnapshot> {
        let captured = project(scope, selection, ssot, read)?;
        let current = std::fs::symlink_metadata(ssot)?;
        ensure!(current.is_dir() && current.dev() == identity.dev() && current.ino() == identity.ino(),
            "skill root changed during capture");
        Ok(captured)
    };
    match database.provider() {
        DatabaseProvider::Sqlite => {
            let mut connection = rusqlite::Connection::open_with_flags(database.sqlite_path()?, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            let transaction = connection.transaction()?;
            let result = capture(&mut |namespace, key| Ok(transaction.query_row(
                "SELECT payload_json FROM gateway_settings WHERE platform_id=?1 AND company_id=?2 AND namespace=?3 AND config_key=?4",
                params![scope.project.tenant_scope().platform_id().as_ref(), scope.project.tenant_scope().company_id().as_ref(), namespace, key],
                |row| row.get(0)).optional()?))?;
            transaction.commit()?;
            Ok(result)
        }
        DatabaseProvider::Postgres => {
            let mut connection = database.connect_postgres()?;
            let mut transaction = connection.build_transaction()
                .isolation_level(postgres::IsolationLevel::RepeatableRead).read_only(true).start()?;
            let result = capture(&mut |namespace, key| Ok(transaction.query_opt(
                "SELECT payload_json FROM gateway_settings WHERE platform_id=$1 AND company_id=$2 AND namespace=$3 AND config_key=$4",
                &[&scope.project.tenant_scope().platform_id().as_ref(), &scope.project.tenant_scope().company_id().as_ref(), &namespace, &key])?
                .map(|row| row.get(0))))?;
            transaction.commit()?;
            Ok(result)
        }
    }
}

#[cfg(target_os = "macos")]
fn project(scope: SnapshotScope<'_>, selected: &SnapshotSelection, root: &Path,
    read: &mut dyn FnMut(&str, &str) -> Result<Option<String>>) -> Result<CapturedSnapshot> {
    let count = selected.providers.len() + selected.mcp.len() + selected.prompts.len()
        + selected.repositories.len() + selected.skills.len() + selected.profiles.len();
    ensure!(count <= 10_000, "too many selected snapshot objects");
    for id in selected.providers.iter().map(|(_, id)| id)
        .chain(selected.prompts.iter().map(|(_, id)| id)).chain(selected.mcp.iter()).chain(selected.profiles.iter()) {
        vw_agent_apps::providers::validate_identity(id).map_err(|_| anyhow::anyhow!("invalid selected snapshot identity"))?;
    }
    let mut versions = BTreeMap::new();
    let mut get = |namespace: &str, key: &str| -> Result<Option<String>> {
        let raw = read(namespace, key)?;
        ensure!(raw.as_ref().is_none_or(|raw| raw.len() <= 16 * 1024 * 1024), "selected record exceeds capture limit");
        versions.insert((namespace.to_owned(), key.to_owned()), raw.as_ref().map(|raw| Sha256::digest(raw.as_bytes()).into()));
        Ok(raw)
    };
    let mut config = SharedSnapshot { data_compat_version: DATA_COMPAT_VERSION,
        providers: vec![], mcp: vec![], prompts: vec![], preferences: None,
        skills: vec![], repositories: vec![], profiles: vec![] };
    let required = |raw: Option<String>| raw.ok_or_else(|| anyhow::anyhow!("selected snapshot object is missing"));
    for (app, id) in &selected.providers {
        let key = format!("{}:{id}", serde_json::to_string(app)?);
        let raw = required(get("app_providers.records", &key)?)?;
        let value: serde_json::Value = serde_json::from_str(&raw).map_err(|_| anyhow::anyhow!("invalid provider record"))?;
        let basic = vw_agent_apps::providers::basic::read(&value).map_err(|_| anyhow::anyhow!("invalid provider record"))?;
        config.providers.push(SharedProvider { id: id.clone(), app_id: *app, name: basic.name,
            protocol: value.get("protocol").filter(|v| !v.is_null()).map(|v| serde_json::from_value(v.clone())).transpose().map_err(|_| anyhow::anyhow!("invalid provider protocol"))?,
            model: value.get("model").filter(|v| !v.is_null()).map(|v| v.as_str().map(str::to_owned).ok_or_else(|| anyhow::anyhow!("invalid provider model"))).transpose()? });
    }
    // The project scope is fixed here; remote input can never select another namespace.
    let mcp_namespace = format!("agent_apps.mcp.catalog.v1.project.{}", serde_json::to_string(scope.project.project_id())?);
    for id in &selected.mcp {
        let raw = required(get(&mcp_namespace, id)?)?;
        #[derive(serde::Deserialize)]
        struct Stored { record: vw_api_types::mcp_management::McpCatalogRecord }
        let stored: Stored = serde_json::from_str(&raw).map_err(|_| anyhow::anyhow!("invalid or retired MCP record"))?;
        ensure!(stored.record.record.id.0 == *id, "MCP identity mismatch");
        ensure!(stored.record.scope == vw_api_types::mcp_management::McpCatalogScope::Project { scope: scope.project.clone() }, "MCP scope mismatch");
        config.mcp.push(SharedMcp { id: id.clone(), name: stored.record.name,
            description: stored.record.description, tags: stored.record.tags });
    }
    for (app, id) in &selected.prompts {
        ensure!(*app != AppId::ClaudeDesktop, "unsupported prompt application");
        let key = serde_json::to_string(&(scope.project.project_id(), scope.host, app))?;
        let raw = required(get("agent_apps.prompts.v1", &key)?)?;
        let catalog: super::prompts_store::PromptCatalog = serde_json::from_str(&raw).map_err(|_| anyhow::anyhow!("invalid prompt catalog"))?;
        let entry = catalog.entries.into_iter().find(|entry| entry.id == *id).ok_or_else(|| anyhow::anyhow!("selected prompt is missing"))?;
        config.prompts.push(SharedPrompt { id: id.clone(), app_id: *app, name: entry.name,
            description: entry.description, content: entry.content });
    }
    if selected.preferences {
        let user = match get("agent_apps_user", scope.principal)? {
            Some(raw) => serde_json::from_str(&raw).map_err(|_| anyhow::anyhow!("invalid user preferences"))?,
            None => vw_agent_apps::apps::preferences::default_user_preferences(),
        };
        use vw_agent_apps::apps::preferences::{normalize_preferences, HostPlatform};
        config.preferences = Some(normalize_preferences(user, Default::default(), &Default::default(), HostPlatform::MacOs)?.user);
    }
    if !selected.repositories.is_empty() {
        let key = serde_json::to_string(scope.project.project_id())?;
        let raw = required(get("agent_apps.skills.repos", &key)?)?;
        let state: crate::gateway::agent_apps_skills_store::SkillReposState = serde_json::from_str(&raw).map_err(|_| anyhow::anyhow!("invalid skill repository state"))?;
        ensure!(state.schema_version == 1, "unsupported skill repository schema");
        for (owner, name) in &selected.repositories {
            let repo = state.repos.iter().find(|r| &r.owner == owner && &r.name == name).ok_or_else(|| anyhow::anyhow!("selected repository is missing"))?;
            vw_agent_apps::skills::repos::validate_repo_ref(&repo.owner, &repo.name, &repo.branch)?;
            config.repositories.push(SharedRepository { owner: repo.owner.clone(), name: repo.name.clone(), branch: repo.branch.clone() });
        }
    }
    if !selected.skills.is_empty() {
        let key = serde_json::to_string(&(scope.project.project_id(), scope.host))?;
        let raw = required(get("agent_apps.skills.installed", &key)?)?;
        let state: crate::gateway::agent_apps_skills_store::SkillInstallsState = serde_json::from_str(&raw).map_err(|_| anyhow::anyhow!("invalid skill installation state"))?;
        ensure!(state.schema_version == 1 && std::fs::canonicalize(&state.ssot_root)? == std::fs::canonicalize(root)?, "skill root or schema mismatch");
        for directory in &selected.skills {
            ensure!(state.skills.iter().any(|skill| &skill.directory == directory), "selected skill is not registered");
            config.skills.push(SharedSkill { id: directory.clone(), directory: directory.clone(), sha256: vw_agent_apps::skills::inventory::compute_dir_hash(&root.join(directory))? });
        }
    }
    if !selected.profiles.is_empty() {
        let raw = required(get("project_profiles", "state_v1")?)?;
        #[derive(serde::Deserialize)]
        struct Profiles { profiles: BTreeMap<String, vw_api_types::project_profiles::ProjectProfile> }
        let state: Profiles = serde_json::from_str(&raw).map_err(|_| anyhow::anyhow!("invalid profile state"))?;
        use vw_api_types::project_profiles::ProjectProfileScope;
        for id in &selected.profiles {
            let profile = state.profiles.get(id).ok_or_else(|| anyhow::anyhow!("selected profile is missing"))?;
            ensure!(profile.id.0 == *id, "profile identity mismatch");
            let mut groups = vec![];
            for group in [ProjectProfileScope::Claude, ProjectProfileScope::ClaudeDesktop, ProjectProfileScope::Codex] {
                let app = group.app_id();
                let provider = profile.payload.providers.get(group).clone();
                ensure!(provider.as_ref().is_none_or(|id| config.providers.iter().any(|p| p.app_id == app && p.id == *id)), "profile provider is not selected");
                let mcp = profile.payload.mcp.get(group).clone();
                ensure!(mcp.as_ref().is_none_or(|ids| ids.iter().all(|id| config.mcp.iter().any(|m| m.id == *id))), "profile MCP is not selected");
                let prompt = profile.payload.prompts.get(group).clone();
                ensure!(prompt.as_ref().is_none_or(|id| id.is_empty() || config.prompts.iter().any(|p| p.app_id == app && p.id == *id)), "profile prompt is not selected");
                let skills = profile.payload.skills.get(group).as_ref().map(|refs| refs.iter().map(|r| {
                    let directory = selected.profile_skill_directories.get(&r.installation_id).ok_or_else(|| anyhow::anyhow!("profile skill requires authorized reference resolution"))?;
                    ensure!(selected.skills.contains(directory), "profile skill is not selected");
                    Ok(directory.clone())
                }).collect::<Result<Vec<_>>>()).transpose()?;
                groups.push(SharedProfileGroup { app_id: app, provider, mcp, skills, prompt });
            }
            config.profiles.push(SharedProfile { id: id.clone(), name: profile.name.clone(), groups });
        }
    }
    // Reject duplicate logical identities rather than publishing ambiguous config.
    let mut unique = BTreeSet::new();
    for (kind, ids) in [("provider", config.providers.iter().map(|p| format!("{:?}:{}", p.app_id, p.id)).collect::<Vec<_>>()),
        ("prompt", config.prompts.iter().map(|p| format!("{:?}:{}", p.app_id, p.id)).collect()),
        ("repository", selected.repositories.iter().map(|(owner, name)| format!("{owner}/{name}")).collect()),
        ("mcp", selected.mcp.clone()), ("skill", selected.skills.clone()), ("profile", selected.profiles.clone())] {
        for id in ids { ensure!(unique.insert((kind, id)), "duplicate selected snapshot identity"); }
    }
    let skills = vw_agent_apps::device_sync::archive_pack::pack_selected_skills_archive(root, &selected.skills).map_err(|_| anyhow::anyhow!("selected skill capture failed"))?;
    Ok(CapturedSnapshot { config, skills, source_versions: versions })
}
