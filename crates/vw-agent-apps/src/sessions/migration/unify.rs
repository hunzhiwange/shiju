//! Official-history unification on the authorized execution host.
//! The caller owns native-writer lifecycle exclusion across this entire operation,
//! including retries and ledger access. Never hold an AppWriteGuard when calling.
use super::{rollout, state_db};
use crate::{
    providers::{live::write_guard::prepare_private_child, paths::open_beneath},
    sessions::source::SourceRegistry,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{self, Read},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use toml_edit::DocumentMut;
use vw_api_types::{
    external_session::{ExternalSessionIdentity, SessionProvider, SessionSource},
    id::ProjectScope,
};

/// Executor-resolved binding, never constructed from client paths. A generation
/// directory is private, precreated by the authorized maintenance owner, unique
/// per enable cycle, and retained for retries. New cycles use new directories.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationBinding {
    pub scope: ProjectScope,
    pub target_host: String,
    pub instance_id: String,
    pub codex_root: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupEntry {
    pub source: PathBuf,
    /// Original physical bytes, including compressed rollout representation.
    pub image: String,
    /// None explicitly means WAL absent. Restore must delete a later WAL and SHM.
    pub wal: Option<String>,
    pub state_database: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationMeta {
    pub version: u32,
    pub binding: GenerationBinding,
    pub complete: bool,
    pub entries: Vec<BackupEntry>,
}

pub struct BackupGeneration {
    directory: File,
    meta: GenerationMeta,
}
impl BackupGeneration {
    /// Read metadata without publishing or changing any backup files.
    pub fn inspect(path: &Path, binding: &GenerationBinding) -> io::Result<Option<GenerationMeta>> {
        let root = open(&binding.codex_root, true)?;
        if !root.metadata()?.is_dir() || std::fs::canonicalize(&binding.codex_root)? != binding.codex_root {
            return Err(invalid());
        }
        let directory = open(path, true)?;
        let permissions = directory.metadata()?;
        if !permissions.is_dir() || permissions.uid() != unsafe { libc::geteuid() } || permissions.mode() & 0o077 != 0 {
            return Err(invalid());
        }
        let Some(bytes) = read_child(&directory, "meta.json", 8 * 1024 * 1024)? else { return Ok(None); };
        let meta: GenerationMeta = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        if meta.version != 1 || &meta.binding != binding { return Err(invalid()); }
        Ok(Some(meta))
    }

    pub fn metadata(&self) -> &GenerationMeta {
        &self.meta
    }

    /// The existing directory must be executor-owned and private. Every open is
    /// no-follow; retain its descriptor so publication cannot follow a replacement.
    pub fn open(path: &Path, binding: GenerationBinding) -> io::Result<Self> {
        let root = open(&binding.codex_root, true)?;
        if std::fs::canonicalize(&binding.codex_root)? != binding.codex_root
            || !root.metadata()?.is_dir()
            || binding.target_host.trim().is_empty()
            || binding.instance_id.trim().is_empty()
        {
            return Err(invalid());
        }
        let directory = open(path, true)?;
        let permissions = directory.metadata()?;
        if permissions.uid() != unsafe { libc::geteuid() } || permissions.mode() & 0o077 != 0 {
            return Err(invalid());
        }
        // Read through the retained directory, not the original pathname.
        let existing = read_child(&directory, "meta.json", 8 * 1024 * 1024)?;
        let meta = match existing {
            Some(bytes) => {
                let meta: GenerationMeta = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
                if meta.version != 1 || meta.binding != binding {
                    return Err(invalid());
                }
                meta
            }
            None => GenerationMeta { version: 1, binding, complete: false, entries: Vec::new() },
        };
        let generation = Self { directory, meta };
        // The binding is durable before any history mutation, even an empty run.
        generation.persist()?;
        Ok(generation)
    }

    pub(super) fn same_directory(&self, other: &Self) -> io::Result<bool> {
        let a = self.directory.metadata()?;
        let b = other.directory.metadata()?;
        Ok((a.dev(), a.ino()) == (b.dev(), b.ino()))
    }

    fn persist(&self) -> io::Result<()> {
        let bytes = serde_json::to_vec(&self.meta).map_err(|_| invalid())?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(invalid());
        }
        write_child(&self.directory, "meta.json", &bytes)
    }

    pub(super) fn backup(
        &mut self,
        path: &Path,
        bytes: &[u8],
        wal: Option<&[u8]>,
        database: bool,
    ) -> io::Result<()> {
        if self.meta.complete {
            return Err(invalid());
        }
        if let Some(entry) = self.meta.entries.iter().find(|entry| entry.source == path) {
            if entry.state_database != database {
                return Err(invalid());
            }
            // Never overwrite the original on a partial retry. Verify referenced
            // blobs still exist and match their content address before proceeding.
            self.verify_image(&entry.image)?;
            if let Some(wal) = &entry.wal {
                self.verify_image(wal)?;
            }
            return Ok(());
        }
        let image = self.image(bytes)?;
        let wal = wal.map(|bytes| self.image(bytes)).transpose()?;
        self.meta.entries.push(BackupEntry {
            source: path.to_owned(),
            image,
            wal,
            state_database: database,
        });
        if let Err(error) = self.persist() {
            self.meta.entries.pop();
            return Err(error);
        }
        Ok(())
    }
    fn image(&self, bytes: &[u8]) -> io::Result<String> {
        let name = format!("{:x}.image", Sha256::digest(bytes));
        write_child(&self.directory, &name, bytes)?;
        Ok(name)
    }
    fn verify_image(&self, name: &str) -> io::Result<()> {
        self.read_image(name).map(|_| ())
    }
    pub(super) fn read_image(&self, name: &str) -> io::Result<Vec<u8>> {
        if name.len() != 70
            || !name.is_ascii()
            || !name.ends_with(".image")
            || !name[..64].bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(invalid());
        }
        let bytes = read_child(&self.directory, name, 512 * 1024 * 1024)?.ok_or_else(invalid)?;
        if format!("{:x}.image", Sha256::digest(&bytes)) != name {
            return Err(invalid());
        }
        Ok(bytes)
    }
}

#[derive(Default)]
pub struct UnifyOutcome {
    pub migrated_rollouts: usize,
    pub migrated_state_rows: usize,
    pub skipped_reason: Option<&'static str>,
}

/// Inputs are executor snapshots of the authorized instance. `identities` must
/// contain the complete freshly scanned scoped history, including SQLite-only
/// threads. Database paths come from codex_state_db_paths and host authorization.
/// Settings/config publication remains owned by ProviderTransactions; this code
/// never creates a parallel configuration store or changes the user's switches.
#[allow(clippy::too_many_arguments)]
pub fn migrate(
    registry: &SourceRegistry,
    generation: &mut BackupGeneration,
    enabled: bool,
    migrate_existing_requested: bool,
    live_config: &str,
    identities: &[ExternalSessionIdentity],
    database_paths: &[PathBuf],
    mut still_enabled: impl FnMut() -> io::Result<bool>,
    mut advanced: impl FnMut(),
) -> io::Result<UnifyOutcome> {
    let skip = |reason| Ok(UnifyOutcome { skipped_reason: Some(reason), ..Default::default() });
    if !enabled {
        return skip("unify_toggle_off");
    }
    if !migrate_existing_requested {
        return skip("stock_migration_not_requested");
    }
    let config = live_config.parse::<DocumentMut>().map_err(|_| invalid())?;
    if config.get("model_provider").and_then(|item| item.as_str()).map(str::trim) != Some("custom")
    {
        return skip("live_not_unified");
    }
    let sources = BTreeSet::from(["openai".to_owned()]);
    migrate_buckets(
        registry,
        generation,
        identities,
        database_paths,
        &sources,
        &mut still_enabled,
        &mut advanced,
    )
}

/// Shared retryable custom-bucket migration for official unification and trusted
/// third-party templates. IDs must be collected by provider_refs from scoped
/// executor-loaded provider records, never accepted from a maintenance request.
#[allow(clippy::too_many_arguments)]
pub fn migrate_buckets(
    registry: &SourceRegistry,
    generation: &mut BackupGeneration,
    identities: &[ExternalSessionIdentity],
    database_paths: &[PathBuf],
    sources: &BTreeSet<String>,
    mut still_enabled: impl FnMut() -> io::Result<bool>,
    mut advanced: impl FnMut(),
) -> io::Result<UnifyOutcome> {
    if generation.meta.complete {
        return Ok(UnifyOutcome { skipped_reason: Some("already_migrated"), ..Default::default() });
    }
    let binding = &generation.meta.binding;
    let mut threads = BTreeSet::new();
    // Preflight the entire authorized input before the first mutation.
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
        threads.insert(identity.session_id.0.clone());
    }
    let mut outcome = UnifyOutcome::default();
    for identity in identities {
        if !still_enabled()? {
            outcome.skipped_reason = Some("toggle_disabled_during_migration");
            return Ok(outcome);
        }
        if matches!(&identity.source, SessionSource::File { .. })
            && rollout::migrate(registry, identity, sources, "custom", |source, bytes| {
                generation.backup(source.path(), bytes, None, false)
            })?
        {
            outcome.migrated_rollouts += 1;
        }
        advanced();
    }
    for path in database_paths {
        if !still_enabled()? {
            outcome.skipped_reason = Some("toggle_disabled_during_migration");
            return Ok(outcome);
        }
        outcome.migrated_state_rows +=
            state_db::migrate(path, &threads, sources, "custom", |path, backup| {
                generation.backup(path, &backup.database, backup.wal.as_deref(), true)
            })?;
        advanced();
    }
    // Caller must serialize switch changes with lifecycle exclusion. False keeps
    // the generation resumable and never records a successful completion.
    if still_enabled()? {
        generation.meta.complete = true;
        if let Err(error) = generation.persist() {
            generation.meta.complete = false;
            return Err(error);
        }
    } else {
        outcome.skipped_reason = Some("toggle_disabled_during_migration");
    }
    Ok(outcome)
}

fn open(path: &Path, directory: bool) -> io::Result<File> {
    open_beneath(Path::new("/"), path, directory).map_err(|_| invalid())
}
fn write_child(directory: &File, name: &str, bytes: &[u8]) -> io::Result<()> {
    prepare_private_child(directory, std::ffi::OsStr::new(name), bytes)
        .and_then(|staged| staged.commit())
        .map_err(|_| invalid())?;
    directory.sync_all()
}
fn read_child(directory: &File, name: &str, limit: u64) -> io::Result<Option<Vec<u8>>> {
    use std::os::fd::{AsRawFd, FromRawFd};
    let name = std::ffi::CString::new(name).map_err(|_| invalid())?;
    // SAFETY: owned descriptor and a fixed or validated direct child name.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        let error = io::Error::last_os_error();
        return if error.kind() == io::ErrorKind::NotFound { Ok(None) } else { Err(error) };
    }
    // SAFETY: openat returned a uniquely owned descriptor.
    let file = unsafe { File::from_raw_fd(fd) };
    if !file.metadata()?.is_file() {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(invalid());
    }
    Ok(Some(bytes))
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid Codex history generation or binding")
}
