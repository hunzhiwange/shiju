//! 应用初始化的跨域配置接线测试。
use serde_json::json;
#[test]
fn app_update_initial_config_defaults_and_rejects_invalid_sources() {
    use crate::app::desktop_update::DesktopUpdate;
    use desktop_release::UpdateSource;
    assert_eq!(DesktopUpdate::from_config(&json!({})).source, Some(UpdateSource::Shiju));
    assert_eq!(
        DesktopUpdate::from_config(&json!({"app_update_source":"github"})).source,
        Some(UpdateSource::Github)
    );
    for value in [json!("other"), json!(null), json!(42)] {
        let state = DesktopUpdate::from_config(&json!({"app_update_source":value}));
        assert!(state.source.is_none());
        assert!(state.error.is_some());
    }
}
