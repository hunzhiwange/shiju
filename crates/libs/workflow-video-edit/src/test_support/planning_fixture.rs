use crate::{planning::*, segments::*, transcript::*};
pub fn source() -> (VideoTranscript, Vec<VideoSegment>, VideoPlanRequirements) {
    let t = VideoTranscript {
        asset_id: "asset".into(),
        duration_sec: 360.0,
        fps: 25.0,
        text: "same ".repeat(6),
        words: (0..6)
            .map(|i| VideoTranscriptWord {
                word_index: i,
                text: "same".into(),
                start: i as f64 * 60.0,
                end: i as f64 * 60.0 + 1.0,
            })
            .collect(),
        source: "fixture".into(),
        diagnostics: vec![],
    };
    let segments = segment_video_transcript(
        &t,
        VideoSegmentOptions { target_sec: 60.0, max_sec: 60.0, context_sec: 5.0 },
    )
    .unwrap();
    (
        t,
        segments,
        VideoPlanRequirements {
            instructions: "test".into(),
            required_types: vec![VisualType::Keyword, VisualType::BRoll, VisualType::MotionGraphic],
            max_effects_per_word: 1,
        },
    )
}
pub fn effect(i: u64, visual_type: VisualType) -> VideoEditEffect {
    let (_, s, _) = source();
    VideoEditEffect {
        id: format!("effect-{i}"),
        chunk_id: s[i as usize].chunk_id.clone(),
        word_index: i,
        end_word_index: i,
        anchor: "same".into(),
        start: i as f64 * 60.0,
        end: i as f64 * 60.0 + 1.0,
        visual_type,
        assets: vec!["make a visual".into()],
    }
}

use model_types::{ChatRequest, ChatResponse, Provider};
use tokio::sync::{mpsc, oneshot};
/// Only external model transport is replaced; production graph and merger run unchanged.
pub struct Endpoint(pub String);
#[async_trait::async_trait]
impl Provider for Endpoint {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        unreachable!()
    }
    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        _: f64,
    ) -> anyhow::Result<ChatResponse> {
        assert_eq!(model, "editable-model");
        let prompt = &request.messages.last().unwrap().content;
        let response = reqwest::Client::new()
            .post(&self.0)
            .body(prompt.clone())
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(ChatResponse {
            text: Some(response),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })
    }
}
pub type Entry = (serde_json::Value, oneshot::Sender<Option<String>>);
pub async fn entered(rx: &mut mpsc::UnboundedReceiver<Entry>) -> Entry {
    tokio::time::timeout(std::time::Duration::from_secs(15), rx.recv()).await.unwrap().unwrap()
}
pub fn context() -> VideoPlanningContext {
    VideoPlanningContext { brief: "bounded summary".into(), max_prompt_bytes: 8192, model: None }
}
pub fn reply(prompt: &serde_json::Value) -> String {
    let i = prompt["segment"]["index"].as_u64().unwrap();
    let e = effect(
        i,
        [VisualType::Keyword, VisualType::BRoll, VisualType::MotionGraphic][i as usize % 3],
    );
    serde_json::to_string(&VideoChunkPlan { chunk_id: e.chunk_id.clone(), effects: vec![e] })
        .unwrap()
}
