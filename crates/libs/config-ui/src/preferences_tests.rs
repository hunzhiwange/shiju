use super::{DEFAULT_TIMEZONE, ModelRoute, PreviewAutoSaveMode, timezone_options};
use std::collections::HashSet;

// 显示标签属于用户界面文案契约，默认模式需避免未预期的频繁保存。
#[test]
fn preview_auto_save_modes_have_stable_labels_and_display_text() {
    let labels = PreviewAutoSaveMode::ALL.map(PreviewAutoSaveMode::label);

    assert_eq!(PreviewAutoSaveMode::default(), PreviewAutoSaveMode::OnFocusChange);
    assert_eq!(labels, ["关闭", "延迟保存", "编辑器失焦时保存", "窗口失焦时保存"]);
    assert_eq!(PreviewAutoSaveMode::AfterDelay.to_string(), "延迟保存");
}

#[test]
fn timezone_options_are_unique_and_include_default() {
    let options = timezone_options();
    let unique = options.iter().copied().collect::<HashSet<_>>();

    assert!(options.contains(&DEFAULT_TIMEZONE));
    assert_eq!(unique.len(), options.len());
}

// 以下细粒度用例锁定公开枚举、默认实现和 serde 名称，便于定位兼容回归。
#[test]
fn preview_auto_save_mode_descriptions() {
    assert_eq!(PreviewAutoSaveMode::Off.description(), "不自动保存，需手动保存当前文件。");
    assert_eq!(
        PreviewAutoSaveMode::AfterDelay.description(),
        "编辑内容变更后，等待短暂延迟再自动保存。"
    );
    assert_eq!(
        PreviewAutoSaveMode::OnFocusChange.description(),
        "当编辑器失去焦点时，自动保存当前文件。"
    );
    assert_eq!(
        PreviewAutoSaveMode::OnWindowChange.description(),
        "当应用窗口失去焦点时，自动保存当前文件。"
    );
}

#[test]
fn preview_auto_save_mode_default_is_on_focus_change() {
    assert_eq!(PreviewAutoSaveMode::default(), PreviewAutoSaveMode::OnFocusChange);
}

#[test]
fn preview_auto_save_mode_serde_roundtrip() {
    for variant in PreviewAutoSaveMode::ALL {
        let json = serde_json::to_string(&variant).unwrap();
        let deserialized: PreviewAutoSaveMode = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, deserialized);
    }
}

#[test]
fn preview_auto_save_mode_serde_camel_case_names() {
    assert_eq!(serde_json::to_value(PreviewAutoSaveMode::Off).unwrap(), serde_json::json!("off"));
    assert_eq!(
        serde_json::to_value(PreviewAutoSaveMode::AfterDelay).unwrap(),
        serde_json::json!("afterDelay")
    );
    assert_eq!(
        serde_json::to_value(PreviewAutoSaveMode::OnFocusChange).unwrap(),
        serde_json::json!("onFocusChange")
    );
    assert_eq!(
        serde_json::to_value(PreviewAutoSaveMode::OnWindowChange).unwrap(),
        serde_json::json!("onWindowChange")
    );
}

#[test]
fn preview_auto_save_mode_constant_is_all_variants() {
    assert_eq!(
        PreviewAutoSaveMode::ALL,
        [
            PreviewAutoSaveMode::Off,
            PreviewAutoSaveMode::AfterDelay,
            PreviewAutoSaveMode::OnFocusChange,
            PreviewAutoSaveMode::OnWindowChange,
        ]
    );
}

#[test]
fn model_route_serde_roundtrip() {
    let route = ModelRoute {
        pattern: "test/*".to_string(),
        provider: "anthropic".to_string(),
        model: "claude-sonnet-4-6".to_string(),
        priority: 10,
    };
    let json = serde_json::to_value(&route).unwrap();
    assert_eq!(json["pattern"], "test/*");
    assert_eq!(json["provider"], "anthropic");
    assert_eq!(json["model"], "claude-sonnet-4-6");
    assert_eq!(json["priority"], 10);

    let deserialized: ModelRoute = serde_json::from_value(json).unwrap();
    assert_eq!(route, deserialized);
}

#[test]
fn model_route_default_fields() {
    let route: ModelRoute = serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(route.pattern.is_empty());
    assert!(route.provider.is_empty());
    assert!(route.model.is_empty());
    assert_eq!(route.priority, 0);
}
