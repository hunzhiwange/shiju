use super::*;
use serde_json::json;

#[test]
fn parse_browser_action_preserves_required_fields() {
    let action = parse_browser_action("fill", &json!({"selector": "#name", "value": "Ada"}))
        .expect("fill action should parse");

    match action {
        BrowserAction::Fill { selector, value } => {
            assert_eq!(selector, "#name");
            assert_eq!(value, "Ada");
        }
        other => panic!("unexpected action: {other:?}"),
    }
}

#[test]
fn parse_browser_action_rejects_missing_required_field() {
    let err = parse_browser_action("open", &json!({})).expect_err("url is required");
    assert!(err.to_string().contains("Missing 'url'"));
}

#[test]
fn action_support_lists_include_computer_use_only_actions() {
    assert!(is_supported_browser_action("open"));
    assert!(is_supported_browser_action("mouse_click"));
    assert!(is_computer_use_only_action("mouse_click"));
    assert!(!is_computer_use_only_action("open"));
    assert!(!is_supported_browser_action("launch_missiles"));
}

#[test]
fn parse_browser_action_covers_optional_and_numeric_fields() {
    assert!(matches!(
        parse_browser_action(
            "snapshot",
            &json!({"interactive_only": false, "compact": false, "depth": u64::MAX})
        )
        .unwrap(),
        BrowserAction::Snapshot { interactive_only: false, compact: false, depth: Some(u32::MAX) }
    ));
    assert!(matches!(
        parse_browser_action("screenshot", &json!({"path": "shot.png", "full_page": true}))
            .unwrap(),
        BrowserAction::Screenshot { path: Some(path), full_page: true } if path == "shot.png"
    ));
    assert!(matches!(
        parse_browser_action("wait", &json!({"selector": "#ready", "ms": 7, "text": "ok"}))
            .unwrap(),
        BrowserAction::Wait { selector: Some(selector), ms: Some(7), text: Some(text) }
            if selector == "#ready" && text == "ok"
    ));
    assert!(matches!(
        parse_browser_action("scroll", &json!({"direction": "down", "pixels": u64::MAX}))
            .unwrap(),
        BrowserAction::Scroll { direction, pixels: Some(u32::MAX) } if direction == "down"
    ));
    assert!(matches!(
        parse_browser_action(
            "find",
            &json!({"by": "role", "value": "button", "find_action": "fill", "fill_value": "Ada"})
        )
        .unwrap(),
        BrowserAction::Find { by, value, action, fill_value: Some(fill) }
            if by == "role" && value == "button" && action == "fill" && fill == "Ada"
    ));
}

#[test]
fn parse_browser_action_covers_all_simple_variants() {
    assert!(matches!(
        parse_browser_action("open", &json!({"url": "https://example.com"})).unwrap(),
        BrowserAction::Open { url } if url == "https://example.com"
    ));
    assert!(matches!(
        parse_browser_action("click", &json!({"selector": "#go"})).unwrap(),
        BrowserAction::Click { selector } if selector == "#go"
    ));
    assert!(matches!(
        parse_browser_action("type", &json!({"selector": "#name", "text": "Ada"})).unwrap(),
        BrowserAction::Type { selector, text } if selector == "#name" && text == "Ada"
    ));
    assert!(matches!(
        parse_browser_action("get_text", &json!({"selector": "main"})).unwrap(),
        BrowserAction::GetText { selector } if selector == "main"
    ));
    assert!(matches!(
        parse_browser_action("get_title", &json!({})).unwrap(),
        BrowserAction::GetTitle
    ));
    assert!(matches!(parse_browser_action("get_url", &json!({})).unwrap(), BrowserAction::GetUrl));
    assert!(matches!(
        parse_browser_action("press", &json!({"key": "Enter"})).unwrap(),
        BrowserAction::Press { key } if key == "Enter"
    ));
    assert!(matches!(
        parse_browser_action("hover", &json!({"selector": ".menu"})).unwrap(),
        BrowserAction::Hover { selector } if selector == ".menu"
    ));
    assert!(matches!(
        parse_browser_action("is_visible", &json!({"selector": "#modal"})).unwrap(),
        BrowserAction::IsVisible { selector } if selector == "#modal"
    ));
    assert!(matches!(parse_browser_action("close", &json!({})).unwrap(), BrowserAction::Close));
}

#[test]
fn parse_browser_action_reports_each_required_field() {
    for (action, args, expected) in [
        ("click", json!({}), "Missing 'selector' for click"),
        ("fill", json!({"selector": "#x"}), "Missing 'value' for fill"),
        ("type", json!({"selector": "#x"}), "Missing 'text' for type"),
        ("get_text", json!({}), "Missing 'selector' for get_text"),
        ("press", json!({}), "Missing 'key' for press"),
        ("hover", json!({}), "Missing 'selector' for hover"),
        ("scroll", json!({}), "Missing 'direction' for scroll"),
        ("is_visible", json!({}), "Missing 'selector' for is_visible"),
        ("find", json!({}), "Missing 'by' for find"),
        ("find", json!({"by": "text"}), "Missing 'value' for find"),
        ("find", json!({"by": "text", "value": "Save"}), "Missing 'find_action' for find"),
        ("unknown", json!({}), "Unsupported browser action"),
    ] {
        let err = parse_browser_action(action, &args).expect_err("action should fail");
        assert!(err.to_string().contains(expected), "{action}: {err}");
    }
}

#[test]
fn parse_browser_action_applies_optional_defaults_and_ignores_wrong_types() {
    assert!(matches!(
        parse_browser_action("snapshot", &json!({})).unwrap(),
        BrowserAction::Snapshot { interactive_only: true, compact: true, depth: None }
    ));
    assert!(matches!(
        parse_browser_action(
            "snapshot",
            &json!({"interactive_only": "false", "compact": 0, "depth": -1})
        )
        .unwrap(),
        BrowserAction::Snapshot { interactive_only: true, compact: true, depth: None }
    ));
    assert!(matches!(
        parse_browser_action("screenshot", &json!({"path": false, "full_page": "true"})).unwrap(),
        BrowserAction::Screenshot { path: None, full_page: false }
    ));
    assert!(matches!(
        parse_browser_action("wait", &json!({})).unwrap(),
        BrowserAction::Wait { selector: None, ms: None, text: None }
    ));
    assert!(matches!(
        parse_browser_action("scroll", &json!({"direction": "up", "pixels": -1})).unwrap(),
        BrowserAction::Scroll { direction, pixels: None } if direction == "up"
    ));
    assert!(matches!(
        parse_browser_action(
            "find",
            &json!({"by": "text", "value": "Save", "find_action": "click"})
        )
        .unwrap(),
        BrowserAction::Find { fill_value: None, .. }
    ));
}

#[test]
fn parse_browser_action_rejects_non_string_required_fields() {
    for (action, args, expected) in [
        ("open", json!({"url": 1}), "Missing 'url' for open action"),
        ("click", json!({"selector": false}), "Missing 'selector' for click"),
        ("fill", json!({"value": "Ada"}), "Missing 'selector' for fill"),
        ("fill", json!({"selector": "#name", "value": 1}), "Missing 'value' for fill"),
        ("type", json!({"text": "Ada"}), "Missing 'selector' for type"),
        ("type", json!({"selector": "#name", "text": false}), "Missing 'text' for type"),
        ("get_text", json!({"selector": []}), "Missing 'selector' for get_text"),
        ("press", json!({"key": null}), "Missing 'key' for press"),
        ("hover", json!({"selector": {}}), "Missing 'selector' for hover"),
        ("scroll", json!({"direction": 1}), "Missing 'direction' for scroll"),
        ("is_visible", json!({"selector": false}), "Missing 'selector' for is_visible"),
        ("find", json!({"by": []}), "Missing 'by' for find"),
        ("find", json!({"by": "role", "value": 1}), "Missing 'value' for find"),
        (
            "find",
            json!({"by": "role", "value": "button", "find_action": false}),
            "Missing 'find_action' for find",
        ),
    ] {
        let err = parse_browser_action(action, &args).expect_err("non-string field should fail");
        assert!(err.to_string().contains(expected), "{action}: {err}");
    }
}

#[test]
fn parse_browser_action_preserves_empty_required_strings() {
    assert!(matches!(
        parse_browser_action("open", &json!({"url": ""})).unwrap(),
        BrowserAction::Open { url } if url.is_empty()
    ));
    assert!(matches!(
        parse_browser_action("fill", &json!({"selector": "", "value": ""})).unwrap(),
        BrowserAction::Fill { selector, value } if selector.is_empty() && value.is_empty()
    ));
}

#[test]
fn action_support_lists_cover_every_declared_action() {
    let standard_actions = [
        "open",
        "snapshot",
        "click",
        "fill",
        "type",
        "get_text",
        "get_title",
        "get_url",
        "screenshot",
        "wait",
        "press",
        "hover",
        "scroll",
        "is_visible",
        "close",
        "find",
    ];
    let computer_use_actions =
        ["mouse_move", "mouse_click", "mouse_drag", "key_type", "key_press", "screen_capture"];

    for action in standard_actions {
        assert!(is_supported_browser_action(action), "{action}");
        assert!(!is_computer_use_only_action(action), "{action}");
    }
    for action in computer_use_actions {
        assert!(is_supported_browser_action(action), "{action}");
        assert!(is_computer_use_only_action(action), "{action}");
    }
    for action in ["", "Open", " open", "unknown"] {
        assert!(!is_supported_browser_action(action), "{action}");
        assert!(!is_computer_use_only_action(action), "{action}");
    }
}

#[test]
fn browser_action_serialization_uses_snake_case_and_preserves_fields() {
    let snapshot = serde_json::to_value(BrowserAction::Snapshot {
        interactive_only: true,
        compact: false,
        depth: Some(3),
    })
    .expect("snapshot should serialize");
    assert_eq!(
        snapshot,
        json!({"snapshot": {"interactive_only": true, "compact": false, "depth": 3}})
    );

    let find = serde_json::to_value(BrowserAction::Find {
        by: "role".into(),
        value: "button".into(),
        action: "fill".into(),
        fill_value: None,
    })
    .expect("find should serialize");
    assert_eq!(
        find,
        json!({
            "find": {
                "by": "role",
                "value": "button",
                "action": "fill",
                "fill_value": null
            }
        })
    );
    assert_eq!(
        serde_json::to_value(BrowserAction::GetTitle).expect("unit action should serialize"),
        json!("get_title")
    );
}

#[test]
fn browser_action_deserialization_applies_serde_defaults_and_rejects_unknown_action() {
    let snapshot: BrowserAction =
        serde_json::from_value(json!({"snapshot": {}})).expect("snapshot should deserialize");
    assert!(matches!(
        snapshot,
        BrowserAction::Snapshot { interactive_only: false, compact: false, depth: None }
    ));

    let screenshot: BrowserAction =
        serde_json::from_value(json!({"screenshot": {}})).expect("screenshot should deserialize");
    assert!(matches!(screenshot, BrowserAction::Screenshot { path: None, full_page: false }));

    let err = serde_json::from_value::<BrowserAction>(json!("unknown"))
        .expect_err("unknown action should fail");
    assert!(err.to_string().contains("unknown variant"));
}
