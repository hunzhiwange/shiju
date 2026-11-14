//! Ledger-scoped, host-executor-only official history restoration.
//! Caller retains lifecycle exclusion against native writers and other executors;
//! do not hold the non-reentrant Codex AppWriteGuard when invoking these functions.
use super::{
    rollout, state_db,
    unify::{BackupGeneration, GenerationBinding},
};
use crate::sessions::{rollout as rollout_files, source::SourceRegistry};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Read},
    path::PathBuf,
};
use toml_edit::DocumentMut;
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

#[derive(Default)]
pub struct RestoreOutcome {
    pub restored_rollouts: usize,
    pub restored_state_rows: usize,
    pub skipped_reason: Option<&'static str>,
}

/// All bindings and paths are resolved and authorized on the execution host.
/// `generations` contains only unification ledgers, never restoration backups;
/// other instance/root generations are ignored. `undo` is a separate private
/// generation retaining the current bytes before any rewrite. It must be unique
/// per restore cycle and retained across retries. No full image is published over
/// live history: newer messages, threads and unrelated provider rows survive.
#[allow(clippy::too_many_arguments)]
pub fn restore(
    registry: &SourceRegistry,
    binding: &GenerationBinding,
    generations: &[BackupGeneration],
    undo: &mut BackupGeneration,
    unify_enabled: bool,
    live_config: &str,
    identities: &[ExternalSessionIdentity],
    database_paths: &[PathBuf],
    mut still_disabled: impl FnMut() -> io::Result<bool>,
    mut advanced: impl FnMut(),
) -> io::Result<RestoreOutcome> {
    let skip = |reason| Ok(RestoreOutcome { skipped_reason: Some(reason), ..Default::default() });
    if unify_enabled {
        return skip("unify_toggle_on");
    }
    let config = live_config.parse::<DocumentMut>().map_err(|_| invalid())?;
    if config.get("model_provider").and_then(|v| v.as_str()).map(str::trim)
        == Some("cc-switch-official")
    {
        return skip("legacy_route_in_use");
    }
    if undo.metadata().binding != *binding {
        return Err(invalid());
    }
    let scoped_ids = validate(registry, binding, identities)?;
    let mut sessions = BTreeSet::new();
    let mut threads: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();
    // Verify every contributing blob before changing the first live source.
    for generation in generations {
        if generation.metadata().binding != *binding {
            continue;
        }
        if generation.same_directory(undo)? {
            return Err(invalid());
        }
        for entry in &generation.metadata().entries {
            let image = generation.read_image(&entry.image)?;
            if entry.state_database {
                // Databases may live outside CODEX_HOME, but only an explicitly
                // authorized current database candidate can contribute a ledger.
                if !database_paths.contains(&entry.source) {
                    continue;
                }
                let wal =
                    entry.wal.as_deref().map(|name| generation.read_image(name)).transpose()?;
                threads
                    .entry(entry.source.clone())
                    .or_default()
                    .extend(database_ledger(&image, wal.as_deref())?);
            } else {
                if !entry.source.starts_with(&binding.codex_root)
                    || !rollout_files::is_rollout_file(&entry.source)
                    || entry.wal.is_some()
                {
                    return Err(invalid());
                }
                let decoded = if rollout_files::is_compressed(&entry.source) {
                    bounded(zstd::stream::read::Decoder::new(image.as_slice())?)?
                } else {
                    image
                };
                sessions.extend(rollout_ledger(&decoded)?);
            }
        }
    }
    sessions.retain(|id| scoped_ids.contains(id));
    for ids in threads.values_mut() {
        ids.retain(|id| scoped_ids.contains(id));
    }
    threads.retain(|_, ids| !ids.is_empty());
    if sessions.is_empty() && threads.is_empty() {
        return skip("no_backup_ledger");
    }
    let sources = BTreeSet::from(["custom".to_owned()]);
    let mut outcome = RestoreOutcome::default();
    for identity in identities {
        if !still_disabled()? {
            outcome.skipped_reason = Some("unify_reenabled_during_restore");
            return Ok(outcome);
        }
        if matches!(&identity.source, SessionSource::File { .. })
            && sessions.contains(&identity.session_id.0)
            && rollout::migrate(registry, identity, &sources, "openai", |source, bytes| {
                undo.backup(source.path(), bytes, None, false)
            })?
        {
            outcome.restored_rollouts += 1;
        }
        advanced();
    }
    for (path, ids) in &threads {
        if !still_disabled()? {
            outcome.skipped_reason = Some("unify_reenabled_during_restore");
            return Ok(outcome);
        }
        outcome.restored_state_rows +=
            state_db::migrate(path, ids, &sources, "openai", |path, backup| {
                undo.backup(path, &backup.database, backup.wal.as_deref(), true)
            })?;
        advanced();
    }
    if outcome.restored_rollouts == 0 && outcome.restored_state_rows == 0 {
        outcome.skipped_reason = Some("nothing_to_restore");
    }
    Ok(outcome)
}

/// Legacy official profile cleanup is deliberately separate from custom-ledger
/// restoration. Never migrate its history while that profile is still selected.
/// Caller supplies freshly scoped identities and an independently retained backup.
#[allow(clippy::too_many_arguments)]
pub fn migrate_legacy(
    registry: &SourceRegistry,
    binding: &GenerationBinding,
    backup: &mut BackupGeneration,
    live_config: &str,
    identities: &[ExternalSessionIdentity],
    database_paths: &[PathBuf],
    mut still_allowed: impl FnMut() -> io::Result<bool>,
    mut advanced: impl FnMut(),
) -> io::Result<RestoreOutcome> {
    let config = live_config.parse::<DocumentMut>().map_err(|_| invalid())?;
    if config.get("model_provider").and_then(|v| v.as_str()).map(str::trim)
        == Some("cc-switch-official")
    {
        return Ok(RestoreOutcome {
            skipped_reason: Some("legacy_route_in_use"),
            ..Default::default()
        });
    }
    if backup.metadata().binding != *binding {
        return Err(invalid());
    }
    let threads = validate(registry, binding, identities)?;
    let sources = BTreeSet::from(["cc-switch-official".to_owned()]);
    let mut outcome = RestoreOutcome::default();
    for identity in identities {
        if !still_allowed()? {
            outcome.skipped_reason = Some("maintenance_state_changed");
            return Ok(outcome);
        }
        if matches!(&identity.source, SessionSource::File { .. })
            && rollout::migrate(registry, identity, &sources, "openai", |source, bytes| {
                backup.backup(source.path(), bytes, None, false)
            })?
        {
            outcome.restored_rollouts += 1;
        }
        advanced();
    }
    for path in database_paths {
        if !still_allowed()? {
            outcome.skipped_reason = Some("maintenance_state_changed");
            return Ok(outcome);
        }
        outcome.restored_state_rows +=
            state_db::migrate(path, &threads, &sources, "openai", |path, image| {
                backup.backup(path, &image.database, image.wal.as_deref(), true)
            })?;
        advanced();
    }
    Ok(outcome)
}

fn validate(
    registry: &SourceRegistry,
    binding: &GenerationBinding,
    identities: &[ExternalSessionIdentity],
) -> io::Result<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for identity in identities {
        if identity.provider != SessionProvider::Codex
            || identity.scope != binding.scope
            || identity.target_host != binding.target_host
            || identity.instance_id != binding.instance_id
        {
            return Err(invalid());
        }
        let source = registry.resolve(identity)?;
        if matches!(&identity.source, SessionSource::File { .. })
            && !source.path().starts_with(&binding.codex_root)
        {
            return Err(invalid());
        }
        ids.insert(identity.session_id.0.clone());
    }
    Ok(ids)
}
fn rollout_ledger(bytes: &[u8]) -> io::Result<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    let mut found = false;
    for line in bytes.split(|b| *b == b'\n') {
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        if value["type"].as_str() != Some("session_meta") {
            continue;
        }
        if found {
            return Err(invalid());
        }
        found = true;
        if value["payload"]["model_provider"].as_str() == Some("openai") {
            let id =
                value["payload"]["id"].as_str().filter(|id| !id.is_empty()).ok_or_else(invalid)?;
            ids.insert(id.to_owned());
        }
    }
    if !found {
        return Err(invalid());
    }
    Ok(ids)
}
fn database_ledger(image: &[u8], wal: Option<&[u8]>) -> io::Result<BTreeSet<String>> {
    // Fresh private directory: None means physically absent WAL, and SHM is
    // always absent. SQLite reconstructs its transient index from the saved WAL.
    // Never copy the current DB's WAL/SHM into this original snapshot.
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("ledger.sqlite");
    std::fs::write(&path, image)?;
    if let Some(wal) = wal {
        std::fs::write(directory.path().join("ledger.sqlite-wal"), wal)?;
    }
    let connection = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|_| invalid())?;
    let mut query = connection
        .prepare("SELECT id FROM threads WHERE model_provider = 'openai'")
        .map_err(|_| invalid())?;
    let rows = query.query_map([], |row| row.get::<_, String>(0)).map_err(|_| invalid())?;
    rows.map(|row| row.map_err(|_| invalid())).collect()
}
fn bounded(reader: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(512 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 512 * 1024 * 1024 {
        return Err(invalid());
    }
    Ok(bytes)
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid Codex official restore ledger or binding")
}
