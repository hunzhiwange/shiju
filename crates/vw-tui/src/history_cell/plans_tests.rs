//! 验证计划历史单元在测量与转录渲染之间共享最终态缓存。
//! 哨兵行能够证明复用发生，而不把断言绑定到具体 Markdown 样式。

use super::*;
use pretty_assertions::assert_eq;

#[test]
// 高度计算完成后写入的缓存内容应原样服务于同宽度的转录请求。
fn finalized_plan_reuses_lines_primed_by_transcript_height() {
    let cell = new_proposed_plan("1. Inspect **markdown**".to_string(), Path::new("/tmp"));
    let width = 48;

    assert_eq!(cell.desired_transcript_height(width), 5);
    cell.rendered_lines
        .cached
        .lock()
        .expect("render cache lock")
        .as_mut()
        .expect("render cache should be populated")
        .1 = vec![HyperlinkLine::from("cached")];

    assert_eq!(
        visible_lines(cell.transcript_hyperlink_lines(width)),
        vec![Line::from("cached")]
    );
}
