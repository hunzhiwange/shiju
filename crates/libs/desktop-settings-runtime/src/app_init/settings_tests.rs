#[test]
fn build_multimodal_settings_loads_ocr_config() {
    let mut config = config_aggregate::Config::default();
    config.multimodal.max_images = 8;
    config.multimodal.max_image_size_mb = 10;
    config.multimodal.allow_remote_fetch = true;
    config.multimodal.ocr.enabled = true;
    config.multimodal.ocr.model_supports_images = false;
    config.multimodal.ocr.model = Some(" vision-provider/vision-model ".to_string());

    let settings = super::build_multimodal_settings(&config);

    assert_eq!(settings.max_images, 8);
    assert_eq!(settings.max_image_size_mb, 10);
    assert!(settings.allow_remote_fetch);
    assert!(settings.ocr_enabled);
    assert!(!settings.model_supports_images);
    assert_eq!(settings.ocr_model, "vision-provider/vision-model");
    assert!(settings.save_error.is_none());
}
