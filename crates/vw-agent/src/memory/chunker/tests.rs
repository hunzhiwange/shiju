//! Markdown 分块器单元测试模块
//!
//! 本模块提供 `chunk_markdown` 函数的全面测试覆盖，验证以下方面：
//! - 空输入和边界条件处理
//! - 标题层级识别与内容关联
//! - 最大 token 限制遵守
//! - 分块索引顺序性
//! - Unicode 和特殊字符支持
//! - 内容完整性（无丢失）

use super::*;

/// 测试空文本和纯空白文本的处理
///
/// 验证：
/// - 空字符串应返回空分块列表
/// - 仅包含空白字符的字符串也应返回空分块列表
#[test]
fn empty_text() {
    assert!(chunk_markdown("", 512).is_empty());
    assert!(chunk_markdown("   ", 512).is_empty());
}

/// 测试单个短段落的处理
///
/// 验证：
/// - 短于最大 token 限制的文本应合并为单个分块
/// - 分块内容应与原始文本一致
/// - 无标题时 heading 字段应为 None
#[test]
fn single_short_paragraph() {
    let chunks = chunk_markdown("Hello world", 512);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "Hello world");
    assert!(chunks[0].heading.is_none());
}

/// 测试标题分节的正确处理
///
/// 验证：
/// - 一级标题（#）应被识别
/// - 二级标题（##）应创建新的分节
/// - 每个分节的内容应正确关联到对应标题
#[test]
fn heading_sections() {
    let text = "# Title\nSome intro.\n\n## Section A\nContent A.\n\n## Section B\nContent B.";
    let chunks = chunk_markdown(text, 512);
    assert!(chunks.len() >= 3);
    assert!(chunks[0].heading.is_none() || chunks[0].heading.as_deref() == Some("# Title"));
}

/// 测试最大 token 限制的遵守
///
/// 构造一个长文本，验证：
/// - 当文本超过限制时，应被分割为多个分块
/// - 每个分块的字符长度应在合理范围内（不超过约 300 字符）
#[test]
fn respects_max_tokens() {
    // 构造包含 200 个句子的长文本
    let long_text: String = (0..200).fold(String::new(), |mut s, i| {
        use std::fmt::Write;
        let _ = writeln!(s, "This is sentence number {i} with some extra words to fill it up.");
        s
    });
    let chunks = chunk_markdown(&long_text, 50);
    assert!(chunks.len() > 1, "Expected multiple chunks, got {}", chunks.len());
    // 验证每个分块不超过预期长度
    for chunk in &chunks {
        assert!(chunk.content.len() <= 300, "Chunk too long: {} chars", chunk.content.len());
    }
}

/// 测试分割后的分块保留标题信息
///
/// 当一个大分节被分割为多个分块时，验证：
/// - 子分块应继承原分节的标题
/// - 所有属于同一分节的分块应具有相同的 heading 值
#[test]
fn preserves_heading_in_split_sections() {
    // 构造一个带有长内容的大分节
    let mut text = String::from("## Big Section\n");
    for i in 0..100 {
        use std::fmt::Write;
        let _ = write!(text, "Line {i} with some content here.\n\n");
    }
    let chunks = chunk_markdown(&text, 50);
    // 应被分割为多个分块
    assert!(chunks.len() > 1);
    // 验证每个带有标题的分块标题一致
    for chunk in &chunks {
        if chunk.heading.is_some() {
            assert_eq!(chunk.heading.as_deref(), Some("## Big Section"));
        }
    }
}

/// 测试分块索引的顺序性
///
/// 验证：
/// - 每个分块的 index 字段应与其在列表中的位置一致
/// - 索引从 0 开始，连续递增
#[test]
fn indexes_are_sequential() {
    let text = "# A\nContent A\n\n# B\nContent B\n\n# C\nContent C";
    let chunks = chunk_markdown(text, 512);
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.index, i);
    }
}

/// 测试短文本的分块数量合理性
///
/// 验证：
/// - 明显小于限制的文本应只产生一个分块
#[test]
fn chunk_count_reasonable() {
    let text = "Hello world. This is a test document.";
    let chunks = chunk_markdown(text, 512);
    assert_eq!(chunks.len(), 1);
}

/// 测试仅包含标题的文档
///
/// 验证：
/// - 没有正文内容、只有标题的文档仍应产生分块
/// - 不应因缺少正文而崩溃
#[test]
fn headings_only_no_body() {
    let text = "# Title\n## Section A\n## Section B\n### Subsection";
    let chunks = chunk_markdown(text, 512);
    assert!(!chunks.is_empty());
}

/// 测试深层嵌套标题的处理
///
/// 验证：
/// - 四级标题（####）及更深层次应被正确处理
/// - 深层标题及其内容不应丢失
#[test]
fn deeply_nested_headings_ignored() {
    let text = "# Top\nIntro\n#### Deep heading\nDeep content";
    let chunks = chunk_markdown(text, 512);
    assert!(!chunks.is_empty());
    // 验证所有内容被正确保留
    let all_content: String = chunks.iter().map(|c| c.content.clone()).collect();
    assert!(all_content.contains("Deep heading"));
    assert!(all_content.contains("Deep content"));
}

/// 测试极长单行（无换行符）文本的处理
///
/// 验证：
/// - 没有换行符的长文本应能被正确分块
/// - 不应因缺少自然分割点而失败
#[test]
fn very_long_single_line_no_newlines() {
    let text = "word ".repeat(5000);
    let chunks = chunk_markdown(&text, 50);
    assert!(!chunks.is_empty());
}

/// 测试仅包含换行符和空白字符的文本
///
/// 验证：
/// - 仅由换行和空白组成的文本应返回空分块列表
#[test]
fn only_newlines_and_whitespace() {
    assert!(chunk_markdown("\n\n\n   \n\n", 512).is_empty());
}

/// 测试 max_tokens 参数为零的边界情况
///
/// 验证：
/// - 零值限制应被合理处理（通常使用默认值）
/// - 不应崩溃，应返回至少一个分块
#[test]
fn max_tokens_zero() {
    let chunks = chunk_markdown("Hello world", 0);
    assert!(!chunks.is_empty());
}

/// 测试 max_tokens 参数为一的极端情况
///
/// 验证：
/// - 极小的限制值下仍应能处理多行文本
/// - 每行可能被分割为独立分块
#[test]
fn max_tokens_one() {
    let text = "Line one\nLine two\nLine three";
    let chunks = chunk_markdown(text, 1);
    assert!(!chunks.is_empty());
}

/// 测试 Unicode 内容的正确处理
///
/// 验证：
/// - 日语、中文等多字节字符应被正确处理
/// - Emoji 表情符号应被保留
/// - 不应出现乱码或字符丢失
#[test]
fn unicode_content() {
    let text = "# 日本語\nこんにちは世界\n\n## Émojis\n🦀 Rust is great 🚀";
    let chunks = chunk_markdown(text, 512);
    assert!(!chunks.is_empty());
    // 验证所有内容被正确保留
    let all: String = chunks.iter().map(|c| c.content.clone()).collect();
    assert!(all.contains("こんにちは"));
    assert!(all.contains("🦀"));
}

/// 测试 FTS5 特殊字符在内容中的处理
///
/// 验证：
/// - 引号、括号、星号等 FTS5 全文搜索特殊字符应被正确保留
/// - 不应对这些字符进行转义或修改
#[test]
fn fts5_special_chars_in_content() {
    let text = "Content with \"quotes\" and (parentheses) and * asterisks *";
    let chunks = chunk_markdown(text, 512);
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].content.contains("\"quotes\""));
}

/// 测试段落间多个空行的处理
///
/// 验证：
/// - 多个连续空行不应产生额外分块
/// - 段落内容应被正确合并到同一分块中
#[test]
fn multiple_blank_lines_between_paragraphs() {
    let text = "Paragraph one.\n\n\n\n\nParagraph two.\n\n\n\nParagraph three.";
    let chunks = chunk_markdown(text, 512);
    // 短文本应合并为单个分块
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].content.contains("Paragraph one"));
    assert!(chunks[0].content.contains("Paragraph three"));
}

/// 测试文本末尾标题的处理
///
/// 验证：
/// - 出现在文本末尾的标题应被正确处理
/// - 不应因标题后无内容而出现问题
#[test]
fn heading_at_end_of_text() {
    let text = "Some content\n# Trailing Heading";
    let chunks = chunk_markdown(text, 512);
    assert!(!chunks.is_empty());
}

/// 测试仅包含单个标题无内容的情况
///
/// 验证：
/// - 仅有一个标题、没有后续内容的文本应产生一个分块
/// - 该分块的 heading 字段应正确记录标题
#[test]
fn single_heading_no_content() {
    let text = "# Just a heading";
    let chunks = chunk_markdown(text, 512);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].heading.as_deref(), Some("# Just a heading"));
}

/// 测试分块后内容的完整性（无丢失）
///
/// 验证：
/// - 将所有分块内容重新组装后，所有原始词汇都应存在
/// - 分块过程不应导致任何内容丢失
#[test]
fn no_content_loss() {
    let text = "# A\nContent A line 1\nContent A line 2\n\n## B\nContent B\n\n## C\nContent C";
    let chunks = chunk_markdown(text, 512);
    // 将所有分块内容重新组装
    let reassembled: String = chunks.iter().fold(String::new(), |mut s, c| {
        use std::fmt::Write;
        let _ = writeln!(s, "{}", c.content);
        s
    });
    // 验证关键词汇未丢失
    for word in ["Content", "line", "1", "2"] {
        assert!(reassembled.contains(word), "Missing word '{word}' in reassembled chunks");
    }
}

#[test]
fn split_on_headings_keeps_preamble_and_recognized_heading_levels() {
    let sections = split_on_headings("Preamble\n# One\nA\n## Two\nB\n### Three\nC\n#### Four\nD");

    assert_eq!(sections.len(), 4);
    assert_eq!(sections[0], (None, "Preamble\n".to_string()));
    assert_eq!(sections[1], (Some("# One".to_string()), "A\n".to_string()));
    assert_eq!(sections[2], (Some("## Two".to_string()), "B\n".to_string()));
    assert_eq!(sections[3], (Some("### Three".to_string()), "C\n#### Four\nD\n".to_string()));
}

#[test]
fn split_on_headings_preserves_empty_heading_sections() {
    let sections = split_on_headings("# A\n## B\nBody");

    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0], (Some("# A".to_string()), String::new()));
    assert_eq!(sections[1], (Some("## B".to_string()), "Body\n".to_string()));
}

#[test]
fn split_on_blank_lines_collapses_whitespace_separators() {
    let paragraphs = split_on_blank_lines("one\n two\n\n   \nthree\n\nfour");

    assert_eq!(
        paragraphs,
        vec!["one\n two\n".to_string(), "three\n".to_string(), "four\n".to_string()]
    );
}

#[test]
fn split_on_blank_lines_ignores_empty_input() {
    assert!(split_on_blank_lines("").is_empty());
    assert!(split_on_blank_lines("\n \n\t\n").is_empty());
}

#[test]
fn split_on_lines_batches_until_limit_and_keeps_oversized_lines() {
    let chunks = split_on_lines("aa\nbb\nlonggggggg\nc\n", 8);

    assert_eq!(chunks, vec!["aa\nbb\n".to_string(), "longgggggg\n".to_string(), "c\n".to_string()]);
}

#[test]
fn split_on_lines_zero_limit_still_returns_lines() {
    assert_eq!(
        split_on_lines("first\nsecond", 0),
        vec!["first\n".to_string(), "second\n".to_string()]
    );
}

#[test]
fn chunk_markdown_reindexes_after_filtering_empty_line_chunks() {
    let chunks = chunk_markdown("# Title\n\n\nbody\n\n\n", 1);

    assert!(!chunks.is_empty());
    for (index, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.index, index);
        assert!(!chunk.content.trim().is_empty());
    }
}

#[test]
fn chunk_markdown_splits_oversized_heading_section_before_line_chunks() {
    let chunks = chunk_markdown("## Oversized\nalpha\nbeta\ngamma\n", 2);

    assert!(chunks.len() > 2);
    assert_eq!(chunks[0].content, "## Oversized");
    assert_eq!(chunks[1].content, "## Oversized");
    assert!(chunks.iter().any(|chunk| chunk.content == "alpha"));
    assert!(chunks.iter().any(|chunk| chunk.content == "beta"));
    assert!(chunks.iter().any(|chunk| chunk.content == "gamma"));
    for (index, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.index, index);
        assert_eq!(chunk.heading.as_deref(), Some("## Oversized"));
    }
}
