//! Markdown 复制目标与上层指令解析接线。

pub(crate) use tui_markdown::markdown::*;

use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};
use std::ops::Range;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CopyTarget {
    Code { language: Option<String>, content: Arc<str> },
    Quote(Arc<str>),
}

pub(crate) fn extract_copy_targets(markdown_source: &str) -> Vec<CopyTarget> {
    let mut targets = Vec::new();
    let mut quote: Option<(Range<usize>, usize, bool)> = None;
    let mut quote_depth = 0usize;
    let mut in_code_block = false;
    let mut code = None;

    for (event, range) in pulldown_cmark::Parser::new(markdown_source).into_offset_iter() {
        match event {
            Event::Start(Tag::BlockQuote(_)) => {
                if quote_depth == 0 {
                    quote = Some((range, targets.len(), false));
                }
                quote_depth += 1;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                quote_depth = quote_depth.saturating_sub(1);
                if quote_depth == 0 {
                    quote = None;
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                if let CodeBlockKind::Fenced(info) = kind {
                    let language = info
                        .split([',', ' ', '\t'])
                        .next()
                        .filter(|language| !language.is_empty())
                        .map(str::to_owned);
                    code = Some((language, String::new()));
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if in_code_block {
                    if let Some((_, content)) = &mut code {
                        if text.starts_with('\n')
                            && range.start > 0
                            && markdown_source.as_bytes()[range.start - 1] == b'\r'
                        {
                            content.push('\r');
                        }
                        content.push_str(&text);
                    }
                } else if let Some((span, position, emitted)) = &mut quote
                    && !*emitted
                    && !crate::git_action_directives::strip_line_directives(&text)
                        .0
                        .trim()
                        .is_empty()
                {
                    let content = markdown_source[span.clone()]
                        .split_inclusive('\n')
                        .map(|line| {
                            line.trim_start_matches(' ')
                                .strip_prefix('>')
                                .map_or(line, |line| line.strip_prefix(' ').unwrap_or(line))
                        })
                        .collect::<String>();
                    targets.insert(*position, CopyTarget::Quote(content.into()));
                    *emitted = true;
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if let Some((language, content)) = code.take() {
                    targets.push(CopyTarget::Code { language, content: content.into() });
                }
            }
            _ => {}
        }
    }

    targets
}

#[cfg(test)]
#[path = "markdown_copy_tests.rs"]
mod tests;
