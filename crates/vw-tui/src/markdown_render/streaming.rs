//! 在写入器的单次解析过程中收集流式 Markdown 渲染元数据。
//!
//! 顶层块偏移始终对应传给本渲染器的精确源码；调用者若先规范化源码，
//! 不得直接把这些偏移应用到原始源码。
//! Streaming markdown render metadata collected during the writer's single parse pass.
//!
//! Top-level block offsets always refer to the exact source passed to this renderer; callers that
//! normalize source before rendering must not apply those offsets to the original source.

use super::DecodedTextMerge;
use super::Event;
use super::HyperlinkLine;
use super::Options;
use super::Parser;
use super::Tag;
use super::Writer;
use super::never_hide_link_destination;
use std::ops::Range;
use std::path::Path;

/// 渲染行以及仅保持最后一个块可变所需的块元数据。
/// Rendered lines and the block metadata needed to keep only the final block mutable.
pub(crate) struct StreamingMarkdownRender {
    /// 与下列元数据由同一次解析生成的带样式输出。
    /// Styled output produced by the same parser pass that collected the metadata below.
    pub(crate) lines: Vec<HyperlinkLine>,
    /// 存在更早块时，最后一个顶层块的字节偏移。
    /// Byte offset of the final top-level block when at least one earlier block exists.
    pub(crate) last_top_level_block_start: Option<usize>,
    /// 引用定义是否可能回溯改变其他块的渲染。
    /// Whether a reference definition can retroactively change another block's rendering.
    pub(crate) has_reference_link_definition: bool,
    /// 首块是否为原始 HTML；此类块与已保留前缀拼接时不插入分隔符。
    /// Whether the first block is raw HTML, which joins a retained prefix without a separator.
    pub(crate) first_top_level_block_is_html: bool,
}

/// 渲染 `input`，同时跟踪最后一个仍可能变化的顶层块。
///
/// 所有字节偏移都索引这里收到的精确 `input`；调用者转换源码后必须先映射回原始位置。
/// Render `input` while tracking the final mutable top-level block.
///
/// Every reported byte offset indexes the exact `input` passed here. Callers that transform source
/// before rendering must map the offset back to their original source before retaining a prefix.
pub(crate) fn render_streaming_markdown_lines_with_width_and_cwd(
    input: &str,
    width: Option<usize>,
    cwd: Option<&Path>,
) -> StreamingMarkdownRender {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(input, options);
    let has_reference_link_definition = parser.reference_definitions().iter().next().is_some();
    let parser = TopLevelBlockTracker {
        iter: DecodedTextMerge::new(parser.into_offset_iter()),
        depth: 0,
        block_count: 0,
        last_start: 0,
        first_is_html: false,
    };
    let mut writer = Writer::new(input, parser, width, cwd, &never_hide_link_destination);
    writer.run();
    StreamingMarkdownRender {
        lines: writer.text,
        last_top_level_block_start: (writer.iter.block_count > 1).then_some(writer.iter.last_start),
        has_reference_link_definition,
        first_top_level_block_is_html: writer.iter.first_is_html,
    }
}

/// 不增加第二次解析遍历，在迭代过程中记录顶层块边界。
/// Records top-level block boundaries without adding a second parser traversal.
struct TopLevelBlockTracker<I> {
    iter: I,
    depth: usize,
    block_count: usize,
    last_start: usize,
    first_is_html: bool,
}

impl<'a, I> Iterator for TopLevelBlockTracker<I>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    type Item = (Event<'a>, Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        let (event, range) = self.iter.next()?;
        if self.depth == 0 && matches!(&event, Event::Start(_) | Event::Rule | Event::Html(_)) {
            self.block_count += 1;
            self.last_start = range.start;
            if self.block_count == 1 {
                self.first_is_html =
                    matches!(&event, Event::Start(Tag::HtmlBlock) | Event::Html(_));
            }
        }
        match event {
            Event::Start(_) => self.depth += 1,
            Event::End(_) => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
        Some((event, range))
    }
}
