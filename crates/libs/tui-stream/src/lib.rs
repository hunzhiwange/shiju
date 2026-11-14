//! Markdown 增量收集、流式渲染与提交节拍。

pub mod markdown_stream;
pub mod streaming;

pub use markdown_stream::MarkdownStreamCollector;
pub use streaming::{StreamRenderMode, StreamState, StreamingCell};

#[cfg(test)]
mod test_helpers;
