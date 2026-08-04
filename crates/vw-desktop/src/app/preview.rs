//! 文件预览模块
//!
//! 本模块提供文件内容预览功能，包括：
//! - 安全读取和显示文件内容（自动截断过大的文件）
//! - 清理 ANSI 转义序列和控制字符
//! - 长行自动换行处理
//! - 搜索匹配高亮支持
//! - LSP（语言服务器协议）集成支持（非 WASM 平台）
//!
//! # 主要组件
//!
//! - [`PreviewTab`]: 预览标签页结构体，存储单个文件的预览状态
//! - [`LspHoverPending`]: LSP 悬停请求的待处理状态
//! - [`LspProgress`]: LSP 进度通知
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::preview::{PreviewTab, safe_preview};
//!
//! // 安全预览文件内容
//! let (content, truncated) = safe_preview("/path/to/file.rs");
//! println!("内容: {}", content);
//! println!("是否被截断: {}", truncated);
//! ```

use crate::app::components::editor::Editor;
use iced::widget::Id;

/// 预览标签页
///
/// 存储单个文件在预览视图中的完整状态，包括：
/// - 文件路径和标题
/// - 文件内容（可能已截断）
/// - 编辑器组件状态
/// - 滚动位置标识
/// - LSP 相关信息（非 WASM 平台）
///
/// # 字段说明
///
/// - `path`: 文件的完整路径
/// - `title`: 显示在标签页上的标题
/// - `content`: 文件的实际内容
/// - `truncated`: 文件是否因过大而被截断
/// - `editor`: 编辑器组件实例
/// - `scroll_id`: 滚动容器的唯一标识符
/// - `lsp_server_key`: LSP 服务器标识键（非 WASM）
/// - `lsp_uri`: LSP 文件 URI（非 WASM）
/// - `lsp_language_id`: LSP 语言标识符（非 WASM）
pub struct PreviewTab {
    /// 文件的完整路径
    pub path: String,
    /// 标签页显示标题
    pub title: String,
    /// 文件内容
    pub content: String,
    /// 编辑器内容是否相对已保存版本发生变化
    pub is_dirty: bool,
    /// 文件是否被截断
    pub truncated: bool,
    /// 自动保存版本号，用于丢弃过期的延迟保存任务。
    pub auto_save_revision: u64,
    /// 编辑器组件
    pub editor: Editor,
    /// 滚动容器 ID
    pub scroll_id: Id,
    /// LSP 服务器键（非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_server_key: Option<&'static str>,
    /// LSP 文件 URI（非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_uri: Option<String>,
    /// LSP 语言标识符（非 WASM 平台）
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_language_id: Option<String>,
}

/// LSP 悬停请求待处理状态
///
/// 当用户在编辑器中悬停时，会创建此结构体来跟踪悬停请求的状态。
/// 包含请求的位置信息、屏幕坐标以及请求时间戳。
///
/// # 字段说明
///
/// - `path`: 悬停目标文件的路径
/// - `position`: LSP 协议中的文本位置（行、列）
/// - `point`: 屏幕上的鼠标位置（用于显示工具提示）
/// - `ready_at`: 请求创建的时间戳（用于超时检测）
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub(crate) struct LspHoverPending {
    /// 悬停目标文件路径
    pub(crate) path: String,
    /// LSP 文本位置（行、列）
    pub(crate) position: iced_code_editor::LspPosition,
    /// 屏幕鼠标位置
    pub(crate) point: iced::Point,
    /// 请求创建时间
    pub(crate) ready_at: std::time::Instant,
}

/// LSP 进度通知
///
/// 表示来自语言服务器的进度更新，通常用于长时间运行的操作
/// （如索引项目、分析代码等）。
///
/// # 字段说明
///
/// - `title`: 进度操作的标题
/// - `message`: 可选的详细消息
/// - `percentage`: 可选的完成百分比（0-100）
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub(crate) struct LspProgress {
    /// 进度标题
    pub(crate) title: String,
    /// 详细消息
    pub(crate) message: Option<String>,
    /// 完成百分比
    pub(crate) percentage: Option<u32>,
}

/// 将文件系统路径转换为文件 URI
///
/// 将操作系统路径转换为 LSP 协议使用的 `file://` URI 格式。
/// 自动处理路径中的空格（转换为 `%20`）。
///
/// # 参数
///
/// - `path`: 要转换的文件系统路径
///
/// # 返回值
///
/// 返回格式为 `file://<path>` 的 URI 字符串
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
/// use crate::app::preview::path_to_file_uri;
///
/// let uri = path_to_file_uri(Path::new("/home/user/my file.rs"));
/// assert_eq!(uri, "file:///home/user/my%20file.rs");
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn path_to_file_uri(path: &std::path::Path) -> String {
    let mut uri = String::from("file://");
    // 将路径中的空格替换为 URL 编码
    let path_str = path.to_string_lossy().replace(' ', "%20");
    uri.push_str(&path_str);
    uri
}

impl std::fmt::Debug for PreviewTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 仅调试输出关键字段，避免大量内容污染日志
        f.debug_struct("PreviewTab").field("path", &self.path).field("title", &self.title).finish()
    }
}

/// 清理 ANSI 转义序列和控制字符
///
/// 从文本中移除所有 ANSI 转义序列和其他控制字符，
/// 保留换行符和制表符。支持的转义序列包括：
/// - CSI 序列（ESC [ ...）：颜色、样式等
/// - OSC 序列（ESC ] ...）：窗口标题、超链接等
///
/// # 参数
///
/// - `s`: 要清理的输入字符串
///
/// # 返回值
///
/// 返回清理后的字符串，不含 ANSI 转义序列和大部分控制字符
///
/// # 处理逻辑
///
/// 1. 遍历字节序列，检测转义序列的起始（0x1b）
/// 2. 跳过完整的 CSI 序列（ESC [ ... 终止符）
/// 3. 跳过完整的 OSC 序列（ESC ] ... BEL 或 ESC \）
/// 4. 移除控制字符（< 0x20），保留换行和制表符
/// 5. 移除 DEL 字符（0x7f）
fn strip_ansi_and_controls(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut i = 0;
    // 预分配输出缓冲区
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());

    while i < bytes.len() {
        let b = bytes[i];

        // 检测转义序列起始
        if b == 0x1b {
            if i + 1 < bytes.len() {
                let next = bytes[i + 1];

                // CSI 序列: ESC [ ... (终止符在 0x40-0x7E)
                if next == b'[' {
                    i += 2;
                    while i < bytes.len() {
                        let c = bytes[i];
                        // CSI 终止符范围：@A-Z[\]^_`a-z{|}~
                        if (0x40..=0x7E).contains(&c) {
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                    continue;
                // OSC 序列: ESC ] ... (BEL 或 ESC \ 结尾)
                } else if next == b']' {
                    i += 2;
                    loop {
                        if i >= bytes.len() {
                            break;
                        }
                        let c = bytes[i];
                        // OSC 以 BEL (0x07) 结尾
                        if c == 0x07 {
                            i += 1;
                            break;
                        }
                        // 或以 ESC \ (ST) 结尾
                        if c == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    continue;
                } else {
                    // 其他转义序列：跳过 ESC 和下一个字符
                    i += 2;
                    continue;
                }
            } else {
                i += 1;
                continue;
            }
        }

        // 移除控制字符（保留换行和制表符）
        if b < 0x20 && b != b'\n' && b != b'\t' {
            i += 1;
            continue;
        }

        // 移除 DEL 字符
        if b == 0x7f {
            i += 1;
            continue;
        }

        out.push(b);
        i += 1;
    }

    String::from_utf8_lossy(&out).to_string()
}

/// 长行自动换行
///
/// 对超过指定长度的行进行换行处理，确保每行不超过最大字符数。
///
/// # 参数
///
/// - `s`: 要处理的输入字符串
/// - `max`: 每行的最大字符数
///
/// # 返回值
///
/// 返回换行处理后的字符串
///
/// # 处理逻辑
///
/// 遍历每个字符，当计数器达到 max 时插入换行符。
/// 遇到自然换行符时重置计数器。
fn wrap_long_lines(s: &str, max: usize) -> String {
    let mut out = String::with_capacity(s.len());
    let mut count = 0usize;

    for ch in s.chars() {
        // 遇到换行符时重置计数
        if ch == '\n' {
            count = 0;
            out.push(ch);
            continue;
        }

        out.push(ch);
        count += 1;

        // 达到最大长度时插入换行
        if count >= max {
            out.push('\n');
            count = 0;
        }
    }

    out
}

/// 安全预览文件文本内容
///
/// 读取文件内容并进行安全处理，包括清理控制字符、
/// 自动换行和截断。返回适合显示的纯文本内容。
///
/// # 参数
///
/// - `path`: 要预览的文件路径
///
/// # 返回值
///
/// 返回处理后的文件内容字符串。如果文件过大，
/// 会在内容开头添加"文件较大，预览已截断。"提示。
///
/// # 限制
///
/// - 最大读取字节数：300,000 字节（约 300KB）
/// - 每行最大字符数：512 字符
/// - 最大行数：5000 行
///
/// # 示例
///
/// ```ignore
/// use crate::app::preview::safe_preview_text;
///
/// let content = safe_preview_text("/path/to/file.txt");
/// println!("{}", content);
/// ```
pub fn safe_preview_text(path: &str) -> String {
    /// 最大读取字节数（约 300KB）
    const MAX_BYTES: usize = 300_000;
    /// 每行最大字符数
    const MAX_LINE: usize = 512;
    /// 最大行数
    const MAX_LINES: usize = 5000;

    let mut truncated = false;
    let mut buf: Vec<u8> = Vec::new();

    // 尝试读取文件（限制字节数）
    let read_ok = (|| {
        use std::fs::File;
        use std::io::{BufReader, Read};

        let f = File::open(path)?;

        // 检查文件大小，标记是否需要截断
        if let Ok(m) = f.metadata()
            && m.len() > MAX_BYTES as u64
        {
            truncated = true;
        }

        let reader = BufReader::new(f);
        let mut take = reader.take(MAX_BYTES as u64);
        take.read_to_end(&mut buf)?;

        Ok::<(), std::io::Error>(())
    })()
    .is_ok();

    // 转换为字符串
    let content = if read_ok {
        String::from_utf8_lossy(&buf).to_string()
    } else {
        // 回退：尝试直接读取整个文件
        std::fs::read_to_string(path).unwrap_or_else(|_| String::new())
    };

    // 清理 ANSI 转义序列和控制字符
    let cleaned = strip_ansi_and_controls(&content);

    // 长行自动换行
    let wrapped = wrap_long_lines(&cleaned, MAX_LINE);

    // 限制行数
    let limited = {
        let mut out = String::new();
        for (line_index, line) in wrapped.lines().enumerate() {
            if line_index >= MAX_LINES {
                truncated = true;
                break;
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    };

    // 如果被截断，添加提示信息
    if truncated {
        let mut head = String::new();
        head.push_str("文件较大，预览已截断。\n\n");
        head.push_str(&limited);
        head
    } else {
        limited
    }
}

/// 安全预览文件（带类型检测）
///
/// 读取文件内容并根据文件类型进行智能处理。
/// 对于二进制文件（图片、压缩包、PDF），返回类型信息而非内容。
/// 对于文本文件，进行安全处理后返回内容。
///
/// # 参数
///
/// - `path`: 要预览的文件路径
///
/// # 返回值
///
/// 返回元组 `(内容, 是否被截断)`：
/// - 内容：文件的文本内容或类型描述
/// - 是否被截断：`true` 表示文件被截断，`false` 表示完整显示
///
/// # 支持的文件类型
///
/// - **图片**: png, jpg, jpeg, gif, bmp, webp, svg
/// - **压缩包**: zip, tar, gz, rar, 7z, bz2, xz
/// - **文档**: pdf
/// - **其他**: 作为文本文件处理
///
/// # 限制
///
/// - 最大读取字节数：300,000 字节（约 300KB）
/// - 每行最大字符数：512 字符
/// - 最大行数：5000 行
///
/// # 示例
///
/// ```ignore
/// use crate::app::preview::safe_preview;
///
/// let (content, truncated) = safe_preview("/path/to/image.png");
/// println!("类型信息: {}", content);
/// assert_eq!(truncated, false);
///
/// let (content, truncated) = safe_preview("/path/to/large.log");
/// println!("内容: {}", content);
/// ```
fn unsupported_preview_kind(path: &str) -> Option<&'static str> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if matches!(ext.as_str(), "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz") {
        Some("压缩包")
    } else if ext == "pdf" {
        Some("PDF")
    } else {
        None
    }
}

fn read_preview_sample(path: &str) -> Option<Vec<u8>> {
    use std::fs::File;
    use std::io::{Read, Take};

    const SAMPLE_BYTES: u64 = 4096;

    let file = File::open(path).ok()?;
    let mut sample = Vec::new();
    let mut take: Take<File> = file.take(SAMPLE_BYTES);
    take.read_to_end(&mut sample).ok()?;
    Some(sample)
}

const PREVIEW_MAX_BYTES: usize = 300_000;
const PREVIEW_MAX_LINE: usize = 512;
const PREVIEW_MAX_LINES: usize = 5000;

fn file_looks_like_text(path: &str) -> bool {
    let Some(sample) = read_preview_sample(path) else {
        return true;
    };

    std::str::from_utf8(&sample).is_ok()
}

fn truncate_to_char_boundary(content: &str, max_bytes: usize) -> (&str, bool) {
    if content.len() <= max_bytes {
        return (content, false);
    }

    let mut end = max_bytes;
    while end > 0 && !content.is_char_boundary(end) {
        end -= 1;
    }

    (&content[..end], true)
}

pub fn format_text_preview_content(content: &str) -> (String, bool) {
    let (bounded, mut truncated) = truncate_to_char_boundary(content, PREVIEW_MAX_BYTES);

    let cleaned = strip_ansi_and_controls(bounded);
    let wrapped = wrap_long_lines(&cleaned, PREVIEW_MAX_LINE);

    let limited = {
        let mut out = String::new();
        for (line_index, line) in wrapped.lines().enumerate() {
            if line_index >= PREVIEW_MAX_LINES {
                truncated = true;
                break;
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    };

    if truncated {
        let mut head = String::new();
        head.push_str("文件较大，预览已截断。\n\n");
        head.push_str(&limited);
        (head, true)
    } else {
        (limited, false)
    }
}

pub fn preview_open_error(path: &str) -> Option<String> {
    if let Some(kind) = unsupported_preview_kind(path) {
        return Some(format!("不支持展开该文件：{} 文件无法打开", kind));
    }

    if !file_looks_like_text(path) {
        return Some("不支持展开该文件：二进制文件无法打开".to_string());
    }

    None
}

pub fn safe_preview(path: &str) -> (String, bool) {
    let mut truncated = false;

    // 获取文件扩展名（转小写）
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // 检测文件类型
    let is_image = matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp");
    let is_svg = ext.as_str() == "svg";
    // 处理图片文件
    if is_image || is_svg {
        let name = std::path::Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path);
        let info = format!("文件: {}\n类型: 图片\n", name);
        return (info, false);
    }

    if let Some(kind) = unsupported_preview_kind(path) {
        let name = std::path::Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path);
        let info = format!("文件: {}\n类型: {}\n不支持文本预览\n", name, kind);
        return (info, false);
    }

    if !file_looks_like_text(path) {
        let name = std::path::Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path);
        let info = format!("文件: {}\n类型: 二进制文件\n不支持文本预览\n", name);
        return (info, false);
    }

    let mut buf: Vec<u8> = Vec::new();

    // 尝试读取文件（限制字节数）
    let read_ok = (|| {
        use std::fs::File;
        use std::io::{BufReader, Read};

        let f = File::open(path)?;

        // 检查文件大小，标记是否需要截断
        if let Ok(m) = f.metadata()
            && m.len() > PREVIEW_MAX_BYTES as u64
        {
            truncated = true;
        }

        let reader = BufReader::new(f);
        let mut take = reader.take(PREVIEW_MAX_BYTES as u64);
        take.read_to_end(&mut buf)?;

        Ok::<(), std::io::Error>(())
    })()
    .is_ok();

    // 转换为字符串
    let content = if read_ok {
        String::from_utf8_lossy(&buf).to_string()
    } else {
        // 回退：尝试直接读取整个文件
        std::fs::read_to_string(path).unwrap_or_else(|_| String::new())
    };

    let (formatted, content_truncated) = format_text_preview_content(&content);
    (formatted, truncated || content_truncated)
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod preview_tests;

/// 计算搜索匹配位置
///
/// 在文本内容中查找所有搜索词的出现位置，返回每个匹配的
/// 行号、起始列和长度。结果按位置排序，便于高亮显示。
///
/// # 参数
///
/// - `content`: 要搜索的文本内容
/// - `terms`: 搜索词列表（大小写敏感）
///
/// # 返回值
///
/// 返回匹配位置列表，每个元素为 `(行号, 起始列, 匹配长度)`。
/// 结果按行号和列号升序排列。
///
/// # 示例
///
/// ```ignore
/// use crate::app::preview::compute_search_matches;
///
/// let content = "hello world\nhello rust";
/// let terms = vec!["hello".to_string()];
/// let matches = compute_search_matches(content, &terms);
///
/// // 返回 [(0, 0, 5), (1, 0, 5)]
/// // 第 0 行第 0 列开始，长度 5
/// // 第 1 行第 0 列开始，长度 5
/// ```
///
/// # 注意
///
/// - 空搜索词会被跳过
/// - 支持一行内多次匹配
/// - 同一位置不会重复报告
pub fn compute_search_matches(content: &str, terms: &[String]) -> Vec<(usize, usize, usize)> {
    let mut out: Vec<(usize, usize, usize)> = Vec::new();

    // 遍历每一行
    for (li, line) in content.lines().enumerate() {
        // 对每个搜索词进行匹配
        for term in terms {
            if term.is_empty() {
                continue;
            }

            let mut start = 0usize;

            // 查找行内所有匹配
            while start <= line.len() {
                if let Some(pos) = line[start..].find(term) {
                    let st = start + pos; // 绝对起始位置
                    let ln = term.len(); // 匹配长度
                    out.push((li, st, ln));
                    start = st + ln; // 继续搜索后续内容
                } else {
                    break;
                }
            }
        }
    }

    // 按行号和列号排序
    out.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    out
}
