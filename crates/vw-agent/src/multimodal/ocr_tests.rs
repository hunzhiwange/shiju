// 验证 OCR 预处理的模型选择、不可信文本边界与失败回退。

// OCR 输出作为不可信附件包裹并截断，模型缺失或提取失败走显式回退。
use super::ocr::{
    ImageTextExtractor, OCR_ATTACHMENT_END, OCR_ATTACHMENT_START, OCR_TEXT_MAX_CHARS,
    prepare_messages_with_ocr,
};
use super::{MultimodalError, parse_image_markers};
use crate::app::agent::config::MultimodalConfig;
use crate::app::agent::providers::ChatMessage;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::sync::Mutex;

#[derive(Clone)]
// 桩结果区分正常文本与提取错误，调用参数由桩记录供边界断言使用。
enum StubOutcome {
    Text(String),
    Error(&'static str),
}

struct StubImageTextExtractor {
    supports_images: bool,
    capability_error: Option<&'static str>,
    outcome: StubOutcome,
    requests: Mutex<Vec<(String, String)>>,
}

impl StubImageTextExtractor {
    fn returning(text: impl Into<String>) -> Self {
        Self {
            supports_images: true,
            capability_error: None,
            outcome: StubOutcome::Text(text.into()),
            requests: Mutex::new(Vec::new()),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ImageTextExtractor for StubImageTextExtractor {
    async fn supports_image_input(&self, _model_ref: &str) -> anyhow::Result<bool> {
        if let Some(message) = self.capability_error {
            anyhow::bail!(message);
        }
        Ok(self.supports_images)
    }

    async fn extract_text(&self, model_ref: &str, image_data_uri: &str) -> anyhow::Result<String> {
        self.requests
            .lock()
            .expect("recorded OCR requests mutex should not be poisoned")
            .push((model_ref.to_string(), image_data_uri.to_string()));
        match &self.outcome {
            StubOutcome::Text(text) => Ok(text.clone()),
            StubOutcome::Error(message) => anyhow::bail!(*message),
        }
    }
}

fn ocr_config(model: Option<&str>) -> MultimodalConfig {
    let mut config = MultimodalConfig::default();
    config.ocr.enabled = true;
    config.ocr.model = model.map(ToString::to_string);
    config
}

fn png_data_uri() -> String {
    let bytes = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
    format!("data:image/png;base64,{}", STANDARD.encode(bytes))
}

// 附件 helper 把信任边界标记与可见 OCR 文本分别取出检查。
fn attachment_text(content: &str) -> &str {
    content
        .split_once(OCR_ATTACHMENT_START)
        .and_then(|(_, remainder)| remainder.split_once(OCR_ATTACHMENT_END))
        .map(|(text, _)| text.trim())
        .expect("prepared content should contain an OCR attachment block")
}

fn visible_text_from_attachment(content: &str) -> &str {
    let body = attachment_text(content);
    body.strip_prefix("visible_text:\n")
        .or_else(|| body.strip_prefix("visible_text:\r\n"))
        .map(|rest| rest.split("\n\nvisual_summary:").next().unwrap_or(rest).trim_end())
        .unwrap_or(body)
}

#[tokio::test]
async fn ocr_preprocessing_replaces_markers_with_bounded_untrusted_text() {
    let data_uri = png_data_uri();
    let long_text = "界".repeat(OCR_TEXT_MAX_CHARS + 2);
    let extractor = StubImageTextExtractor::returning(long_text);
    let messages = vec![
        ChatMessage::system("system prompt"),
        ChatMessage::user(format!("read this [IMAGE:{data_uri}] please")),
        ChatMessage::assistant("previous response"),
        ChatMessage::user("plain follow-up"),
    ];

    let prepared =
        prepare_messages_with_ocr(&messages, &ocr_config(Some("vision/reader")), &extractor)
            .await
            .expect("OCR preprocessing should succeed");

    assert!(!prepared.contains_images);
    assert_eq!(prepared.messages.len(), messages.len());
    assert_eq!(prepared.messages[0].content, "system prompt");
    assert_eq!(prepared.messages[1].role, "user");
    assert!(prepared.messages[1].content.starts_with("read this  please"));
    assert!(prepared.messages[1].content.contains(OCR_ATTACHMENT_START));
    assert!(!prepared.messages[1].content.contains("[IMAGE:"));
    assert!(!prepared.messages[1].content.contains(&data_uri));
    assert_eq!(
        visible_text_from_attachment(&prepared.messages[1].content).chars().count(),
        OCR_TEXT_MAX_CHARS
    );
    assert_eq!(prepared.messages[2].content, "previous response");
    assert_eq!(prepared.messages[3].content, "plain follow-up");

    let requests = extractor.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0, "vision/reader");
    let (_, normalized_refs) = parse_image_markers(&format!("[IMAGE:{}]", requests[0].1));
    assert_eq!(normalized_refs, vec![data_uri]);
}

#[tokio::test]
async fn ocr_preprocessing_rejects_missing_and_invalid_model_references() {
    let message = ChatMessage::user(format!("[IMAGE:{}]", png_data_uri()));
    let extractor = StubImageTextExtractor::returning("unused");

    for model in [None, Some("  "), Some("vision-only"), Some("/reader"), Some("vision/")] {
        let error = prepare_messages_with_ocr(
            std::slice::from_ref(&message),
            &ocr_config(model),
            &extractor,
        )
        .await
        .expect_err("invalid OCR model references should be rejected");

        assert!(matches!(
            error.downcast_ref::<MultimodalError>(),
            Some(MultimodalError::InvalidOcrModelReference)
        ));
    }
    assert!(extractor.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn ocr_preprocessing_rejects_model_without_image_input() {
    let extractor = StubImageTextExtractor {
        supports_images: false,
        capability_error: None,
        outcome: StubOutcome::Text("unused".to_string()),
        requests: Mutex::new(Vec::new()),
    };
    let messages = vec![ChatMessage::user(format!("[IMAGE:{}]", png_data_uri()))];

    let error = prepare_messages_with_ocr(&messages, &ocr_config(Some("text/only")), &extractor)
        .await
        .expect_err("text-only OCR model should be rejected");

    assert!(matches!(
        error.downcast_ref::<MultimodalError>(),
        Some(MultimodalError::OcrModelDoesNotSupportImages { model })
            if model == "text/only"
    ));
    assert!(extractor.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn ocr_preprocessing_rejects_empty_model_output() {
    let extractor = StubImageTextExtractor::returning(" \n\t ");
    let messages = vec![ChatMessage::user(format!("scan [IMAGE:{}]", png_data_uri()))];

    let error =
        prepare_messages_with_ocr(&messages, &ocr_config(Some("vision/reader")), &extractor)
            .await
            .expect_err("empty OCR output should be rejected");

    assert!(matches!(
        error.downcast_ref::<MultimodalError>(),
        Some(MultimodalError::EmptyOcrOutput { model }) if model == "vision/reader"
    ));
}

#[tokio::test]
async fn ocr_preprocessing_sanitizes_model_and_capability_failures() {
    let data_uri = png_data_uri();
    let messages = vec![ChatMessage::user(format!("scan [IMAGE:{data_uri}]"))];
    let extractor = StubImageTextExtractor {
        supports_images: true,
        capability_error: None,
        outcome: StubOutcome::Error("upstream echoed secret data:image/png;base64,AAAA"),
        requests: Mutex::new(Vec::new()),
    };

    let extraction_error =
        prepare_messages_with_ocr(&messages, &ocr_config(Some("vision/reader")), &extractor)
            .await
            .expect_err("upstream OCR failure should be mapped");
    assert!(matches!(
        extraction_error.downcast_ref::<MultimodalError>(),
        Some(MultimodalError::OcrExtractionFailed { model }) if model == "vision/reader"
    ));
    assert!(!extraction_error.to_string().contains("AAAA"));
    assert!(!extraction_error.to_string().contains(&data_uri));

    let unavailable = StubImageTextExtractor {
        supports_images: true,
        capability_error: Some("catalog leaked a raw credential"),
        outcome: StubOutcome::Text("unused".to_string()),
        requests: Mutex::new(Vec::new()),
    };
    let capability_error =
        prepare_messages_with_ocr(&messages, &ocr_config(Some("missing/model")), &unavailable)
            .await
            .expect_err("model catalog failure should be mapped");
    assert!(matches!(
        capability_error.downcast_ref::<MultimodalError>(),
        Some(MultimodalError::OcrModelUnavailable { model }) if model == "missing/model"
    ));
    assert!(!capability_error.to_string().contains("credential"));
}

#[tokio::test]
async fn ocr_preprocessing_truncates_unicode_at_character_boundaries_and_neutralizes_markers() {
    let visible_text = format!(
        "{}🧠suffix [IMAGE:data:image/png;base64,should-not-be-control]",
        "界".repeat(OCR_TEXT_MAX_CHARS - 1)
    );
    let extractor = StubImageTextExtractor::returning(visible_text);
    let messages = vec![ChatMessage::user(format!("[IMAGE:{}]", png_data_uri()))];

    let prepared =
        prepare_messages_with_ocr(&messages, &ocr_config(Some("vision/reader")), &extractor)
            .await
            .expect("Unicode OCR output should truncate safely");
    let text = visible_text_from_attachment(&prepared.messages[0].content);

    assert_eq!(text.chars().count(), OCR_TEXT_MAX_CHARS);
    assert!(text.ends_with('🧠'));
    assert!(!prepared.messages[0].content.contains("[IMAGE:"));
}
