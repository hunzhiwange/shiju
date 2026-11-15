//! 会话事件的追加写入与读取遍历逻辑。

use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::acp_jsonrpc::is_acp_json_rpc_message;
use crate::{increment_perf_counter, measure_perf};
use crate::queue_lease_store::{is_process_alive, wait_ms};
use acp_paths::queue_paths::default_home_dir;
use crate::session_event_log::{
    DEFAULT_EVENT_MAX_SEGMENTS, DEFAULT_EVENT_SEGMENT_MAX_BYTES, session_base_dir,
    session_event_active_path, session_event_lock_path, session_event_segment_path,
};
use crate::session_persistence::{iso_now, resolve_session_record, write_session_record};
use crate::types::{AcpJsonRpcMessage, SessionRecord};

#[cfg(test)]
#[path = "session_events_tests.rs"]
mod session_events_tests;

const LOCK_RETRY_MS: u64 = 15;
const EVENT_LOCK_STALE_MS: i128 = 15_000;

#[derive(Debug, Clone)]
struct LockHandle {
    file_path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct EventLockPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionEventWriterOptions {
    pub max_segment_bytes: Option<i64>,
    pub max_segments: Option<i64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionEventAppendOptions {
    pub checkpoint: bool,
}

fn default_home_dir_required() -> io::Result<PathBuf> {
    default_home_dir()
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "home directory is unavailable"))
}

async fn ensure_session_dir(home_dir: &Path) -> io::Result<PathBuf> {
    let session_dir = session_base_dir(home_dir);
    fs::create_dir_all(&session_dir).await?;
    Ok(session_dir)
}

async fn path_exists(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

async fn stat_size(path: &Path) -> u64 {
    fs::metadata(path).await.map(|metadata| metadata.len()).unwrap_or_default()
}

async fn count_existing_segments(session_id: &str, max_segments: i64, home_dir: &Path) -> usize {
    let mut count = 0usize;

    for segment in 1..=max_segments {
        if path_exists(&session_event_segment_path(session_id, segment, home_dir)).await {
            count += 1;
        }
    }

    if path_exists(&session_event_active_path(session_id, home_dir)).await {
        count += 1;
    }

    count
}

async fn resolve_session_max_segments(session_id: &str) -> i64 {
    match resolve_session_record(session_id).await {
        Ok(record) if record.event_log.max_segments > 0 => record.event_log.max_segments,
        _ => DEFAULT_EVENT_MAX_SEGMENTS,
    }
}

async fn rotate_segments(session_id: &str, max_segments: i64, home_dir: &Path) -> io::Result<()> {
    let overflow = session_event_segment_path(session_id, max_segments, home_dir);
    match fs::remove_file(&overflow).await {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    for segment in (1..max_segments).rev() {
        let from = session_event_segment_path(session_id, segment, home_dir);
        if !path_exists(&from).await {
            continue;
        }

        let to = session_event_segment_path(session_id, segment + 1, home_dir);
        fs::rename(from, to).await?;
    }

    let active = session_event_active_path(session_id, home_dir);
    if path_exists(&active).await {
        fs::rename(active, session_event_segment_path(session_id, 1, home_dir)).await?;
    }

    Ok(())
}

fn parse_event_lock_payload(raw: &str) -> EventLockPayload {
    serde_json::from_str(raw).unwrap_or_default()
}

fn lock_age_ms(created_at: Option<&str>) -> i128 {
    let Some(created_at) = created_at else {
        return i128::MAX;
    };
    let Ok(timestamp) = OffsetDateTime::parse(created_at, &Rfc3339) else {
        return i128::MAX;
    };

    (OffsetDateTime::now_utc() - timestamp).whole_milliseconds()
}

async fn remove_stale_event_lock(lock_path: &Path) -> bool {
    let payload = match fs::read_to_string(lock_path).await {
        Ok(payload) => payload,
        Err(error) if error.kind() == ErrorKind::NotFound => return true,
        Err(_) => return false,
    };

    let parsed = parse_event_lock_payload(&payload);
    let pid_alive = is_process_alive(parsed.pid);
    if pid_alive && lock_age_ms(parsed.created_at.as_deref()) <= EVENT_LOCK_STALE_MS {
        return false;
    }

    match fs::remove_file(lock_path).await {
        Ok(()) => {
            increment_perf_counter("session.events.stale_lock_recovered", 1);
            true
        }
        Err(error) if error.kind() == ErrorKind::NotFound => true,
        Err(_) => false,
    }
}

async fn acquire_events_lock(session_id: &str, home_dir: &Path) -> io::Result<LockHandle> {
    let _ = ensure_session_dir(home_dir).await?;
    let lock_path = session_event_lock_path(session_id, home_dir);
    let payload = serde_json::to_vec_pretty(&EventLockPayload {
        pid: Some(std::process::id()),
        created_at: Some(iso_now()),
    })
    .map_err(|error| io::Error::other(error.to_string()))?;

    loop {
        let open_result = OpenOptions::new().create_new(true).write(true).open(&lock_path).await;
        match open_result {
            Ok(mut file) => {
                file.write_all(&payload).await?;
                file.write_all(b"\n").await?;
                return Ok(LockHandle { file_path: lock_path.clone() });
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                if remove_stale_event_lock(&lock_path).await {
                    continue;
                }
                wait_ms(LOCK_RETRY_MS).await;
            }
            Err(error) => return Err(error),
        }
    }
}

async fn release_events_lock(lock: &LockHandle) -> io::Result<()> {
    match fs::remove_file(&lock.file_path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn normalize_positive_i64(value: Option<i64>, fallback: i64) -> i64 {
    match value {
        Some(value) if value > 0 => value,
        _ => fallback,
    }
}

pub struct SessionEventWriter {
    record: SessionRecord,
    lock: LockHandle,
    home_dir: PathBuf,
    max_segment_bytes: i64,
    max_segments: i64,
    active_path: PathBuf,
    active_size_bytes: u64,
    segment_count: i64,
    closed: bool,
}

impl SessionEventWriter {
    pub async fn open(
        record: SessionRecord,
        options: SessionEventWriterOptions,
    ) -> io::Result<Self> {
        let home_dir = default_home_dir_required()?;
        let lock = acquire_events_lock(&record.vwacp_record_id, &home_dir).await?;
        let max_segment_bytes = normalize_positive_i64(
            options.max_segment_bytes.or(Some(record.event_log.max_segment_bytes)),
            DEFAULT_EVENT_SEGMENT_MAX_BYTES,
        );
        let max_segments = normalize_positive_i64(
            options.max_segments.or(Some(record.event_log.max_segments)),
            DEFAULT_EVENT_MAX_SEGMENTS,
        );
        let active_path = session_event_active_path(&record.vwacp_record_id, &home_dir);
        let active_size_bytes = stat_size(&active_path).await;
        let segment_count = if record.event_log.segment_count > 0 {
            record.event_log.segment_count
        } else {
            count_existing_segments(&record.vwacp_record_id, max_segments, &home_dir).await.max(1)
                as i64
        };

        Ok(Self {
            record,
            lock,
            home_dir,
            max_segment_bytes,
            max_segments,
            active_path,
            active_size_bytes,
            segment_count,
            closed: false,
        })
    }

    pub fn get_record(&self) -> &SessionRecord {
        &self.record
    }

    pub async fn append_message(
        &mut self,
        message: &AcpJsonRpcMessage,
        options: SessionEventAppendOptions,
    ) -> io::Result<()> {
        self.append_messages(std::slice::from_ref(message), options).await
    }

    pub async fn append_messages(
        &mut self,
        messages: &[AcpJsonRpcMessage],
        options: SessionEventAppendOptions,
    ) -> io::Result<()> {
        if self.closed {
            return Err(io::Error::other("SessionEventWriter is closed"));
        }
        if messages.is_empty() {
            return Ok(());
        }

        let _ = ensure_session_dir(&self.home_dir).await?;

        measure_perf("session.events.append_batch", || async {
            for message in messages {
                let value = serde_json::to_value(message)
                    .map_err(|error| io::Error::new(ErrorKind::InvalidData, error.to_string()))?;
                if !is_acp_json_rpc_message(&value) {
                    return Err(io::Error::new(
                        ErrorKind::InvalidInput,
                        "Attempted to persist invalid ACP JSON-RPC payload",
                    ));
                }

                let mut line = serde_json::to_vec(message)
                    .map_err(|error| io::Error::new(ErrorKind::InvalidData, error.to_string()))?;
                line.push(b'\n');
                let line_bytes = line.len() as u64;
                if self.active_size_bytes > 0
                    && self.active_size_bytes + line_bytes > self.max_segment_bytes as u64
                {
                    rotate_segments(
                        &self.record.vwacp_record_id,
                        self.max_segments,
                        &self.home_dir,
                    )
                    .await?;
                    self.active_path =
                        session_event_active_path(&self.record.vwacp_record_id, &self.home_dir);
                    self.active_size_bytes = 0;
                    self.segment_count = (self.segment_count + 1).min(self.max_segments);
                    increment_perf_counter("session.events.rotate", 1);
                }

                let mut file =
                    OpenOptions::new().create(true).append(true).open(&self.active_path).await?;
                file.write_all(&line).await?;
                self.active_size_bytes += line_bytes;

                self.record.last_seq += 1;
                if let Some(id) = value.get("id") {
                    match id {
                        Value::String(id) => self.record.last_request_id = Some(id.clone()),
                        Value::Number(id) => self.record.last_request_id = Some(id.to_string()),
                        _ => {}
                    }
                }

                let write_ts = iso_now();
                self.record.last_used_at = write_ts.clone();
                self.record.event_log.active_path = self.active_path.to_string_lossy().into_owned();
                self.record.event_log.segment_count = self.segment_count;
                self.record.event_log.max_segment_bytes = self.max_segment_bytes;
                self.record.event_log.max_segments = self.max_segments;
                self.record.event_log.last_write_at = Some(write_ts);
                self.record.event_log.last_write_error = None;
            }

            Ok(())
        })
        .await?;

        if options.checkpoint {
            write_session_record(&self.record)
                .await
                .map_err(|error| io::Error::other(error.to_string()))?;
        }

        Ok(())
    }

    pub async fn checkpoint(&self) -> io::Result<()> {
        if self.closed {
            return Err(io::Error::other("SessionEventWriter is closed"));
        }

        write_session_record(&self.record)
            .await
            .map_err(|error| io::Error::other(error.to_string()))
    }

    pub async fn close(&mut self, options: SessionEventAppendOptions) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }

        let checkpoint_result = if options.checkpoint {
            write_session_record(&self.record)
                .await
                .map_err(|error| io::Error::other(error.to_string()))
        } else {
            Ok(())
        };

        self.closed = true;
        let release_result = release_events_lock(&self.lock).await;

        checkpoint_result?;
        release_result
    }
}

pub async fn list_session_events(session_id: &str) -> io::Result<Vec<AcpJsonRpcMessage>> {
    let home_dir = default_home_dir_required()?;
    let max_segments = resolve_session_max_segments(session_id).await;
    let mut files = Vec::new();

    for segment in (1..=max_segments).rev() {
        let file_path = session_event_segment_path(session_id, segment, &home_dir);
        if path_exists(&file_path).await {
            files.push(file_path);
        }
    }

    let active = session_event_active_path(session_id, &home_dir);
    if path_exists(&active).await {
        files.push(active);
    }

    let mut events = Vec::new();
    for file_path in files {
        let payload = fs::read_to_string(file_path).await?;
        for line in payload.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if !is_acp_json_rpc_message(&value) {
                continue;
            }
            if let Ok(message) = serde_json::from_value::<AcpJsonRpcMessage>(value) {
                events.push(message);
            }
        }
    }

    Ok(events)
}
