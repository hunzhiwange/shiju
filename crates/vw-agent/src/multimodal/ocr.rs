//! 会话模型的图片分析适配器。

pub use agent_multimodal::ocr::*;
use agent_multimodal::analysis::{
    RawImageAnalysis, analysis_system_prompt, analysis_user_prompt_with_image,
    parse_analysis_model_output,
};
use crate::app::agent::provider::provider as model_catalog;
use crate::app::agent::providers::session_llm::SessionLlmProvider;
use model_types::Provider;
use async_trait::async_trait;

/// 使用当前会话 Gateway 和模型目录的生产 OCR 适配器。
pub struct SessionImageTextExtractor {
    provider: SessionLlmProvider,
}

impl SessionImageTextExtractor {
    /// 创建声明图片输入能力的会话模型适配器。
    pub fn new() -> Self {
        Self { provider: SessionLlmProvider::with_vision(true) }
    }
}

impl Default for SessionImageTextExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ImageTextExtractor for SessionImageTextExtractor {
    async fn supports_image_input(&self, model_ref: &str) -> anyhow::Result<bool> {
        let parsed = model_catalog::parse_model(model_ref);
        let model = model_catalog::get_model(&parsed.provider_id, &parsed.model_id).await?;
        Ok(model.capabilities.input.image)
    }

    async fn accepts_chat_images(&self, model_ref: &str) -> anyhow::Result<bool> {
        let parsed = model_catalog::parse_model(model_ref);
        let model = model_catalog::get_model(&parsed.provider_id, &parsed.model_id).await?;
        if let Some(declared) = &model.capabilities.catalog_execution {
            anyhow::ensure!(
                declared.input.as_ref().is_none_or(|input| input.image),
                "model {model_ref} does not support image input"
            );
            return Ok(true);
        }
        Ok(model.capabilities.input.image)
    }

    async fn extract_text(&self, model_ref: &str, image_data_uri: &str) -> anyhow::Result<String> {
        let message = analysis_user_prompt_with_image(image_data_uri);
        self.provider
            .chat_with_system(Some(analysis_system_prompt()), &message, model_ref, 0.0)
            .await
    }

    async fn analyze_image(
        &self,
        model_ref: &str,
        image_data_uri: &str,
    ) -> anyhow::Result<RawImageAnalysis> {
        let output = self.extract_text(model_ref, image_data_uri).await?;
        Ok(parse_analysis_model_output(&output))
    }
}
