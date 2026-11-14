use super::library::{add_icon, delete_icon, rename_icon};
use base64::Engine;
use vw_api_types::agent_apps::{AgentAppsErrorCode, CustomIconEntry};

fn image(pixel: u8) -> String {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&[pixel]).unwrap();
    }
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}

fn entry() -> CustomIconEntry {
    CustomIconEntry { id: "old".into(), name: "原名".into(), data: image(0), created_at: 10 }
}

#[test]
fn cc_base_icon_add_duplicate_preserves_entry_and_collection() {
    let icons = vec![entry()];
    let (result, updated) = add_icon(&icons, "新名", &image(0), "new", 20).unwrap();
    assert!(result.existed);
    assert_eq!(result.icon, icons[0]);
    assert_eq!(updated, None);
}

#[test]
fn cc_base_icon_add_new_prepends_normalized_entry() {
    let icons = vec![entry()];
    let (result, updated) = add_icon(&icons, " 新图 ", &image(1), "new", 20).unwrap();
    assert!(!result.existed);
    assert_eq!(
        result.icon,
        CustomIconEntry { id: "new".into(), name: "新图".into(), data: image(1), created_at: 20 }
    );
    assert_eq!(updated.unwrap(), vec![result.icon, icons[0].clone()]);
    assert_eq!(icons, vec![entry()]);
}

#[test]
fn cc_base_icon_add_full_library_accepts_duplicate() {
    let icons = vec![entry(); 100];
    let (result, updated) = add_icon(&icons, "新名", &image(0), "new", 20).unwrap();
    assert!(result.existed);
    assert_eq!(result.icon, icons[0]);
    assert_eq!(updated, None);
}

#[test]
fn cc_base_icon_add_full_library_rejects_new_image() {
    assert_eq!(
        add_icon(&vec![entry(); 100], "新图", &image(1), "new", 20),
        Err(AgentAppsErrorCode::Conflict)
    );
}

#[test]
fn cc_base_icon_add_accepts_hundredth_entry() {
    let (_, updated) = add_icon(&vec![entry(); 99], "新图", &image(1), "new", 20).unwrap();
    assert_eq!(updated.unwrap().len(), 100);
}

#[test]
fn cc_base_icon_add_rejects_invalid_name_before_deduplication() {
    for name in [" ".to_owned(), "图".repeat(65)] {
        assert_eq!(
            add_icon(&[entry()], &name, &image(0), "new", 20),
            Err(AgentAppsErrorCode::InvalidInput)
        );
    }
}

#[test]
fn cc_base_icon_add_rejects_invalid_image_before_deduplication() {
    let mut invalid = entry();
    invalid.data = "invalid".into();
    assert_eq!(
        add_icon(&[invalid], "图", "invalid", "new", 20),
        Err(AgentAppsErrorCode::InvalidInput)
    );
}

#[test]
fn cc_base_icon_rename_changes_only_matching_name() {
    let mut other = entry();
    other.id = "other".into();
    let icons = vec![other.clone(), entry()];
    let updated = rename_icon(&icons, "old", " 新名称 ").unwrap();
    let mut renamed = entry();
    renamed.name = "新名称".into();
    assert_eq!(updated, vec![other, renamed]);
    assert_eq!(icons[1], entry());
}

#[test]
fn cc_base_icon_rename_missing_id_returns_not_found() {
    assert_eq!(rename_icon(&[entry()], "missing", "名称"), Err(AgentAppsErrorCode::NotFound));
    assert_eq!(rename_icon(&[], "old", "名称"), Err(AgentAppsErrorCode::NotFound));
}

#[test]
fn cc_base_icon_rename_rejects_invalid_names() {
    for name in ["".to_owned(), " \t　".to_owned(), "图".repeat(65)] {
        assert_eq!(rename_icon(&[entry()], "old", &name), Err(AgentAppsErrorCode::InvalidInput));
    }
}

#[test]
fn cc_base_icon_rename_accepts_64_chinese_characters() {
    let name = "图".repeat(64);
    let updated = rename_icon(&[entry()], "old", &format!(" {name} ")).unwrap();
    assert_eq!(updated[0].name, name);
}

#[test]
fn cc_base_icon_delete_missing_entry_leaves_library_unchanged() {
    let icons = vec![entry()];
    assert_eq!(delete_icon(&icons, "missing"), None);
    assert_eq!(delete_icon(&[], "old"), None);
    assert_eq!(icons, vec![entry()]);
}

#[test]
fn cc_base_icon_delete_repeated_deletion_is_unchanged() {
    let updated = delete_icon(&[entry()], "old").unwrap();
    assert!(updated.is_empty());
    assert_eq!(delete_icon(&updated, "old"), None);
}

#[test]
fn cc_base_icon_delete_preserves_other_entries_order_and_original_images() {
    let mut first = entry();
    first.id = "first".into();
    let mut last = entry();
    last.id = "last".into();
    last.data = image(1);
    let icons = vec![first.clone(), entry(), last.clone()];
    let original = icons.clone();
    assert_eq!(delete_icon(&icons, "old"), Some(vec![first, last]));
    assert_eq!(icons, original);
}

#[test]
fn cc_base_icon_delete_removes_all_matching_ids() {
    let icons = vec![entry(), entry()];
    assert_eq!(delete_icon(&icons, "old"), Some(vec![]));
}
