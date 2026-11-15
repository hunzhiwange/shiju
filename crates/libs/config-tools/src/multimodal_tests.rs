// 多模态处理设置资源上限，并在未配置 OCR 时优先复用模型视觉能力。
#[test]
fn multimodal_config_defaults_and_effective_limits_are_safe() {
    let config = super::MultimodalConfig::default();
    assert_eq!(config.max_images, 10);
    assert_eq!(config.max_image_size_mb, 5);
    assert!(config.allow_remote_fetch);
    assert_eq!(config.effective_limits(), (10, 5));

    let parsed: super::MultimodalConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(parsed.max_images, 10);
    assert_eq!(parsed.max_image_size_mb, 5);
    assert!(parsed.allow_remote_fetch);

    let too_small = super::MultimodalConfig {
        max_images: 0,
        max_image_size_mb: 0,
        allow_remote_fetch: false,
        ocr: super::MultimodalOcrConfig::default(),
    };
    assert_eq!(too_small.effective_limits(), (1, 1));

    let too_large = super::MultimodalConfig {
        max_images: 99,
        max_image_size_mb: 99,
        allow_remote_fetch: true,
        ocr: super::MultimodalOcrConfig::default(),
    };
    assert_eq!(too_large.effective_limits(), (16, 20));
}

#[test]
fn multimodal_ocr_config_defaults_to_model_vision_and_round_trips_model_selection() {
    let config = super::MultimodalConfig::default();
    assert!(!config.ocr.enabled);
    assert!(config.ocr.model_supports_images);
    assert_eq!(config.ocr.model, None);

    let parsed: super::MultimodalConfig = serde_json::from_str("{}").unwrap();
    assert!(!parsed.ocr.enabled);
    assert!(parsed.ocr.model_supports_images);
    assert_eq!(parsed.ocr.model, None);

    let selected: super::MultimodalConfig = serde_json::from_value(serde_json::json!({
        "ocr": {
            "enabled": true,
            "model_supports_images": false,
            "model": "openai/gpt-4.1-mini"
        }
    }))
    .unwrap();
    assert!(selected.ocr.enabled);
    assert!(!selected.ocr.model_supports_images);
    assert_eq!(selected.ocr.model.as_deref(), Some("openai/gpt-4.1-mini"));

    let round_tripped: super::MultimodalConfig =
        serde_json::from_value(serde_json::to_value(selected).unwrap()).unwrap();
    assert!(round_tripped.ocr.enabled);
    assert!(!round_tripped.ocr.model_supports_images);
    assert_eq!(round_tripped.ocr.model.as_deref(), Some("openai/gpt-4.1-mini"));
}
