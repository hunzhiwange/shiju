use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::core::capabilities::{ImageInputSupport, TextInputSupport};
use crate::core::image_model::{ImageModel, ImageModelOptions, ImageModelResponse};
use crate::core::messages::UserImage;
use crate::error::{Error, Result};

/// 图片模型请求。
#[derive(Debug, Clone)]
pub struct ImageModelRequest<M: ImageModel> {
    /// 当前请求使用的图片模型。
    pub model: M,
    /// 请求选项。
    pub(crate) options: ImageModelOptions,
}

impl<M: ImageModel> ImageModelRequest<M> {
    /// 创建请求 builder。
    pub fn builder() -> ImageModelRequestBuilder<M> {
        ImageModelRequestBuilder::default()
    }

    /// 发起图片生成或编辑请求。
    pub async fn generate_image(&self) -> Result<ImageModelResponse> {
        if self.options.prompt.is_none() && self.options.files.is_empty() {
            return Err(Error::InvalidInput(
                "image request requires a prompt or at least one input image".to_string(),
            ));
        }

        self.model.generate_image(self.options.clone()).await
    }
}

impl<M: ImageModel> Deref for ImageModelRequest<M> {
    type Target = ImageModelOptions;

    fn deref(&self) -> &Self::Target {
        &self.options
    }
}

impl<M: ImageModel> DerefMut for ImageModelRequest<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.options
    }
}

/// 必须先设置模型的 builder 阶段。
pub struct ModelStage {}

/// 设置可选参数的 builder 阶段。
pub struct OptionsStage {}

/// `ImageModelRequest` 的 type-state builder。
pub struct ImageModelRequestBuilder<M: ImageModel, State = ModelStage> {
    model: Option<M>,
    options: ImageModelOptions,
    state: std::marker::PhantomData<State>,
}

impl<M: ImageModel, State> Deref for ImageModelRequestBuilder<M, State> {
    type Target = ImageModelOptions;

    fn deref(&self) -> &Self::Target {
        &self.options
    }
}

impl<M: ImageModel, State> DerefMut for ImageModelRequestBuilder<M, State> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.options
    }
}

impl<M: ImageModel> ImageModelRequestBuilder<M> {
    fn default() -> Self {
        Self {
            model: None,
            options: ImageModelOptions::builder()
                .prompt(None)
                .files(vec![])
                .mask(None)
                .n(None)
                .size(None)
                .aspect_ratio(None)
                .seed(None)
                .headers(None)
                .body(None)
                .build()
                .unwrap(),
            state: std::marker::PhantomData,
        }
    }
}

impl<M: ImageModel> ImageModelRequestBuilder<M, ModelStage> {
    /// 设置请求使用的图片模型。
    pub fn model(self, model: M) -> ImageModelRequestBuilder<M, OptionsStage> {
        ImageModelRequestBuilder {
            model: Some(model),
            options: self.options,
            state: std::marker::PhantomData,
        }
    }
}

impl<M: ImageModel> ImageModelRequestBuilder<M, OptionsStage> {
    /// 设置提示词。
    pub fn prompt(self, prompt: impl Into<String>) -> Self
    where
        M: TextInputSupport,
    {
        let mut this = self;
        this.options.prompt = Some(prompt.into());
        this
    }

    /// 追加一张输入图片。
    pub fn file(self, file: UserImage) -> Self
    where
        M: ImageInputSupport,
    {
        let mut this = self;
        this.options.files.push(file);
        this
    }

    /// 设置输入图片列表。
    pub fn files(self, files: impl Into<Vec<UserImage>>) -> Self
    where
        M: ImageInputSupport,
    {
        let mut this = self;
        this.options.files = files.into();
        this
    }

    /// 设置遮罩图。
    pub fn mask(self, mask: UserImage) -> Self
    where
        M: ImageInputSupport,
    {
        let mut this = self;
        this.options.mask = Some(mask);
        this
    }

    /// 设置输出图片数量。
    pub fn n(mut self, n: usize) -> Self {
        self.options.n = Some(n);
        self
    }

    /// 设置输出尺寸。
    pub fn size(mut self, size: impl Into<String>) -> Self {
        self.options.size = Some(size.into());
        self
    }

    /// 设置输出纵横比。
    pub fn aspect_ratio(mut self, aspect_ratio: impl Into<String>) -> Self {
        self.options.aspect_ratio = Some(aspect_ratio.into());
        self
    }

    /// 设置随机种子。
    pub fn seed(mut self, seed: impl Into<u32>) -> Self {
        self.options.seed = Some(seed.into());
        self
    }

    /// 设置请求级自定义 headers。
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.options.headers = Some(headers);
        self
    }

    /// 设置请求级 body 覆盖字段。
    pub fn body(mut self, body: serde_json::Value) -> Self {
        if let serde_json::Value::Object(map) = body {
            self.options.body = Some(map);
        }
        self
    }

    /// 构建最终请求。
    pub fn build(self) -> ImageModelRequest<M> {
        let model = self
            .model
            .unwrap_or_else(|| unreachable!("model must be set before build"));

        ImageModelRequest {
            model,
            options: self.options,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::image_model::ImageModelUsage;
    use async_trait::async_trait;

    #[derive(Debug, Clone)]
    struct TestImageModel;

    impl TextInputSupport for TestImageModel {}
    impl ImageInputSupport for TestImageModel {}

    #[async_trait]
    impl ImageModel for TestImageModel {
        async fn generate_image(&self, _input: ImageModelOptions) -> Result<ImageModelResponse> {
            Ok(ImageModelResponse {
                images: vec!["ZmFrZS1pbWFnZQ==".to_string()],
                usage: Some(ImageModelUsage {
                    input_tokens: Some(12),
                    output_tokens: Some(34),
                    total_tokens: Some(46),
                }),
                warnings: vec![],
            })
        }
    }

    #[test]
    fn test_image_request_builder_accepts_body_and_files() {
        let request = ImageModelRequest::builder()
            .model(TestImageModel)
            .prompt("paint a lighthouse")
            .file(UserImage::new("https://example.com/input.png"))
            .aspect_ratio("16:9")
            .body(serde_json::json!({
                "quality": "high"
            }))
            .build();

        assert_eq!(
            request.options.prompt.as_deref(),
            Some("paint a lighthouse")
        );
        assert_eq!(request.options.files.len(), 1);
        assert_eq!(request.options.aspect_ratio.as_deref(), Some("16:9"));
        assert_eq!(
            request.options.body,
            Some(
                serde_json::json!({
                    "quality": "high"
                })
                .as_object()
                .expect("body should be an object")
                .clone()
            )
        );
    }

    #[tokio::test]
    async fn test_image_request_requires_prompt_or_input_files() {
        let error = ImageModelRequest::builder()
            .model(TestImageModel)
            .build()
            .generate_image()
            .await
            .expect_err("request without prompt or files should fail");

        assert_eq!(
            error,
            Error::InvalidInput(
                "image request requires a prompt or at least one input image".to_string()
            )
        );
    }
}
