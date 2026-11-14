//! Markdown 文档分块模块
//!
//! 该模块提供了将 Markdown 文本按照语义边界和 token 限制进行智能分块的功能。
//! 主要用于在处理大型 Markdown 文档时，将其分割成适合模型处理的小块。
//!
//! # 核心功能
//!
//! - **语义感知分块**：优先在标题边界处分割，保持文档结构的完整性
//! - **自适应大小控制**：根据指定的 token 限制动态调整分块大小
//! - **多级分割策略**：按标题 → 段落 → 行的优先级进行分块
//!
//! # 分块策略
//!
//! 1. 首先按 Markdown 标题（#、##、###）将文档分割为多个节（section）
//! 2. 如果单个节的大小在限制内，则保留为一个完整分块
//! 3. 如果节过大，则按段落（空行分隔）进行分割
//! 4. 如果段落过大，则按行进行分割
//!
//! # 示例
//!
//! ```ignore
//! use memory_algorithms::chunker::chunk_markdown;
//!
//! let markdown = r#"
//! # 第一章
//!
//! 这是第一段内容。
//!
//! # 第二章
//!
//! 这是第二段内容。
//! "#;
//!
//! let chunks = chunk_markdown(markdown, 100);
//! for chunk in chunks {
//!     println!("Chunk {}: {:?}", chunk.index, chunk.heading);
//! }
//! ```

use std::rc::Rc;

/// 文档分块单元
///
/// 表示 Markdown 文档经过分块后的一个片段。
/// 每个分块包含其内容、在序列中的索引以及所属的标题信息。
///
/// # 字段说明
///
/// - `index`：分块在整个序列中的位置索引（从 0 开始）
/// - `content`：分块的文本内容
/// - `heading`：分块所属的 Markdown 标题（如果有），使用引用计数以节省内存
///
/// # 示例
///
/// ```ignore
/// use std::rc::Rc;
///
/// let chunk = Chunk {
///     index: 0,
///     content: "文档内容".to_string(),
///     heading: Some(Rc::from("# 标题")),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct Chunk {
    /// 分块在序列中的位置索引
    pub index: usize,
    /// 分块的文本内容
    pub content: String,
    /// 分块所属的标题（如果有），使用 Rc 以支持多个分块共享同一标题
    pub heading: Option<Rc<str>>,
}

/// 将 Markdown 文本分块
///
/// 该函数是本模块的主入口，负责将 Markdown 文本按照 token 限制分割成多个块。
/// 分块算法采用多级策略，优先保持语义完整性：
///
/// 1. **标题级分块**：优先在 Markdown 标题处分割
/// 2. **段落级分块**：如果节过大，则在段落边界（空行）处分割
/// 3. **行级分块**：如果段落过大，则按行分割
///
/// # 参数
///
/// - `text`：待分块的 Markdown 文本
/// - `max_tokens`：每个分块的最大 token 数量（注意：函数使用 `max_tokens * 4` 估算字符数）
///
/// # 返回值
///
/// 返回一个 `Chunk` 向量，每个 `Chunk` 包含：
/// - 在序列中的索引（自动编号，从 0 开始）
/// - 分块内容（已去除首尾空白）
/// - 所属标题（如果有）
///
/// # 算法细节
///
/// - **字符估算**：使用 `max_tokens * 4` 估算最大字符数（基于 token 平均长度假设）
/// - **标题保留**：每个分块都会包含其所属的标题（如果有）
/// - **空白清理**：最终会移除空白内容，并重新索引
///
/// # 示例
///
/// ```ignore
/// let markdown = "# 标题\n\n这是内容。\n\n## 子标题\n\n更多内容。";
/// let chunks = chunk_markdown(markdown, 50);
///
/// assert!(!chunks.is_empty());
/// for chunk in chunks {
///     println!("索引 {}: {}", chunk.index, chunk.content);
/// }
/// ```
///
/// # 边界情况
///
/// - 空文本或仅包含空白的文本返回空向量
/// - 空内容分块会在最后被过滤掉
pub fn chunk_markdown(text: &str, max_tokens: usize) -> Vec<Chunk> {
    // 处理空文本或仅包含空白的文本
    if text.trim().is_empty() {
        return Vec::new();
    }

    // 基于 token 数估算最大字符数（假设平均每个 token 约 4 个字符）
    let max_chars = max_tokens * 4;
    // 按标题将文档分割为多个节
    let sections = split_on_headings(text);
    // 预分配向量容量以提高性能
    let mut chunks = Vec::with_capacity(sections.len());

    // 遍历每个节进行处理
    for (heading, body) in sections {
        // 将标题字符串转换为引用计数的字符串切片
        let heading: Option<Rc<str>> = heading.map(Rc::from);
        // 如果有标题，则将标题和正文合并；否则只使用正文
        let full = if let Some(ref h) = heading { format!("{h}\n{body}") } else { body.clone() };

        // 如果整个节的大小在限制内，直接作为一个分块
        if full.len() <= max_chars {
            chunks.push(Chunk {
                index: chunks.len(),
                content: full.trim().to_string(),
                heading: heading.clone(),
            });
        } else {
            // 节过大，需要按段落分割
            let paragraphs = split_on_blank_lines(&body);
            // 初始化当前累积内容，如果有标题则先加上标题
            let mut current = heading.as_deref().map_or_else(String::new, |h| format!("{h}\n"));

            // 遍历每个段落
            for para in paragraphs {
                // 如果添加当前段落会超过限制，且当前累积内容非空，则先保存
                if current.len() + para.len() > max_chars && !current.trim().is_empty() {
                    chunks.push(Chunk {
                        index: chunks.len(),
                        content: current.trim().to_string(),
                        heading: heading.clone(),
                    });
                    // 重置当前累积内容，重新加上标题
                    current = heading.as_deref().map_or_else(String::new, |h| format!("{h}\n"));
                }

                // 如果单个段落就超过限制，需要按行分割
                if para.len() > max_chars {
                    // 先保存当前累积内容（如果有）
                    if !current.trim().is_empty() {
                        chunks.push(Chunk {
                            index: chunks.len(),
                            content: current.trim().to_string(),
                            heading: heading.clone(),
                        });
                        current = heading.as_deref().map_or_else(String::new, |h| format!("{h}\n"));
                    }
                    // 按行分割大段落
                    for line_chunk in split_on_lines(&para, max_chars) {
                        chunks.push(Chunk {
                            index: chunks.len(),
                            content: line_chunk.trim().to_string(),
                            heading: heading.clone(),
                        });
                    }
                } else {
                    // 段落大小合适，添加到当前累积内容
                    current.push_str(&para);
                    current.push('\n');
                }
            }

            // 保存最后剩余的累积内容
            if !current.trim().is_empty() {
                chunks.push(Chunk {
                    index: chunks.len(),
                    content: current.trim().to_string(),
                    heading: heading.clone(),
                });
            }
        }
    }

    // 过滤掉空内容的分块
    chunks.retain(|c| !c.content.is_empty());

    // 重新索引以确保索引连续且从 0 开始
    for (i, chunk) in chunks.iter_mut().enumerate() {
        chunk.index = i;
    }

    chunks
}

/// 按 Markdown 标题分割文本
///
/// 将文本按照一级（#）、二级（##）和三级（###）标题进行分割，
/// 每个节包含其标题和标题之后到下一个标题之前的所有内容。
///
/// # 参数
///
/// - `text`：待分割的 Markdown 文本
///
/// # 返回值
///
/// 返回一个元组向量，每个元组包含：
/// - 第一个元素：标题字符串（可选），如果该节以标题开头则有值
/// - 第二个元素：该节的正文内容（不包含标题本身）
///
/// # 分割规则
///
/// - 标题必须位于行首
/// - 仅识别 `# `、`## `、`### ` 三种标题格式（注意标题符号后必须有空格）
/// - 标题行本身会作为新节的标题，不包含在正文中
///
/// # 示例
///
/// ```ignore
/// let text = "# 标题1\n内容1\n## 标题2\n内容2\n无标题内容";
/// let sections = split_on_headings(text);
///
/// // 结果包含 3 个节：
/// // 1. (Some("# 标题1"), "内容1\n")
/// // 2. (Some("## 标题2"), "内容2\n")
/// // 3. (None, "无标题内容")
/// ```
fn split_on_headings(text: &str) -> Vec<(Option<String>, String)> {
    let mut sections = Vec::new();
    // 当前节的标题（如果有）
    let mut current_heading: Option<String> = None;
    // 当前节的正文内容
    let mut current_body = String::new();

    // 逐行处理文本
    for line in text.lines() {
        // 检查是否是标题行（一级、二级或三级标题）
        if line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### ") {
            // 遇到新标题，先保存当前节（如果有内容）
            if !current_body.trim().is_empty() || current_heading.is_some() {
                sections.push((current_heading.take(), std::mem::take(&mut current_body)));
            }
            // 设置新标题
            current_heading = Some(line.to_string());
        } else {
            // 非标题行，添加到当前正文
            current_body.push_str(line);
            current_body.push('\n');
        }
    }

    // 保存最后一个节
    if !current_body.trim().is_empty() || current_heading.is_some() {
        sections.push((current_heading, current_body));
    }

    sections
}

/// 按空行分割文本为段落
///
/// 将文本按照空行（仅包含空白字符的行）分割成多个段落。
/// 连续的非空行会被归并到同一个段落中。
///
/// # 参数
///
/// - `text`：待分割的文本
///
/// # 返回值
///
/// 返回一个字符串向量，每个字符串代表一个段落（包含段落内的换行符）
///
/// # 分割规则
///
/// - 空行定义为仅包含空白字符的行（使用 `trim().is_empty()` 判断）
/// - 连续的空行会被视为一个分割点
/// - 段落内的换行符会被保留
///
/// # 示例
///
/// ```ignore
/// let text = "第一段第一行\n第一段第二行\n\n第二段内容\n";
/// let paragraphs = split_on_blank_lines(text);
///
/// assert_eq!(paragraphs.len(), 2);
/// assert_eq!(paragraphs[0], "第一段第一行\n第一段第二行\n");
/// assert_eq!(paragraphs[1], "第二段内容\n");
/// ```
fn split_on_blank_lines(text: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    // 当前累积的段落内容
    let mut current = String::new();

    // 逐行处理文本
    for line in text.lines() {
        if line.trim().is_empty() {
            // 遇到空行，保存当前段落（如果有内容）
            if !current.trim().is_empty() {
                paragraphs.push(std::mem::take(&mut current));
            }
        } else {
            // 非空行，添加到当前段落
            current.push_str(line);
            current.push('\n');
        }
    }

    // 保存最后一个段落
    if !current.trim().is_empty() {
        paragraphs.push(current);
    }

    paragraphs
}

/// 按行分割文本以符合字符限制
///
/// 当单个段落过长时，使用该函数按行进行分割，
/// 确保每个分块的字符数不超过指定限制。
///
/// # 参数
///
/// - `text`：待分割的文本
/// - `max_chars`：每个分块的最大字符数
///
/// # 返回值
///
/// 返回一个字符串向量，每个字符串代表一个分块（包含行内换行符）
///
/// # 分割策略
///
/// - 尽可能多地累积行，只要不超过 `max_chars` 限制
/// - 当添加下一行会超过限制时，保存当前累积内容并开始新分块
/// - 行内的换行符会被保留
/// - 预分配容量以提高性能
///
/// # 注意事项
///
/// - 该函数不保证每行都能单独放入限制内，如果单行超过 `max_chars`，
///   该行仍会被作为一个分块返回
/// - `max_chars` 为 0 时会使用 1 作为最小值以避免除零错误
///
/// # 示例
///
/// ```ignore
/// let text = "第一行\n第二行\n第三行\n第四行\n";
/// let chunks = split_on_lines(text, 10);
///
/// // 假设每行长度合适，会尽可能多地累积行
/// for (i, chunk) in chunks.iter().enumerate() {
///     println!("分块 {}: {:?}", i, chunk);
/// }
/// ```
fn split_on_lines(text: &str, max_chars: usize) -> Vec<String> {
    // 预分配向量容量，基于文本长度和最大字符数估算
    let mut chunks = Vec::with_capacity(text.len() / max_chars.max(1) + 1);
    // 当前累积的分块内容
    let mut current = String::new();

    // 逐行处理文本
    for line in text.lines() {
        // 如果添加当前行会超过限制，且当前已有内容，则先保存
        if current.len() + line.len() + 1 > max_chars && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
        }
        // 将当前行添加到累积内容（注意：即使超过限制也会添加，保证不丢失内容）
        current.push_str(line);
        current.push('\n');
    }

    // 保存最后剩余的内容
    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// 单元测试模块
///
/// 测试文件位于 `tests.rs` 中，包含针对分块功能的各类测试用例。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
