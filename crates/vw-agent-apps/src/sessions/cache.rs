//! Bounded process-local transcript cache. Authorization is repeated by callers;
//! a cache hit never replaces SourceRegistry resolution on the execution host.
use std::{collections::VecDeque, fs, io, path::{Path, PathBuf}, sync::{Arc, LazyLock, Mutex}, time::SystemTime};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionMessage, SessionProvider};
use super::{content_file::safe_join, source::ValidatedSource};

pub const MAX_ENTRIES: usize = 8;
pub const MAX_TOTAL_BYTES: usize = 96 * 1024 * 1024;
const MAX_FINGERPRINT_FILES: usize = 100_000;

#[derive(Debug)]
pub struct Transcript {
    messages: Vec<SessionMessage>,
    message_bytes: Vec<usize>,
    approx_bytes: usize,
}
impl Transcript {
    pub fn new(messages: Vec<SessionMessage>) -> Result<Self, String> {
        let message_bytes: Vec<usize> = messages.iter().map(|m| serde_json::to_vec(m)
            .map(|v| v.len()).map_err(|_| "cannot size session transcript".to_string())).collect::<Result<_, _>>()?;
        let approx_bytes = message_bytes.iter().try_fold(0usize, |sum, size| sum.checked_add(*size))
            .ok_or("session transcript is too large")?;
        Ok(Self { messages, message_bytes, approx_bytes })
    }
    pub fn messages(&self) -> &[SessionMessage] { &self.messages }
    pub fn message_bytes(&self) -> &[usize] { &self.message_bytes }
    pub fn approx_bytes(&self) -> usize { self.approx_bytes }
}

// Keep individual paths and timestamps rather than aggregate maximum mtime/size:
// replacing one part with another of equal size must still invalidate the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint(Vec<FileStamp>);
#[derive(Debug, Clone, PartialEq, Eq)]
struct FileStamp { path: PathBuf, modified: SystemTime, len: u64, directory: bool }

pub fn source_fingerprint(source: &ValidatedSource) -> io::Result<Fingerprint> {
    let mut stamps = Vec::new();
    stamp(source.root(), source.path(), false, &mut stamps)?;
    if source.record_id().is_some() {
        let name = source.path().file_name().ok_or_else(invalid)?.to_os_string();
        let mut wal = name; wal.push("-wal");
        stamp(source.root(), &source.path().with_file_name(wal), true, &mut stamps)?;
    } else {
        match source.identity().provider {
            SessionProvider::Grokbuild => stamp(source.root(), &source.path().with_file_name("chat_history.jsonl"), true, &mut stamps)?,
            SessionProvider::Opencode if source.path().is_dir() => {
                // Message filenames are server-discovered IDs, never client locators.
                for entry in fs::read_dir(source.path())? {
                    let path = entry?.path();
                    if path.extension().and_then(|v| v.to_str()) == Some("json") {
                        let id = path.file_stem().ok_or_else(invalid)?;
                        stamp(source.root(), &source.root().join("part").join(id), true, &mut stamps)?;
                    }
                }
            }
            _ => {}
        }
    }
    stamps.sort_by(|a,b| a.path.cmp(&b.path));
    Ok(Fingerprint(stamps))
}
fn invalid() -> io::Error { io::Error::new(io::ErrorKind::InvalidData, "invalid session cache source") }
fn stamp(root: &Path, path: &Path, optional: bool, stamps: &mut Vec<FileStamp>) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Err(e) if optional && e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
        Ok(_) => {}
    }
    let relative = path.strip_prefix(root).map_err(|_| invalid())?.to_str().ok_or_else(invalid)?;
    if Path::new(relative).components().count() > 64 { return Err(invalid()); }
    let checked = safe_join(root, relative).map_err(|_| invalid())?;
    let meta = fs::metadata(&checked)?;
    if stamps.len() >= MAX_FINGERPRINT_FILES || (!meta.is_file() && !meta.is_dir()) { return Err(invalid()); }
    stamps.push(FileStamp { path: checked.clone(), modified: meta.modified()?, len: meta.len(), directory: meta.is_dir() });
    if meta.is_dir() {
        for entry in fs::read_dir(checked)? { stamp(root, &entry?.path(), false, stamps)?; }
    }
    Ok(())
}

struct Entry { identity: ExternalSessionIdentity, root: PathBuf, path: PathBuf, fingerprint: Fingerprint, transcript: Arc<Transcript> }
#[derive(Default)]
pub struct TranscriptCache { entries: Mutex<VecDeque<Entry>> }
impl TranscriptCache {
    pub fn get_or_load<F>(&self, source: &ValidatedSource, load: F) -> Result<(Arc<Transcript>, bool), String>
    where F: FnOnce() -> Result<Vec<SessionMessage>, String> {
        let before = source_fingerprint(source).map_err(|_| "cannot fingerprint session source")?;
        {
            let mut entries = self.entries.lock().map_err(|_| "session cache lock poisoned")?;
            if let Some(index) = entries.iter().position(|e| &e.identity == source.identity()) {
                if let Some(entry) = entries.remove(index) {
                    if entry.root == source.root() && entry.path == source.path() && entry.fingerprint == before {
                        let value = Arc::clone(&entry.transcript);
                        entries.push_back(entry);
                        return Ok((value, true));
                    }
                }
            }
        }
        // Parse outside the lock. Never publish a parse spanning a source update.
        let value = Arc::new(Transcript::new(load()?)?);
        let after = source_fingerprint(source).map_err(|_| "cannot fingerprint session source")?;
        if before != after { return Err("session changed while reading; retry".into()); }
        let mut entries = self.entries.lock().map_err(|_| "session cache lock poisoned")?;
        entries.retain(|e| &e.identity != source.identity());
        if value.approx_bytes <= MAX_TOTAL_BYTES {
            while entries.len() >= MAX_ENTRIES || entries.iter().map(|e| e.transcript.approx_bytes).sum::<usize>() > MAX_TOTAL_BYTES - value.approx_bytes {
                if entries.pop_front().is_none() { break; }
            }
            entries.push_back(Entry { identity: source.identity().clone(), root: source.root().to_owned(), path: source.path().to_owned(), fingerprint: after, transcript: Arc::clone(&value) });
        }
        Ok((value, false))
    }
    pub fn remove(&self, identity: &ExternalSessionIdentity) -> Result<(), String> {
        self.entries.lock().map_err(|_| "session cache lock poisoned")?.retain(|e| &e.identity != identity);
        Ok(())
    }
}
static GLOBAL: LazyLock<TranscriptCache> = LazyLock::new(TranscriptCache::default);
pub fn global() -> &'static TranscriptCache { &GLOBAL }
