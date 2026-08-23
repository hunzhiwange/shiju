//! 独立窗口导航工具栏的契约测试。
//!
//! 验证生成的控件与图标保持稳定、安全 URL 才能进入工具栏 IPC，
//! 并检查工具栏和内容视图在不同窗口模式下的区域划分与坐标转换。

use super::toolbar::{
    TOOLBAR_REGION_HEIGHT, ToolbarAction, ViewRegion, parse_toolbar_ipc_message,
    toolbar_address_script, toolbar_html, window_regions,
};

#[test]
fn toolbar_html_reuses_work_browser_controls_and_icons() {
    let html = toolbar_html();
    let action_positions =
        ["back", "forward", "navigate", "reload", "open_external", "close"].map(|action| {
            html.find(&format!(r#"data-action="{action}""#))
                .unwrap_or_else(|| panic!("missing toolbar action: {action}"))
        });

    assert!(action_positions.windows(2).all(|positions| positions[0] < positions[1]));
    assert!(html.contains(r#"id="address""#));
    assert!(html.contains("width: 28px"));
    assert!(html.contains("height: 28px"));
    assert!(html.contains("width: 16px"));
    assert!(html.contains("body {\n    padding: 0;"));
    assert!(html.contains("height: 100%"));
    assert!(html.contains("border-radius: 0"));
    assert!(html.contains("border-radius: 12px"));
    assert!(html.contains("(event.metaKey || event.ctrlKey)"));
    assert!(html.contains("address.select()"));

    for icon in [
        include_str!("../../../assets/icons/bootstrap/arrow-left.svg"),
        include_str!("../../../assets/icons/bootstrap/arrow-right.svg"),
        include_str!("../../../assets/icons/bootstrap/arrow-clockwise.svg"),
        include_str!("../../../assets/icons/bootstrap/globe.svg"),
        include_str!("../../../assets/icons/bootstrap/x-lg.svg"),
    ] {
        assert!(html.contains(icon));
    }

    let script = toolbar_address_script("https://example.com/a\"b").unwrap();
    assert!(script.contains(r#""https://example.com/a\"b""#));
}

#[test]
fn toolbar_ipc_accepts_controls_and_safe_urls() {
    assert_eq!(parse_toolbar_ipc_message(r#"{"type":"back"}"#), Some(ToolbarAction::Back));
    assert_eq!(parse_toolbar_ipc_message(r#"{"type":"forward"}"#), Some(ToolbarAction::Forward));
    assert_eq!(parse_toolbar_ipc_message(r#"{"type":"reload"}"#), Some(ToolbarAction::Reload));
    assert_eq!(parse_toolbar_ipc_message(r#"{"type":"close"}"#), Some(ToolbarAction::Clear));
    assert_eq!(
        parse_toolbar_ipc_message(r#"{"type":"navigate","url":" example.com/docs "}"#),
        Some(ToolbarAction::Navigate("https://example.com/docs".to_string()))
    );
    assert_eq!(
        parse_toolbar_ipc_message(
            r#"{"type":"open_external","url":"HTTPS://example.com/path?q=1"}"#
        ),
        Some(ToolbarAction::OpenExternal("HTTPS://example.com/path?q=1".to_string()))
    );

    for message in [
        r#"{"type":"navigate","url":"javascript:alert(1)"}"#,
        r#"{"type":"navigate","url":"file:///tmp/private"}"#,
        r#"{"type":"navigate","url":"https://example.com/a b"}"#,
        r#"{"type":"open_external","url":"data:text/html,unsafe"}"#,
        r#"{"type":"unknown"}"#,
        "not-json",
    ] {
        assert_eq!(parse_toolbar_ipc_message(message), None, "accepted {message}");
    }
}

#[test]
fn window_regions_reserve_toolbar_only_for_independent_mode() {
    assert_eq!(
        window_regions(900, 600, true),
        (
            Some(ViewRegion { x: 0, y: 0, width: 900, height: TOOLBAR_REGION_HEIGHT }),
            ViewRegion {
                x: 0,
                y: TOOLBAR_REGION_HEIGHT,
                width: 900,
                height: 600 - TOOLBAR_REGION_HEIGHT,
            }
        )
    );
    assert_eq!(
        window_regions(900, 600, false),
        (None, ViewRegion { x: 0, y: 0, width: 900, height: 600 })
    );
}

#[test]
fn view_region_converts_to_logical_wry_bounds() {
    let region = ViewRegion { x: 12, y: 46, width: 876, height: 554 };

    assert_eq!(
        region.to_wry_rect(),
        wry::Rect {
            position: wry::dpi::LogicalPosition::new(12, 46).into(),
            size: wry::dpi::LogicalSize::new(876, 554).into(),
        }
    );
}
