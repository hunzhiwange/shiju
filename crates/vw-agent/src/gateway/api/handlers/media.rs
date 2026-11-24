//! Authenticated, session-scoped media jobs and bounded result delivery.

mod deployments;
mod frames;

use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::PathBuf,
    sync::OnceLock,
};

use agent_gateway::api::{
    auth::extract_auth_skey,
    media::{
        MAX_MEDIA_ASSET_BYTES, MAX_MEDIA_SUBMIT_BYTES, MAX_MEDIA_UPLOAD_BYTES, MEDIA_ASSET_ROUTE,
        MEDIA_JOB_ROUTE, MEDIA_JOBS_PATH, MEDIA_MODELS_PATH, MediaAsset, MediaAssetSource,
        MediaErrorCode as Code, MediaInput, MediaJobId, MediaJobScope, MediaJobState,
        MediaJobStore, MediaModel, MediaModels, MediaOperation, MediaReservation, MediaResult,
        MediaSessionQuery, MediaSubmit, bounded_media_asset_stream, media_error_response,
    },
};
use agent_multimodal::assets::{ImageAssetScope, ImageAssetStore};
use axum::{
    Extension, Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, FromRequest, Path, Query, Request, rejection::QueryRejection},
    http::{HeaderMap, StatusCode, header},
    middleware::{Next, from_fn},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use llm_runtime::aisdk::media::{MediaFailure, MediaFile, MediaOutput};
use sha2::{Digest, Sha256};
use tokio::{io::AsyncReadExt, sync::Semaphore};
use tokio_util::sync::CancellationToken;
use vw_api_transport::TenantScope;

use deployments::Route;

const MAX_ACTIVE_WORKERS: usize = 4;
const MAX_ASSET_READERS: usize = 8;

static WORKERS: Semaphore = Semaphore::const_new(MAX_ACTIVE_WORKERS);
static READERS: Semaphore = Semaphore::const_new(MAX_ASSET_READERS);
static REQUESTS: Semaphore = Semaphore::const_new(MAX_ACTIVE_WORKERS);

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    let api_path = |path: &'static str| path.strip_prefix("/api").expect("media contract prefix");
    Router::new()
        .route(api_path(MEDIA_MODELS_PATH), get(models))
        .route(
            api_path(MEDIA_JOBS_PATH),
            post(submit).layer(DefaultBodyLimit::max(MAX_MEDIA_SUBMIT_BYTES)),
        )
        .route(api_path(MEDIA_JOB_ROUTE), get(job).delete(cancel))
        .route(api_path(MEDIA_ASSET_ROUTE), get(asset))
        .route_layer(from_fn(authenticate))
}

#[derive(Clone)]
struct Identity {
    tenant: TenantScope,
    principal: [u8; 32],
}

async fn authenticate(
    Extension(tenant): Extension<TenantScope>,
    mut request: Request,
    next: Next,
) -> Response {
    match authenticate_headers(tenant, request.headers()).await {
        Ok(identity) => {
            request.extensions_mut().insert(identity);
            next.run(request).await
        }
        Err(code) => media_error_response(code),
    }
}

async fn authenticate_headers(tenant: TenantScope, headers: &HeaderMap) -> Result<Identity, Code> {
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Err(Code::Unauthorized);
    }
    let token = extract_auth_skey(headers).ok_or(Code::Unauthorized)?;
    let config = agent_config::runtime::get_global_for_tenant(&tenant)
        .await
        .map_err(|_| Code::Unavailable)?;
    let guard = gateway_pairing::pairing::PairingGuard::from_skeys(
        config.gateway.auth_enabled,
        &config.gateway.skeys,
    );
    if !guard.auth_enabled() || !guard.is_authenticated(token) {
        return Err(Code::Unauthorized);
    }
    Ok(Identity { tenant, principal: Sha256::digest(token.as_bytes()).into() })
}

struct MediaContext {
    scope: MediaJobScope,
    image_scope: ImageAssetScope,
    session_id: String,
    directory: String,
}

impl Identity {
    async fn session(&self, session: MediaSessionQuery) -> Result<MediaContext, Code> {
        let scope = MediaJobScope::new(&self.tenant, self.principal, session.clone())?;
        let info = super::voice::Host::load_session(self.tenant.clone(), session.session_id)
            .await
            .map_err(|_| Code::Unavailable)?
            .ok_or(Code::NotFound)?;
        let image_scope = ImageAssetScope::from_request_context(Some(&info.project_id), &info.id)
            .map_err(|_| Code::Unavailable)?;
        Ok(MediaContext { scope, image_scope, session_id: info.id, directory: info.directory })
    }
}
use gateway_voice::host::VoiceHost as _;

struct Storage {
    jobs: MediaJobStore,
    images: ImageAssetStore,
    streams: PathBuf,
}

fn storage() -> Result<&'static Storage, Code> {
    static STORAGE: OnceLock<Storage> = OnceLock::new();
    static OPEN: parking_lot::Mutex<()> = parking_lot::const_mutex(());
    if let Some(storage) = STORAGE.get() {
        return Ok(storage);
    }
    let _open = OPEN.lock();
    if let Some(storage) = STORAGE.get() {
        return Ok(storage);
    }
    let media = crate::global::paths().data.join("media");
    let streams = media.join("assets");
    let mut directory = std::fs::DirBuilder::new();
    directory.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        directory.mode(0o700);
    }
    directory.create(&streams).map_err(|_| Code::Unavailable)?;
    let jobs = MediaJobStore::open(&media.join("jobs.sqlite3"))?;
    Ok(STORAGE.get_or_init(|| Storage {
        jobs,
        images: ImageAssetStore::from_global_paths(),
        streams,
    }))
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, Code> + Send + 'static,
) -> Result<T, Code> {
    tokio::task::spawn_blocking(work).await.map_err(|_| Code::Unavailable)?
}

fn respond<T: IntoResponse>(result: Result<T, Code>) -> Response {
    let mut response = match result {
        Ok(value) => value.into_response(),
        Err(code) => media_error_response(code),
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store"));
    response
}

fn query_session(
    query: Result<Query<MediaSessionQuery>, QueryRejection>,
) -> Result<MediaSessionQuery, Code> {
    query.map(|Query(query)| query).map_err(|_| Code::InvalidRequest)
}

async fn models(
    Extension(identity): Extension<Identity>,
    query: Result<Query<MediaSessionQuery>, QueryRejection>,
) -> Response {
    respond(
        async {
            let _slot = REQUESTS.try_acquire().map_err(|_| Code::Backpressure)?;
            let context = identity.session(query_session(query)?).await?;
            let routes = deployments::routes(identity.tenant, context.directory, None).await?;
            let mut models = BTreeMap::<_, MediaModel>::new();
            let can_extract = deployments::frame_extractor().is_some();
            for Route { model, capability, .. } in routes {
                if capability
                    == app_models::media_capabilities::MediaCapability::VideoUnderstandingFrames
                    && !can_extract
                {
                    continue;
                }
                let key = (model.deployment_id.clone(), model.model_id.clone());
                models
                    .entry(key)
                    .and_modify(|existing| {
                        for operation in &model.operations {
                            if !existing.operations.contains(operation) {
                                existing.operations.push(*operation);
                            }
                        }
                    })
                    .or_insert(model);
            }
            let models = MediaModels { models: models.into_values().collect() };
            let body = serde_json::to_vec(&models).map_err(|_| Code::Unavailable)?;
            if body.len() > agent_gateway::api::media::MAX_MEDIA_JSON_BYTES {
                return Err(Code::Unavailable);
            }
            Ok(([(header::CONTENT_TYPE, "application/json")], body))
        }
        .await,
    )
}

async fn submit(Extension(identity): Extension<Identity>, request: Request) -> Response {
    respond(
        async {
            let _slot = REQUESTS.try_acquire().map_err(|_| Code::Backpressure)?;
            let Json(request) =
                Json::<MediaSubmit>::from_request(request, &()).await.map_err(|error| {
                    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
                        Code::PayloadTooLarge
                    } else {
                        Code::InvalidRequest
                    }
                })?;
            let context = identity
                .session(MediaSessionQuery { session_id: request.session_id.clone() })
                .await?;
            let (request, context, files) = blocking(move || {
                request.validate()?;
                let files = inputs(storage()?, &context, &request.inputs)?;
                validate_inputs(request.operation, request.text.as_deref(), &files)?;
                Ok((request, context, files))
            })
            .await?;
            let extractor = deployments::frame_extractor();
            let selected = (request.deployment_id.clone(), request.model_id.clone());
            let routes =
                deployments::routes(identity.tenant, context.directory.clone(), Some(selected))
                    .await?;
            let (route, asr) = deployments::select(routes, &request)?;
            if asr.is_some() && extractor.is_none() {
                return Err(Code::Unavailable);
            }
            blocking(move || {
                let store = storage()?;
                let worker = WORKERS.try_acquire().ok();
                match store.jobs.reserve_with_capacity(
                    &context.scope,
                    &request,
                    worker.is_some(),
                )? {
                    MediaReservation::Existing(job) => Ok((StatusCode::OK, Json(job))),
                    MediaReservation::Start { job, cancellation } => {
                        // Once the journal reserves this request, always launch its worker,
                        // even if the HTTP request was dropped while reserving.
                        let worker = worker.expect("a new job requires a worker slot");
                        tokio::spawn(async move {
                            let _worker = worker;
                            execute(
                                request,
                                context,
                                route,
                                asr.zip(extractor),
                                files,
                                cancellation,
                            )
                            .await;
                        });
                        Ok((StatusCode::ACCEPTED, Json(job)))
                    }
                }
            })
            .await
        }
        .await,
    )
}

fn inputs(
    store: &Storage,
    context: &MediaContext,
    inputs: &[MediaInput],
) -> Result<Vec<MediaFile>, Code> {
    let mut files = Vec::with_capacity(inputs.len());
    let mut remaining = MAX_MEDIA_UPLOAD_BYTES;
    for input in inputs {
        let file = match input {
            MediaInput::Upload { mime, .. } => {
                let bytes = input.decode_upload()?.ok_or(Code::InvalidRequest)?;
                if mime.starts_with("image/") && image_mime(&bytes)? != mime.as_str() {
                    return Err(Code::UnsupportedMedia);
                }
                MediaFile { mime: mime.clone(), bytes }
            }
            MediaInput::Asset { asset_id } => {
                let (job, asset) = store.jobs.find_asset(&context.scope, asset_id)?;
                if asset.byte_len > remaining as u64 {
                    return Err(Code::PayloadTooLarge);
                }
                let bytes = match asset.source {
                    MediaAssetSource::Image { .. } => store.jobs.read_image(
                        &context.scope,
                        job,
                        asset_id,
                        &store.images,
                        &context.image_scope,
                    )?,
                    MediaAssetSource::Stream => {
                        let mut bytes = Vec::new();
                        std::fs::File::open(store.streams.join(&asset.id))
                            .map_err(|_| Code::NotFound)?
                            .take(asset.byte_len + 1)
                            .read_to_end(&mut bytes)
                            .map_err(|_| Code::Unavailable)?;
                        if bytes.len() as u64 != asset.byte_len {
                            return Err(Code::Unavailable);
                        }
                        bytes
                    }
                };
                MediaFile { mime: asset.mime, bytes }
            }
        };
        remaining = remaining.checked_sub(file.bytes.len()).ok_or(Code::PayloadTooLarge)?;
        files.push(file);
    }
    Ok(files)
}

fn image_mime(bytes: &[u8]) -> Result<&'static str, Code> {
    let format = image::guess_format(bytes).map_err(|_| Code::UnsupportedMedia)?;
    if !matches!(
        format,
        image::ImageFormat::Png
            | image::ImageFormat::Jpeg
            | image::ImageFormat::WebP
            | image::ImageFormat::Gif
    ) {
        return Err(Code::UnsupportedMedia);
    }
    Ok(format.to_mime_type())
}

fn validate_inputs(
    operation: MediaOperation,
    text: Option<&str>,
    files: &[MediaFile],
) -> Result<(), Code> {
    use MediaOperation::*;
    let image = |file: &MediaFile| file.mime.starts_with("image/");
    let valid = match operation {
        ImageGenerate | SpeechSynthesize => files.is_empty(),
        ImageEdit | ImageUnderstand => !files.is_empty() && files.iter().all(image),
        VideoGenerate => files.len() <= 1 && files.iter().all(image),
        AudioTranscribe => matches!(files, [file] if file.mime.starts_with("audio/")),
        VideoUnderstand => matches!(files, [file] if file.mime.starts_with("video/")),
    };
    if !valid {
        return Err(Code::UnsupportedMedia);
    }
    if matches!(operation, ImageGenerate | ImageEdit | VideoGenerate | SpeechSynthesize)
        && text.is_none_or(|text| text.trim().is_empty())
    {
        return Err(Code::InvalidRequest);
    }
    Ok(())
}

async fn execute(
    request: MediaSubmit,
    context: MediaContext,
    route: Route,
    asr: Option<(Route, PathBuf)>,
    mut files: Vec<MediaFile>,
    cancellation: CancellationToken,
) {
    let id = request.request_id;
    let scope = context.scope.clone();
    if blocking(move || {
        let store = storage()?;
        let state = MediaJobState::Running { progress_percent: None };
        store.jobs.update(&scope, id, state).inspect_err(|_| store.jobs.abandon(&scope, id))
    })
    .await
    .is_err()
    {
        return;
    }
    let progress_scope = context.scope.clone();
    let (progress, mut updates) = tokio::sync::mpsc::channel(1);
    let progress_worker = tokio::spawn(async move {
        while let Some(value) = updates.recv().await {
            let scope = progress_scope.clone();
            let _ = blocking(move || {
                storage()?.jobs.update(
                    &scope,
                    id,
                    MediaJobState::Running { progress_percent: Some(value) },
                )
            })
            .await;
        }
    });
    let output = async {
        let mut text = request.text;
        if let Some((asr, executable)) = asr {
            let video = files.pop().ok_or(Code::UnsupportedMedia)?;
            let (frames, audio) = frames::extract(&executable, video, &cancellation).await?;
            files = frames;
            let prompt = text.as_deref().unwrap_or("Describe the supplied video.");
            let transcript = if let Some(audio) = audio {
                asr.endpoint.execute(None, vec![audio], &cancellation, |_| {}).await
                    .map_err(failure_code)?.text.ok_or(Code::UpstreamFailed)?
                    .chars().take(2048).collect::<String>()
            } else {
                String::new()
            };
            text = Some(format!(
                "{prompt}\nUse the sampled frames and any transcript excerpt from the first 30 seconds.\nTranscript:\n{transcript}"
            ));
        }
        Ok::<_, Code>(route.endpoint.execute(text, files, &cancellation, |value| {
            let _ = progress.try_send(value);
        }).await)
    }.await;
    drop(progress);
    let _ = progress_worker.await;
    let _ = blocking(move || {
        let store = storage()?;
        if cancellation.is_cancelled() {
            return Ok(());
        }
        let result = match output {
            Ok(Ok(output)) => publish(store, &context, id, output),
            Ok(Err(MediaFailure::SubmissionUnknown)) => {
                store.jobs.update(&context.scope, id, MediaJobState::SubmissionUnknown).map(|_| ())
            }
            Ok(Err(error)) => failed(store, &context, id, failure_code(error)),
            Err(code) => failed(store, &context, id, code),
        };
        result.inspect_err(|_| store.jobs.abandon(&context.scope, id))
    })
    .await;
}

fn failure_code(error: MediaFailure) -> Code {
    match error {
        MediaFailure::InvalidInput => Code::InvalidRequest,
        MediaFailure::Unsupported => Code::UnsupportedMedia,
        MediaFailure::TooLarge => Code::PayloadTooLarge,
        MediaFailure::Upstream | MediaFailure::SubmissionUnknown | MediaFailure::Canceled => {
            Code::UpstreamFailed
        }
    }
}

fn failed(store: &Storage, context: &MediaContext, id: MediaJobId, code: Code) -> Result<(), Code> {
    store.jobs.update(&context.scope, id, MediaJobState::Failed { code }).map(|_| ())
}

fn publish(
    store: &Storage,
    context: &MediaContext,
    id: MediaJobId,
    output: MediaOutput,
) -> Result<(), Code> {
    let mut streams = Vec::new();
    let state = (|| {
        let mut assets = Vec::new();
        for file in output.files {
            let length = file.bytes.len() as u64;
            if length == 0 || length > MAX_MEDIA_ASSET_BYTES {
                return Err(Code::PayloadTooLarge);
            }
            let asset = if file.mime.starts_with("image/") {
                let mime = image_mime(&file.bytes)?;
                let image_id = store
                    .images
                    .put(&context.image_scope, &file.bytes, mime, &file.bytes, mime)
                    .map_err(|_| Code::Unavailable)?;
                store
                    .images
                    .register_session_ref(&context.image_scope, &image_id, &context.session_id)
                    .map_err(|_| Code::Unavailable)?;
                MediaJobStore::image_descriptor(
                    &context.scope,
                    &store.images,
                    &context.image_scope,
                    &image_id,
                )?
            } else {
                let id = MediaJobId::new_v4().simple().to_string();
                let mime = if file.mime == "application/octet-stream" {
                    frames::video_mime(&file.bytes).ok_or(Code::UnsupportedMedia)?.into()
                } else {
                    file.mime
                };
                let asset = MediaAsset {
                    id,
                    mime,
                    byte_len: length,
                    width: None,
                    height: None,
                    duration_ms: None,
                    source: MediaAssetSource::Stream,
                };
                asset.validate()?;
                let mut temporary = tempfile::NamedTempFile::new_in(&store.streams)
                    .map_err(|_| Code::Unavailable)?;
                temporary.write_all(&file.bytes).map_err(|_| Code::Unavailable)?;
                temporary.as_file().sync_all().map_err(|_| Code::Unavailable)?;
                let path = store.streams.join(&asset.id);
                temporary.persist_noclobber(&path).map_err(|_| Code::Unavailable)?;
                streams.push(path);
                asset
            };
            assets.push(asset);
        }
        let result = MediaResult { text: output.text, assets };
        result.validate()?;
        store.jobs.update(&context.scope, id, MediaJobState::Succeeded { result })?;
        Ok(())
    })();
    if let Err(code) = state {
        for path in streams {
            let _ = std::fs::remove_file(path);
        }
        return failed(store, context, id, code);
    }
    Ok(())
}

async fn job(
    Extension(identity): Extension<Identity>,
    Path(id): Path<String>,
    query: Result<Query<MediaSessionQuery>, QueryRejection>,
) -> Response {
    respond(read_job(identity, id, query, false).await.map(Json))
}

async fn cancel(
    Extension(identity): Extension<Identity>,
    Path(id): Path<String>,
    query: Result<Query<MediaSessionQuery>, QueryRejection>,
) -> Response {
    respond(read_job(identity, id, query, true).await.map(Json))
}

async fn read_job(
    identity: Identity,
    id: String,
    query: Result<Query<MediaSessionQuery>, QueryRejection>,
    cancel: bool,
) -> Result<agent_gateway::api::media::MediaJob, Code> {
    let id = MediaJobId::parse_str(&id).map_err(|_| Code::NotFound)?;
    let context = identity.session(query_session(query)?).await?;
    blocking(move || {
        let store = storage()?;
        if cancel {
            store.jobs.cancel(&context.scope, id)
        } else {
            store.jobs.get(&context.scope, id)
        }
    })
    .await
}

async fn asset(
    Extension(identity): Extension<Identity>,
    Path((id, asset_id)): Path<(String, String)>,
    query: Result<Query<MediaSessionQuery>, QueryRejection>,
) -> Response {
    respond(
        async {
            let reader = READERS.try_acquire().map_err(|_| Code::Backpressure)?;
            let id = MediaJobId::parse_str(&id).map_err(|_| Code::NotFound)?;
            let context = identity.session(query_session(query)?).await?;
            let (asset, file) = blocking(move || {
                let store = storage()?;
                let asset = store.jobs.asset(&context.scope, id, &asset_id)?;
                let file = match asset.source {
                    MediaAssetSource::Image { .. } => {
                        let bytes = store.jobs.read_image(
                            &context.scope,
                            id,
                            &asset_id,
                            &store.images,
                            &context.image_scope,
                        )?;
                        AssetFile::Image(bytes)
                    }
                    MediaAssetSource::Stream => {
                        let file = std::fs::File::open(store.streams.join(&asset.id))
                            .map_err(|_| Code::NotFound)?;
                        if file.metadata().map_err(|_| Code::Unavailable)?.len() != asset.byte_len {
                            return Err(Code::Unavailable);
                        }
                        AssetFile::Stream(tokio::fs::File::from_std(file))
                    }
                };
                Ok((asset, file))
            })
            .await?;
            let body = match file {
                AssetFile::Image(bytes) => {
                    Body::from_stream(futures_util::stream::once(async move {
                        let _reader = reader;
                        Ok::<_, std::io::Error>(Bytes::from(bytes))
                    }))
                }
                AssetFile::Stream(file) => {
                    let stream = futures_util::stream::try_unfold(
                        (file, reader),
                        |(mut file, reader)| async move {
                            let mut buffer = vec![0; 64 * 1024];
                            let read = file.read(&mut buffer).await?;
                            buffer.truncate(read);
                            Ok::<_, std::io::Error>(
                                (read != 0).then_some((Bytes::from(buffer), (file, reader))),
                            )
                        },
                    );
                    let stream = bounded_media_asset_stream(&asset, stream)?
                        .map_err(|code| std::io::Error::other(code.message()));
                    Body::from_stream(stream)
                }
            };
            Ok((
                [
                    (header::CONTENT_TYPE, asset.mime),
                    (header::CONTENT_LENGTH, asset.byte_len.to_string()),
                    (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
                ],
                body,
            ))
        }
        .await,
    )
}

enum AssetFile {
    Image(Vec<u8>),
    Stream(tokio::fs::File),
}

use futures_util::TryStreamExt as _;

#[cfg(test)]
#[path = "media_tests.rs"]
mod tests;
