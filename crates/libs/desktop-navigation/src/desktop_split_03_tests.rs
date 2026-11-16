use crate::components::top_bar::types::{ExternalOpenApp, RuntimePlatform};
#[test]
fn apply_external_apps_state_selects_available_priority_and_keeps_finder_available() {
    let mut platform = None;
    let mut available = std::collections::HashMap::new();
    let mut selected = ExternalOpenApp::VSCode;

    crate::external_apps_update::apply_external_apps_state(
        &mut platform,
        &mut available,
        &mut selected,
        Some(RuntimePlatform::MacOs),
        vec![
            ("cursor".to_string(), false),
            ("zed".to_string(), true),
            ("unknown".to_string(), true),
        ],
    );

    assert_eq!(platform, Some(RuntimePlatform::MacOs));
    assert_eq!(selected, ExternalOpenApp::Zed);
    assert_eq!(available.get(&ExternalOpenApp::Finder), Some(&true));
    assert_eq!(available.get(&ExternalOpenApp::Cursor), Some(&false));
}

#[test]
fn apply_external_apps_state_preserves_current_app_when_still_available() {
    let mut platform = Some(RuntimePlatform::Linux);
    let mut available = std::collections::HashMap::new();
    let mut selected = ExternalOpenApp::Cursor;

    crate::external_apps_update::apply_external_apps_state(
        &mut platform,
        &mut available,
        &mut selected,
        None,
        vec![("cursor".to_string(), true)],
    );

    assert_eq!(platform, Some(RuntimePlatform::Linux));
    assert_eq!(selected, ExternalOpenApp::Cursor);
}

#[test]
fn apply_external_apps_state_falls_back_when_duplicate_marks_current_unavailable() {
    let mut platform = None;
    let mut available = std::collections::HashMap::new();
    let mut selected = ExternalOpenApp::Cursor;

    crate::external_apps_update::apply_external_apps_state(
        &mut platform,
        &mut available,
        &mut selected,
        Some(RuntimePlatform::Linux),
        vec![
            ("cursor".to_string(), true),
            ("cursor".to_string(), false),
            ("trae".to_string(), false),
        ],
    );

    assert_eq!(selected, ExternalOpenApp::Finder);
    assert_eq!(available.get(&ExternalOpenApp::Cursor), Some(&false));
    assert_eq!(available.get(&ExternalOpenApp::Finder), Some(&true));
}
