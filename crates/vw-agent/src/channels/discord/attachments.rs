//! Discord 消息附件处理模块
//!
//! 本模块负责处理 Discord 消息中的各类附件，根据附件类型执行不同的处理逻辑：
//!
//! - **图片附件**：生成 `[IMAGE:<url>]` 标记，供代理识别图片内容
//! - **文本附件**：直接获取文本内容并内联到消息中
//! - **其他类型**：跳过处理并记录调试日志
//!
//! # 主要功能
//!
//! - 附件类型识别（基于 MIME 类型和文件扩展名）
//!
//! # 安全考虑
//!
//! - 网络请求失败时优雅降级，不影响主流程

use std::path::Path;

/// 处理 Discord 消息附件，返回要追加到代理消息上下文的字符串
///
/// 根据附件的 MIME 类型执行不同的处理策略：
///
/// - `image/*` 附件：转换为 `[IMAGE:<url>]` 标记
/// - `application/octet-stream` 或缺失 MIME 类型：通过文件名/URL 扩展名判断是否为图片
/// - `text/*` 附件：获取内容并内联
/// - 其他类型：跳过处理
///
/// # 参数
///
/// * `attachments` - Discord 附件 JSON 数组，每个附件包含 `content_type`、`filename`、`url` 等字段
/// * `client` - HTTP 客户端，用于获取附件内容
///
/// # 返回值
///
/// 返回处理后的字符串，各部分用 `\n---\n` 分隔。若无有效附件则返回空字符串。
///
/// # 示例
///
/// ```ignore
/// let attachments = vec![
///     json!({"content_type": "image/png", "filename": "photo.png", "url": "https://cdn.discord.app/..."}),
/// ];
/// let result = process_attachments(&attachments, &client).await;
/// // result: "[IMAGE:https://cdn.discord.app/...]"
/// ```
///
/// # 错误处理
///
/// - 获取附件失败时记录警告日志并跳过该附件
/// - 不会因单个附件处理失败而中断整体流程
pub(super) async fn process_attachments(
    attachments: &[serde_json::Value],
    client: &reqwest::Client,
) -> String {
    // 收集所有处理后的附件内容片段
    let mut parts: Vec<String> = Vec::new();

    for att in attachments {
        // 提取附件的基本信息：MIME 类型、文件名、URL
        let ct = att.get("content_type").and_then(|v| v.as_str()).unwrap_or("");
        let name = att.get("filename").and_then(|v| v.as_str()).unwrap_or("file");

        // URL 是必需的，无 URL 则跳过
        let Some(url) = att.get("url").and_then(|v| v.as_str()) else {
            tracing::warn!(name, "discord: attachment has no url, skipping");
            continue;
        };

        // 根据附件类型分发处理逻辑
        if is_image_attachment(ct, name, url) {
            // 图片类型：生成标记供代理识别
            parts.push(format!("[IMAGE:{url}]"));
        } else if ct.starts_with("text/") {
            // 文本类型：直接获取内容并内联
            match client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(text) = resp.text().await {
                        parts.push(format!("[{name}]\n{text}"));
                    }
                }
                Ok(resp) => {
                    tracing::warn!(name, status = %resp.status(), "discord attachment fetch failed");
                }
                Err(e) => {
                    tracing::warn!(name, error = %e, "discord attachment fetch error");
                }
            }
        } else {
            // 不支持的附件类型，记录调试日志
            tracing::debug!(
                name,
                content_type = ct,
                "discord: skipping unsupported attachment type"
            );
        }
    }

    // 用分隔符连接所有片段
    parts.join("\n---\n")
}

/// 规范化 MIME 类型字符串
///
/// 移除 MIME 类型中的参数部分（如 `; charset=utf-8`）并转换为小写。
///
/// # 参数
///
/// * `content_type` - 原始 Content-Type 字符串
///
/// # 返回值
///
/// 规范化后的 MIME 类型字符串（小写，无参数）
///
/// # 示例
///
/// ```
/// # use crate::app::agent::channels::discord::attachments::normalize_content_type;
/// assert_eq!(normalize_content_type("Image/PNG"), "image/png");
/// assert_eq!(normalize_content_type("audio/mpeg; charset=utf-8"), "audio/mpeg");
/// assert_eq!(normalize_content_type(""), "");
/// ```
pub(super) fn normalize_content_type(content_type: &str) -> String {
    // MIME 类型格式为 "type/subtype; params"，取分号前的部分
    content_type.split(';').next().unwrap_or("").trim().to_ascii_lowercase()
}

/// 判断附件是否为图片类型
///
/// 判断逻辑：
/// 1. 如果 MIME 类型以 `image/` 开头，返回 `true`
/// 2. 如果 MIME 类型非空且非 `application/octet-stream`，信任 MIME 类型并返回 `false`
/// 3. 否则，通过文件名或 URL 的扩展名判断
///
/// # 参数
///
/// * `content_type` - MIME 类型字符串
/// * `filename` - 附件文件名
/// * `url` - 附件 URL
///
/// # 返回值
///
/// 如果是图片类型返回 `true`，否则返回 `false`
///
/// # 设计说明
///
/// 对于 `application/octet-stream` 或缺失 MIME 类型的情况，回退到扩展名判断。
/// 这是为了兼容某些服务器不正确设置 MIME 类型的场景。
pub(super) fn is_image_attachment(content_type: &str, filename: &str, url: &str) -> bool {
    let normalized_content_type = normalize_content_type(content_type);

    // MIME 类型非空时的处理
    if !normalized_content_type.is_empty() {
        // 明确的图片类型
        if normalized_content_type.starts_with("image/") {
            return true;
        }
        // 非 octet-stream 的明确非图片类型，信任 MIME 类型避免误判
        if normalized_content_type != "application/octet-stream" {
            return false;
        }
    }

    // MIME 类型为空或为 octet-stream 时，通过扩展名判断
    has_image_extension(filename) || has_image_extension(url)
}

/// 从媒体路径中提取文件扩展名
///
/// 处理 URL 中的查询参数和片段标识符，只取路径部分的扩展名。
///
/// # 参数
///
/// * `value` - 文件路径或 URL
///
/// # 返回值
///
/// 成功返回 `Some(小写扩展名)`，无扩展名返回 `None`
///
/// # 示例
///
/// ```ignore
/// assert_eq!(extension_from_media_path("audio.mp3"), Some("mp3".to_string()));
/// assert_eq!(extension_from_media_path("https://example.com/audio.ogg?v=1"), Some("ogg".to_string()));
/// assert_eq!(extension_from_media_path("noextension"), None);
/// ```
pub(super) fn extension_from_media_path(value: &str) -> Option<String> {
    // 移除 URL 查询参数（? 后的内容）
    let base = value.split('?').next().unwrap_or(value);
    // 移除片段标识符（# 后的内容）
    let base = base.split('#').next().unwrap_or(base);
    // 使用标准库提取扩展名并转为小写
    Path::new(base).extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase())
}

/// 检查路径或 URL 是否具有图片扩展名
///
/// # 参数
///
/// * `value` - 文件路径或 URL
///
/// # 返回值
///
/// 具有支持的图片扩展名返回 `true`，否则返回 `false`
///
/// # 支持的图片格式
///
/// - `png` - PNG 格式
/// - `jpg` / `jpeg` - JPEG 格式
/// - `gif` - GIF 格式
/// - `webp` - WebP 格式
/// - `bmp` - BMP 格式
/// - `tif` / `tiff` - TIFF 格式
/// - `svg` - SVG 矢量图
/// - `avif` - AVIF 格式
/// - `heic` / `heif` - HEIF 格式（Apple 设备常用）
pub(super) fn has_image_extension(value: &str) -> bool {
    matches!(
        extension_from_media_path(value).as_deref(),
        Some(
            "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "webp"
                | "bmp"
                | "tif"
                | "tiff"
                | "svg"
                | "avif"
                | "heic"
                | "heif"
        )
    )
}
