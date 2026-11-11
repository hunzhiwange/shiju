use crate::computer_control::*;
use serde_json::{Value, json};

fn feature(state: &str) -> Value {
    json!({"enabled": true, "readiness": state, "system_authorization": if state == "ready" { "granted" } else { "denied" },
        "connection": if state == "ready" { "connected" } else { "disconnected" }, "backend": "available", "capabilities": []})
}

#[test]
fn cc_control_02_five_independent_states_roundtrip() {
    let value = json!({
        "permission_version": 7,
        "native_apps": feature("not_installed"), "chrome": feature("unauthorized"),
        "excel": feature("disconnected"), "word": feature("ready"),
        "locked_use": {"status": feature("unsupported"), "probe": "unsupported"}
    });
    let status: ComputerControlStatus = serde_json::from_value(value.clone()).unwrap();
    assert!(status.chrome.enabled);
    assert_eq!(status.chrome.readiness, ControlReadiness::Unauthorized);
    assert_ne!(status.excel.readiness, status.word.readiness);
    assert_eq!(status.locked_use.probe, LockedUseProbe::Unsupported);
    assert_eq!(serde_json::to_value(status).unwrap(), value);
    for probe in ["not_probed", "supported", "unsupported"] {
        serde_json::from_value::<LockedUseProbe>(json!(probe)).unwrap();
    }
}

#[test]
fn cc_control_02_structured_actions_and_identity_roundtrip() {
    for (target, action) in [
        (
            json!({"kind":"native_app", "bundle_id":"org.example.fixture", "process_id":42}),
            json!({"action":"native_observe", "screenshot":true}),
        ),
        (
            json!({"kind":"native_app", "bundle_id":"org.example.fixture", "process_id":42}),
            json!({"action":"native_input", "input":{"kind":"click", "x":10, "y":20, "button":"left"}}),
        ),
        (
            json!({"kind":"chrome_tab", "tab_id":12, "origin":"https://example.org"}),
            json!({"action":"chrome", "command":{"kind":"navigate", "url":"https://example.org/path"}}),
        ),
        (
            json!({"kind":"excel_workbook", "document_id":"workbook-1"}),
            json!({"action":"excel", "command":{"kind":"write_range", "sheet":"Sheet1", "range":"A1:B1", "values":[["fixture", 12]]}}),
        ),
        (
            json!({"kind":"word_document", "document_id":"document-1"}),
            json!({"action":"word", "command":{"kind":"replace_text", "search":"before", "replacement":"after"}}),
        ),
    ] {
        let value = json!({"request_id":"request-1", "session_id":"session-1",
            "permission_version":3, "target":target, "action":action});
        let request: ComputerControlRequest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(request.permission_version, PermissionVersion(3));
        assert_eq!(serde_json::to_value(request).unwrap(), value);
    }
}

#[test]
fn cc_control_02_unknown_actions_and_code_payloads_are_rejected() {
    for value in [
        json!({"action":"execute", "code":"anything"}),
        json!({"action":"native_observe", "screenshot":true, "shell":"anything"}),
        json!({"action":"chrome", "command":{"kind":"evaluate", "code":"anything"}}),
        json!({"action":"excel", "command":{"kind":"run", "code":"anything"}}),
        json!({"action":"word", "command":{"kind":"run", "code":"anything"}}),
        json!({"action":"native_input", "input":{"kind":"apple_script", "code":"anything"}}),
    ] {
        assert!(serde_json::from_value::<ComputerControlAction>(value).is_err());
    }
}

#[test]
fn cc_control_02_results_roundtrip_without_credentials_or_local_paths() {
    let value = json!({"request_id":"request-1", "session_id":"session-1",
        "permission_version":4, "outcome":{"status":"succeeded", "output":{
            "kind":"observation", "text":"fixture", "screenshots":[{
                "media_id":"media-1", "mime_type":"image/png", "byte_length":1024,
                "width":640, "height":480}]}}});
    let result: ComputerControlResult = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), value);
    for secret in ["token", "pairing_token", "api_key", "path"] {
        let mut contaminated = value.clone();
        contaminated[secret] = json!("sensitive-fixture");
        assert!(serde_json::from_value::<ComputerControlResult>(contaminated).is_err());
    }
    let rejected = json!({"request_id":"request-1", "session_id":"session-1",
        "permission_version":5, "outcome":{"status":"rejected", "error":"stale_permission"}});
    let result: ComputerControlResult = serde_json::from_value(rejected.clone()).unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), rejected);
}

#[test]
fn cc_control_02_screenshot_references_enforce_limits_and_no_paths() {
    let good = json!({"media_id":"media-1", "mime_type":"image/png", "byte_length":1024,
        "width":640, "height":480});
    for (key, invalid) in [
        ("byte_length", json!(0)),
        ("byte_length", json!(MAX_SCREENSHOT_BYTES + 1)),
        ("width", json!(0)),
        ("height", json!(16385)),
        ("media_id", json!("/Users/fixture/screenshot.png")),
        ("media_id", json!("https://example.org/image.png")),
        ("mime_type", json!("text/html")),
        ("path", json!("/tmp/fixture.png")),
    ] {
        let mut value = good.clone();
        value[key] = invalid;
        assert!(serde_json::from_value::<ScreenshotReference>(value).is_err(), "{key}");
    }
    assert!(
        ScreenshotReference::new(
            "media-1".into(),
            ScreenshotMimeType::Png,
            MAX_SCREENSHOT_BYTES + 1,
            640,
            480
        )
        .is_err()
    );
    let screenshot = ScreenshotReference::new(
        "media-1".into(),
        ScreenshotMimeType::Png,
        MAX_SCREENSHOT_BYTES,
        640,
        480,
    )
    .unwrap();
    assert_eq!(screenshot.byte_length(), MAX_SCREENSHOT_BYTES);
}

#[test]
fn cc_control_02_closed_command_vocabulary_roundtrips() {
    let actions = [
        json!({"action":"stop"}),
        json!({"action":"native_input","input":{"kind":"activate"}}),
        json!({"action":"native_input","input":{"kind":"scroll","delta_x":0,"delta_y":20}}),
        json!({"action":"native_input","input":{"kind":"press","key":"A","modifiers":["meta"]}}),
        json!({"action":"native_input","input":{"kind":"type_text","text":"fixture"}}),
        json!({"action":"chrome","command":{"kind":"snapshot"}}),
        json!({"action":"chrome","command":{"kind":"screenshot"}}),
        json!({"action":"chrome","command":{"kind":"click","selector":"@ref1"}}),
        json!({"action":"chrome","command":{"kind":"fill","selector":"@ref1","text":"fixture"}}),
        json!({"action":"chrome","command":{"kind":"press","key":"Enter","modifiers":[]}}),
        json!({"action":"chrome","command":{"kind":"scroll","delta_x":0,"delta_y":20}}),
        json!({"action":"excel","command":{"kind":"read_range","sheet":"Sheet1","range":"A1"}}),
        json!({"action":"word","command":{"kind":"read_document"}}),
        json!({"action":"word","command":{"kind":"find_text","text":"fixture"}}),
        json!({"action":"word","command":{"kind":"insert_text","position":"end","text":"fixture"}}),
    ];
    for action in actions {
        let parsed: ComputerControlAction = serde_json::from_value(action.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), action);
    }
    for value in [json!("fixture"), json!(12), json!(true), Value::Null] {
        let parsed: OfficeCellValue = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    }
    assert!(serde_json::from_value::<OfficeCellValue>(json!({"code":"anything"})).is_err());
}
