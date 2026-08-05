//! 消息内容块解析器。
//!
//! 该模块负责把原始消息字符串切分为思考块、工具块与普通文本块，
//! 并提供与渲染缓存配合使用的块结构转换能力。

use crate::app::models::ParsedChatBlock;

/// 渲染块类型枚举
///
/// 用于在解析消息内容时标识不同类型的区块，支持：
/// - 思考块（Think）：AI 的推理过程，可展开/折叠
/// - 工具块（Tool）：AI 调用的工具操作记录
/// - 文本块（Text）：普通文本内容
pub(super) enum RenderBlock<'a> {
    /// 思考块
    /// - `content`: 思考内容文本
    /// - `open`: 是否为未闭合的思考块（流式输出时可能未闭合）
    Think { content: &'a str, open: bool },

    /// 工具调用块
    /// - `raw`: 原始工具调用文本（以 "tool " 开头）
    Tool { raw: &'a str },

    /// 普通文本块
    /// - `content`: 文本内容
    Text { content: &'a str },
}

#[derive(Clone, Copy)]
enum SpecialBlockKind {
    ThinkTag,
    ReasoningMarker,
    Tool,
}

pub(crate) fn hash_chat_content(raw: &str) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    raw.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn owned_blocks_from_raw(raw: &str) -> Vec<ParsedChatBlock> {
    parse_blocks(raw)
        .into_iter()
        .filter_map(|block| match block {
            RenderBlock::Think { content, open } => {
                Some(ParsedChatBlock::Think { content: content.to_string(), open })
            }
            RenderBlock::Tool { raw } => Some(ParsedChatBlock::Tool { raw: raw.to_string() }),
            RenderBlock::Text { content } => {
                let content = strip_orphan_think_close_tags(content);
                (!content.is_empty()).then_some(ParsedChatBlock::Text { content })
            }
        })
        .collect()
}

pub(super) fn borrowed_blocks(
    blocks: &[ParsedChatBlock],
) -> impl Iterator<Item = RenderBlock<'_>> + '_ {
    blocks.iter().map(|block| match block {
        ParsedChatBlock::Think { content, open } => {
            RenderBlock::Think { content: content.as_str(), open: *open }
        }
        ParsedChatBlock::Tool { raw } => RenderBlock::Tool { raw: raw.as_str() },
        ParsedChatBlock::Text { content } => RenderBlock::Text { content: content.as_str() },
    })
}

/// 查找思考块的开始标签
///
/// 在字符串中搜索 `<think ...>` 形式的开始标签。
/// 支持带属性的标签（如 `<think time="123">`）。
///
/// # 参数
/// - `s`: 要搜索的字符串
///
/// # 返回值
/// - `Some((pos, len))`: 找到标签时返回（标签起始位置，标签长度）
/// - `None`: 未找到有效标签
///
/// # 示例
/// ```ignore
/// let s = "一些文本<think time=\"1s\">思考内容";
/// assert_eq!(find_think_open_tag(s), Some((12, 16)));
/// ```
fn find_think_open_tag(s: &str) -> Option<(usize, usize)> {
    let mut search_from = 0usize;
    while let Some(pos_rel) = s[search_from..].find("<think") {
        let pos = search_from + pos_rel;

        if s[pos..].starts_with("</think") {
            search_from = pos + 1;
            continue;
        }

        let Some(tag_end_rel) = s[pos..].find('>') else {
            return None;
        };
        let tag_end = pos + tag_end_rel;

        let tag = &s[pos + "<think".len()..tag_end];
        if tag.is_empty() || tag.chars().next().is_some_and(char::is_whitespace) {
            return Some((pos, tag_end_rel + 1));
        }
        search_from = pos + 1;
    }
    None
}

/// 查找思考块的结束标签
///
/// 在字符串中搜索 `</think ...>` 形式的闭合标签。
///
/// # 参数
/// - `s`: 要搜索的字符串
///
/// # 返回值
/// - `Some((pos, len))`: 找到标签时返回（标签起始位置，标签长度）
/// - `None`: 未找到有效标签
fn find_think_close_tag(s: &str) -> Option<(usize, usize)> {
    let mut search_from = 0usize;
    while let Some(pos_rel) = s[search_from..].find("</think") {
        let pos = search_from + pos_rel;

        let Some(tag_end_rel) = s[pos..].find('>') else {
            return None;
        };
        let tag_end = pos + tag_end_rel;

        let tag = &s[pos + "</think".len()..tag_end];
        if tag.is_empty() || tag.chars().next().is_some_and(char::is_whitespace) {
            return Some((pos, tag_end_rel + 1));
        }
        search_from = pos + 1;
    }
    None
}

fn strip_orphan_think_close_tags(mut s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    while let Some((pos, tag_len)) = find_think_close_tag(s) {
        out.push_str(&s[..pos]);
        s = &s[pos + tag_len..];
    }
    out.push_str(s);
    out
}

/// 查找位于行首的 `[reasoning]` 推理块标记。
fn find_reasoning_marker(s: &str) -> Option<(usize, usize)> {
    for (pos, _) in s.match_indices("[reasoning]") {
        if pos > 0 && !s[..pos].ends_with('\n') {
            continue;
        }

        let suffix = &s[pos + "[reasoning]".len()..];
        let marker_len = if suffix.starts_with("\r\n") {
            "[reasoning]".len() + 2
        } else if suffix.starts_with('\n') {
            "[reasoning]".len() + 1
        } else if suffix.is_empty() {
            "[reasoning]".len()
        } else {
            continue;
        };
        return Some((pos, marker_len));
    }
    None
}

/// 查找工具调用块的起始位置
///
/// 在字符串中搜索 "tool " 关键字的位置。
/// 仅识别位于行首的工具块，避免把读取文件内容中的普通文本误判为工具卡。
///
/// # 参数
/// - `s`: 要搜索的字符串
///
/// # 返回值
/// - `Some(pos)`: 找到工具调用块的起始位置
/// - `None`: 未找到有效的工具调用块起始
fn find_tool_start(s: &str) -> Option<usize> {
    for (idx, _) in s.match_indices("tool ") {
        let boundary_ok = if idx == 0 {
            true
        } else if s[..idx].ends_with('\n') {
            true
        } else {
            let prefix = s[..idx].trim_end_matches([' ', '\t']);
            prefix.ends_with(':') || prefix.ends_with('：')
        };

        if boundary_ok && parse_tool_block(&s[idx..]).is_some() {
            return Some(idx);
        }
    }
    None
}

/// 解析工具调用块
///
/// 从以 "tool " 开头的字符串中解析完整的工具调用块。
/// 工具调用块由 "tool " 前缀加上 JSON 对象组成。
///
/// # 参数
/// - `s`: 以 "tool " 开头的字符串
///
/// # 返回值
/// - `Some(len)`: 成功解析时返回工具调用块的长度
/// - `None`: 无法解析有效的工具调用块
///
/// # 解析策略
/// 逐行读取并累积，尝试解析为 JSON，直到成功或超过最大行数限制（64行）
fn parse_tool_block(s: &str) -> Option<usize> {
    if !s.starts_with("tool ") {
        return None;
    }

    let Some(line_end) = s.find('\n') else {
        return None;
    };

    let mut idx = line_end + 1;
    let mut buf = String::new();

    for _ in 0..64 {
        if idx >= s.len() {
            break;
        }

        let next_end = s[idx..].find('\n').map(|offset| idx + offset).unwrap_or(s.len());
        let line = &s[idx..next_end];

        if !buf.is_empty() {
            buf.push('\n');
        }
        buf.push_str(line);

        if serde_json::from_str::<serde_json::Value>(buf.trim()).is_ok() {
            return Some(if next_end < s.len() { next_end + 1 } else { next_end });
        }

        if next_end >= s.len() {
            break;
        }
        idx = next_end + 1;
    }
    None
}

/// 将工具块和文本块分割并添加到输出列表
///
/// 递归地从字符串中提取工具块和文本块，并添加到输出向量中。
///
/// # 参数
/// - `out`: 输出的渲染块向量
/// - `s`: 待解析的字符串切片
fn push_tool_and_text_blocks<'a>(out: &mut Vec<RenderBlock<'a>>, mut s: &'a str) {
    loop {
        let Some(pos) = find_tool_start(s) else {
            if !s.is_empty() {
                out.push(RenderBlock::Text { content: s });
            }
            break;
        };

        if pos > 0 {
            out.push(RenderBlock::Text { content: &s[..pos] });
        }

        let rest = &s[pos..];
        let Some(consumed) = parse_tool_block(rest) else {
            out.push(RenderBlock::Text { content: rest });
            break;
        };

        out.push(RenderBlock::Tool { raw: &rest[..consumed] });
        s = &rest[consumed..];
    }
}

/// 解析消息内容为渲染块列表
///
/// 将原始消息内容解析为一系列渲染块，包括思考块、工具块和文本块。
/// 解析器会正确处理嵌套和混合的情况。
///
/// # 参数
/// - `raw`: 原始消息内容字符串
///
/// # 返回值
/// 返回解析后的渲染块向量
///
/// # 解析规则
/// 1. 优先识别 `<think ...>...</think` 标签对
/// 2. 在思考块外识别 `tool ...` 工具调用块
/// 3. 其余内容作为普通文本块
fn parse_blocks(raw: &str) -> Vec<RenderBlock<'_>> {
    let mut out: Vec<RenderBlock<'_>> = Vec::new();
    let mut rest = raw;

    loop {
        let think_pos = find_think_open_tag(rest).map(|(pos, _)| pos);
        let reasoning_pos = find_reasoning_marker(rest).map(|(pos, _)| pos);
        let tool_pos = find_tool_start(rest);

        let next_pos = [
            think_pos.map(|pos| (pos, SpecialBlockKind::ThinkTag)),
            reasoning_pos.map(|pos| (pos, SpecialBlockKind::ReasoningMarker)),
            tool_pos.map(|pos| (pos, SpecialBlockKind::Tool)),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|(pos, _)| *pos);

        let Some((pos, kind)) = next_pos else {
            if !rest.is_empty() {
                out.push(RenderBlock::Text { content: rest });
            }
            break;
        };

        if pos > 0 {
            out.push(RenderBlock::Text { content: &rest[..pos] });
        }
        rest = &rest[pos..];

        match kind {
            SpecialBlockKind::Tool => {
                let Some(consumed) = parse_tool_block(rest) else {
                    out.push(RenderBlock::Text { content: rest });
                    break;
                };
                out.push(RenderBlock::Tool { raw: &rest[..consumed] });
                rest = &rest[consumed..];
                continue;
            }
            SpecialBlockKind::ReasoningMarker => {
                let Some((_, marker_len)) = find_reasoning_marker(rest) else {
                    out.push(RenderBlock::Text { content: rest });
                    break;
                };
                rest = &rest[marker_len..];
                if let Some(tool_start) = find_tool_start(rest) {
                    out.push(RenderBlock::Think {
                        content: &rest[..tool_start],
                        open: false,
                    });
                    push_tool_and_text_blocks(&mut out, &rest[tool_start..]);
                } else {
                    out.push(RenderBlock::Think { content: rest, open: false });
                }
                break;
            }
            SpecialBlockKind::ThinkTag => {}
        }

        let Some((_, open_tag_len)) = find_think_open_tag(rest) else {
            out.push(RenderBlock::Text { content: rest });
            break;
        };
        rest = &rest[open_tag_len..];

        let Some((end, close_tag_len)) = find_think_close_tag(rest) else {
            if let Some(tool_start) = find_tool_start(rest) {
                if tool_start > 0 {
                    out.push(RenderBlock::Think { content: &rest[..tool_start], open: true });
                } else {
                    out.push(RenderBlock::Think { content: "", open: true });
                }
                push_tool_and_text_blocks(&mut out, &rest[tool_start..]);
            } else {
                out.push(RenderBlock::Think { content: rest, open: true });
            }
            break;
        };

        let think_chunk = &rest[..end];
        if let Some(tool_start) = find_tool_start(think_chunk) {
            if tool_start > 0 {
                out.push(RenderBlock::Think { content: &think_chunk[..tool_start], open: false });
            } else {
                out.push(RenderBlock::Think { content: "", open: false });
            }
            push_tool_and_text_blocks(&mut out, &think_chunk[tool_start..]);
        } else {
            out.push(RenderBlock::Think { content: think_chunk, open: false });
        }

        rest = &rest[end + close_tag_len..];
    }

    out
}
