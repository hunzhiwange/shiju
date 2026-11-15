//! Telegram 附件处理模块
//!
//! 本模块提供 Telegram 消息附件的解析、验证和安全处理功能。
//! 主要职责包括：
//! - 接收附件的结构化表示与类型分类
//! - 附件路径解析与工作区安全校验（防止路径遍历攻击）
//! - 附件标记语言的解析（例如 `[IMAGE:/path/to/file.jpg]`）
//! - 文件名净化与输出路径生成
//!
//! # 安全约束
//!
//! 所有路径解析函数都强制执行工作区边界检查，
//! 确保附件访问不会逃逸到工作区之外。

use anyhow::Context;
use std::path::{Path, PathBuf};
use tokio::fs;

/// 接收到的 Telegram 附件元数据
///
/// 表示从 Telegram API 接收到的附件信息，
/// 包含文件标识符、可选的文件名、大小、说明文字以及类型分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IncomingAttachment {
    /// Telegram 服务器上的唯一文件标识符，用于下载文件
    pub(super) file_id: String,
    /// 原始文件名（可能不存在）
    pub(super) file_name: Option<String>,
    /// 文件大小（字节），可能不存在
    pub(super) file_size: Option<u64>,
    /// 附件的说明文字或描述
    pub(super) caption: Option<String>,
    /// 附件的类型分类
    pub(super) kind: IncomingAttachmentKind,
}

/// 接收附件的类型枚举
///
/// 分类从 Telegram 接收到的附件类型，
/// 主要区分图片和通用文档。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IncomingAttachmentKind {
    /// 通用文档类型（非图片的文件）
    Document,
    /// 图片类型（Telegram 单独处理的图片消息）
    Photo,
}

/// Telegram 附件类型枚举
///
/// 定义 Telegram Bot API 支持发送的附件类型，
/// 用于构建出站消息中的附件标记。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TelegramAttachmentKind {
    /// 图片类型（png、jpg、gif、webp、bmp 等）
    Image,
    /// 通用文档类型（pdf、txt、zip 等）
    Document,
    /// 视频类型（mp4、mov、webm 等）
    Video,
    /// 音频类型（mp3、m4a、wav 等）
    Audio,
    /// 语音消息类型（ogg、opus 等）
    Voice,
}

/// Telegram 附件的结构化表示
///
/// 包含附件类型和目标路径或 URL。
/// 用于在消息解析时提取附件信息，并在发送时构造 API 调用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TelegramAttachment {
    /// 附件的类型分类
    pub(super) kind: TelegramAttachmentKind,
    /// 附件目标：可以是文件路径或 HTTP(S) URL
    pub(super) target: String,
}

impl TelegramAttachmentKind {
    /// 从标记字符串解析附件类型
    ///
    /// 将用户友好的标记字符串转换为附件类型枚举。
    /// 支持多种别名以提高容错性。
    ///
    /// # 参数
    ///
    /// - `marker`: 标记字符串（不区分大小写）
    ///
    /// # 返回值
    ///
    /// - `Some(TelegramAttachmentKind)`: 识别到的类型
    /// - `None`: 无法识别的标记
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use TelegramAttachmentKind::*;
    /// assert_eq!(from_marker("IMAGE"), Some(Image));
    /// assert_eq!(from_marker("photo"), Some(Image));
    /// assert_eq!(from_marker("DOCUMENT"), Some(Document));
    /// assert_eq!(from_marker("unknown"), None);
    /// ```
    pub(super) fn from_marker(marker: &str) -> Option<Self> {
        match marker.trim().to_ascii_uppercase().as_str() {
            "IMAGE" | "PHOTO" => Some(Self::Image),
            "DOCUMENT" | "FILE" => Some(Self::Document),
            "VIDEO" => Some(Self::Video),
            "AUDIO" => Some(Self::Audio),
            "VOICE" => Some(Self::Voice),
            _ => None,
        }
    }
}

/// 检查文件扩展名是否为图片格式
///
/// 通过文件扩展名判断是否属于支持的图片格式。
///
/// # 参数
///
/// - `path`: 文件路径引用
///
/// # 返回值
///
/// - `true`: 文件扩展名为 png、jpg、jpeg、gif、webp 或 bmp
/// - `false`: 无扩展名或非图片扩展名
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
/// assert!(is_image_extension(Path::new("photo.jpg")));
/// assert!(is_image_extension(Path::new("icon.PNG")));
/// assert!(!is_image_extension(Path::new("document.pdf")));
/// ```
pub(super) fn is_image_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp"
            )
        })
        .unwrap_or(false)
}

/// 格式化附件内容为可读字符串
///
/// 根据附件类型和文件路径生成用户可读的描述文本。
/// 图片类型使用简洁标记格式，文档类型显示文件名和路径。
///
/// # 参数
///
/// - `kind`: 附件类型分类
/// - `local_filename`: 显示用的文件名
/// - `local_path`: 文件的本地路径
///
/// # 返回值
///
/// 格式化后的字符串：
/// - 图片类型：`[IMAGE:/path/to/file.jpg]`
/// - 其他类型：`[Document: filename] /path/to/file`
pub(super) fn format_attachment_content(
    kind: IncomingAttachmentKind,
    local_filename: &str,
    local_path: &Path,
) -> String {
    match kind {
        IncomingAttachmentKind::Photo | IncomingAttachmentKind::Document
            if is_image_extension(local_path) =>
        {
            format!("[IMAGE:{}]", local_path.display())
        }
        _ => {
            format!("[Document: {}] {}", local_filename, local_path.display())
        }
    }
}

/// 检查目标字符串是否为 HTTP(S) URL
///
/// 简单检查字符串是否以 `http://` 或 `https://` 开头。
///
/// # 参数
///
/// - `target`: 目标字符串
///
/// # 返回值
///
/// - `true`: 以 http:// 或 https:// 开头
/// - `false`: 其他情况
pub(super) fn is_http_url(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

/// 净化附件文件名以确保安全使用
///
/// 对输入文件名进行安全处理：
/// - 提取纯文件名部分（去除目录路径）
/// - 将路径分隔符替换为下划线
/// - 限制文件名长度为 128 个字符
/// - 拒绝特殊目录名（`.` 和 `..`）
///
/// # 参数
///
/// - `file_name`: 原始文件名（可能包含路径）
///
/// # 返回值
///
/// - `Some(String)`: 净化后的安全文件名
/// - `None`: 文件名无效或被拒绝
///
/// # 示例
///
/// ```ignore
/// assert_eq!(sanitize_attachment_filename("doc.pdf"), Some("doc.pdf".to_string()));
/// assert_eq!(sanitize_attachment_filename("/path/to/file.txt"), Some("file.txt".to_string()));
/// assert_eq!(sanitize_attachment_filename(".."), None);
/// ```
pub(super) fn sanitize_attachment_filename(file_name: &str) -> Option<String> {
    // 提取文件名部分（去除路径前缀）
    let basename = Path::new(file_name).file_name()?.to_str()?.trim();

    // 拒绝空文件名和特殊目录
    if basename.is_empty() || basename == "." || basename == ".." {
        return None;
    }

    // 替换路径分隔符并限制长度
    let sanitized: String = basename.replace(['/', '\\'], "_").chars().take(128).collect();

    // 再次检查净化后的结果
    if sanitized.is_empty() || sanitized == "." || sanitized == ".." {
        None
    } else {
        Some(sanitized)
    }
}

/// 净化自动生成的文件扩展名
///
/// 对扩展名进行规范化处理：
/// - 仅保留字母数字字符
/// - 转换为小写
/// - 限制长度为 8 个字符
/// - 空结果时默认返回 "jpg"
///
/// # 参数
///
/// - `raw_ext`: 原始扩展名字符串
///
/// # 返回值
///
/// 净化后的扩展名字符串
pub(super) fn sanitize_generated_extension(raw_ext: &str) -> String {
    let cleaned: String = raw_ext
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>()
        .to_ascii_lowercase();
    if cleaned.is_empty() { "jpg".to_string() } else { cleaned }
}

/// 解析并验证工作区内的附件路径（读取用）
///
/// 将目标字符串解析为工作区内的绝对路径，并执行安全验证：
/// - 拒绝包含空字节的路径
/// - 规范化工作区前缀（/workspace/）
/// - 防止路径遍历攻击（确保解析后的路径不逃逸工作区）
/// - 验证路径指向真实文件
///
/// # 参数
///
/// - `workspace`: 工作区根目录路径
/// - `target`: 目标路径字符串（可以是绝对路径、相对路径或 /workspace/ 前缀路径）
///
/// # 返回值
///
/// - `Ok(PathBuf)`: 解析后的绝对文件路径
/// - `Err`: 路径无效、不存在、逃逸工作区或不是文件
///
/// # 错误
///
/// - 路径包含空字节
/// - 路径不存在
/// - 路径逃逸工作区边界
/// - 路径不是文件（可能是目录或其他类型）
///
/// # 安全说明
///
/// 此函数是防止路径遍历攻击的关键防线，确保用户无法通过
/// 相对路径（如 `../../../etc/passwd`）访问工作区外的文件。
pub(super) fn resolve_workspace_attachment_path(
    workspace: &Path,
    target: &str,
) -> anyhow::Result<PathBuf> {
    // 拒绝包含空字节的路径（防止 C 字符串攻击）
    if target.contains('\0') {
        anyhow::bail!("Telegram attachment path contains null byte");
    }

    // 获取工作区的规范化路径（用于后续边界检查）
    let workspace_root = workspace.canonicalize().unwrap_or_else(|_| workspace.to_path_buf());

    // 根据目标格式构建候选路径
    let candidate = if let Some(rel) = target.strip_prefix("/workspace/") {
        // /workspace/ 前缀：相对于工作区根目录
        workspace.join(rel)
    } else if target == "/workspace" {
        // /workspace 本身：工作区根目录
        workspace.to_path_buf()
    } else {
        // 其他情况：根据是否绝对路径决定拼接方式
        let raw = Path::new(target);
        if raw.is_absolute() { raw.to_path_buf() } else { workspace.join(raw) }
    };

    // 规范化候选路径（解析符号链接和 . ..）
    let resolved = candidate
        .canonicalize()
        .with_context(|| format!("Telegram attachment path not found: {target}"))?;

    // 边界检查：确保解析后的路径仍在工作区内
    if !resolved.starts_with(&workspace_root) {
        anyhow::bail!("Telegram attachment path escapes workspace: {target}");
    }

    // 类型检查：确保路径指向文件而非目录
    if !resolved.is_file() {
        anyhow::bail!("Telegram attachment path is not a file: {}", resolved.display());
    }

    Ok(resolved)
}

/// 解析并创建附件输出路径（写入用）
///
/// 为接收到的 Telegram 附件准备安全的保存路径：
/// - 净化文件名
/// - 在工作区内的 telegram_files 子目录中创建路径
/// - 防止通过符号链接逃逸工作区
/// - 验证现有文件类型（如果存在）
///
/// # 参数
///
/// - `workspace`: 工作区根目录路径
/// - `file_name`: 原始文件名
///
/// # 返回值
///
/// - `Ok(PathBuf)`: 可安全写入的绝对文件路径
/// - `Err`: 文件名无效、目录创建失败、路径逃逸或类型不匹配
///
/// # 异步
///
/// 此函数为异步函数，涉及文件系统 I/O 操作。
///
/// # 错误
///
/// - 文件名无效（空、特殊目录等）
/// - 无法创建目录
/// - 保存目录逃逸工作区边界
/// - 目标路径是符号链接（安全风险）
/// - 目标路径存在但不是常规文件
///
/// # 安全说明
///
/// 拒绝通过符号链接写入是防止 TOCTOU 攻击和
/// 工作区外文件覆盖的重要措施。
pub(super) async fn resolve_workspace_attachment_output_path(
    workspace: &Path,
    file_name: &str,
) -> anyhow::Result<PathBuf> {
    // 净化文件名，确保安全
    let safe_name = sanitize_attachment_filename(file_name)
        .ok_or_else(|| anyhow::anyhow!("invalid attachment filename: {file_name}"))?;

    // 确保工作区目录存在
    fs::create_dir_all(workspace).await?;
    let workspace_root =
        fs::canonicalize(workspace).await.unwrap_or_else(|_| workspace.to_path_buf());

    // 在工作区内创建专用的附件存储子目录
    let save_dir = workspace.join("telegram_files");
    fs::create_dir_all(&save_dir).await?;
    let resolved_save_dir = fs::canonicalize(&save_dir).await.with_context(|| {
        format!("failed to resolve Telegram attachment save directory: {}", save_dir.display())
    })?;

    // 边界检查：确保保存目录在工作区内
    if !resolved_save_dir.starts_with(&workspace_root) {
        anyhow::bail!(
            "Telegram attachment save directory escapes workspace: {}",
            save_dir.display()
        );
    }

    // 构建完整的输出路径
    let output_path = resolved_save_dir.join(safe_name);

    // 检查现有文件（如果存在）的类型和安全性
    match fs::symlink_metadata(&output_path).await {
        Ok(meta) => {
            // 拒绝通过符号链接写入（防止工作区外文件操作）
            if meta.file_type().is_symlink() {
                anyhow::bail!(
                    "refusing to write Telegram attachment through symlink: {}",
                    output_path.display()
                );
            }
            // 确保现有路径是常规文件
            if !meta.is_file() {
                anyhow::bail!(
                    "Telegram attachment output path is not a regular file: {}",
                    output_path.display()
                );
            }
        }
        // 文件不存在是正常情况
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        // 其他错误需要报告
        Err(e) => return Err(e.into()),
    }

    Ok(output_path)
}

/// 从目标路径或 URL 推断附件类型
///
/// 通过文件扩展名自动判断附件应该使用的类型。
/// 会忽略 URL 中的查询参数和锚点。
///
/// # 参数
///
/// - `target`: 目标路径或 URL 字符串
///
/// # 返回值
///
/// - `Some(TelegramAttachmentKind)`: 成功识别的附件类型
/// - `None`: 无扩展名或扩展名不在已知列表中
///
/// # 支持的扩展名
///
/// - **图片**: png、jpg、jpeg、gif、webp、bmp
/// - **视频**: mp4、mov、mkv、avi、webm
/// - **音频**: mp3、m4a、wav、flac
/// - **语音**: ogg、oga、opus
/// - **文档**: pdf、txt、md、csv、json、zip、tar、gz、doc、docx、xls、xlsx、ppt、pptx
///
/// # 示例
///
/// ```ignore
/// use TelegramAttachmentKind::*;
/// assert_eq!(infer_attachment_kind_from_target("photo.jpg"), Some(Image));
/// assert_eq!(infer_attachment_kind_from_target("video.mp4?v=1"), Some(Video));
/// assert_eq!(infer_attachment_kind_from_target("data.json"), Some(Document));
/// assert_eq!(infer_attachment_kind_from_target("unknown.xyz"), None);
/// ```
pub(super) fn infer_attachment_kind_from_target(target: &str) -> Option<TelegramAttachmentKind> {
    // 移除 URL 查询参数和锚点，只保留路径部分
    let normalized = target.split('?').next().unwrap_or(target).split('#').next().unwrap_or(target);

    // 提取并规范化扩展名
    let extension =
        Path::new(normalized).extension().and_then(|ext| ext.to_str())?.to_ascii_lowercase();

    // 根据扩展名映射到附件类型
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => Some(TelegramAttachmentKind::Image),
        "mp4" | "mov" | "mkv" | "avi" | "webm" => Some(TelegramAttachmentKind::Video),
        "mp3" | "m4a" | "wav" | "flac" => Some(TelegramAttachmentKind::Audio),
        "ogg" | "oga" | "opus" => Some(TelegramAttachmentKind::Voice),
        "pdf" | "txt" | "md" | "csv" | "json" | "zip" | "tar" | "gz" | "doc" | "docx" | "xls"
        | "xlsx" | "ppt" | "pptx" => Some(TelegramAttachmentKind::Document),
        _ => None,
    }
}

/// 解析纯路径格式的附件（无标记前缀）
///
/// 尝试将整条消息解析为单个附件路径或 URL。
/// 仅在消息满足以下条件时返回附件：
/// - 单行文本
/// - 不含空白字符
/// - 可识别为已知附件类型的扩展名
/// - 是有效的 HTTP(S) URL 或存在的本地文件
///
/// # 参数
///
/// - `message`: 原始消息文本
///
/// # 返回值
///
/// - `Some(TelegramAttachment)`: 成功解析的附件
/// - `None`: 消息不符合纯路径格式
///
/// # 支持的格式
///
/// - 裸路径: `/path/to/file.jpg`
/// - URL: `https://example.com/image.png`
/// - 引号包围: `"/path/to/file.pdf"` 或 `'path/to/file.mp3'`
/// - 反引号包围: `` `/path/to/file.mp4` ``
/// - file:// 前缀: `file:///path/to/file.doc`
///
/// # 示例
///
/// ```ignore
/// assert!(parse_path_only_attachment("https://example.com/photo.jpg").is_some());
/// assert!(parse_path_only_attachment("/data/report.pdf").is_some());
/// assert!(parse_path_only_attachment("multiple\nlines").is_none());
/// ```
pub(super) fn parse_path_only_attachment(message: &str) -> Option<TelegramAttachment> {
    let trimmed = message.trim();

    // 拒绝空消息和多行消息
    if trimmed.is_empty() || trimmed.contains('\n') {
        return None;
    }

    // 移除外围的引号或反引号
    let candidate = trimmed.trim_matches(|c| matches!(c, '`' | '"' | '\''));

    // 拒绝含空白字符的消息（可能包含其他内容）
    if candidate.chars().any(char::is_whitespace) {
        return None;
    }

    // 移除 file:// 前缀（如果存在）
    let candidate = candidate.strip_prefix("file://").unwrap_or(candidate);

    // 从目标推断附件类型
    let kind = infer_attachment_kind_from_target(candidate)?;

    // 验证目标存在性：HTTP URL 或本地文件
    if !is_http_url(candidate) && !Path::new(candidate).exists() {
        return None;
    }

    Some(TelegramAttachment { kind, target: candidate.to_string() })
}

/// 查找匹配的闭合方括号位置
///
/// 在字符串中查找与起始方括号匹配的闭合方括号，
/// 正确处理嵌套的方括号结构。
///
/// # 参数
///
/// - `s`: 起始方括号之后的字符串
///
/// # 返回值
///
/// - `Some(usize)`: 匹配的闭合方括号的字节位置
/// - `None`: 未找到匹配的闭合方括号
///
/// # 示例
///
/// ```ignore
/// assert_eq!(find_matching_close("abc]"), Some(3));
/// assert_eq!(find_matching_close("a[b]c]"), Some(4));
/// assert_eq!(find_matching_close("no match"), None);
/// ```
fn find_matching_close(s: &str) -> Option<usize> {
    let mut depth = 1usize; // 从深度 1 开始（已进入第一层方括号）

    for (i, ch) in s.char_indices() {
        match ch {
            '[' => depth += 1, // 嵌套层级增加
            ']' => {
                depth -= 1; // 嵌套层级减少
                if depth == 0 {
                    return Some(i); // 找到匹配的闭合括号
                }
            }
            _ => {}
        }
    }
    None // 未找到匹配的闭合括号
}

/// 解析消息中的附件标记并提取附件
///
/// 扫描消息文本，识别并提取所有符合格式 `[KIND:target]` 的附件标记。
/// 返回清理后的消息文本和提取到的附件列表。
///
/// # 标记格式
///
/// ```text
/// [IMAGE:/path/to/photo.jpg]
/// [DOCUMENT:https://example.com/file.pdf]
/// [VIDEO:/workspace/output.mp4]
/// [AUDIO:song.mp3]
/// ```
///
/// # 参数
///
/// - `message`: 原始消息文本
///
/// # 返回值
///
/// 元组 `(cleaned_message, attachments)`:
/// - `cleaned_message`: 移除附件标记后的消息文本（已 trim）
/// - `attachments`: 提取到的附件向量
///
/// # 嵌套处理
///
/// 正确处理嵌套的方括号结构，将无法解析的嵌套内容保留在消息中。
///
/// # 示例
///
/// ```ignore
/// let (text, atts) = parse_attachment_markers("See [IMAGE:photo.jpg] attached");
/// assert_eq!(text, "See attached");
/// assert_eq!(atts.len(), 1);
/// ```
pub(super) fn parse_attachment_markers(message: &str) -> (String, Vec<TelegramAttachment>) {
    let mut cleaned = String::with_capacity(message.len());
    let mut attachments = Vec::new();
    let mut cursor = 0; // 当前处理位置

    while cursor < message.len() {
        // 查找下一个起始方括号
        let Some(open_rel) = message[cursor..].find('[') else {
            // 没有更多方括号，追加剩余内容
            cleaned.push_str(&message[cursor..]);
            break;
        };

        let open = cursor + open_rel;
        // 追加方括号之前的文本
        cleaned.push_str(&message[cursor..open]);

        // 查找匹配的闭合方括号
        let Some(close_rel) = find_matching_close(&message[open + 1..]) else {
            // 未找到匹配的闭合括号，保留原样
            cleaned.push_str(&message[open..]);
            break;
        };

        let close = open + 1 + close_rel;
        // 提取方括号内的标记内容
        let marker = &message[open + 1..close];

        // 尝试解析标记为 "KIND:target" 格式
        let parsed = marker.split_once(':').and_then(|(kind, target)| {
            let kind = TelegramAttachmentKind::from_marker(kind)?;
            let target = target.trim();
            if target.is_empty() {
                return None;
            }
            Some(TelegramAttachment { kind, target: target.to_string() })
        });

        if let Some(attachment) = parsed {
            // 成功解析：添加到附件列表（不保留在消息中）
            attachments.push(attachment);
        } else {
            // 解析失败：保留原始标记在消息中
            cleaned.push_str(&message[open..=close]);
        }

        cursor = close + 1; // 移动到闭合括号之后
    }

    (cleaned.trim().to_string(), attachments)
}

/// Telegram Bot API 单个文件下载的最大字节数限制
///
/// Telegram Bot API 对文件大小有硬性限制（通常为 20MB），
/// 超过此限制的文件需要使用不同的下载策略或通知用户。
///
/// 当前设置为 20MB (20 * 1024 * 1024 = 20,971,520 字节)。
pub(super) const TELEGRAM_MAX_FILE_DOWNLOAD_BYTES: u64 = 20 * 1024 * 1024;
