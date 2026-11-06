use super::icons::{read, update};
use base64::Engine;
use vw_api_types::app_providers::icons::AppProviderIconChange as Change;
fn image(w: u32, h: u32) -> String {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&vec![0; (w * h * 4) as usize]).unwrap();
    }
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}
use crate::icons::library;
use vw_api_types::agent_apps::CustomIconEntry;
fn add(data: &str) -> CustomIconEntry {
    library::add_icon(&[], "Icon", data, "fixture", 1).unwrap().0.icon
}
#[test]
fn cc04_icons_roundtrip_and_reference_check() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("profile.json");
    let data = image(96, 96);
    let icon = add(&data);
    let rows = vec![icon.clone()];
    assert!(library::add_icon(&rows, "Other", &data, "other", 2).unwrap().0.existed);
    let stored = serde_json::json!({"apiKey":"fixture","future":true,"iconColor":"red"});
    let selected = update(&stored, &Change::Select { icon_id: icon.id.clone() }, &rows).unwrap();
    assert_eq!(read(&selected).unwrap().data.as_deref(), Some(data.as_str()));
    std::fs::write(&file, serde_json::to_string(&selected).unwrap()).unwrap();
    let restored = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    let renamed = library::rename_icon(&rows, &icon.id, "New").unwrap();
    assert_eq!(renamed[0].name, "New");
    assert!(library::delete_icon(&renamed, &icon.id).unwrap().is_empty());
    assert_eq!(read(&restored).unwrap().data, Some(data.clone()));
    assert!(update(&stored, &Change::Select { icon_id: icon.id }, &[]).is_err());
    let cleared = update(&restored, &Change::Clear, &[]).unwrap();
    assert_eq!(cleared, serde_json::json!({"apiKey":"fixture","future":true}));
    assert_eq!(stored["iconColor"], "red");
}
#[test]
fn cc04_icons_reject_unsafe_payload() {
    let stored = serde_json::json!({"apiKey":"fixture"});
    let good = image(1, 1);
    for data in [
        "file:///private/fixture.png".into(),
        "/private/fixture.png".into(),
        "https://fixture.test/icon.png".into(),
        "data:image/svg+xml;base64,PHN2Zy8+".into(),
        "data:image/png;base64,!!!".into(),
        format!("data:image/png;base64,{}", "A".repeat(65536)),
        image(257, 1),
        good[..good.len() - 8].into(),
    ] {
        assert!(library::add_icon(&[], "Icon", &data, "fixture", 1).is_err());
        let imported = CustomIconEntry {
            id: "fixture".into(),
            name: "Icon".into(),
            data: data.clone(),
            created_at: 1,
        };
        assert!(
            update(&stored, &Change::Select { icon_id: "fixture".into() }, &[imported]).is_err()
        );
        assert!(read(&serde_json::json!({"icon":data})).is_err());
    }
    assert!(read(&serde_json::json!("invalid")).is_err());
    assert!(update(&stored, &Change::Select { icon_id: "../path".into() }, &[]).is_err());
}
