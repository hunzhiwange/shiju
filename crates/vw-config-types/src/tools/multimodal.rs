use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 多模态 OCR 图片模型配置（`[multimodal.ocr]` 配置段）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MultimodalOcrConfig {
    /// 是否启用图片 OCR。默认关闭。
    #[serde(default)]
    pub enabled: bool,
    /// 当前模型是否支持直接接收图片。默认开启；开启时优先由当前模型处理图片。
    #[serde(default = "default_model_supports_images")]
    pub model_supports_images: bool,
    /// OCR 图片模型的完整 `provider/model` 引用。
    #[serde(default)]
    pub model: Option<String>,
}

fn default_model_supports_images() -> bool {
    true
}

impl Default for MultimodalOcrConfig {
    fn default() -> Self {
        Self { enabled: false, model_supports_images: default_model_supports_images(), model: None }
    }
}

/// 多模态图片处理配置（`[multimodal]` 配置段）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MultimodalConfig {
    /// 每次请求允许携带的图片附件数量上限。
    #[serde(default = "default_multimodal_max_images")]
    pub max_images: usize,
    /// 在 base64 编码前允许的单张图片大小上限，单位为 MiB。
    #[serde(default = "default_multimodal_max_image_size_mb")]
    pub max_image_size_mb: usize,
    /// 是否允许抓取远程图片 URL（http/https）。默认开启。
    #[serde(default = "default_allow_remote_fetch")]
    pub allow_remote_fetch: bool,
    /// 图片 OCR 降级配置。
    #[serde(default)]
    pub ocr: MultimodalOcrConfig,
}

fn default_multimodal_max_images() -> usize {
    10
}

fn default_multimodal_max_image_size_mb() -> usize {
    5
}

fn default_allow_remote_fetch() -> bool {
    true
}

impl MultimodalConfig {
    /// 将配置值裁剪到运行时允许的安全范围内。
    pub fn effective_limits(&self) -> (usize, usize) {
        let max_images = self.max_images.clamp(1, 16);
        let max_image_size_mb = self.max_image_size_mb.clamp(1, 20);
        (max_images, max_image_size_mb)
    }
}

impl Default for MultimodalConfig {
    fn default() -> Self {
        Self {
            max_images: default_multimodal_max_images(),
            max_image_size_mb: default_multimodal_max_image_size_mb(),
            allow_remote_fetch: default_allow_remote_fetch(),
            ocr: MultimodalOcrConfig::default(),
        }
    }
}
