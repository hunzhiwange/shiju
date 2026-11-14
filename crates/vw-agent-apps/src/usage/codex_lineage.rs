//! Cross-rollout usage projection. Callers supply only authorized source paths.
//!
//! Results replace a rollout projection using its stable line identities; they
//! must not be appended blindly on every scan. No ledger or cursor is owned here.
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use super::codex_delta::{CodexDeltaParser, CodexUsageDelta, Counters};
use crate::sessions::rollout;

type Timestamp = DateTime<FixedOffset>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: SystemTime,
    #[cfg(target_os = "macos")]
    device: u64,
    #[cfg(target_os = "macos")]
    inode: u64,
    #[cfg(target_os = "macos")]
    changed: (i64, i64),
}

impl FileStamp {
    fn read(file: &File) -> io::Result<Self> {
        let metadata = file.metadata()?;
        #[cfg(target_os = "macos")]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            len: metadata.len(),
            modified: metadata.modified()?,
            #[cfg(target_os = "macos")]
            device: metadata.dev(),
            #[cfg(target_os = "macos")]
            inode: metadata.ino(),
            #[cfg(target_os = "macos")]
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Signature {
    total: Option<Counters>,
    last: Option<Counters>,
}

fn signature(event: &Value) -> Option<Signature> {
    if event.get("type")?.as_str()? != "event_msg"
        || event.pointer("/payload/type")?.as_str()? != "token_count"
    {
        return None;
    }
    let info = event.pointer("/payload/info")?;
    let total = info.get("total_token_usage").and_then(Counters::parse);
    let last = info.get("last_token_usage").and_then(Counters::parse);
    (total.is_some() || last.is_some()).then_some(Signature { total, last })
}

#[derive(Debug)]
struct Timeline {
    events: Vec<Value>,
    meta_line: usize,
    thread: String,
    parent: Option<String>,
    history: Option<String>,
    continuation: bool,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn label(value: Option<&Value>) -> Option<String> {
    value?.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned)
}

fn timestamp(event: &Value) -> Option<Timestamp> {
    DateTime::parse_from_rfc3339(event.get("timestamp")?.as_str()?).ok()
}

fn uuid(value: &str) -> io::Result<String> {
    uuid::Uuid::parse_str(value)
        .map(|id| id.hyphenated().to_string())
        .map_err(|_| invalid("invalid Codex lineage UUID"))
}

/// (logical thread, segment). A single UUID filename identifies both.
fn filename_ids(path: &Path) -> io::Result<(String, String)> {
    let logical = rollout::logical_path(path);
    let stem = logical
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| invalid("missing rollout filename"))?;
    if !stem.starts_with("rollout-") {
        return Err(invalid("expected Codex rollout filename"));
    }
    let end = stem.len();
    let segment = uuid(stem.get(end.saturating_sub(36)..).unwrap_or_default())?;
    let thread = if end >= 73 && stem.as_bytes()[end - 37] == b'_' {
        uuid(stem.get(end - 73..end - 37).unwrap_or_default())?
    } else {
        segment.clone()
    };
    Ok((thread, segment))
}

impl Timeline {
    fn parse(path: &Path, events: Vec<Value>) -> io::Result<Self> {
        let meta_line = events
            .iter()
            .position(|event| event.get("type").and_then(Value::as_str) == Some("session_meta"))
            .ok_or_else(|| invalid("rollout has no root session_meta"))?;
        let payload = &events[meta_line]["payload"];
        let thread = uuid(
            &label(
                payload
                    .get("id")
                    .or_else(|| payload.get("thread_id"))
                    .or_else(|| payload.get("threadId")),
            )
            .ok_or_else(|| invalid("session_meta has no thread ID"))?,
        )?;
        let (filename_thread, segment) = filename_ids(path)?;
        if filename_thread != thread && segment != thread {
            return Err(invalid("rollout filename and session_meta IDs disagree"));
        }
        let forked = label(payload.get("forked_from_id")).map(|s| uuid(&s)).transpose()?;
        let spawned = label(payload.pointer("/source/subagent/thread_spawn/parent_thread_id"))
            .map(|s| uuid(&s))
            .transpose()?;
        if forked.is_some() && spawned.is_some() && forked != spawned {
            return Err(invalid("conflicting explicit Codex parents"));
        }
        let parent = forked.or(spawned);
        if parent.as_ref() == Some(&thread) {
            return Err(invalid("Codex thread cannot be its own parent"));
        }
        let history =
            label(payload.pointer("/history_base/thread_id")).map(|s| uuid(&s)).transpose()?;
        let paginated = payload.get("history_mode").and_then(Value::as_str) == Some("paginated");
        let continuation = (filename_thread == thread
            && segment != thread
            && (paginated || history.is_some() || parent.is_some()))
            || history.as_ref() == Some(&thread);
        // Usage before root metadata cannot be assigned a safe lineage.
        if events[..meta_line].iter().any(|event| signature(event).is_some()) {
            return Err(invalid("token usage precedes root session_meta"));
        }
        Ok(Self { events, meta_line, thread, parent, history, continuation })
    }

    fn high_water(&self) -> Option<Counters> {
        let mut result: Option<Counters> = None;
        for total in self.events.iter().filter_map(signature).filter_map(|s| s.total) {
            result.get_or_insert_with(Counters::default).advance(&total);
        }
        result
    }
}

/// Index and read cache for one authorized source scope. Do not reuse across
/// tenants or execution hosts. Refresh the file list when discovery changes.
#[derive(Debug, Default)]
pub struct CodexLineage {
    index: HashMap<String, Vec<PathBuf>>,
    cache: HashMap<PathBuf, (FileStamp, Arc<Timeline>)>,
}

impl CodexLineage {
    pub fn new(files: Vec<PathBuf>) -> io::Result<Self> {
        let mut result = Self::default();
        result.replace_files(files)?;
        Ok(result)
    }

    /// Rebuild atomically; additions, removals and archive moves take effect on
    /// the next projection, including a previously missing parent.
    pub fn replace_files(&mut self, files: Vec<PathBuf>) -> io::Result<()> {
        let mut index: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for path in rollout::dedupe_siblings(files) {
            let (thread, segment) = filename_ids(&path)?;
            index.entry(thread.clone()).or_default().push(path.clone());
            if segment != thread {
                index.entry(segment).or_default().push(path);
            }
        }
        for paths in index.values_mut() {
            paths.sort_by(|a, b| {
                rollout::logical_path(a)
                    .file_name()
                    .cmp(&rollout::logical_path(b).file_name())
                    .then_with(|| a.cmp(b))
            });
            paths.dedup();
        }
        self.cache.retain(|path, _| index.values().any(|paths| paths.contains(path)));
        self.index = index;
        Ok(())
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    fn load(&mut self, path: &Path) -> io::Result<Arc<Timeline>> {
        let file = File::open(path)?;
        let before = FileStamp::read(&file)?;
        #[cfg(target_os = "macos")]
        if let Some((stamp, timeline)) = self.cache.get(path) {
            if stamp == &before {
                return Ok(Arc::clone(timeline));
            }
        }
        self.cache.remove(path);
        let mut reader = rollout::reader_from_file(path, file.try_clone()?)?;
        let mut events = Vec::new();
        let mut line = Vec::new();
        loop {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 {
                break;
            }
            // Preserve physical line identity even for damaged/non-JSON lines.
            events.push(serde_json::from_slice(&line).unwrap_or(Value::Null));
        }
        if before != FileStamp::read(&file)? || before != FileStamp::read(&File::open(path)?)? {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "rollout changed while reading",
            ));
        }
        let timeline = Arc::new(Timeline::parse(path, events)?);
        #[cfg(target_os = "macos")]
        self.cache.insert(path.to_path_buf(), (before, Arc::clone(&timeline)));
        Ok(timeline)
    }

    fn previous_baseline(&mut self, path: &Path, child: &Timeline) -> io::Result<Counters> {
        let mut candidates = self.index.get(&child.thread).cloned().unwrap_or_default();
        if let Some(history) = &child.history {
            candidates.extend(self.index.get(history).into_iter().flatten().cloned());
        }
        let current = rollout::logical_path(path);
        candidates.retain(|p| rollout::logical_path(p).file_name() < current.file_name());
        candidates.sort_by_key(|p| rollout::logical_path(p).file_name().map(|s| s.to_owned()));
        // Compare replicas of the immediately preceding segment before trusting
        // a baseline. Never fall back to an older page on a read/content error.
        let previous = candidates.last().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "missing preceding Codex segment")
        })?;
        let (_, previous_id) = filename_ids(previous)?;
        let baseline = self
            .load(previous)?
            .high_water()
            .ok_or_else(|| invalid("preceding Codex segment has no cumulative baseline"))?;
        for candidate in &candidates {
            if filename_ids(candidate)?.1 == previous_id
                && self.load(candidate)?.high_water().as_ref() != Some(&baseline)
            {
                return Err(invalid("conflicting preceding Codex segment replicas"));
            }
        }
        Ok(baseline)
    }

    fn parent_signatures(&mut self, parent: &str, cutoff: Timestamp) -> io::Result<Vec<Signature>> {
        let candidates = self.index.get(parent).cloned().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "missing parent Codex rollout")
        })?;
        let mut segments: BTreeMap<String, (Vec<Signature>, Option<Timestamp>)> = BTreeMap::new();
        let mut result = Vec::new();
        for path in candidates {
            let timeline = self.load(&path)?;
            let mut signatures = Vec::new();
            let mut end = None;
            for event in &timeline.events {
                if let Some(time) = timestamp(event) {
                    end = Some(end.map_or(time, |old: Timestamp| old.max(time)));
                }
                if let Some(sig) = signature(event) {
                    let time = timestamp(event)
                        .ok_or_else(|| invalid("parent token_count lacks valid timestamp"))?;
                    if time <= cutoff {
                        signatures.push(sig);
                    }
                }
            }
            let (_, segment) = filename_ids(&path)?;
            if let Some((previous, previous_end)) = segments.get_mut(&segment) {
                if previous != &signatures {
                    return Err(invalid("conflicting parent Codex rollout replicas"));
                }
                *previous_end = match (*previous_end, end) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    _ => None,
                };
            } else {
                result.extend(signatures.iter().cloned());
                segments.insert(segment, (signatures, end));
            }
        }
        if segments.values().filter_map(|(_, end)| *end).max().is_none_or(|end| end < cutoff) {
            return Err(invalid("parent Codex rollout has not reached fork timestamp"));
        }
        Ok(result)
    }

    /// Recompute a rollout, excluding inherited history. An error defers the
    /// whole projection; callers must retain their last committed cursor/data.
    pub fn project(&mut self, path: &Path) -> io::Result<Vec<CodexUsageDelta>> {
        if !self.index.values().any(|paths| paths.iter().any(|p| p == path)) {
            return Err(invalid("rollout is outside the supplied source index"));
        }
        let child = self.load(path)?;
        let mut parser = CodexDeltaParser::new();
        let mut replay = Vec::new();
        if !child.continuation {
            if let Some(parent) = &child.parent {
                let cutoff = timestamp(&child.events[child.meta_line])
                    .ok_or_else(|| invalid("fork session_meta lacks valid timestamp"))?;
                replay = self.parent_signatures(parent, cutoff)?;
            }
        }
        let mut parent_offset = 0;
        let mut matching_prefix = !replay.is_empty();
        let mut initialized = !child.continuation;
        let mut result = Vec::new();
        for event in &child.events {
            let sig = signature(event);
            if !initialized && sig.is_some() {
                // Exact last usage establishes its own local baseline and does
                // not require a preceding page to be available.
                if sig.as_ref().is_some_and(|s| s.last.is_none()) {
                    let baseline = self.previous_baseline(path, &child)?;
                    parser.inherit_baseline(&baseline, event);
                }
                initialized = sig.as_ref().is_some_and(|s| s.total.is_some());
            }
            let inherited = if matching_prefix {
                if let Some(sig) = sig.as_ref() {
                    if let Some(offset) = replay[parent_offset..].iter().position(|s| s == sig) {
                        parent_offset += offset + 1;
                        true
                    } else {
                        matching_prefix = false;
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };
            // Inherited snapshots still advance cumulative and dedup state.
            if let Some(mut delta) = parser.observe(event) {
                if !inherited {
                    // Replayed metadata may name an ancestor; ownership always
                    // comes from this rollout's validated root metadata.
                    delta.session_id = Some(child.thread.clone());
                    result.push(delta);
                }
            }
        }
        Ok(result)
    }
}
