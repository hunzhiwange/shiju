use super::*;

#[test]
fn map_locale_tag_handles_supported_locale_variants() {
    assert_eq!(map_locale_tag("zh-CN"), Some(LarkAckLocale::ZhCn));
    assert_eq!(map_locale_tag("zh-Hant"), Some(LarkAckLocale::ZhTw));
    assert_eq!(map_locale_tag("en-US"), Some(LarkAckLocale::En));
    assert_eq!(map_locale_tag("ja_JP"), Some(LarkAckLocale::Ja));
    assert_eq!(map_locale_tag(""), None);
}

#[test]
fn map_locale_tag_rejects_unsupported_language() {
    assert_eq!(map_locale_tag(" fr-FR "), None);
}

#[test]
fn lark_ack_pool_is_non_empty_for_every_locale() {
    for locale in [LarkAckLocale::ZhCn, LarkAckLocale::ZhTw, LarkAckLocale::En, LarkAckLocale::Ja] {
        assert!(!lark_ack_pool(locale).is_empty());
    }
}

#[test]
fn locale_hint_searches_nested_payloads_and_arrays() {
    let payload = serde_json::json!({
        "event": {
            "sender": [
                { "profile": { "ignored": true } },
                { "profile": { "user_locale": "zh-TW" } }
            ]
        }
    });

    assert_eq!(find_locale_hint(&payload), Some("zh-TW".to_string()));
}

#[test]
fn detect_locale_from_post_content_uses_top_level_locale_keys() {
    assert_eq!(
        detect_locale_from_post_content(r#"{"en_us":{"content":[]}}"#),
        Some(LarkAckLocale::En)
    );
    assert_eq!(
        detect_locale_from_post_content(r#"{"zh_hant":{"content":[]}}"#),
        Some(LarkAckLocale::ZhTw)
    );
    assert_eq!(detect_locale_from_post_content("not-json"), None);
}

#[test]
fn script_helpers_classify_kana_han_and_locale_specific_characters() {
    assert!(is_japanese_kana('あ'));
    assert!(is_japanese_kana('カ'));
    assert!(!is_japanese_kana('A'));

    assert!(is_cjk_han('中'));
    assert!(is_traditional_only_han('體'));
    assert!(is_simplified_only_han('体'));
}

#[test]
fn detect_locale_from_text_prefers_kana_then_traditional_then_simplified() {
    assert_eq!(detect_locale_from_text("こんにちは"), Some(LarkAckLocale::Ja));
    assert_eq!(detect_locale_from_text("繁體中文很強"), Some(LarkAckLocale::ZhTw));
    assert_eq!(detect_locale_from_text("简体中文很强"), Some(LarkAckLocale::ZhCn));
    assert_eq!(detect_locale_from_text("中文"), Some(LarkAckLocale::ZhCn));
    assert_eq!(detect_locale_from_text("plain ascii"), None);
}

#[test]
fn detect_lark_ack_locale_reads_message_content_paths_before_fallback_text() {
    let payload = serde_json::json!({
        "event": {
            "message": {
                "content": "{\"ja_jp\":{\"content\":[]}}"
            }
        }
    });

    assert_eq!(detect_lark_ack_locale(Some(&payload), "plain ascii"), LarkAckLocale::Ja);
}

#[test]
fn random_from_pool_returns_item_from_supplied_pool() {
    static POOL: &[&str] = &["ONE", "TWO", "THREE"];

    assert!(POOL.contains(&random_from_pool(POOL)));
}
// 本文件验证 Lark 确认消息的语言检测、区域映射与随机选择。
