//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::{App, Message, update_multimodal_config_async};
use iced::Task;
use vw_config_types::tools::{MultimodalConfig, MultimodalOcrConfig};

use super::messages::{MultimodalMessage, SettingsMessage};

const INVALID_OCR_MODEL_ERROR: &str = "启用图片 OCR 前请选择有效的 provider/model 模型";

fn valid_ocr_model(model: &str) -> Option<&str> {
    let model = model.trim();
    let (provider, model_id) = model.split_once('/')?;
    (!provider.trim().is_empty() && !model_id.trim().is_empty()).then_some(model)
}

fn multimodal_config_from_state(
    state: &crate::app::state::MultimodalSettingsState,
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

fn persist_multimodal_settings(app: &mut App) -> Task<Message> {
    let Ok(config) = multimodal_config_from_state(&app.multimodal_settings) else {
        return Task::none();
    };

    update_multimodal_config_async(move |multimodal| {
        *multimodal = config;
    })
}

#[cfg(test)]
#[path = "multimodal_tests.rs"]
mod multimodal_tests;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Multimodal(message) = message else {
        return Task::none();
    };

    match message {
        MultimodalMessage::MaxImagesChanged(value) => {
            app.multimodal_settings.max_images = value.clamp(1, 16)
        }
        MultimodalMessage::MaxImageSizeMbChanged(value) => {
            app.multimodal_settings.max_image_size_mb = value.clamp(1, 20)
        }
        MultimodalMessage::AllowRemoteFetchToggled(value) => {
            app.multimodal_settings.allow_remote_fetch = value
        }
        MultimodalMessage::ModelSupportsImagesToggled(value) => {
            if !value
                && app.multimodal_settings.ocr_enabled
                && valid_ocr_model(&app.multimodal_settings.ocr_model).is_none()
            {
                app.multimodal_settings.save_error = Some(INVALID_OCR_MODEL_ERROR.to_string());
                return Task::none();
            }
            app.multimodal_settings.model_supports_images = value
        }
        MultimodalMessage::OcrEnabledToggled(true) => {
            if !app.multimodal_settings.model_supports_images
                && valid_ocr_model(&app.multimodal_settings.ocr_model).is_none()
            {
                app.multimodal_settings.ocr_enabled = false;
                app.multimodal_settings.save_error = Some(INVALID_OCR_MODEL_ERROR.to_string());
                return Task::none();
            }
            app.multimodal_settings.ocr_enabled = true;
        }
        MultimodalMessage::OcrEnabledToggled(false) => {
            app.multimodal_settings.ocr_enabled = false;
        }
        MultimodalMessage::OcrModelChanged(value) => {
            app.multimodal_settings.ocr_model = value;
        }
    }

    app.multimodal_settings.save_error = None;
    persist_multimodal_settings(app)
}
