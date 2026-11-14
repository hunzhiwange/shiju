use super::*;

#[test]
// 用户和助手事件应成为可按宽度重新渲染的来源单元。
fn v3_events_become_source_backed_transcript_cells() {
    let cells = events_to_transcript_cells(&[
        V3DisplayEvent::User { text: "hello".into(), attachments: Vec::new() },
        V3DisplayEvent::Assistant { markdown: "world".into() },
    ]);
    let rendered = transcript_lines(&cells, 40)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("hello"));
    assert!(rendered.contains("world"));
}

#[test]
// 空事件流必须显示明确占位内容，而非返回空单元列表。
fn empty_transcript_has_explicit_placeholder() {
    let cells = events_to_transcript_cells(&[]);
    assert_eq!(cells.len(), 1);
    assert!(transcript_lines(&cells, 40)[0].to_string().contains("No transcript"));
}
