//! 验证 WebDriver 端点探测、选择器转义及按键名称映射。
//! 选择器转义覆盖引号、反斜杠和换行，XPath 混合引号必须构造安全字面量。

use super::*;
use fantoccini::key::Key as WebDriverKey;
use std::time::Duration;

#[test]
fn webdriver_endpoint_reachable_rejects_invalid_and_unsupported_urls_without_connecting() {
    assert!(!webdriver_endpoint_reachable("not a URL", Duration::ZERO));
    assert!(!webdriver_endpoint_reachable("ftp://127.0.0.1:4444", Duration::ZERO));
}

#[test]
fn webdriver_endpoint_reachable_accepts_a_listening_loopback_endpoint() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("loopback listener should bind for the reachability probe");
    let address = listener.local_addr().expect("listener should expose its bound address");

    assert!(webdriver_endpoint_reachable(
        &format!("http://{address}/wd/hub"),
        Duration::from_secs(1),
    ));
}

#[test]
fn selector_for_find_maps_each_strategy_to_its_selector_syntax() {
    assert_eq!(selector_for_find("role", "button"), r#"[role=\"button\"]"#);
    assert_eq!(selector_for_find("label", "Email"), "label=Email");
    assert_eq!(selector_for_find("placeholder", "Search"), r#"[placeholder=\"Search\"]"#);
    assert_eq!(selector_for_find("testid", "submit"), r#"[data-testid=\"submit\"]"#);
    assert_eq!(selector_for_find("unknown", "Continue"), "text=Continue");
}

#[test]
fn selector_for_find_escapes_attribute_values_and_preserves_text_values() {
    let value = "a\\b\"c\nd";

    assert_eq!(selector_for_find("role", value), r#"[role=\"a\\b\"c d\"]"#);
    assert_eq!(selector_for_find("label", value), "label=a\\b\"c\nd");
    assert_eq!(selector_for_find("other", value), "text=a\\b\"c\nd");
}

#[test]
fn parse_selector_builds_text_and_label_xpaths() {
    match parse_selector(" text=Hello ") {
        SelectorKind::XPath(value) => {
            assert_eq!(value, r#"//*[contains(normalize-space(.), "Hello")]"#);
        }
        SelectorKind::Css(_) => panic!("text selector should become XPath"),
    }

    match parse_selector("label=Email") {
        SelectorKind::XPath(value) => assert_eq!(
            value,
            r#"(//label[contains(normalize-space(.), "Email")]/following::*[self::input or self::textarea or self::select][1] | //*[@aria-label and contains(normalize-space(@aria-label), "Email")] | //label[contains(normalize-space(.), "Email")])"#
        ),
        SelectorKind::Css(_) => panic!("label selector should become XPath"),
    }
}

#[test]
fn parse_selector_builds_reference_and_trimmed_css_selectors() {
    match parse_selector("  @node  ") {
        SelectorKind::Css(value) => assert_eq!(value, r#"[data-zc-ref=\"@node\"]"#),
        SelectorKind::XPath(_) => panic!("reference selector should become CSS"),
    }

    match parse_selector("  #login-form  ") {
        SelectorKind::Css(value) => assert_eq!(value, "#login-form"),
        SelectorKind::XPath(_) => panic!("plain selector should remain CSS"),
    }
}

#[test]
fn parse_selector_handles_empty_selector_values() {
    match parse_selector("") {
        SelectorKind::Css(value) => assert!(value.is_empty()),
        SelectorKind::XPath(_) => panic!("empty selector should remain CSS"),
    }

    match parse_selector("text=") {
        SelectorKind::XPath(value) => {
            assert_eq!(value, r#"//*[contains(normalize-space(.), "")]"#);
        }
        SelectorKind::Css(_) => panic!("text prefix should become XPath"),
    }

    match parse_selector("@") {
        SelectorKind::Css(value) => assert_eq!(value, r#"[data-zc-ref=\"@\"]"#),
        SelectorKind::XPath(_) => panic!("reference prefix should become CSS"),
    }
}

#[test]
fn css_attr_escape_escapes_backslashes_quotes_and_line_feeds() {
    assert_eq!(css_attr_escape(""), "");
    assert_eq!(css_attr_escape("a\\b"), "a\\\\b");
    assert_eq!(css_attr_escape("a\"b"), "a\\\"b");
    assert_eq!(css_attr_escape("a\nb"), "a b");
}

#[test]
fn xpath_contains_text_uses_a_safe_literal_for_mixed_quotes() {
    assert_eq!(
        xpath_contains_text("a\"b'c"),
        r#"//*[contains(normalize-space(.), concat("a",'"',"b'c"))]"#
    );
}

#[test]
// XPath 字面量覆盖单双引号混用，避免文本选择器通过拼接改变表达式结构。
fn xpath_literal_handles_empty_single_quote_double_quote_and_mixed_inputs() {
    assert_eq!(xpath_literal(""), r#""""#);
    assert_eq!(xpath_literal("it's ready"), r#""it's ready""#);
    assert_eq!(xpath_literal("say \"hi\""), r#"'say "hi"'"#);
    assert_eq!(xpath_literal("a\"b'c"), r#"concat("a",'"',"b'c")"#);
    assert_eq!(xpath_literal("a\"\"'b"), r#"concat("a",'"','"',"'b")"#);
}

#[test]
fn webdriver_key_maps_supported_names_and_aliases() {
    let cases = [
        (" ENTER ", WebDriverKey::Enter.to_string()),
        ("return", WebDriverKey::Return.to_string()),
        ("tab", WebDriverKey::Tab.to_string()),
        ("escape", WebDriverKey::Escape.to_string()),
        ("esc", WebDriverKey::Escape.to_string()),
        ("backspace", WebDriverKey::Backspace.to_string()),
        ("delete", WebDriverKey::Delete.to_string()),
        ("space", WebDriverKey::Space.to_string()),
        ("arrowup", WebDriverKey::Up.to_string()),
        ("up", WebDriverKey::Up.to_string()),
        ("arrowdown", WebDriverKey::Down.to_string()),
        ("down", WebDriverKey::Down.to_string()),
        ("arrowleft", WebDriverKey::Left.to_string()),
        ("left", WebDriverKey::Left.to_string()),
        ("arrowright", WebDriverKey::Right.to_string()),
        ("right", WebDriverKey::Right.to_string()),
        ("home", WebDriverKey::Home.to_string()),
        ("end", WebDriverKey::End.to_string()),
        ("pageup", WebDriverKey::PageUp.to_string()),
        ("pagedown", WebDriverKey::PageDown.to_string()),
    ];

    for (input, expected) in cases {
        assert_eq!(webdriver_key(input), expected, "unexpected mapping for {input}");
    }
}

#[test]
fn webdriver_key_normalizes_unknown_names() {
    assert_eq!(webdriver_key("  Custom-Key  "), "custom-key");
    assert_eq!(webdriver_key(""), "");
}
