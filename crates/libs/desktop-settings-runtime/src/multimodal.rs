use crate::state::*;
use crate::messages::*;
use config_tools::{MultimodalConfig, MultimodalOcrConfig};
const INVALID_OCR_MODEL_ERROR: &str = "启用视觉理解前请选择有效的 provider/model 多模态模型";

fn valid_ocr_model(model: &str) -> Option<&str> {
    let model = model.trim();
    let (provider, model_id) = model.split_once('/')?;
    (!provider.trim().is_empty() && !model_id.trim().is_empty()).then_some(model)
}

fn multimodal_config_from_state(
    state: &MultimodalSettingsState,
) -> Result<MultimodalConfig, String> {
    let ocr_model = valid_ocr_model(&state.ocr_model).map(str::to_string);
    if state.ocr_enabled && !state.model_supports_images && ocr_model.is_none() {
        return Err(INVALID_OCR_MODEL_ERROR.to_string());
    }

    Ok(MultimodalConfig {
        max_images: state.max_images.clamp(1, 16) as usize,
        max_image_size_mb: state.max_image_size_mb.clamp(1, 20) as usize,
        allow_remote_fetch: state.allow_remote_fetch,
        ocr: MultimodalOcrConfig {
            enabled: state.ocr_enabled,
            model_supports_images: state.model_supports_images,
            model: ocr_model,
        },
    })
}

fn persist_multimodal_settings(state: &mut MultimodalSettingsState) -> Option<MultimodalConfig> {
    let Ok(config) = multimodal_config_from_state(&state) else {
        return None;
    };

    Some(config)
}


/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回需要宿主持久化的配置；仅本地更新或校验失败时返回 None。
pub fn update(state: &mut MultimodalSettingsState, message: SettingsMessage) -> Option<MultimodalConfig> {
    let SettingsMessage::Multimodal(message) = message else {
        return None;
    };

    match message {
        MultimodalMessage::MaxImagesChanged(value) => {
            state.max_images = value.clamp(1, 16)
        }
        MultimodalMessage::MaxImageSizeMbChanged(value) => {
            state.max_image_size_mb = value.clamp(1, 20)
        }
        MultimodalMessage::AllowRemoteFetchToggled(value) => {
            state.allow_remote_fetch = value
        }
        MultimodalMessage::ModelSupportsImagesToggled(value) => {
            if !value
                && state.ocr_enabled
                && valid_ocr_model(&state.ocr_model).is_none()
            {
                state.save_error = Some(INVALID_OCR_MODEL_ERROR.to_string());
                return None;
            }
            state.model_supports_images = value
        }
        MultimodalMessage::OcrEnabledToggled(true) => {
            if !state.model_supports_images
                && valid_ocr_model(&state.ocr_model).is_none()
            {
                state.ocr_enabled = false;
                state.save_error = Some(INVALID_OCR_MODEL_ERROR.to_string());
                return None;
            }
            state.ocr_enabled = true;
        }
        MultimodalMessage::OcrEnabledToggled(false) => {
            state.ocr_enabled = false;
        }
        MultimodalMessage::OcrModelChanged(value) => {
            state.ocr_model = value;
        }
    }

    state.save_error = None;
    persist_multimodal_settings(state)
}
#[cfg(test)]
#[path = "multimodal_tests.rs"]
mod multimodal_tests;
