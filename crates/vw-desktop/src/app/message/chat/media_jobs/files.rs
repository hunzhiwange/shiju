use super::JobRef;
use gateway_endpoint::GatewayEndpoint;
use iced::futures::TryStreamExt;
use sha2::{Digest, Sha256};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use vw_gateway_client::{
    GatewayClient,
    media::{MediaAsset, MediaJobId, MediaSessionQuery},
};

pub(super) fn job_index(endpoint: &GatewayEndpoint, session_id: &str) -> Option<PathBuf> {
    let tenant = endpoint.tenant_scope();
    let identity = serde_json::to_vec(&(
        endpoint.base_url(),
        tenant.platform_id(),
        tenant.company_id(),
        endpoint.user_id(),
        session_id,
    ))
    .ok()?;
    let key = hex::encode(Sha256::digest(identity));
    crate::app::project_dirs().map(|dirs| dirs.data_local_dir().join("media-jobs").join(key))
}

fn job_file(index: &Path, id: MediaJobId) -> PathBuf {
    index.join(id.to_string()).with_extension("json")
}

pub(super) fn remember_job(index: &Path, job: JobRef) -> io::Result<()> {
    desktop_terminal::workspace::write_json(&job_file(index, job.id), &job)
}

pub(super) fn read_job_refs(index: &Path) -> io::Result<Vec<JobRef>> {
    let entries = match std::fs::read_dir(index) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
        Err(error) => return Err(error),
    };
    let mut jobs = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() || path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let Some(id) =
            path.file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse::<MediaJobId>().ok())
        else {
            continue;
        };
        if let Ok(Some(job)) = desktop_terminal::workspace::read_json::<JobRef>(&path)
            && job.id == id
        {
            jobs.push(job);
        }
    }
    Ok(jobs)
}

const MEDIA_TYPES: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("webp", "image/webp"),
    ("gif", "image/gif"),
    ("bmp", "image/bmp"),
    ("avif", "image/avif"),
    ("mp4", "video/mp4"),
    ("mov", "video/quicktime"),
    ("webm", "video/webm"),
    ("m4v", "video/x-m4v"),
    ("mpeg", "video/mpeg"),
    ("mpg", "video/mpeg"),
    ("wav", "audio/wav"),
    ("wav", "audio/x-wav"),
    ("mp3", "audio/mpeg"),
    ("m4a", "audio/mp4"),
    ("m4a", "audio/x-m4a"),
    ("aac", "audio/aac"),
    ("flac", "audio/flac"),
    ("ogg", "audio/ogg"),
    ("opus", "audio/opus"),
    ("webm", "audio/webm"),
];

pub(crate) fn asset_extension(mime: &str) -> &'static str {
    MEDIA_TYPES.iter().find(|(_, candidate)| *candidate == mime).map_or("media", |(ext, _)| *ext)
}

pub(super) async fn download_asset(
    client: &GatewayClient,
    scope: &MediaSessionQuery,
    id: MediaJobId,
    asset: &MediaAsset,
    save_to: Option<&Path>,
) -> Result<tempfile::NamedTempFile, &'static str> {
    let mut builder = tempfile::Builder::new();
    let suffix = format!(".{}", asset_extension(&asset.mime));
    builder.prefix("shiju-media-").suffix(&suffix);
    let mut file = match save_to.and_then(Path::parent) {
        Some(directory) => builder.tempfile_in(directory),
        None => builder.tempfile(),
    }
    .map_err(|_| "无法写入媒体文件")?;
    let mut stream = client.media_asset(scope, id, asset).await.map_err(super::api_error)?;
    while let Some(chunk) = stream.try_next().await.map_err(super::api_error)? {
        file.write_all(&chunk).map_err(|_| "无法写入媒体文件")?;
    }
    file.flush().map_err(|_| "无法写入媒体文件")?;
    Ok(file)
}

pub(super) fn image_preview(path: &Path) -> Result<iced::widget::image::Handle, &'static str> {
    let error = "无法预览媒体图片";
    let mut reader = image::ImageReader::open(path)
        .and_then(image::ImageReader::with_guessed_format)
        .map_err(|_| error)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let rgba = reader.decode().map_err(|_| error)?.thumbnail(640, 384).into_rgba8();
    Ok(iced::widget::image::Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw()))
}
