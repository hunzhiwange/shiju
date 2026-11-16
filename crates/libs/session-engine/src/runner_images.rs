//! Session runner implementation; instantiated with the concrete IO host.
use super::*;

pub fn build_image_turn_context_for_request<H: RunnerHost>(
    req: &Request<H>,
    session: &Session,
    config: &agent_config::runtime_schema::Config,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
) -> Result<Option<agent_multimodal::context::ImageTurnContext>, String> {
    use agent_multimodal::DISABLE_OCR_OPTION;
    use agent_multimodal::assets::{ImageAssetScope, ImageAssetStore};
    use agent_multimodal::context::ImageTurnContext;

    use agent_multimodal::{contains_image_markers, parse_image_markers};
    use model_types::ChatMessage;

    // OCR itself is routed through the local gateway. Its nested request still
    // contains the image marker, so it must reach the configured vision model
    // without entering this preprocessing path again.
    if req.options.get(DISABLE_OCR_OPTION).and_then(Value::as_bool) == Some(true) {
        return Ok(None);
    }

    let resume_history_only =
        req.options.get("desktop_resume_history_only").and_then(Value::as_bool).unwrap_or(false);

    let history: Vec<ChatMessage> = session
        .messages
        .iter()
        .filter(|message| {
            matches!(message.role, Role::User | Role::Assistant | Role::System | Role::Developer)
        })
        .map(|message| match message.role {
            Role::User => ChatMessage::user(&message.content),
            Role::Assistant => ChatMessage::assistant(&message.content),
            Role::System => ChatMessage::system(&message.content),
            Role::Developer => {
                ChatMessage { role: "developer".into(), content: message.content.clone() }
            }
            Role::Tool => ChatMessage::assistant(&message.content),
        })
        .collect();

    if !contains_image_markers(&history) && parse_image_markers(&req.query).1.is_empty() {
        return Ok(None);
    }

    // ACP 在协商后的 prompt 能力边界校验图片，不查询普通供应商模型目录。
    if is_acp_request(&req.options) {
        return Ok(None);
    }
    let extractor = H::image_extractor();
    let model_ref = req.model.as_deref().ok_or("图片请求缺少已选择的模型")?;
    let Some(multimodal_config) =
        prompt::block_on(agent_multimodal::ocr::image_preprocessing_config(
            &config.multimodal,
            model_ref,
            &extractor,
        ))
        .map_err(|error| format!("图片处理配置无效：{error}"))?
    else {
        return Ok(None);
    };
    if !on_event(StreamEvent::Delta("<think>正在识别图片，请稍候…</think>\n\n".to_string()))
    {
        return Err("图片识别已取消".to_string());
    }

    // 当前轮用户内容：非 resume 时取最后一条 user；resume 时全部按历史处理。
    let (history_messages, current_user_content) = if resume_history_only {
        (history, None)
    } else {
        let mut prior = history;
        let current = prior
            .iter()
            .rposition(|message| message.role == "user")
            .map(|index| prior.remove(index).content);
        (prior, current)
    };

    let scope =
        ImageAssetScope::from_request_context(req.app_session_scope.as_deref(), &req.session)
            .map_err(|error| format!("invalid image asset scope: {error}"))?;
    let store = ImageAssetStore::from_global_paths();
    match prompt::block_on(ImageTurnContext::build(
        &store,
        scope,
        &history_messages,
        current_user_content.as_deref(),
        resume_history_only,
        &multimodal_config,
        req.model.as_deref(),
        &extractor,
    )) {
        Ok(ctx) if !ctx.images.is_empty() => Ok(Some(ctx)),
        Ok(_) => Ok(None),
        Err(error) => Err(format!("image OCR preprocessing failed: {error}")),
    }
}
