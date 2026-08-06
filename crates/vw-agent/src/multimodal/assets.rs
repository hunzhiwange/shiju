//! 内容寻址图片资产存储。
//!
//! 首次进入的图片按范围与完整 SHA-256 持久化到 `global.paths().data` 下的稳定目录，
//! 同时保存原图与压缩后的模型输入。路径只由受控范围编码和内容哈希构造，禁止用户相对路径片段。
//! 写入采用临时目录 + 同步关闭 + 原子重命名；元数据不含源路径、Base64、OCR 或视觉摘要。

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 资产元数据序列化版本。
pub const IMAGE_ASSET_METADATA_VERSION: u32 = 1;

const ASSETS_DIR_NAME: &str = "multimodal";
const IMAGES_DIR_NAME: &str = "image_assets";
const METADATA_FILE: &str = "metadata.json";
const ORIGINAL_FILE: &str = "original.bin";
const MODEL_INPUT_FILE: &str = "model_input.bin";

/// 由原图完整 SHA-256 确定的稳定图片标识。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ImageAssetId(String);

impl ImageAssetId {
    /// 从已校验的完整 SHA-256 十六进制字符串构造。
    pub fn from_hex(hex: impl Into<String>) -> Result<Self> {
        let hex = hex.into();
        validate_sha256_hex(&hex)?;
        Ok(Self(hex))
    }

    /// 返回完整 SHA-256 十六进制字符串。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ImageAssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// 资产隔离范围：优先 `app_session_scope`，缺失时使用 session 标识。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageAssetScope {
    key: String,
}

impl ImageAssetScope {
    /// 由显式范围键构造；空白键会被拒绝。
    pub fn new(key: impl Into<String>) -> Result<Self> {
        let key = key.into();
        let trimmed = key.trim();
        if trimmed.is_empty() {
            bail!("image asset scope key must be non-empty");
        }
        Ok(Self { key: trimmed.to_string() })
    }

    /// 范围优先取 `app_session_scope`，否则回退到 session 标识。
    pub fn from_request_context(
        app_session_scope: Option<&str>,
        session_id: &str,
    ) -> Result<Self> {
        if let Some(scope) = app_session_scope.map(str::trim).filter(|value| !value.is_empty()) {
            return Self::new(scope);
        }
        Self::new(session_id)
    }

    /// 逻辑范围键（项目或会话标识），不直接用于磁盘路径。
    pub fn key(&self) -> &str {
        &self.key
    }

    /// 受控路径片段：范围键的 SHA-256，避免路径分隔符或用户片段进入目录名。
    pub fn path_segment(&self) -> String {
        hex::encode(Sha256::digest(self.key.as_bytes()))
    }
}

/// 资产磁盘变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageAssetVariant {
    /// 源大小校验后的原始字节。
    Original,
    /// 前置压缩管线产出的模型输入字节。
    ModelInput,
}

/// 持久化元数据：仅含版本、MIME、尺寸、字节数、时间戳与会话引用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageAssetMetadata {
    /// 元数据格式版本。
    pub version: u32,
    /// 内容寻址 ID（原图完整 SHA-256）。
    pub image_id: ImageAssetId,
    /// 原图 MIME。
    pub original_mime: String,
    /// 模型输入 MIME（可能与原图不同）。
    pub model_input_mime: String,
    /// 原图字节数。
    pub original_byte_len: usize,
    /// 模型输入字节数。
    pub model_input_byte_len: usize,
    /// 原图像素宽（无法解析时为 `None`）。
    pub width: Option<u32>,
    /// 原图像素高（无法解析时为 `None`）。
    pub height: Option<u32>,
    /// 创建时间（Unix 毫秒）。
    pub created_at_ms: u64,
    /// 最近访问时间（Unix 毫秒）。
    pub last_accessed_at_ms: u64,
    /// 引用该资产的会话 ID 集合（生命周期任务使用）。
    #[serde(default)]
    pub session_refs: BTreeSet<String>,
}

/// 范围隔离的内容寻址图片资产存储。
#[derive(Debug, Clone)]
pub struct ImageAssetStore {
    root: PathBuf,
}

impl ImageAssetStore {
    /// 在给定 data 根目录下创建存储（测试可注入临时目录）。
    pub fn new(data_root: impl Into<PathBuf>) -> Self {
        Self {
            root: data_root
                .into()
                .join(ASSETS_DIR_NAME)
                .join(IMAGES_DIR_NAME),
        }
    }

    /// 使用进程全局持久 `data` 目录；不使用可版本清理的 cache。
    pub fn from_global_paths() -> Self {
        Self::new(crate::global::paths().data.clone())
    }

    /// 存储根目录（`{data}/multimodal/image_assets`）。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 写入或复用范围内资产，返回稳定 `image_id`。
    ///
    /// `original_bytes` 必须已通过源大小与 MIME 校验；`model_input_*` 复用前置压缩结果。
    pub fn put(
        &self,
        scope: &ImageAssetScope,
        original_bytes: &[u8],
        original_mime: &str,
        model_input_bytes: &[u8],
        model_input_mime: &str,
    ) -> Result<ImageAssetId> {
        if original_bytes.is_empty() {
            bail!("original image bytes must be non-empty");
        }
        if model_input_bytes.is_empty() {
            bail!("model input image bytes must be non-empty");
        }

        let image_id = ImageAssetId::from_hex(content_sha256_hex(original_bytes))?;
        let asset_dir = self.asset_dir(scope, &image_id);

        if asset_dir.join(METADATA_FILE).is_file() {
            let existing = self.read_metadata_file(&asset_dir)?;
            self.validate_existing_metadata(
                &existing,
                &image_id,
                original_bytes.len(),
                model_input_bytes.len(),
                original_mime,
                model_input_mime,
            )?;
            self.touch_access(&asset_dir, existing)?;
            return Ok(image_id);
        }

        let now = now_ms();
        let (width, height) = probe_dimensions(original_bytes);
        let metadata = ImageAssetMetadata {
            version: IMAGE_ASSET_METADATA_VERSION,
            image_id: image_id.clone(),
            original_mime: original_mime.to_ascii_lowercase(),
            model_input_mime: model_input_mime.to_ascii_lowercase(),
            original_byte_len: original_bytes.len(),
            model_input_byte_len: model_input_bytes.len(),
            width,
            height,
            created_at_ms: now,
            last_accessed_at_ms: now,
            session_refs: BTreeSet::new(),
        };

        self.publish_asset(scope, &image_id, original_bytes, model_input_bytes, &metadata)?;
        Ok(image_id)
    }

    /// 读取范围内资产元数据；跨范围或不存在时返回明确错误。
    pub fn get_metadata(
        &self,
        scope: &ImageAssetScope,
        image_id: &ImageAssetId,
    ) -> Result<ImageAssetMetadata> {
        let asset_dir = self.require_published_dir(scope, image_id)?;
        let metadata = self.read_metadata_file(&asset_dir)?;
        self.touch_access(&asset_dir, metadata.clone())?;
        Ok(metadata)
    }

    /// 读取原图或模型输入变体字节。
    pub fn read_variant(
        &self,
        scope: &ImageAssetScope,
        image_id: &ImageAssetId,
        variant: ImageAssetVariant,
    ) -> Result<Vec<u8>> {
        let asset_dir = self.require_published_dir(scope, image_id)?;
        let path = match variant {
            ImageAssetVariant::Original => asset_dir.join(ORIGINAL_FILE),
            ImageAssetVariant::ModelInput => asset_dir.join(MODEL_INPUT_FILE),
        };
        fs::read(&path).with_context(|| {
            format!(
                "failed to read image asset variant {:?} at {}",
                variant,
                path.display()
            )
        })
    }

    /// 判断范围内是否已发布该资产。
    pub fn is_published(&self, scope: &ImageAssetScope, image_id: &ImageAssetId) -> bool {
        self.asset_dir(scope, image_id).join(METADATA_FILE).is_file()
    }

    /// 已发布资产目录（分析缓存等派生文件写入此处的子目录）。
    pub fn asset_directory(
        &self,
        scope: &ImageAssetScope,
        image_id: &ImageAssetId,
    ) -> Result<PathBuf> {
        self.require_published_dir(scope, image_id)
    }

    /// 删除资产目录下的全部分析缓存文件（生命周期回收使用）。
    pub fn remove_analysis_caches(
        &self,
        scope: &ImageAssetScope,
        image_id: &ImageAssetId,
    ) -> Result<()> {
        let analysis_dir = self.asset_dir(scope, image_id).join("analysis");
        if analysis_dir.exists() {
            fs::remove_dir_all(&analysis_dir).with_context(|| {
                format!(
                    "failed to remove image analysis cache directory {}",
                    analysis_dir.display()
                )
            })?;
        }
        Ok(())
    }

    /// 幂等登记 session → image 引用。
    pub fn register_session_ref(
        &self,
        scope: &ImageAssetScope,
        image_id: &ImageAssetId,
        session_id: &str,
    ) -> Result<()> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            bail!("session id must be non-empty for image asset reference");
        }
        let asset_dir = self.require_published_dir(scope, image_id)?;
        let mut metadata = self.read_metadata_file(&asset_dir)?;
        if metadata.session_refs.insert(session_id.to_string()) {
            self.write_metadata_atomic(&asset_dir, &metadata)?;
        }
        self.add_session_index_entry(scope, session_id, image_id)?;
        Ok(())
    }

    /// 释放某会话在范围内的全部图片引用；最后一个引用删除资产与分析缓存。
    pub fn release_session_refs(&self, scope: &ImageAssetScope, session_id: &str) -> Result<()> {
        let session_id = session_id.trim();
        if session_id.is_empty() {
            return Ok(());
        }
        let image_ids = self.take_session_index(scope, session_id)?;
        for image_id_hex in image_ids {
            let Ok(image_id) = ImageAssetId::from_hex(image_id_hex) else {
                continue;
            };
            let asset_dir = self.asset_dir(scope, &image_id);
            if !asset_dir.join(METADATA_FILE).is_file() {
                continue;
            }
            let mut metadata = self.read_metadata_file(&asset_dir)?;
            metadata.session_refs.remove(session_id);
            if metadata.session_refs.is_empty() {
                let _ = self.remove_analysis_caches(scope, &image_id);
                let _ = fs::remove_dir_all(&asset_dir);
            } else {
                self.write_metadata_atomic(&asset_dir, &metadata)?;
            }
        }
        Ok(())
    }

    /// 判断资产是否仍被至少一个会话引用。
    pub fn has_session_refs(&self, scope: &ImageAssetScope, image_id: &ImageAssetId) -> Result<bool> {
        let metadata = self.get_metadata(scope, image_id)?;
        Ok(!metadata.session_refs.is_empty())
    }

    fn scope_dir(&self, scope: &ImageAssetScope) -> PathBuf {
        self.root.join("scopes").join(scope.path_segment())
    }

    fn asset_dir(&self, scope: &ImageAssetScope, image_id: &ImageAssetId) -> PathBuf {
        let hash = image_id.as_str();
        let shard = &hash[..2];
        self.scope_dir(scope).join("assets").join(shard).join(hash)
    }

    fn require_published_dir(
        &self,
        scope: &ImageAssetScope,
        image_id: &ImageAssetId,
    ) -> Result<PathBuf> {
        let asset_dir = self.asset_dir(scope, image_id);
        if !asset_dir.join(METADATA_FILE).is_file() {
            bail!(
                "image asset '{}' is not published in the requested scope",
                image_id.as_str()
            );
        }
        Ok(asset_dir)
    }

    fn publish_asset(
        &self,
        scope: &ImageAssetScope,
        image_id: &ImageAssetId,
        original_bytes: &[u8],
        model_input_bytes: &[u8],
        metadata: &ImageAssetMetadata,
    ) -> Result<()> {
        let parent = self
            .asset_dir(scope, image_id)
            .parent()
            .map(Path::to_path_buf)
            .context("image asset path missing parent")?;
        fs::create_dir_all(&parent).with_context(|| {
            format!("failed to create image asset parent directory {}", parent.display())
        })?;

        let staging = parent.join(format!(".tmp-{}.{}", image_id.as_str(), staging_suffix()));
        if staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        fs::create_dir_all(&staging).with_context(|| {
            format!("failed to create staging directory {}", staging.display())
        })?;

        let publish_result = (|| -> Result<()> {
            write_file_synced(&staging.join(ORIGINAL_FILE), original_bytes)?;
            write_file_synced(&staging.join(MODEL_INPUT_FILE), model_input_bytes)?;
            let encoded = serde_json::to_vec_pretty(metadata)
                .context("failed to serialize image asset metadata")?;
            write_file_synced(&staging.join(METADATA_FILE), &encoded)?;
            sync_dir(&staging)?;

            let final_dir = self.asset_dir(scope, image_id);
            if final_dir.join(METADATA_FILE).is_file() {
                // 并发写入已完成：校验后丢弃 staging。
                let existing = self.read_metadata_file(&final_dir)?;
                self.validate_existing_metadata(
                    &existing,
                    image_id,
                    original_bytes.len(),
                    model_input_bytes.len(),
                    &metadata.original_mime,
                    &metadata.model_input_mime,
                )?;
                return Ok(());
            }

            if final_dir.exists() {
                // 可见半成品目录：不允许当作已发布资产。
                let _ = fs::remove_dir_all(&final_dir);
            }

            fs::rename(&staging, &final_dir).with_context(|| {
                format!(
                    "failed to atomically publish image asset to {}",
                    final_dir.display()
                )
            })?;
            sync_dir(&parent)?;
            Ok(())
        })();

        if staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        publish_result
    }

    fn read_metadata_file(&self, asset_dir: &Path) -> Result<ImageAssetMetadata> {
        let path = asset_dir.join(METADATA_FILE);
        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read image asset metadata {}", path.display()))?;
        let metadata: ImageAssetMetadata = serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse image asset metadata {}", path.display()))?;
        if metadata.version != IMAGE_ASSET_METADATA_VERSION {
            bail!(
                "unsupported image asset metadata version {} (expected {})",
                metadata.version,
                IMAGE_ASSET_METADATA_VERSION
            );
        }
        validate_sha256_hex(metadata.image_id.as_str())?;
        Ok(metadata)
    }

    fn validate_existing_metadata(
        &self,
        existing: &ImageAssetMetadata,
        image_id: &ImageAssetId,
        original_len: usize,
        model_input_len: usize,
        original_mime: &str,
        model_input_mime: &str,
    ) -> Result<()> {
        if existing.image_id != *image_id {
            bail!("stored image_id does not match content hash");
        }
        if existing.original_byte_len != original_len {
            bail!("existing asset original byte length mismatch");
        }
        if existing.model_input_byte_len != model_input_len {
            bail!("existing asset model input byte length mismatch");
        }
        if !existing.original_mime.eq_ignore_ascii_case(original_mime) {
            bail!("existing asset original MIME mismatch");
        }
        if !existing.model_input_mime.eq_ignore_ascii_case(model_input_mime) {
            bail!("existing asset model input MIME mismatch");
        }
        Ok(())
    }

    fn touch_access(
        &self,
        asset_dir: &Path,
        mut metadata: ImageAssetMetadata,
    ) -> Result<()> {
        metadata.last_accessed_at_ms = now_ms();
        self.write_metadata_atomic(asset_dir, &metadata)
    }

    fn write_metadata_atomic(&self, asset_dir: &Path, metadata: &ImageAssetMetadata) -> Result<()> {
        let encoded = serde_json::to_vec_pretty(metadata)
            .context("failed to serialize image asset metadata")?;
        let tmp = asset_dir.join(format!(".{METADATA_FILE}.tmp-{}", staging_suffix()));
        write_file_synced(&tmp, &encoded)?;
        fs::rename(&tmp, asset_dir.join(METADATA_FILE)).with_context(|| {
            format!(
                "failed to update image asset metadata at {}",
                asset_dir.display()
            )
        })?;
        let _ = fs::remove_file(&tmp);
        Ok(())
    }

    fn session_index_path(&self, scope: &ImageAssetScope, session_id: &str) -> PathBuf {
        let session_key = hex::encode(Sha256::digest(session_id.as_bytes()));
        self.scope_dir(scope).join("session_refs").join(format!("{session_key}.json"))
    }

    fn add_session_index_entry(
        &self,
        scope: &ImageAssetScope,
        session_id: &str,
        image_id: &ImageAssetId,
    ) -> Result<()> {
        let path = self.session_index_path(scope, session_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut ids: BTreeSet<String> = if path.is_file() {
            serde_json::from_slice(&fs::read(&path)?)
                .unwrap_or_default()
        } else {
            BTreeSet::new()
        };
        ids.insert(image_id.as_str().to_string());
        let encoded = serde_json::to_vec_pretty(&ids)?;
        let tmp = path.with_extension("json.tmp");
        write_file_synced(&tmp, &encoded)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    fn take_session_index(
        &self,
        scope: &ImageAssetScope,
        session_id: &str,
    ) -> Result<Vec<String>> {
        let path = self.session_index_path(scope, session_id);
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let ids: BTreeSet<String> = serde_json::from_slice(&fs::read(&path)?).unwrap_or_default();
        let _ = fs::remove_file(&path);
        Ok(ids.into_iter().collect())
    }
}

fn content_sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn validate_sha256_hex(value: &str) -> Result<()> {
    if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("image_id must be a full 64-character SHA-256 hex digest");
    }
    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn staging_suffix() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn probe_dimensions(bytes: &[u8]) -> (Option<u32>, Option<u32>) {
    let reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().ok();
    match reader.and_then(|decoder| decoder.into_dimensions().ok()) {
        Some((width, height)) => (Some(width), Some(height)),
        None => (None, None),
    }
}

fn write_file_synced(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = File::create(path)
        .with_context(|| format!("failed to create file {}", path.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("failed to write file {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync file {}", path.display()))?;
    Ok(())
}

fn sync_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let file = File::open(path)
            .with_context(|| format!("failed to open directory for sync {}", path.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to sync directory {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn asset_dir_for_tests(
    store: &ImageAssetStore,
    scope: &ImageAssetScope,
    image_id: &ImageAssetId,
) -> PathBuf {
    store.asset_dir(scope, image_id)
}
