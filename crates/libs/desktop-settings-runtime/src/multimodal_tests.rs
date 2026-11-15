use super::*;
fn app() -> MultimodalSettingsState { Default::default() }
#[test]
fn ocr_settings_require_valid_model_before_enable_and_map_to_config() {
    let mut app = app();
    app.ocr_enabled = false;
    app.model_supports_images = false;
    app.ocr_model.clear();

    let _ =
        update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::OcrEnabledToggled(true)));
    assert!(!app.ocr_enabled);
    assert_eq!(
        app.save_error.as_deref(),
        Some("启用视觉理解前请选择有效的 provider/model 多模态模型")
    );

    for invalid in ["/model", "provider/", "provider"] {
        app.ocr_enabled = false;
        let _ = update(
            &mut app,
            SettingsMessage::Multimodal(MultimodalMessage::OcrModelChanged(invalid.to_string())),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Multimodal(MultimodalMessage::OcrEnabledToggled(true)),
        );
        assert!(!app.ocr_enabled, "{invalid} must be rejected");
        let config = multimodal_config_from_state(&app)
            .expect("disabled OCR may keep an incomplete draft");
        assert!(!config.ocr.enabled);
        assert!(config.ocr.model.is_none());
    }

    let _ = update(
        &mut app,
        SettingsMessage::Multimodal(MultimodalMessage::OcrModelChanged(
            " vision-provider/vision-model ".to_string(),
        )),
    );
    let _ =
        update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::OcrEnabledToggled(true)));

    assert!(app.ocr_enabled);
    assert!(app.save_error.is_none());
    let config =
        multimodal_config_from_state(&app).expect("valid OCR settings map");
    assert!(config.ocr.enabled);
    assert_eq!(config.ocr.model.as_deref(), Some("vision-provider/vision-model"));
}
#[test]
fn model_supports_images_defaults_on_and_allows_ocr_without_fallback_model() {
    let mut app = app();
    assert!(app.model_supports_images);

    let _ =
        update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::OcrEnabledToggled(true)));

    assert!(app.ocr_enabled);
    let config = multimodal_config_from_state(&app)
        .expect("model vision mode should not require an OCR model");
    assert!(config.ocr.model_supports_images);
    assert!(config.ocr.model.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Multimodal(MultimodalMessage::ModelSupportsImagesToggled(false)),
    );
    assert!(app.model_supports_images);
    assert!(app.save_error.is_some());

    let _ = update(
        &mut app,
        SettingsMessage::Multimodal(MultimodalMessage::OcrModelChanged(
            "provider/model".to_string(),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Multimodal(MultimodalMessage::ModelSupportsImagesToggled(false)),
    );
    assert!(!app.model_supports_images);
    assert!(
        !multimodal_config_from_state(&app)
            .expect("fallback OCR model should make configuration valid")
            .ocr
            .model_supports_images
    );
}
#[test]
fn selecting_empty_image_model_clears_the_configured_model() {
    let mut app = app();
    app.ocr_model = "provider/model".to_string();

    let _ = update(
        &mut app,
        SettingsMessage::Multimodal(MultimodalMessage::OcrModelChanged(String::new())),
    );

    assert!(app.ocr_model.is_empty());
    assert!(
        multimodal_config_from_state(&app)
            .expect("empty image model should be valid when the active model supports images")
            .ocr
            .model
            .is_none()
    );
}
#[test]
fn max_images_changed_clamps_out_of_range_and_preserves_in_range_values() {
    let mut app = app();
    app.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImagesChanged(0)));
    assert_eq!(app.max_images, 1);
    assert!(app.save_error.is_none());

    let _ = update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImagesChanged(8)));
    assert_eq!(app.max_images, 8);

    let _ = update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImagesChanged(99)));
    assert_eq!(app.max_images, 16);
}
#[test]
fn max_image_size_changed_clamps_out_of_range_and_preserves_in_range_values() {
    let mut app = app();
    app.save_error = Some("old".to_string());

    let _ =
        update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImageSizeMbChanged(0)));
    assert_eq!(app.max_image_size_mb, 1);
    assert!(app.save_error.is_none());

    let _ =
        update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImageSizeMbChanged(10)));
    assert_eq!(app.max_image_size_mb, 10);

    let _ =
        update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImageSizeMbChanged(99)));
    assert_eq!(app.max_image_size_mb, 20);
}
#[test]
fn allow_remote_fetch_toggled_sets_both_boolean_values_and_clears_save_error() {
    let mut app = app();
    app.allow_remote_fetch = false;
    app.save_error = Some("old".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Multimodal(MultimodalMessage::AllowRemoteFetchToggled(true)),
    );
    assert!(app.allow_remote_fetch);
    assert!(app.save_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Multimodal(MultimodalMessage::AllowRemoteFetchToggled(false)),
    );
    assert!(!app.allow_remote_fetch);
}
