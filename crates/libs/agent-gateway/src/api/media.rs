//! Session-owned media jobs. The gateway owns one store per database and reserves
//! work before calling a provider. HTTP authentication and provider dispatch live
//! in the route integration.

use std::{
    collections::HashMap,
    io,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use agent_multimodal::assets::{ImageAssetId, ImageAssetScope, ImageAssetStore, ImageAssetVariant};
use axum::{
    Json,
    body::Bytes,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::{Stream, StreamExt, stream};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use vw_api_transport::id::TenantScope;

use MediaErrorCode as Code;
pub use vw_api_media::*;

const MAX_ACTIVE_JOBS: usize = 32;

pub fn media_error_response(code: Code) -> Response {
    let status = StatusCode::from_u16(code.http_status()).expect("valid media HTTP status");
    (status, Json(MediaErrorResponse { error: MediaErrorDetail { code } })).into_response()
}

/// Construct only after both gateway authentication and session authorization.
/// The principal is a stable authenticated identity digest, never a raw credential.
#[derive(Clone)]
pub struct MediaJobScope {
    owner: [u8; 32],
    session_id: String,
}

impl MediaJobScope {
    pub fn new(
        tenant: &TenantScope,
        principal: [u8; 32],
        session: MediaSessionQuery,
    ) -> Result<Self, Code> {
        session.validate()?;
        let mut hash = Sha256::new();
        hash.update(principal);
        for value in [
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            tenant.user_id().as_ref(),
            session.session_id.as_str(),
        ] {
            hash.update((value.len() as u64).to_le_bytes());
            hash.update(value.as_bytes());
        }
        Ok(Self { owner: hash.finalize().into(), session_id: session.session_id })
    }
}

pub enum MediaReservation {
    /// Only this branch may dispatch the provider call.
    Start {
        job: MediaJob,
        cancellation: CancellationToken,
    },
    Existing(MediaJob),
}

pub struct MediaJobStore {
    inner: Mutex<MediaJobStoreInner>,
    now_ms: fn() -> u64,
}

type JobKey = ([u8; 32], MediaJobId);

struct MediaJobStoreInner {
    database: Connection,
    active: HashMap<JobKey, CancellationToken>,
}

impl MediaJobStore {
    /// Open the local state journal. Call blocking methods on a blocking worker
    /// from async gateway handlers.
    pub fn open(path: &Path) -> Result<Self, Code> {
        Self::open_with_clock(path, || {
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
        })
    }

    fn open_with_clock(path: &Path, now_ms: fn() -> u64) -> Result<Self, Code> {
        let database = Connection::open(path).map_err(|_| Code::Unavailable)?;
        database.busy_timeout(Duration::from_secs(2)).map_err(|_| Code::Unavailable)?;
        database
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS media_jobs (
                owner BLOB NOT NULL,
                id TEXT NOT NULL,
                digest BLOB NOT NULL,
                snapshot TEXT NOT NULL,
                PRIMARY KEY (owner, id)
            );",
            )
            .map_err(|_| Code::Unavailable)?;
        Ok(Self {
            inner: Mutex::new(MediaJobStoreInner { database, active: HashMap::new() }),
            now_ms,
        })
    }

    pub fn reserve(
        &self,
        scope: &MediaJobScope,
        request: &MediaSubmit,
    ) -> Result<MediaReservation, Code> {
        if scope.session_id != request.session_id {
            return Err(Code::NotFound);
        }
        request.validate()?;
        // The journal only keeps the digest of submitted prompts and input bytes.
        let mut hash = RequestDigest(Sha256::new());
        serde_json::to_writer(&mut hash, request).map_err(|_| Code::InvalidRequest)?;
        let digest: [u8; 32] = hash.0.finalize().into();
        let mut store = self.inner.lock();
        let key = (scope.owner, request.request_id);
        if let Some(existing) = store.existing(&key, &digest)? {
            return Ok(MediaReservation::Existing(existing));
        }
        if store.active.len() >= MAX_ACTIVE_JOBS {
            return Err(Code::Backpressure);
        }

        let now = (self.now_ms)();
        let job = MediaJob {
            id: request.request_id,
            session_id: request.session_id.clone(),
            operation: request.operation,
            created_at_ms: now,
            updated_at_ms: now,
            state: MediaJobState::Submitted,
        };
        let inserted = store
            .database
            .execute(
                "INSERT INTO media_jobs (owner, id, digest, snapshot) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (owner, id) DO NOTHING",
                params![&key.0[..], key.1.to_string(), &digest[..], encode(&job)?],
            )
            .map_err(|_| Code::Unavailable)?;
        if inserted == 0 {
            return store
                .existing(&key, &digest)?
                .map(MediaReservation::Existing)
                .ok_or(Code::Unavailable);
        }

        let cancellation = CancellationToken::new();
        store.active.insert(key, cancellation.clone());
        Ok(MediaReservation::Start { job, cancellation })
    }

    pub fn get(&self, scope: &MediaJobScope, id: MediaJobId) -> Result<MediaJob, Code> {
        let store = self.inner.lock();
        let key = (scope.owner, id);
        let (_, snapshot) = store.load(&key)?.ok_or(Code::NotFound)?;
        store.visible_job(&key, &snapshot)
    }

    /// Apply a worker update after reserving a job, or reconcile a verified
    /// provider outcome for a previously uncertain submission.
    pub fn update(
        &self,
        scope: &MediaJobScope,
        id: MediaJobId,
        state: MediaJobState,
    ) -> Result<MediaJob, Code> {
        if matches!(state, MediaJobState::Submitted | MediaJobState::Canceled) {
            return Err(Code::InvalidRequest);
        }
        self.change(scope, id, state)
    }

    /// Cancel pending work and notify its provider dispatch after persisting the
    /// decision. A repeated cancel or cancel after completion returns the current job.
    pub fn cancel(&self, scope: &MediaJobScope, id: MediaJobId) -> Result<MediaJob, Code> {
        self.change(scope, id, MediaJobState::Canceled)
    }

    fn change(
        &self,
        scope: &MediaJobScope,
        id: MediaJobId,
        state: MediaJobState,
    ) -> Result<MediaJob, Code> {
        use MediaJobState::*;
        let mut store = self.inner.lock();
        let key = (scope.owner, id);
        let (_, snapshot) = store.load(&key)?.ok_or(Code::NotFound)?;
        let mut job = store.visible_job(&key, &snapshot)?;
        if job.state == state
            || (matches!(state, Canceled) && matches!(job.state, Succeeded { .. } | Failed { .. }))
        {
            return Ok(job);
        }

        match (&job.state, &state) {
            (Submitted | Running { .. }, Running { progress_percent }) => {
                let previous = match job.state {
                    Running { progress_percent } => progress_percent,
                    _ => None,
                };
                if progress_percent.is_some_and(|percent| percent > 100)
                    || (previous.is_some() && *progress_percent < previous)
                {
                    return Err(Code::InvalidRequest);
                }
            }
            (Submitted | Running { .. } | SubmissionUnknown, Succeeded { result }) => {
                result.validate()?
            }
            (Submitted | Running { .. } | SubmissionUnknown, Failed { .. } | Canceled)
            | (Submitted | Running { .. }, SubmissionUnknown) => {}
            _ => return Err(Code::Conflict),
        }

        job.state = state;
        job.updated_at_ms = (self.now_ms)().max(job.updated_at_ms);
        let changed = store.database.execute(
            "UPDATE media_jobs SET snapshot = ?1 WHERE owner = ?2 AND id = ?3 AND snapshot = ?4",
            params![encode(&job)?, &key.0[..], id.to_string(), snapshot],
        ).map_err(|_| Code::Unavailable)?;
        if changed == 0 {
            return Err(Code::Conflict);
        }
        if !matches!(job.state, Running { .. })
            && let Some(cancellation) = store.active.remove(&key)
            && matches!(job.state, Canceled)
        {
            cancellation.cancel();
        }
        Ok(job)
    }

    /// Authorize reads only after the job has successfully published the requested asset.
    pub fn asset(
        &self,
        scope: &MediaJobScope,
        id: MediaJobId,
        asset_id: &str,
    ) -> Result<MediaAsset, Code> {
        if !valid_asset_id(asset_id) {
            return Err(Code::NotFound);
        }
        let MediaJobState::Succeeded { result } = self.get(scope, id)?.state else {
            return Err(Code::NotFound);
        };
        result.assets.into_iter().find(|asset| asset.id == asset_id).ok_or(Code::NotFound)
    }

    /// Build a result from the existing image registry. Use the already authorized
    /// session engine image scope. The registry must contain this session's reference.
    pub fn image_descriptor(
        scope: &MediaJobScope,
        images: &ImageAssetStore,
        image_scope: &ImageAssetScope,
        image_id: &ImageAssetId,
    ) -> Result<MediaAsset, Code> {
        // ImageAssetId derives Deserialize, so also validate IDs received through serde.
        let image_id = ImageAssetId::from_hex(image_id.as_str()).map_err(|_| Code::NotFound)?;
        let metadata = images.get_metadata(image_scope, &image_id).map_err(|_| Code::NotFound)?;
        if !metadata.session_refs.contains(&scope.session_id) {
            return Err(Code::NotFound);
        }
        let asset = MediaAsset {
            id: image_id.to_string(),
            mime: metadata.original_mime,
            byte_len: metadata.original_byte_len as u64,
            width: metadata.width,
            height: metadata.height,
            duration_ms: None,
            source: MediaAssetSource::Image { image_id: image_id.to_string() },
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn read_image(
        &self,
        scope: &MediaJobScope,
        id: MediaJobId,
        asset_id: &str,
        images: &ImageAssetStore,
        image_scope: &ImageAssetScope,
    ) -> Result<Vec<u8>, Code> {
        let asset = self.asset(scope, id, asset_id)?;
        let MediaAssetSource::Image { image_id } = &asset.source else {
            return Err(Code::UnsupportedMedia);
        };
        let image_id = ImageAssetId::from_hex(image_id.as_str()).map_err(|_| Code::NotFound)?;
        let image = Self::image_descriptor(scope, images, image_scope, &image_id)?;
        if image.mime != asset.mime || image.byte_len != asset.byte_len {
            return Err(Code::Unavailable);
        }
        let bytes = images
            .read_variant(image_scope, &image_id, ImageAssetVariant::Original)
            .map_err(|_| Code::NotFound)?;
        if bytes.len() as u64 != asset.byte_len {
            return Err(Code::Unavailable);
        }
        Ok(bytes)
    }
}

impl MediaJobStoreInner {
    fn load(&self, key: &JobKey) -> Result<Option<(Vec<u8>, String)>, Code> {
        self.database
            .query_row(
                "SELECT digest, snapshot FROM media_jobs WHERE owner = ?1 AND id = ?2",
                params![&key.0[..], key.1.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| Code::Unavailable)
    }

    fn existing(&self, key: &JobKey, digest: &[u8; 32]) -> Result<Option<MediaJob>, Code> {
        let Some((previous, snapshot)) = self.load(key)? else { return Ok(None) };
        if previous.as_slice() != digest {
            return Err(Code::Conflict);
        }
        self.visible_job(key, &snapshot).map(Some)
    }

    fn visible_job(&self, key: &JobKey, snapshot: &str) -> Result<MediaJob, Code> {
        let mut job: MediaJob = serde_json::from_str(snapshot).map_err(|_| Code::Unavailable)?;
        if matches!(job.state, MediaJobState::Submitted | MediaJobState::Running { .. })
            && !self.active.contains_key(key)
        {
            job.state = MediaJobState::SubmissionUnknown;
        }
        Ok(job)
    }
}

// Stream serialization into the digest without making a second upload-sized buffer.
struct RequestDigest(Sha256);

impl io::Write for RequestDigest {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn encode(job: &MediaJob) -> Result<String, Code> {
    serde_json::to_string(job).map_err(|_| Code::Unavailable)
}

/// Wrap audio/video bytes after resolving a stream source for an authorized asset.
/// Reject short or oversized streams without buffering the whole asset.
pub fn bounded_media_asset_stream<S, E>(
    asset: &MediaAsset,
    source: S,
) -> Result<impl Stream<Item = Result<Bytes, Code>> + use<S, E>, Code>
where
    S: Stream<Item = Result<Bytes, E>>,
{
    asset.validate()?;
    if !matches!(asset.source, MediaAssetSource::Stream) {
        return Err(Code::UnsupportedMedia);
    }
    Ok(stream::try_unfold(
        (Box::pin(source), asset.byte_len),
        |(mut source, remaining)| async move {
            match source.next().await {
                Some(Ok(bytes)) if bytes.len() as u64 <= remaining => {
                    let remaining = remaining - bytes.len() as u64;
                    Ok(Some((bytes, (source, remaining))))
                }
                None if remaining == 0 => Ok(None),
                _ => Err(Code::Unavailable),
            }
        },
    ))
}

#[cfg(test)]
#[path = "media_tests.rs"]
mod tests;
