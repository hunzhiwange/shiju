//! 图片模型抽象。
//!
//! 这个模块提供与 `EmbeddingModel` / `LanguageModel` 平行的公开图片能力，
//! 用来承载文生图与图编辑请求。
//!
//! 设计目标：
//! - 对外暴露统一的 `ImageModel` trait 与 `ImageModelRequest` builder。
//! - 复用现有的 `UserImage` 作为输入图片描述，避免额外的重复公开类型。
//! - 保留 `headers` / `body` 扩展口，允许调用方注入 provider-specific 字段。

/// 图片请求 builder 与相关类型。
#[cfg(feature = "image-model-request")]
pub mod request;

use crate::core::messages::UserImage;
use crate::error::Result;
use async_trait::async_trait;
use derive_builder::Builder;
#[cfg(feature = "image-model-request")]
pub use request::ImageModelRequest;
use std::collections::HashMap;

/// 图片模型请求选项。
#[derive(Debug, Clone, Builder)]
pub struct ImageModelOptions {
    /// 用于指导图片生成或编辑的文本提示词。
    pub prompt: Option<String>,
    /// 输入图片列表。
    ///
    /// 当为空时，通常表示纯文生图；当存在至少一张输入图片时，通常表示图编辑。
    pub files: Vec<UserImage>,
    /// 可选的编辑遮罩图。
    pub mask: Option<UserImage>,
    /// 期望生成的图片数量。
    pub n: Option<usize>,
    /// 输出尺寸。
    ///
    /// 这是一个 provider-agnostic 的自由字符串字段，例如 `1024x1024`。
    pub size: Option<String>,
    /// 输出纵横比，例如 `16:9`。
    pub aspect_ratio: Option<String>,
    /// 随机种子。
    pub seed: Option<u32>,
    /// 自定义 HTTP 头。
    pub headers: Option<HashMap<String, String>>,
    /// 额外注入到 provider 请求体顶层的字段。
    pub body: Option<serde_json::Map<String, serde_json::Value>>,
}

impl ImageModelOptions {
    /// 创建 `ImageModelOptions` builder。
    pub fn builder() -> ImageModelOptionsBuilder {
        ImageModelOptionsBuilder::default()
    }
}

/// 图片模型统一 trait。
#[async_trait]
pub trait ImageModel: Clone + Send + Sync + std::fmt::Debug + 'static {
    /// 执行图片生成或编辑请求。
    // async_trait adds #[must_use] to a future that is already must-use.
    #[allow(clippy::double_must_use)]
    async fn generate_image(&self, input: ImageModelOptions) -> Result<ImageModelResponse>;
}

/// 图片请求中的非致命告警。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageModelWarning {
    /// 当前模型或 provider 不支持某个通用字段。
    Unsupported {
        /// 不支持的字段名称。
        feature: &'static str,
        /// 额外说明，例如替代字段建议。
        details: Option<String>,
    },
}

/// 图片请求的 token 使用量。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageModelUsage {
    /// 输入 token 数。
    pub input_tokens: Option<usize>,
    /// 输出 token 数。
    pub output_tokens: Option<usize>,
    /// 总 token 数。
    pub total_tokens: Option<usize>,
}

/// 图片请求响应。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageModelResponse {
    /// 生成出的图片，统一以 base64 编码字符串表示。
    pub images: Vec<String>,
    /// 请求使用量。
    pub usage: Option<ImageModelUsage>,
    /// 请求中的非致命告警。
    pub warnings: Vec<ImageModelWarning>,
}

impl ImageModelResponse {
    /// 以切片形式访问所有图片。
    pub fn images(&self) -> &[String] {
        &self.images
    }

    /// 访问第一张图片。
    pub fn image(&self) -> Option<&String> {
        self.images.first()
    }

    /// 消费响应并返回图片列表。
    pub fn into_images(self) -> Vec<String> {
        self.images
    }

    /// 返回使用量信息。
    pub fn usage(&self) -> Option<&ImageModelUsage> {
        self.usage.as_ref()
    }

    /// 返回告警列表。
    pub fn warnings(&self) -> &[ImageModelWarning] {
        &self.warnings
    }
}
