use super::TelegramChannel;

#[test]
fn escape_html_covers_quotes_and_angle_brackets() {
    assert_eq!(
        TelegramChannel::escape_html("\"x\" <tag> & 'y'"),
        "&quot;x&quot; &lt;tag&gt; &amp; &#39;y&#39;"
    );
}

#[test]
fn markdown_to_telegram_html_escapes_link_href() {
    assert_eq!(
        TelegramChannel::markdown_to_telegram_html("[x](https://e.test?a=\"b\")"),
        "<a href=\"https://e.test?a=&quot;b&quot;\">x</a>"
    );
}

#[test]
fn markdown_to_telegram_html_formats_inline_markdown() {
    let html = TelegramChannel::markdown_to_telegram_html(
        "# Title\n**bold** __also bold__ *italic* `code<&>` ~~gone~~",
    );

    assert_eq!(
        html,
        "<b>Title</b>\n<b>bold</b> <b>also bold</b> <i>italic</i> <code>code&lt;&amp;&gt;</code> <s>gone</s>"
    );
}

#[test]
fn markdown_to_telegram_html_formats_headers_and_escapes_title() {
    assert_eq!(
        TelegramChannel::markdown_to_telegram_html("## <Title> & 'quoted'"),
        "<b>&lt;Title&gt; &amp; &#39;quoted&#39;</b>"
    );
}

#[test]
fn markdown_to_telegram_html_formats_unicode_inline_segments() {
    let html = TelegramChannel::markdown_to_telegram_html(
        "Hi **粗体<&>** __also__ *斜体* `let x = \"y\"` ~~删除~~",
    );

    assert_eq!(
        html,
        "Hi <b>粗体&lt;&amp;&gt;</b> <b>also</b> <i>斜体</i> <code>let x = &quot;y&quot;</code> <s>删除</s>"
    );
}

#[test]
fn markdown_to_telegram_html_only_links_http_schemes() {
    let html = TelegramChannel::markdown_to_telegram_html(
        "[safe](http://example.test) [also](https://example.test) [raw](mailto:x@y.test)",
    );

    assert_eq!(
        html,
        "<a href=\"http://example.test\">safe</a> <a href=\"https://example.test\">also</a> [raw](mailto:x@y.test)"
    );
}

#[test]
fn markdown_to_telegram_html_keeps_malformed_links_as_text() {
    let html = TelegramChannel::markdown_to_telegram_html(
        "[missing bracket (https://example.test) [no paren] https://example.test [open](https://example.test",
    );

    assert_eq!(
        html,
        "[missing bracket (https://example.test) [no paren] https://example.test [open](https://example.test"
    );
}

#[test]
fn markdown_to_telegram_html_escapes_plain_text_and_unclosed_markup() {
    assert_eq!(
        TelegramChannel::markdown_to_telegram_html("<tag attr='x'>& **open"),
        "&lt;tag attr=&#39;x&#39;&gt;&amp; **open"
    );
}

#[test]
fn markdown_to_telegram_html_keeps_empty_italic_and_unclosed_code_as_text() {
    assert_eq!(
        TelegramChannel::markdown_to_telegram_html("prefix ** *x* `open ~~gone"),
        "prefix ** <i>x</i> `open ~~gone"
    );
}

#[test]
fn markdown_to_telegram_html_converts_fenced_code_blocks() {
    let html = TelegramChannel::markdown_to_telegram_html("before\n```\nif a < b && c\n```\nafter");

    assert_eq!(html, "before\n<pre><code>if a &lt; b &amp;&amp; c</code></pre>\nafter");
}

#[test]
fn markdown_to_telegram_html_closes_trailing_code_block() {
    let html = TelegramChannel::markdown_to_telegram_html("```\nno close <tag>");

    assert_eq!(html, "<pre><code>no close &lt;tag&gt;</code></pre>");
}

#[test]
fn markdown_to_telegram_html_ignores_empty_trailing_code_block() {
    assert_eq!(TelegramChannel::markdown_to_telegram_html("before\n```"), "before");
}

#[test]
fn markdown_to_telegram_html_preserves_closed_empty_code_block() {
    assert_eq!(TelegramChannel::markdown_to_telegram_html("```\n```"), "<pre><code></code></pre>");
}

#[test]
fn markdown_to_telegram_html_treats_unclosed_italic_marker_as_plain_text() {
    assert_eq!(TelegramChannel::markdown_to_telegram_html("**x** *open"), "<b>x</b> *open");
}
// 本文件验证 Markdown 到 Telegram HTML 的转义、格式化与异常输入保留。
