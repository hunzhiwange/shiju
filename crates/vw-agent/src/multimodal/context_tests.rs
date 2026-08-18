use super::analysis::{ImageAnalysisResult, analysis_cache_path_for_tests};
use super::assets::{ImageAssetId, ImageAssetScope, ImageAssetStore};
use super::context::{ImageTurnContext, TrackedImage, content_contains_image_bytes};
use super::ocr::ImageTextExtractor;
use crate::app::agent::config::MultimodalConfig;
use crate::app::agent::providers::ChatMessage;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingExtractor {
    calls: AtomicUsize,
}

struct AlwaysFailingExtractor {
    calls: AtomicUsize,
}

struct UnsupportedExtractor;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ImageTextExtractor for AlwaysFailingExtractor {
    async fn supports_image_input(&self, _model_ref: &str) -> anyhow::Result<bool> {
        Ok(true)
    }

    async fn extract_text(
        &self,
        _model_ref: &str,
        _image_data_uri: &str,
    ) -> anyhow::Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        anyhow::bail!("secret upstream error containing data:image/png;base64,LEAK")
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ImageTextExtractor for UnsupportedExtractor {
    async fn supports_image_input(&self, _model_ref: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn extract_text(
        &self,
        _model_ref: &str,
        _image_data_uri: &str,
    ) -> anyhow::Result<String> {
        panic!("unsupported models must not reach extraction")
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ImageTextExtractor for CountingExtractor {
    async fn supports_image_input(&self, _model_ref: &str) -> anyhow::Result<bool> {
        Ok(true)
    }

    async fn extract_text(
        &self,
        _model_ref: &str,
        _image_data_uri: &str,
    ) -> anyhow::Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(serde_json::json!({
            "visible_text": "ocr-text",
            "visual_summary": "a diagram",
        })
        .to_string())
    }
}

fn tracked(source: &str, is_current: bool, data_uri: &str, image_id: &str) -> TrackedImage {
    let image_id = ImageAssetId::from_hex(image_id).unwrap();
    let analysis = ImageAnalysisResult {
        version: 1,
        image_id: image_id.clone(),
        visible_text: "ocr-text".into(),
        visual_summary: "a diagram".into(),
        model_ref: "vision/reader".into(),
        prompt_version: "v1".into(),
    };
    let reference_text = super::analysis::format_image_reference_text(&image_id, &analysis);
    TrackedImage {
        image_id,
        is_current,
        source_ref: source.into(),
        model_input_data_uri: data_uri.into(),
        reference_text,
        analysis,
    }
}

fn context_with(images: Vec<TrackedImage>, resume_history_only: bool) -> ImageTurnContext {
    ImageTurnContext::from_tracked(
        ImageAssetScope::new("proj").unwrap(),
        images,
        resume_history_only,
        false,
    )
}

fn context_with_ocr(images: Vec<TrackedImage>) -> ImageTurnContext {
    ImageTurnContext::from_tracked(ImageAssetScope::new("proj").unwrap(), images, false, true)
}

fn png_fixture() -> Vec<u8> {
    use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb};
    use std::io::Cursor;

    let image = ImageBuffer::from_fn(32, 24, |x, y| Rgb([(x % 255) as u8, (y % 255) as u8, 1]));
    let mut bytes = Vec::new();
    DynamicImage::ImageRgb8(image)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    bytes
}

#[test]
fn new_image_is_attached_to_first_successful_step_only() {
    let data_uri = "data:image/jpeg;base64,AAAA";
    let image = tracked("/tmp/a.png", true, data_uri, &"ab".repeat(32));
    let mut ctx = context_with(vec![image.clone()], false);

    let first = ctx.rewrite_content(&format!("look [IMAGE:{}]", image.source_ref));
    assert!(first.contains(data_uri), "first step should attach compressed image");
    assert!(ctx.should_attach_image_bytes());

    ctx.mark_first_success();
    let second = ctx.rewrite_content(&format!("look [IMAGE:{}]", image.source_ref));
    assert!(!second.contains("data:image"), "after success must drop image bytes");
    assert!(second.contains("[VW_IMAGE:"));
    assert!(second.contains("ocr-text"));
    assert!(!ctx.should_attach_image_bytes());
}

#[test]
fn retry_before_first_success_keeps_image() {
    let data_uri = "data:image/jpeg;base64,BBBB";
    let image = tracked("/tmp/b.png", true, data_uri, &"cd".repeat(32));
    let ctx = context_with(vec![image.clone()], false);

    let attempt1 = ctx.rewrite_content(&format!("[IMAGE:{}]", image.source_ref));
    let attempt2 = ctx.rewrite_content(&format!("[IMAGE:{}]", image.source_ref));
    assert!(attempt1.contains(data_uri));
    assert!(attempt2.contains(data_uri));
    assert!(!ctx.has_first_success());
    assert!(ctx.should_attach_image_bytes());
}

#[test]
fn tool_step_after_success_uses_reference_and_analysis() {
    let data_uri = "data:image/jpeg;base64,CCCC";
    let image = tracked("/tmp/c.png", true, data_uri, &"ef".repeat(32));
    let mut ctx = context_with(vec![image.clone()], false);
    ctx.mark_first_success();

    let mut messages = vec![ChatMessage::user(format!("inspect [IMAGE:{}]", image.source_ref))];
    ctx.rewrite_chat_messages(&mut messages);
    assert!(!messages[0].content.contains("data:image"));
    assert!(messages[0].content.contains("[VW_IMAGE:"));
    assert!(messages[0].content.contains("visual_summary"));
    assert!(ctx.assert_chat_messages_safe(&messages).is_ok());
}

#[test]
fn history_and_resume_only_never_attach_image_bytes() {
    let data_uri = "data:image/jpeg;base64,DDDD";
    let history_image = tracked("/tmp/hist.png", false, data_uri, &"11".repeat(32));
    let ctx = context_with(vec![history_image.clone()], false);
    let rewritten = ctx.rewrite_content(&format!("old [IMAGE:{}]", history_image.source_ref));
    assert!(!rewritten.contains("data:image"));
    assert!(rewritten.contains("[VW_IMAGE:"));
    assert!(!ctx.should_attach_image_bytes());

    let resume_image = tracked("/tmp/resume.png", true, data_uri, &"22".repeat(32));
    let resume_ctx = context_with(vec![resume_image.clone()], true);
    let resume_text =
        resume_ctx.rewrite_content(&format!("resume [IMAGE:{}]", resume_image.source_ref));
    assert!(!resume_text.contains("data:image"));
    assert!(!resume_ctx.should_attach_image_bytes());
    assert!(!content_contains_image_bytes(&resume_text));
}

#[test]
fn ocr_enabled_sends_text_instead_of_image_bytes_on_first_step() {
    let data_uri = "data:image/jpeg;base64,EEEE";
    let image = tracked("/tmp/ocr.png", true, data_uri, &"33".repeat(32));
    let ctx = context_with_ocr(vec![image.clone()]);

    let first = ctx.rewrite_content(&format!("look [IMAGE:{}]", image.source_ref));
    assert!(!first.contains("data:image"), "OCR mode must not attach compressed image bytes");
    assert!(first.contains("[VW_IMAGE:"));
    assert!(first.contains("ocr-text"));
    assert!(first.contains("visual_summary"));
    assert!(!ctx.should_attach_image_bytes());
}

#[tokio::test]
async fn legacy_loop_reuses_cached_analysis() {
    let temp = tempfile::tempdir().unwrap();
    let store = ImageAssetStore::new(temp.path());
    let scope = ImageAssetScope::new("legacy-scope").unwrap();
    let extractor = CountingExtractor { calls: AtomicUsize::new(0) };
    let mut config = MultimodalConfig::default();
    config.ocr.enabled = true;
    config.ocr.model_supports_images = false;
    config.ocr.model = Some("vision/reader".to_string());

    // 生成可用 PNG fixture。
    let png = png_fixture();
    let path = temp.path().join("shot.png");
    std::fs::write(&path, &png).unwrap();
    let source = path.to_string_lossy().to_string();

    let ctx1 = ImageTurnContext::build(
        &store,
        scope.clone(),
        &[],
        Some(&format!("see [IMAGE:{source}]")),
        false,
        &config,
        Some("vision/reader"),
        &extractor,
    )
    .await
    .expect("build context");
    assert_eq!(extractor.calls.load(Ordering::SeqCst), 1);
    assert_eq!(ctx1.images.len(), 1);
    assert!(
        !ctx1.should_attach_image_bytes(),
        "enabled OCR must prefer reference text over image bytes"
    );
    let rewritten = ctx1.rewrite_content(&format!("see [IMAGE:{source}]"));
    assert!(!rewritten.contains("data:image"));
    assert!(rewritten.contains("ocr-text"));

    let ctx2 = ImageTurnContext::build(
        &store,
        scope,
        &[],
        Some(&format!("see again [IMAGE:{source}]")),
        false,
        &config,
        Some("vision/reader"),
        &extractor,
    )
    .await
    .expect("rebuild should hit analysis cache");
    assert_eq!(
        extractor.calls.load(Ordering::SeqCst),
        1,
        "cached analysis must not call the model again"
    );
    assert_eq!(ctx1.images[0].image_id, ctx2.images[0].image_id);
}

#[tokio::test]
async fn image_context_continues_with_unavailable_reference_after_analysis_exhaustion() {
    let temp = tempfile::tempdir().unwrap();
    let store = ImageAssetStore::new(temp.path());
    let scope = ImageAssetScope::new("analysis-exhaustion").unwrap();
    let path = temp.path().join("failure.png");
    std::fs::write(&path, png_fixture()).unwrap();
    let source = path.to_string_lossy().to_string();
    let extractor = AlwaysFailingExtractor { calls: AtomicUsize::new(0) };
    let mut config = MultimodalConfig::default();
    config.ocr.enabled = true;
    config.ocr.model_supports_images = false;
    config.ocr.model = Some("vision/reader".to_string());

    let context = ImageTurnContext::build(
        &store,
        scope.clone(),
        &[],
        Some(&format!("inspect [IMAGE:{source}]")),
        false,
        &config,
        Some("vision/reader"),
        &extractor,
    )
    .await
    .expect("exhausted recoverable analysis must degrade without ending the turn");

    assert_eq!(extractor.calls.load(Ordering::SeqCst), 4);
    assert_eq!(context.images.len(), 1);
    let image = &context.images[0];
    let rewritten = context.rewrite_content(&format!("inspect [IMAGE:{source}]"));
    assert!(rewritten.contains("[VW_IMAGE:"));
    assert!(rewritten.contains("图片识别暂不可用"));
    assert!(!rewritten.contains("data:image"));
    assert!(!rewritten.contains("secret upstream error"));
    assert!(!context.should_attach_image_bytes());
    assert!(
        !analysis_cache_path_for_tests(&store, &scope, &image.image_id, "vision/reader").exists(),
        "unavailable fallback must not be persisted as a successful analysis"
    );
}

#[tokio::test]
async fn image_context_keeps_current_image_for_visual_model_after_analysis_exhaustion() {
    let temp = tempfile::tempdir().unwrap();
    let store = ImageAssetStore::new(temp.path());
    let scope = ImageAssetScope::new("visual-analysis-exhaustion").unwrap();
    let path = temp.path().join("visual-failure.png");
    std::fs::write(&path, png_fixture()).unwrap();
    let source = path.to_string_lossy().to_string();
    let extractor = AlwaysFailingExtractor { calls: AtomicUsize::new(0) };
    let mut config = MultimodalConfig::default();
    config.ocr.model = Some("vision/reader".to_string());

    let mut context = ImageTurnContext::build(
        &store,
        scope,
        &[],
        Some(&format!("inspect [IMAGE:{source}]")),
        false,
        &config,
        Some("vision/reader"),
        &extractor,
    )
    .await
    .expect("visual mode should keep the persisted current image after analysis exhaustion");

    let first = context.rewrite_content(&format!("inspect [IMAGE:{source}]"));
    assert!(context.should_attach_image_bytes());
    assert!(first.contains("data:image"));
    context.mark_first_success();
    let later = context.rewrite_content(&format!("inspect [IMAGE:{source}]"));
    assert!(!later.contains("data:image"));
    assert!(later.contains("图片识别暂不可用"));
}

#[tokio::test]
async fn image_context_rejects_model_without_image_support() {
    let temp = tempfile::tempdir().unwrap();
    let store = ImageAssetStore::new(temp.path());
    let scope = ImageAssetScope::new("unsupported-analysis-model").unwrap();
    let path = temp.path().join("unsupported.png");
    std::fs::write(&path, png_fixture()).unwrap();
    let source = path.to_string_lossy().to_string();
    let mut config = MultimodalConfig::default();
    config.ocr.enabled = true;
    config.ocr.model_supports_images = false;
    config.ocr.model = Some("text/only".to_string());

    let error = ImageTurnContext::build(
        &store,
        scope,
        &[],
        Some(&format!("inspect [IMAGE:{source}]")),
        false,
        &config,
        Some("text/only"),
        &UnsupportedExtractor,
    )
    .await
    .expect_err("capability mismatch must remain an explicit error");

    assert!(error.to_string().contains("does not support image input"));
}

#[tokio::test]
async fn image_context_does_not_hide_asset_store_failure() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("asset-store-failure.png");
    std::fs::write(&path, png_fixture()).unwrap();
    let source = path.to_string_lossy().to_string();
    let store_root = temp.path().join("not-a-directory");
    std::fs::write(&store_root, b"blocks asset directory creation").unwrap();
    let store = ImageAssetStore::new(&store_root);
    let extractor = AlwaysFailingExtractor { calls: AtomicUsize::new(0) };
    let mut config = MultimodalConfig::default();
    config.ocr.model = Some("vision/reader".to_string());

    let error = ImageTurnContext::build(
        &store,
        ImageAssetScope::new("asset-store-failure").unwrap(),
        &[],
        Some(&format!("inspect [IMAGE:{source}]")),
        false,
        &config,
        Some("vision/reader"),
        &extractor,
    )
    .await
    .expect_err("asset persistence failures must remain explicit");

    assert!(!error.to_string().is_empty());
    assert_eq!(extractor.calls.load(Ordering::SeqCst), 0);
}
