use config_ui::AppSystemSettingsConfig;
fn assert_f32_close(actual: f32, expected: f32) { assert!((actual - expected).abs() < 0.001, "expected {actual} to be close to {expected}"); }
#[test]
fn normalize_system_settings_config_replaces_invalid_font_size() {
    let cfg = AppSystemSettingsConfig {
        editor_font_size: f32::NAN,
        editor_line_height: 99.0,
        editor_auto_line_height: false,
        ..AppSystemSettingsConfig::default()
    };

    let normalized = super::normalize_system_settings_config(cfg);

    assert_eq!(normalized.editor_font_size, 16.0);
    assert_eq!(normalized.editor_line_height, 99.0);
}

#[test]
fn normalize_system_settings_config_replaces_non_positive_and_infinite_font_sizes() {
    for editor_font_size in [0.0, -1.0, f32::INFINITY, f32::NEG_INFINITY] {
        let cfg = AppSystemSettingsConfig {
            editor_font_size,
            editor_line_height: 24.0,
            editor_auto_line_height: false,
            ..AppSystemSettingsConfig::default()
        };

        let normalized = super::normalize_system_settings_config(cfg);

        assert_eq!(normalized.editor_font_size, 16.0);
        assert_eq!(normalized.editor_line_height, 24.0);
    }
}

#[test]
fn normalize_system_settings_config_auto_line_height_tracks_font_size() {
    let cfg = AppSystemSettingsConfig {
        editor_font_size: 15.0,
        editor_line_height: 1.0,
        editor_auto_line_height: true,
        ..AppSystemSettingsConfig::default()
    };

    let normalized = super::normalize_system_settings_config(cfg);

    assert_eq!(normalized.editor_font_size, 15.0);
    assert_eq!(normalized.editor_line_height, 21.0);
}

#[test]
fn normalize_system_settings_config_auto_line_height_uses_repaired_font_size() {
    let cfg = AppSystemSettingsConfig {
        editor_font_size: f32::NAN,
        editor_line_height: 1.0,
        editor_auto_line_height: true,
        ..AppSystemSettingsConfig::default()
    };

    let normalized = super::normalize_system_settings_config(cfg);

    assert_eq!(normalized.editor_font_size, 16.0);
    assert_eq!(normalized.editor_line_height, 22.4);
}

#[test]
fn normalize_system_settings_config_replaces_invalid_manual_line_height() {
    for editor_line_height in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let cfg = AppSystemSettingsConfig {
            editor_font_size: 16.0,
            editor_line_height,
            editor_auto_line_height: false,
            ..AppSystemSettingsConfig::default()
        };

        let normalized = super::normalize_system_settings_config(cfg);

        assert_eq!(normalized.editor_font_size, 16.0);
        assert_eq!(normalized.editor_line_height, 19.6);
    }
}

#[test]
fn normalize_system_settings_config_replaces_overflowing_auto_line_height() {
    let cfg = AppSystemSettingsConfig {
        editor_font_size: f32::MAX,
        editor_line_height: 1.0,
        editor_auto_line_height: true,
        ..AppSystemSettingsConfig::default()
    };

    let normalized = super::normalize_system_settings_config(cfg);

    assert_eq!(normalized.editor_font_size, f32::MAX);
    assert_eq!(normalized.editor_line_height, 19.6);
}
