//! Discord 通道附件处理模块（发送方向）
//!
//! 本模块负责处理从 VibeWindow 代理向 Discord 发送消息时的附件解析与分类工作。
//! 主要功能包括：
//!
//! - 从消息文本中解析附件标记（格式：`[KIND:target]`）
//! - 将附件分类为本地文件或远程 URL
//! - 安全地解析本地文件路径（防止路径逃逸攻击）
//!
//! ## 支持的附件类型
//!
//! - `IMAGE` / `PHOTO`：图片附件
//! - `DOCUMENT` / `FILE`：文档附件
//! - `VIDEO`：视频附件
//! - `AUDIO`：音频附件
//! - `VOICE`：语音附件
//!
//! ## 附件标记格式
//!
//! 在消息中使用 `[TYPE:path_or_url]` 格式来指定附件，例如：
//!
//! ```text
//! 请查看这个文档：[DOCUMENT:/workspace/report.pdf]
//! 这是一张图片：[IMAGE:https://example.com/image.png]
//! ```

use anyhow::Context;
use std::path::{Path, PathBuf};

/// Discord 附件类型枚举
///
/// 定义了 Discord 消息可以携带的不同附件类型。
/// 每种类型对应 Discord API 中不同的附件处理方式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DiscordAttachmentKind {
    /// 图片类型（如 PNG、JPEG、GIF 等）
    Image,
    /// 文档类型（如 PDF、TXT、DOCX 等）
    Document,
    /// 视频类型（如 MP4、WEBM 等）
    Video,
    /// 音频类型（如 MP3、WAV 等）
    Audio,
    /// 语音消息类型（Discord 特有的语音录制格式）
    Voice,
}

impl DiscordAttachmentKind {
    /// 从标记字符串解析附件类型
    ///
    /// 将用户在消息中输入的类型标记转换为对应的枚举变体。
    /// 解析时忽略大小写和前后空白字符。
    ///
    /// # 参数
    ///
    /// - `kind`：类型标记字符串，例如 "IMAGE"、"photo"、"Document" 等
    ///
    /// # 返回值
    ///
    /// - `Some(DiscordAttachmentKind)`：成功解析时返回对应的类型
    /// - `None`：无法识别的类型标记
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use DiscordAttachmentKind::Image;
    /// assert_eq!(DiscordAttachmentKind::from_marker("IMAGE"), Some(Image));
    /// assert_eq!(DiscordAttachmentKind::from_marker("photo"), Some(Image));
    /// assert_eq!(DiscordAttachmentKind::from_marker("unknown"), None);
    /// ```
    pub(super) fn from_marker(kind: &str) -> Option<Self> {
        match kind.trim().to_ascii_uppercase().as_str() {
            "IMAGE" | "PHOTO" => Some(Self::Image),
            "DOCUMENT" | "FILE" => Some(Self::Document),
            "VIDEO" => Some(Self::Video),
            "AUDIO" => Some(Self::Audio),
            "VOICE" => Some(Self::Voice),
            _ => None,
        }
    }

    /// 获取类型的标记名称
    ///
    /// 返回该附件类型对应的标准标记字符串（大写形式），
    /// 用于日志记录和调试输出。
    ///
    /// # 返回值
    ///
    /// 返回静态字符串，表示该类型的标准标记名称
    ///
    /// # 示例
    ///
    /// ```ignore
    /// assert_eq!(DiscordAttachmentKind::Image.marker_name(), "IMAGE");
    /// assert_eq!(DiscordAttachmentKind::Document.marker_name(), "DOCUMENT");
    /// ```
    pub(super) fn marker_name(&self) -> &'static str {
        match self {
            Self::Image => "IMAGE",
            Self::Document => "DOCUMENT",
            Self::Video => "VIDEO",
            Self::Audio => "AUDIO",
            Self::Voice => "VOICE",
        }
    }
}

/// Discord 附件信息结构体
///
/// 封装了从消息中解析出的单个附件的所有相关信息，
/// 包括附件类型和目标路径/URL。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiscordAttachment {
    /// 附件的类型（图片、文档、视频等）
    pub(super) kind: DiscordAttachmentKind,
    /// 附件目标，可以是本地文件路径或远程 URL
    pub(super) target: String,
}

/// 从消息文本中解析附件标记
///
/// 扫描消息内容，提取所有符合 `[TYPE:target]` 格式的附件标记，
/// 同时返回清理后的消息文本（移除了所有有效的附件标记）。
///
/// # 参数
///
/// - `message`：原始消息文本，可能包含附件标记
///
/// # 返回值
///
/// 返回一个元组：
/// - `String`：清理后的消息文本（已移除有效附件标记）
/// - `Vec<DiscordAttachment>`：解析出的附件列表
///
/// # 解析规则
///
/// 1. 附件标记格式为 `[KIND:target]`，其中 KIND 为类型，target 为路径或 URL
/// 2. 类型标记不区分大小写（IMAGE、image、Image 均可）
/// 3. 无效的标记（无法识别的类型或空目标）会被保留在原文本中
/// 4. 未闭合的方括号会被保留在原文本中
///
/// # 示例
///
/// ```ignore
/// let (text, attachments) = parse_attachment_markers(
///     "请看这张图 [IMAGE:/photo.png] 和这个文档 [FILE:report.pdf]"
/// );
/// // text = "请看这张图  和这个文档 "
/// // attachments 包含两个附件
/// ```
pub(super) fn parse_attachment_markers(message: &str) -> (String, Vec<DiscordAttachment>) {
    // 预分配与原消息等长的容量，减少内存重分配
    let mut cleaned = String::with_capacity(message.len());
    let mut attachments = Vec::new();
    // 游标，记录当前解析位置
    let mut cursor = 0usize;

    // 遍历消息中的所有左方括号位置
    while let Some(rel_start) = message[cursor..].find('[') {
        // 转换为绝对位置
        let start = cursor + rel_start;
        // 将方括号之前的普通文本追加到清理结果中
        cleaned.push_str(&message[cursor..start]);

        // 查找对应的右方括号
        let Some(rel_end) = message[start..].find(']') else {
            // 未找到右方括号，保留剩余文本并结束解析
            cleaned.push_str(&message[start..]);
            cursor = message.len();
            break;
        };
        let end = start + rel_end;
        // 提取方括号内的标记文本
        let marker_text = &message[start + 1..end];

        // 尝试解析标记：按第一个冒号分割为类型和目标
        let parsed = marker_text.split_once(':').and_then(|(kind, target)| {
            // 解析类型标记
            let kind = DiscordAttachmentKind::from_marker(kind)?;
            // 清理目标字符串的前后空白
            let target = target.trim();
            // 目标不能为空
            if target.is_empty() {
                return None;
            }
            Some(DiscordAttachment { kind, target: target.to_string() })
        });

        if let Some(attachment) = parsed {
            // 有效标记：添加到附件列表，不保留在清理文本中
            attachments.push(attachment);
        } else {
            // 无效标记：原样保留在清理文本中（包含方括号）
            cleaned.push_str(&message[start..=end]);
        }

        // 移动游标到右方括号之后
        cursor = end + 1;
    }

    // 追加消息末尾剩余的普通文本
    if cursor < message.len() {
        cleaned.push_str(&message[cursor..]);
    }

    // 返回清理后的文本（去除首尾空白）和附件列表
    (cleaned.trim().to_string(), attachments)
}

/// 对附件进行分类
///
/// 将解析出的附件列表分类为本地文件和远程 URL 两类，
/// 以便后续分别处理（本地文件需要上传，远程 URL 可直接引用）。
///
/// # 参数
///
/// - `attachments`：待分类的附件列表
///
/// # 返回值
///
/// 返回一个三元组：
/// - `Vec<DiscordAttachment>`：需要从本地上传的文件附件
/// - `Vec<String>`：可直接引用的远程 URL 列表
/// - `Vec<String>`：无法解析的标记列表（当前实现中始终为空）
///
/// # URL 识别规则
///
/// - 以 `https://` 开头的字符串被识别为远程 URL
/// - 以 `http://` 开头的字符串被识别为远程 URL
/// - 其他情况视为本地文件路径
pub(super) fn classify_outgoing_attachments(
    attachments: &[DiscordAttachment],
) -> (Vec<DiscordAttachment>, Vec<String>, Vec<String>) {
    let mut local_files = Vec::new();
    let mut remote_urls = Vec::new();
    // 当前实现中未使用的未解析标记列表
    let unresolved_markers = Vec::new();

    for attachment in attachments {
        let target = attachment.target.trim();
        // 判断是否为远程 URL
        if target.starts_with("https://") || target.starts_with("http://") {
            remote_urls.push(target.to_string());
            continue;
        }

        // 非 URL 则视为本地文件
        local_files.push(attachment.clone());
    }

    (local_files, remote_urls, unresolved_markers)
}

/// 将远程 URL 和未解析标记内联到消息内容中
///
/// 当某些附件无法作为真正的附件上传时（如远程 URL），
/// 将它们以纯文本形式追加到消息内容中。
///
/// # 参数
///
/// - `content`：原始消息内容
/// - `remote_urls`：远程 URL 列表
/// - `unresolved_markers`：未解析的标记列表
///
/// # 返回值
///
/// 返回合并后的消息文本，各部分用换行符分隔
///
/// # 示例
///
/// ```ignore
/// let result = with_inline_attachment_urls(
///     "请查看这些资源",
///     &["https://example.com/file.pdf".to_string()],
///     &[]
/// );
/// // result = "请查看这些资源\nhttps://example.com/file.pdf"
/// ```
pub(super) fn with_inline_attachment_urls(
    content: &str,
    remote_urls: &[String],
    unresolved_markers: &[String],
) -> String {
    let mut lines = Vec::new();
    // 添加消息内容（非空时）
    if !content.trim().is_empty() {
        lines.push(content.trim().to_string());
    }
    // 添加远程 URL（每个占一行）
    if !remote_urls.is_empty() {
        lines.extend(remote_urls.iter().cloned());
    }
    // 添加未解析的标记（每个占一行）
    if !unresolved_markers.is_empty() {
        lines.extend(unresolved_markers.iter().cloned());
    }
    // 用换行符连接所有行
    lines.join("\n")
}

/// 安全地解析本地附件的绝对路径
///
/// 将用户指定的附件路径解析为绝对路径，同时执行安全检查
/// 以防止路径逃逸攻击（访问工作目录之外的文件）。
///
/// # 参数
///
/// - `workspace_dir`：工作目录的路径，必须已配置才能处理本地文件
/// - `target`：用户指定的文件路径（可以是相对路径、绝对路径或 /workspace/ 前缀路径）
///
/// # 返回值
///
/// - `Ok(PathBuf)`：解析后的绝对文件路径
/// - `Err`：路径解析失败，可能原因包括：
///   - 工作目录未配置
///   - 文件不存在
///   - 路径逃逸出工作目录（安全违规）
///   - 路径不是普通文件
///
/// # 路径解析规则
///
/// 1. `/workspace/xxx` 形式的路径会相对于工作目录解析
/// 2. `/workspace` 等同于工作目录本身
/// 3. 绝对路径直接使用
/// 4. 相对路径相对于工作目录解析
///
/// # 安全性
///
/// 此函数会验证解析后的路径：
/// - 必须位于工作目录内部（防止目录遍历攻击）
/// - 必须是一个实际存在的普通文件
///
/// # 示例
///
/// ```ignore
/// let path = resolve_local_attachment_path(
///     Some(&PathBuf::from("/home/user/workspace")),
///     "/workspace/report.pdf"
/// )?;
/// // path = /home/user/workspace/report.pdf
/// ```
pub(super) fn resolve_local_attachment_path(
    workspace_dir: Option<&PathBuf>,
    target: &str,
) -> anyhow::Result<PathBuf> {
    // 检查工作目录是否已配置
    let workspace = workspace_dir.as_ref().ok_or_else(|| {
        anyhow::anyhow!("workspace_dir is not configured; local file attachments are disabled")
    })?;
    // 获取工作目录的规范路径（解析符号链接等）
    // 如果规范化失败则使用原始路径
    let workspace_root = workspace.canonicalize().unwrap_or_else(|_| workspace.to_path_buf());

    // 根据目标路径的格式进行解析
    let target_path = if let Some(rel) = target.strip_prefix("/workspace/") {
        // /workspace/xxx 形式：相对于工作目录
        workspace.join(rel)
    } else if target == "/workspace" {
        // /workspace 形式：工作目录本身
        workspace.to_path_buf()
    } else {
        let path = Path::new(target);
        if path.is_absolute() {
            // 绝对路径：直接使用
            path.to_path_buf()
        } else {
            // 相对路径：相对于工作目录
            workspace.join(path)
        }
    };

    // 规范化目标路径，同时验证文件是否存在
    let resolved = target_path
        .canonicalize()
        .with_context(|| format!("attachment path not found: {target}"))?;

    // 安全检查：确保路径没有逃逸出工作目录
    if !resolved.starts_with(&workspace_root) {
        anyhow::bail!("attachment path escapes workspace: {target}");
    }

    // 验证路径指向一个普通文件（而非目录或特殊文件）
    if !resolved.is_file() {
        anyhow::bail!("attachment path is not a file: {}", resolved.display());
    }

    Ok(resolved)
}
