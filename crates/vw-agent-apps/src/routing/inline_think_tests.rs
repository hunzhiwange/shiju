use super::inline_think::InlineThinkSplitter;

fn collect(parts: &[&str]) -> (String, String) {
    let mut splitter = InlineThinkSplitter::default();
    let (mut thinking, mut text) = (String::new(), String::new());
    for (t, x) in parts.iter().map(|part| splitter.push(part)).collect::<Vec<_>>() {
        thinking.push_str(&t.unwrap_or_default());
        text.push_str(&x.unwrap_or_default());
    }
    let (t, x) = splitter.flush();
    thinking.push_str(&t.unwrap_or_default());
    text.push_str(&x.unwrap_or_default());
    assert_eq!(splitter.flush(), (None, None));
    (thinking, text)
}

#[test]
fn cc_route_061_inline_think_expected_behavior() {
    for source in [
        "  <think>\r\n思考\n</think>\n正文",
        "<thinking>思考</thinking>正文",
        "<think>思考</thinking>正文",
    ] {
        for split in (0..=source.len()).filter(|i| source.is_char_boundary(*i)) {
            assert_eq!(
                collect(&[&source[..split], &source[split..]]),
                ("思考".into(), "正文".into())
            );
        }
    }
}

#[test]
fn cc_route_061_flush_and_literal_text() {
    for source in ["<thi", "正文<think>literal</think>", "\nnormal"] {
        assert_eq!(collect(&[source]), (String::new(), source.into()));
    }
    assert_eq!(collect(&["<think>partial</th"]), ("partial</th".into(), String::new()));
    let mut splitter = InlineThinkSplitter::default();
    assert_eq!(splitter.push("<think>ongoing"), (Some("ongoing".into()), None));
}
