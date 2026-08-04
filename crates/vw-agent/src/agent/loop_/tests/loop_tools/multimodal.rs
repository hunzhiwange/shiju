//! 多模态工具循环测试。
//!
//! 本模块覆盖代理循环在收到图片输入时的能力检查、载荷限制和正常
//! 视觉请求路径，确保在调用模型前先完成本地安全校验。

use super::*;

#[tokio::test]
async fn run_tool_call_loop_returns_structured_error_for_non_vision_provider() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = NonVisionProvider { calls: Arc::clone(&calls) };

    let mut history = vec![ChatMessage::user(
        "please inspect [IMAGE:data:image/png;base64,iVBORw0KGgo=]".to_string(),
    )];
    let tools_registry: Vec<Box<dyn Tool>> = Vec::new();
    let observer = NoopObserver;

    // 非视觉模型不应收到图片请求；提前失败能避免把用户图片泄露给
    // 不具备该能力的 provider，同时给上层返回可分类的能力错误。
    let err = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "mock-provider",
        "mock-model",
        0.0,
        true,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        3,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect_err("provider without vision support should fail");

    assert!(err.to_string().contains("provider_capability_error"));
    assert!(err.to_string().contains("capability=vision"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn run_tool_call_loop_rejects_oversized_image_payload() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = VisionProvider { calls: Arc::clone(&calls) };

    let oversized_payload = STANDARD.encode(vec![0_u8; (1024 * 1024) + 1]);
    let mut history =
        vec![ChatMessage::user(format!("[IMAGE:data:image/png;base64,{oversized_payload}]"))];

    let tools_registry: Vec<Box<dyn Tool>> = Vec::new();
    let observer = NoopObserver;

    let multimodal = crate::app::agent::config::MultimodalConfig {
        max_images: 4,
        max_image_size_mb: 1,
        allow_remote_fetch: false,
        ocr: Default::default(),
    };

    // 图片大小限制必须在 provider 调用前执行，避免 oversized data URL
    // 进入网络请求或模型上下文。
    let err = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "mock-provider",
        "mock-model",
        0.0,
        true,
        None,
        "cli",
        &multimodal,
        3,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect_err("oversized payload must fail");

    assert!(err.to_string().contains("multimodal image size limit exceeded"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn run_tool_call_loop_accepts_valid_multimodal_request_flow() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = VisionProvider { calls: Arc::clone(&calls) };

    let mut history = vec![ChatMessage::user(
        "Analyze this [IMAGE:data:image/png;base64,iVBORw0KGgo=]".to_string(),
    )];
    let tools_registry: Vec<Box<dyn Tool>> = Vec::new();
    let observer = NoopObserver;

    // 合法图片输入应完整穿过多模态校验，并且只触发一次模型调用。
    let result = run_tool_call_loop(
        &provider,
        &mut history,
        &tools_registry,
        &observer,
        "mock-provider",
        "mock-model",
        0.0,
        true,
        None,
        "cli",
        &crate::app::agent::config::MultimodalConfig::default(),
        3,
        None,
        None,
        None,
        None,
        &[],
    )
    .await
    .expect("valid multimodal payload should pass");

    assert_eq!(result, "vision-ok");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
