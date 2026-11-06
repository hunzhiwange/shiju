use super::presentation::{read, update};
use vw_api_types::app_providers::presentation::AppProviderPresentation;
fn request() -> AppProviderPresentation {
    AppProviderPresentation {
        sort_index: Some(2),
        category: Some("团队分类".into()),
        notes: Some("备注\n第二行".into()),
    }
}
fn stored() -> serde_json::Value {
    serde_json::json!({"apiKey":"fixture-secret","name":"fixture","meta":{"authBinding":"fixture-account"},"current":true,"sortIndex":9})
}
#[test]
fn cc04_presentation_order_survives_reload() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("catalog.json");
    std::fs::write(&path, serde_json::to_vec(&update(&stored(), &request()).unwrap()).unwrap())
        .unwrap();
    let reloaded = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(read(&reloaded).unwrap(), request());
}
#[test]
fn cc04_presentation_metadata_does_not_change_live() {
    let root = tempfile::tempdir().unwrap();
    let live = root.path().join("settings.json");
    std::fs::write(&live, b"native fixture").unwrap();
    let before = stored();
    let after = update(&before, &request()).unwrap();
    assert_eq!(after["category"], "团队分类");
    for key in ["apiKey", "name", "meta", "current"] {
        assert_eq!(after[key], before[key]);
    }
    assert_eq!(std::fs::read(live).unwrap(), b"native fixture");
    assert_eq!(
        read(&update(&after, &AppProviderPresentation::default()).unwrap()).unwrap(),
        AppProviderPresentation::default()
    );
}

#[test]
fn cc04_presentation_rejects_invalid_text_and_storage() {
    for input in [
        AppProviderPresentation { category: Some("bad\ncategory".into()), ..request() },
        AppProviderPresentation { category: Some("x".repeat(129)), ..request() },
        AppProviderPresentation { notes: Some("bad\0notes".into()), ..request() },
        AppProviderPresentation { notes: Some("x".repeat(8193)), ..request() },
    ] {
        assert!(update(&stored(), &input).is_err());
    }
    for invalid in [serde_json::json!(null), serde_json::json!([]), serde_json::json!(42)] {
        assert!(read(&invalid).is_err());
        assert!(update(&invalid, &request()).is_err());
    }
}
