//! Regression test: ensure that `StatusIndicatorWidget` sanitises ANSI escape
//! sequences so that no raw `\x1b` bytes are written into the backing
//! buffer.  Rendering logic is tricky to unit‑test end‑to‑end, therefore we
//! verify the *public* contract of `ansi_escape_line()` which the widget now
//! relies on.
//!
//! 回归目标是确保 `StatusIndicatorWidget` 清理 ANSI 转义序列，不把原始 `\x1b` 写入缓冲区。
//! 端到端渲染难以隔离，因此验证控件依赖的 `ansi_escape_line()` 公开契约。

use codex_ansi_escape::ansi_escape_line;

#[test]
// 带颜色的输入应只留下三个可打印字符，控制字节不得进入返回行。
fn ansi_escape_line_strips_escape_sequences() {
    let text_in_ansi_red = "\x1b[31mRED\x1b[0m";

    // The returned line must contain three printable glyphs and **no** raw
    // escape bytes.
    // 返回行必须包含三个可打印字形，并且不含任何原始转义字节。
    let line = ansi_escape_line(text_in_ansi_red);

    let combined: String = line
        .spans
        .iter()
        .map(|span| span.content.to_string())
        .collect();

    assert_eq!(combined, "RED");
}
