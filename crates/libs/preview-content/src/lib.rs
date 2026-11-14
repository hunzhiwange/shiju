//! 预览文件读取、文本清理、换行、搜索与文件 URI 转换。

pub const PREVIEW_MAX_BYTES: usize = 300_000;
const PREVIEW_MAX_LINE: usize = 512;
const PREVIEW_MAX_LINES: usize = 5000;

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
/// use preview_content::path_to_file_uri;
///
/// let uri = path_to_file_uri(Path::new("/home/user/my file.rs"));
/// assert_eq!(uri, "file:///home/user/my%20file.rs");
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn path_to_file_uri(path: &std::path::Path) -> String {
    let mut uri = String::from("file://");
    // 将路径中的空格替换为 URL 编码
    let path_str = path.to_string_lossy().replace(' ', "%20");
    uri.push_str(&path_str);
    uri
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
pub fn strip_ansi_and_controls(s: &str) -> String {
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
pub fn wrap_long_lines(s: &str, max: usize) -> String {
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
/// use preview_content::compute_search_matches;
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

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

mod file;
pub use file::{
    FilePreview, UnsupportedPreviewKind, file_looks_like_text, read_file_preview, read_preview_text,
    unsupported_preview_kind,
};
