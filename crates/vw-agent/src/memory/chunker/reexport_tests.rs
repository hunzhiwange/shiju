use super::{Chunk, chunk_markdown};

#[test]
fn chunker_reexport_preserves_chunk_identity_and_unicode() {
    let chunks: Vec<memory_algorithms::chunker::Chunk> =
        chunk_markdown("# 标题\n中文内容 🦀", 512);
    assert_eq!(chunks.len(), 1);
    let chunk: &Chunk = &chunks[0];
    assert_eq!(chunk.index, 0);
    assert_eq!(chunk.heading.as_deref(), Some("# 标题"));
    assert!(chunk.content.contains("中文内容 🦀"));
}
