use serde_json::json;
pub struct CloudFixtureProvider {
    pub invalid_assets: bool,
}

#[async_trait::async_trait]
impl model_types::Provider for CloudFixtureProvider {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("unexpected transport")
    }
    async fn chat(
        &self,
        request: model_types::ChatRequest<'_>,
        _: &str,
        _: f64,
    ) -> anyhow::Result<model_types::ChatResponse> {
        let prompt = &request.messages[0].content;
        let text = if prompt.contains("Return only a short brief") {
            "brief".to_string()
        } else if prompt.contains("keyword visual segments") {
            json!([{"anchor":"A","start":0,"end":1,"visual_type":"keyword",
                "assets":if self.invalid_assets {json!([])} else {json!(["title"])} }])
            .to_string()
        } else {
            json!([
                {"anchor":"B","start":1,"end":2,"visual_type":"b_roll","assets":["footage"]},
                {"anchor":"C","start":2,"end":3,"visual_type":"motion_graphic","assets":["diagram"]}
            ])
            .to_string()
        };
        Ok(model_types::ChatResponse {
            text: Some(text),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })
    }
}
