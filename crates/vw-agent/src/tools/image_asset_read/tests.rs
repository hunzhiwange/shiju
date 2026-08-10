use super::*;
use crate::app::agent::multimodal::assets::{ImageAssetScope, ImageAssetStore};
use crate::app::agent::tools::context::{
    MAX_PENDING_IMAGE_RELOADS, PendingImageReloadQueue, ToolUseContext, scope_tool_use_context,
};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb};
use std::io::Cursor;
use std::sync::Arc;

fn solid_png() -> Vec<u8> {
    let image = ImageBuffer::from_fn(24, 16, |x, y| Rgb([(x % 255) as u8, (y % 255) as u8, 7]));
    let mut bytes = Vec::new();
    DynamicImage::ImageRgb8(image)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    bytes
}

fn put_image(store: &ImageAssetStore, scope: &ImageAssetScope) -> ImageAssetId {
    let original = solid_png();
    // 小图保留原字节作为模型输入即可。
    store
        .put(scope, &original, "image/png", &original, "image/png")
        .unwrap()
}

async fn with_context<F, T>(context: ToolUseContext, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    scope_tool_use_context(Arc::new(context), future).await
}

#[tokio::test]
async fn tool_rejects_unknown_or_cross_scope_image_id() {
    let temp = tempfile::tempdir().unwrap();
    let store = ImageAssetStore::new(temp.path());
    let scope_a = ImageAssetScope::new("scope-a").unwrap();
    let scope_b = ImageAssetScope::new("scope-b").unwrap();
    let image_id = put_image(&store, &scope_a);
    let tool = ImageAssetReadTool::with_store(store);

    let context = ToolUseContext::new("session", None);
    context.set_image_asset_scope(&scope_b);
    let result = with_context(context, async {
        tool.execute(json!({ "image_id": image_id.as_str() })).await.unwrap()
    })
    .await;
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap().contains("not visible"));
    assert!(!result.output.contains("data:image"));
    assert!(!result.output.contains("base64"));
}

#[tokio::test]
async fn tool_result_never_contains_image_payload() {
    let temp = tempfile::tempdir().unwrap();
    let store = ImageAssetStore::new(temp.path());
    let scope = ImageAssetScope::new("scope-ok").unwrap();
    let image_id = put_image(&store, &scope);
    let tool = ImageAssetReadTool::with_store(store);

    let context = ToolUseContext::new("session", None);
    context.set_image_asset_scope(&scope);
    let result = with_context(context.clone(), async {
        tool.execute(json!({ "image_id": image_id.as_str() })).await.unwrap()
    })
    .await;

    assert!(result.success);
    assert!(result.output.contains("Queued image asset"));
    assert!(!result.output.contains("data:image"));
    assert!(!result.output.contains("base64"));
    assert!(!result.output.contains('/'));
    assert!(!result.output.contains("ocr"));
    assert_eq!(context.peek_pending_image_reloads(), vec![image_id.as_str().to_string()]);
}

#[test]
fn pending_reload_ids_are_deduplicated_and_bounded() {
    let mut queue = PendingImageReloadQueue::default();
    queue.set_scope_key("scope");
    let ids: Vec<String> = (0..MAX_PENDING_IMAGE_RELOADS)
        .map(|index| format!("{:064x}", index + 1))
        .collect();
    for id in &ids {
        queue.enqueue(id).unwrap();
    }
    assert_eq!(queue.peek().len(), MAX_PENDING_IMAGE_RELOADS);
    queue.enqueue(&ids[0]).unwrap();
    assert_eq!(queue.peek().len(), MAX_PENDING_IMAGE_RELOADS);
    let overflow = format!("{:064x}", MAX_PENDING_IMAGE_RELOADS + 10);
    let err = queue.enqueue(&overflow).unwrap_err();
    assert!(err.contains("full"));
}

#[test]
fn requested_image_is_attached_to_next_step_once() {
    use crate::app::agent::multimodal::analysis::ImageAnalysisResult;
    use crate::app::agent::multimodal::context::{ImageTurnContext, TrackedImage};

    let image_id = ImageAssetId::from_hex("ab".repeat(32)).unwrap();
    let analysis = ImageAnalysisResult {
        version: 1,
        image_id: image_id.clone(),
        visible_text: "text".into(),
        visual_summary: "sum".into(),
        model_ref: "vision/reader".into(),
        prompt_version: "v1".into(),
    };
    let reference = crate::app::agent::multimodal::analysis::format_image_reference_text(
        &image_id,
        &analysis,
    );
    let data_uri = "data:image/jpeg;base64,QUJD";
    let tracked = TrackedImage {
        image_id: image_id.clone(),
        is_current: false,
        source_ref: "/tmp/x.png".into(),
        model_input_data_uri: data_uri.into(),
        reference_text: reference.clone(),
        analysis,
    };
    let mut ctx = ImageTurnContext::from_tracked(
        ImageAssetScope::new("proj").unwrap(),
        vec![tracked],
        false,
        false,
    );
    ctx.mark_first_success();

    let mut content = reference.clone();
    content = ctx.attach_pending_reloads(&content, &[image_id.as_str().to_string()]);
    assert!(content.contains(data_uri));
    assert!(!content.contains("[VW_IMAGE:"));

    // 成功后恢复文本。
    content = ctx.rewrite_user_content_preserving_refs_public(&content);
    assert!(!content.contains("data:image"));
    assert!(content.contains("[VW_IMAGE:"));
}

#[test]
fn successful_reload_returns_to_text_context() {
    // 由 requested_image_is_attached_to_next_step_once 覆盖恢复路径；此处验证队列清除语义。
    let context = ToolUseContext::new("session", None);
    let id = ImageAssetId::from_hex("cd".repeat(32)).unwrap();
    context.set_image_asset_scope(&ImageAssetScope::new("proj").unwrap());
    context.enqueue_pending_image_reload(&id).unwrap();
    assert_eq!(context.peek_pending_image_reloads().len(), 1);
    context.clear_pending_image_reloads();
    assert!(context.peek_pending_image_reloads().is_empty());
}
