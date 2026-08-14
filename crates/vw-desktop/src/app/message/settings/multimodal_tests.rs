use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn ocr_settings_require_valid_model_before_enable_and_map_to_config() {
    let mut app = app();
    app.multimodal_settings.ocr_enabled = false;
    app.multimodal_settings.model_supports_images = false;
    app.multimodal_settings.ocr_model.clear();

    let _ =
        update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::OcrEnabledToggled(true)));
    assert!(!app.multimodal_settings.ocr_enabled);
    assert_eq!(
        app.multimodal_settings.save_error.as_deref(),
        Some("启用图片 OCR 前请选择有效的 provider/model 模型")
    );

    for invalid in ["/model", "provider/", "provider"] {
        app.multimodal_settings.ocr_enabled = false;
        let _ = update(
            &mut app,
            SettingsMessage::Multimodal(MultimodalMessage::OcrModelChanged(invalid.to_string())),
        );
        let _ = update(
            &mut app,
            SettingsMessage::Multimodal(MultimodalMessage::OcrEnabledToggled(true)),
        );
        assert!(!app.multimodal_settings.ocr_enabled, "{invalid} must be rejected");
        let config = multimodal_config_from_state(&app.multimodal_settings)
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

    assert!(app.multimodal_settings.ocr_enabled);
    assert!(app.multimodal_settings.save_error.is_none());
    let config =
        multimodal_config_from_state(&app.multimodal_settings).expect("valid OCR settings map");
    assert!(config.ocr.enabled);
    assert_eq!(config.ocr.model.as_deref(), Some("vision-provider/vision-model"));
}

#[test]
fn model_supports_images_defaults_on_and_allows_ocr_without_fallback_model() {
    let mut app = app();
    assert!(app.multimodal_settings.model_supports_images);

    let _ =
        update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::OcrEnabledToggled(true)));

    assert!(app.multimodal_settings.ocr_enabled);
    let config = multimodal_config_from_state(&app.multimodal_settings)
        .expect("model vision mode should not require an OCR model");
    assert!(config.ocr.model_supports_images);
    assert!(config.ocr.model.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Multimodal(MultimodalMessage::ModelSupportsImagesToggled(false)),
    );
    assert!(app.multimodal_settings.model_supports_images);
    assert!(app.multimodal_settings.save_error.is_some());

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
    assert!(!app.multimodal_settings.model_supports_images);
    assert!(
        !multimodal_config_from_state(&app.multimodal_settings)
            .expect("fallback OCR model should make configuration valid")
            .ocr
            .model_supports_images
    );
}

#[test]
fn max_images_changed_clamps_out_of_range_and_preserves_in_range_values() {
    let mut app = app();
    app.multimodal_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImagesChanged(0)));
    assert_eq!(app.multimodal_settings.max_images, 1);
    assert!(app.multimodal_settings.save_error.is_none());

    let _ = update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImagesChanged(8)));
    assert_eq!(app.multimodal_settings.max_images, 8);

    let _ = update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImagesChanged(99)));
    assert_eq!(app.multimodal_settings.max_images, 16);
}

#[test]
fn max_image_size_changed_clamps_out_of_range_and_preserves_in_range_values() {
    let mut app = app();
    app.multimodal_settings.save_error = Some("old".to_string());

    let _ =
        update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImageSizeMbChanged(0)));
    assert_eq!(app.multimodal_settings.max_image_size_mb, 1);
    assert!(app.multimodal_settings.save_error.is_none());

    let _ =
        update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImageSizeMbChanged(10)));
    assert_eq!(app.multimodal_settings.max_image_size_mb, 10);

    let _ =
        update(&mut app, SettingsMessage::Multimodal(MultimodalMessage::MaxImageSizeMbChanged(99)));
    assert_eq!(app.multimodal_settings.max_image_size_mb, 20);
}

#[test]
fn allow_remote_fetch_toggled_sets_both_boolean_values_and_clears_save_error() {
    let mut app = app();
    app.multimodal_settings.allow_remote_fetch = false;
    app.multimodal_settings.save_error = Some("old".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Multimodal(MultimodalMessage::AllowRemoteFetchToggled(true)),
    );
    assert!(app.multimodal_settings.allow_remote_fetch);
    assert!(app.multimodal_settings.save_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Multimodal(MultimodalMessage::AllowRemoteFetchToggled(false)),
    );
    assert!(!app.multimodal_settings.allow_remote_fetch);
}

#[test]
fn update_ignores_messages_for_other_settings_sections() {
    let mut app = app();
    app.multimodal_settings.max_images = 7;
    app.multimodal_settings.max_image_size_mb = 12;
    app.multimodal_settings.allow_remote_fetch = true;
    app.multimodal_settings.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::SchedulerHelpOpen);

    assert_eq!(app.multimodal_settings.max_images, 7);
    assert_eq!(app.multimodal_settings.max_image_size_mb, 12);
    assert!(app.multimodal_settings.allow_remote_fetch);
    assert_eq!(app.multimodal_settings.save_error.as_deref(), Some("keep"));
}

#[test]
fn multimodal_changes_schedule_persistence_tasks() {
    let mut app = app();

    for message in [
        MultimodalMessage::MaxImagesChanged(4),
        MultimodalMessage::MaxImageSizeMbChanged(8),
        MultimodalMessage::AllowRemoteFetchToggled(true),
        MultimodalMessage::ModelSupportsImagesToggled(true),
        MultimodalMessage::OcrModelChanged("provider/model".to_string()),
        MultimodalMessage::OcrEnabledToggled(true),
    ] {
        let task = update(&mut app, SettingsMessage::Multimodal(message));
        assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    }
}
