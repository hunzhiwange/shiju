use desktop_i18n::project::ProjectFeedback;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use sha2::{Digest, Sha256};
#[cfg(not(target_arch = "wasm32"))]
use image::{ImageFormat, RgbaImage};
/// is_supported_image_attachment 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub fn is_supported_image_attachment(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
            )
        })
        .unwrap_or(false)
}

const CHAT_IMAGE_ATTACHMENT_DIR: &str = "chat_image_attachments";

fn chat_image_attachment_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|dirs| {
        config_runtime_paths::home_config_dir(dirs.home_dir()).join(CHAT_IMAGE_ATTACHMENT_DIR)
    })
}

fn path_within_dir(path: &Path, dir: &Path) -> bool {
    match (path.canonicalize(), dir.canonicalize()) {
        (Ok(canonical_path), Ok(canonical_dir)) => canonical_path.starts_with(&canonical_dir),
        _ => false,
    }
}

fn sanitize_attachment_stem(path: &Path) -> String {
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("image");
    let sanitized = stem
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if sanitized.is_empty() { "image".to_string() } else { sanitized }
}

fn build_image_attachment_snapshot_path(
    source: &Path,
    metadata: &std::fs::Metadata,
    snapshot_dir: &Path,
) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(source.to_string_lossy().as_bytes());
    hasher.update(metadata.len().to_le_bytes());
    if let Ok(modified) = metadata.modified()
        && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
    {
        hasher.update(duration.as_secs().to_le_bytes());
        hasher.update(duration.subsec_nanos().to_le_bytes());
    }

    let digest = hex::encode(hasher.finalize());
    let short_digest = &digest[..16];
    let stem = sanitize_attachment_stem(source);
    let extension = source.extension().and_then(|value| value.to_str()).unwrap_or("img");
    snapshot_dir.join(format!("{stem}-{short_digest}.{extension}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn build_clipboard_image_attachment_path(
    width: u32,
    height: u32,
    rgba_bytes: &[u8],
    snapshot_dir: &Path,
) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(width.to_le_bytes());
    hasher.update(height.to_le_bytes());
    hasher.update(rgba_bytes);

    let digest = hex::encode(hasher.finalize());
    snapshot_dir.join(format!("clipboard-{}.png", &digest[..16]))
}

fn stabilize_image_attachment(
    source: &Path,
    metadata: &std::fs::Metadata,
    project_root: Option<&Path>,
    snapshot_dir: &Path,
) -> Result<PathBuf, ProjectFeedback> {
    if project_root.is_some_and(|root| path_within_dir(source, root))
        || path_within_dir(source, snapshot_dir)
    {
        return Ok(source.to_path_buf());
    }

    std::fs::create_dir_all(snapshot_dir).map_err(|error| {
        ProjectFeedback::new(
            "创建图片附件目录失败：%s（%s）",
            vec![snapshot_dir.display().to_string().into(), error.to_string().into()],
        )
    })?;

    let target = build_image_attachment_snapshot_path(source, metadata, snapshot_dir);
    if !target.exists() {
        std::fs::copy(source, &target).map_err(|error| {
            ProjectFeedback::new(
                "复制图片附件失败：%s（%s）",
                vec![source.display().to_string().into(), error.to_string().into()],
            )
        })?;
    }

    Ok(target)
}

fn attachment_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

/// collect_local_attachments 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub fn collect_local_attachments(
    files: &[String],
    project_path: Option<&str>,
    max_image_count: usize,
    max_image_size_mb: usize,
    picked: Vec<String>,
) -> (Vec<String>, Vec<ProjectFeedback>) {
    let max_images = max_image_count.clamp(1, 16) as usize;
    let max_bytes =
        (max_image_size_mb.clamp(1, 20) as u64).saturating_mul(1024 * 1024);
    let mut seen = files.iter().cloned().collect::<HashSet<_>>();
    let project_root = project_path.map(Path::new);
    let snapshot_dir = chat_image_attachment_dir();
    let mut image_count =
        files.iter().filter(|path| is_supported_image_attachment(Path::new(path))).count();
    let mut accepted = Vec::new();
    let mut errors = Vec::new();

    for raw in picked {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        let source = Path::new(trimmed);
        let resolved = match source.canonicalize() {
            Ok(path) => path,
            Err(error) => {
                errors.push(ProjectFeedback::new(
                    "读取失败：%s（%s）",
                    vec![source.display().to_string().into(), error.to_string().into()],
                ));
                continue;
            }
        };

        if !resolved.is_file() {
            errors.push(ProjectFeedback::new(
                "不支持文件夹：%s",
                vec![resolved.display().to_string().into()],
            ));
            continue;
        }

        if is_supported_image_attachment(&resolved) {
            let metadata = match std::fs::metadata(&resolved) {
                Ok(metadata) => metadata,
                Err(error) => {
                    errors.push(ProjectFeedback::new(
                        "读取大小失败：%s（%s）",
                        vec![resolved.display().to_string().into(), error.to_string().into()],
                    ));
                    continue;
                }
            };

            if metadata.len() > max_bytes {
                errors.push(ProjectFeedback::new(
                    "图片过大：%s（超过 %u MB）",
                    vec![
                        attachment_name(&resolved).into(),
                        max_image_size_mb.clamp(1, 20).into(),
                    ],
                ));
                continue;
            }

            if image_count >= max_images {
                errors.push(ProjectFeedback::new(
                    "已达图片上限：%s（最多 %u 张）",
                    vec![attachment_name(&resolved).into(), max_images.into()],
                ));
                continue;
            }

            let Some(snapshot_dir) = snapshot_dir.as_deref() else {
                errors.push(ProjectFeedback::new("无法确定图片附件目录", vec![]));
                continue;
            };

            let stable_path = match stabilize_image_attachment(
                &resolved,
                &metadata,
                project_root,
                snapshot_dir,
            ) {
                Ok(path) => path,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };

            let normalized = stable_path.to_string_lossy().to_string();
            if !seen.insert(normalized.clone()) {
                continue;
            }

            image_count += 1;
            accepted.push(normalized);
            continue;
        }

        let normalized = resolved.to_string_lossy().to_string();
        if !seen.insert(normalized.clone()) {
            continue;
        }

        accepted.push(normalized);
    }

    (accepted, errors)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn persist_clipboard_image_attachment(
    width: u32,
    height: u32,
    rgba_bytes: &[u8],
) -> Result<String, ProjectFeedback> {
    let Some(snapshot_dir) = chat_image_attachment_dir() else {
        return Err(ProjectFeedback::new("无法确定图片附件目录", vec![]));
    };

    std::fs::create_dir_all(&snapshot_dir).map_err(|error| {
        ProjectFeedback::new(
            "创建图片附件目录失败：%s（%s）",
            vec![snapshot_dir.display().to_string().into(), error.to_string().into()],
        )
    })?;

    let target = build_clipboard_image_attachment_path(width, height, rgba_bytes, &snapshot_dir);
    if !target.exists() {
        let image = RgbaImage::from_raw(width, height, rgba_bytes.to_vec())
            .ok_or_else(|| ProjectFeedback::new("剪贴板图片数据无效", vec![]))?;
        image.save_with_format(&target, ImageFormat::Png).map_err(|error| {
            ProjectFeedback::new(
                "保存剪贴板图片失败：%s（%s）",
                vec![target.display().to_string().into(), error.to_string().into()],
            )
        })?;
    }

    Ok(target.to_string_lossy().to_string())
}


#[cfg(test)]
#[path = "attachments_tests.rs"]
mod attachments_tests;
