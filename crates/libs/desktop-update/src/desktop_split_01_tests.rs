use crate::app::desktop_update::{DesktopUpdate, source_patch};
use desktop_release::UpdateSource;
#[test]
fn desktop_integration_47_validates_update_sources() {
    assert_eq!(
        DesktopUpdate::from_config(&serde_json::json!({})).source,
        Some(UpdateSource::Shiju)
    );
    for source in [UpdateSource::Shiju, UpdateSource::Github] {
        let state = DesktopUpdate::from_config(&source_patch(source));
        assert_eq!(state.source, Some(source));
        assert!(state.error.is_none());
    }
    for value in [
        serde_json::json!("cc-switch"),
        serde_json::json!("https://example.com/latest"),
        serde_json::Value::Null,
    ] {
        let state = DesktopUpdate::from_config(&serde_json::json!({"app_update_source": value}));
        assert!(state.source.is_none());
        assert!(state.error.is_some());
        assert!(!state.installable());
    }
}
