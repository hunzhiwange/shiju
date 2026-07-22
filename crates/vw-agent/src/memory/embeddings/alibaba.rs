use super::EmbeddingProvider;
use async_trait::async_trait;

/// 阿里 DashScope OpenAI 兼容嵌入提供者。
///
/// 目前用于 `alibaba` 与 `alibaba-cn` 两个区域化 provider id。
pub struct AlibabaEmbedding {
    provider_name: &'static str,
    base_url: String,
    api_key: String,
    model: String,
    dims: usize,
}

impl AlibabaEmbedding {
    pub fn new(
        provider_name: &'static str,
        base_url: &str,
        api_key: &str,
        model: &str,
        dims: usize,
    ) -> Self {
        Self {
            provider_name,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            dims,
        }
    }

    pub fn embeddings_url(&self) -> String {
        format!("{}/embeddings", self.base_url)
    }

    fn http_client(&self) -> reqwest::Client {
        crate::app::agent::config::build_runtime_proxy_client("memory.embeddings.alibaba")
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl EmbeddingProvider for AlibabaEmbedding {
    fn name(&self) -> &str {
        self.provider_name
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let body = serde_json::json!({
            "model": self.model,
            "input": texts,
            "dimensions": self.dims,
            "encoding_format": "float",
        });

        let resp = self
            .http_client()
            .post(self.embeddings_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Alibaba embedding API error {status}: {text}");
        }

        let json: serde_json::Value = resp.json().await?;
        let data = json
            .get("data")
            .and_then(|data| data.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid Alibaba embedding response: missing 'data'"))?;

        let mut embeddings = Vec::with_capacity(data.len());
        for item in data {
            let embedding = item
                .get("embedding")
                .and_then(|embedding| embedding.as_array())
                .ok_or_else(|| anyhow::anyhow!("Invalid Alibaba embedding item"))?;

            #[allow(clippy::cast_possible_truncation)]
            let vec: Vec<f32> =
                embedding.iter().filter_map(|value| value.as_f64().map(|f| f as f32)).collect();
            embeddings.push(vec);
        }

        Ok(embeddings)
    }
}

#[cfg(test)]
#[path = "alibaba_tests.rs"]
mod alibaba_tests;
