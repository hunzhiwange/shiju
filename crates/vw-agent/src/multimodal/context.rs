//! 本轮图片上下文：首轮直传压缩图，历史与首次成功后复用引用与分析结果。
//!
//! 状态机：
//! - 历史图片 / resume-only：始终使用 `[VW_IMAGE:id]` + 有界 OCR/摘要
//! - `multimodal.ocr.enabled && !multimodal.ocr.model_supports_images`：当前新图片也直接使用 OCR/摘要文本，不再首轮直传压缩图
//! - 当前新图片（OCR 关闭）：首次成功 LLM step 前保留压缩 data URI；成功后切换为引用文本
//! - 首次 step 网络失败不切换状态，重试仍可携带图片

use super::analysis::{
    ANALYSIS_PROMPT_VERSION, ImageAnalysisCache, ImageAnalysisResult, format_image_reference_text,
    resolve_analysis_model,
};
use super::assets::{ImageAssetId, ImageAssetScope, ImageAssetStore};
use super::ocr::ImageTextExtractor;
use super::{
    IMAGE_MARKER_PREFIX, MaterializedImage, MultimodalError, materialize_image_reference,
    parse_image_markers,
};
use crate::app::agent::config::MultimodalConfig;
use crate::app::agent::providers::ChatMessage;
use anyhow::Result;

const DATA_IMAGE_PREFIX: &str = "data:image";
const ANALYSIS_UNAVAILABLE_TEXT: &str = "图片识别暂不可用";

/// 单张受跟踪图片。
#[derive(Debug, Clone)]
pub struct TrackedImage {
    /// 内容寻址 ID。
    pub image_id: ImageAssetId,
    /// 是否属于当前用户轮次（非历史）。
    pub is_current: bool,
    /// 原始标记内容（路径 / URL / data URI）。
    pub source_ref: String,
    /// 压缩模型输入 data URI（仅当前图片在首次成功前使用）。
    pub model_input_data_uri: String,
    /// 稳定引用 + 分析结果文本。
    pub reference_text: String,
    /// 结构化分析结果。
    pub analysis: ImageAnalysisResult,
}

/// 单次请求的图片上下文状态机。
#[derive(Debug, Clone)]
pub struct ImageTurnContext {
    /// 资产范围。
    pub scope: ImageAssetScope,
    /// 受跟踪图片（按发现顺序）。
    pub images: Vec<TrackedImage>,
    /// 是否已完成首次成功的 LLM step。
    first_success: bool,
    /// Desktop resume-only：全部图片按历史处理。
    resume_history_only: bool,
    /// 已启用 OCR：主模型只接收引用 + 有界文字，不直传压缩图。
    ocr_text_only: bool,
}

impl ImageTurnContext {
    /// 空上下文（无图片）。
    pub fn empty(scope: ImageAssetScope) -> Self {
        Self::from_tracked(scope, Vec::new(), false, false)
    }

    /// 测试/组装：设置受跟踪图片与 resume / OCR 标志。
    pub fn from_tracked(
        scope: ImageAssetScope,
        images: Vec<TrackedImage>,
        resume_history_only: bool,
        ocr_text_only: bool,
    ) -> Self {
        Self { scope, images, first_success: false, resume_history_only, ocr_text_only }
    }

    /// 从用户消息构建上下文：落盘资产、分析缓存，并区分历史/当前图片。
    #[allow(clippy::too_many_arguments)]
    pub async fn build(
        store: &ImageAssetStore,
        scope: ImageAssetScope,
        history_messages: &[ChatMessage],
        current_user_content: Option<&str>,
        resume_history_only: bool,
        config: &MultimodalConfig,
        current_model: Option<&str>,
        extractor: &dyn ImageTextExtractor,
    ) -> Result<Self> {
        let ocr_text_only = config.ocr.enabled && !config.ocr.model_supports_images;
        let mut images = Vec::new();
        let mut total_refs = 0usize;
        for message in history_messages {
            if message.role == "user" {
                total_refs += parse_image_markers(&message.content).1.len();
            }
        }
        if let Some(content) = current_user_content {
            if !resume_history_only {
                total_refs += parse_image_markers(content).1.len();
            }
        }
        if total_refs == 0 {
            return Ok(Self {
                scope,
                images,
                first_success: false,
                resume_history_only,
                ocr_text_only,
            });
        }

        let model_ref = resolve_analysis_model(config, current_model, extractor).await?;
        let analysis_cache = ImageAnalysisCache::new();

        for message in history_messages {
            if message.role != "user" {
                continue;
            }
            let (_, refs) = parse_image_markers(&message.content);
            for source_ref in refs {
                let tracked = persist_and_analyze(
                    store,
                    &scope,
                    &source_ref,
                    false,
                    total_refs,
                    config,
                    &model_ref,
                    &analysis_cache,
                    extractor,
                )
                .await?;
                images.push(tracked);
            }
        }

        if let Some(content) = current_user_content.filter(|_| !resume_history_only) {
            let (_, refs) = parse_image_markers(content);
            for source_ref in refs {
                let tracked = persist_and_analyze(
                    store,
                    &scope,
                    &source_ref,
                    true,
                    total_refs,
                    config,
                    &model_ref,
                    &analysis_cache,
                    extractor,
                )
                .await?;
                images.push(tracked);
            }
        }

        Ok(Self { scope, images, first_success: false, resume_history_only, ocr_text_only })
    }

    /// 将本轮图片引用登记到给定会话（幂等）。
    pub fn register_session_refs(&self, store: &ImageAssetStore, session_id: &str) -> Result<()> {
        for image in &self.images {
            store.register_session_ref(&self.scope, &image.image_id, session_id)?;
        }
        Ok(())
    }

    /// 是否仍应在下一步携带压缩图。
    pub fn should_attach_image_bytes(&self) -> bool {
        if self.resume_history_only || self.first_success || self.ocr_text_only {
            return false;
        }
        self.images.iter().any(|image| image.is_current)
    }

    /// 标记首次 LLM step 成功；之后切换为引用文本。
    pub fn mark_first_success(&mut self) {
        self.first_success = true;
    }

    /// 首次成功是否已发生。
    pub fn has_first_success(&self) -> bool {
        self.first_success
    }

    /// 按当前状态改写用户消息内容中的图片标记。
    pub fn rewrite_content(&self, content: &str) -> String {
        let (cleaned, refs) = parse_image_markers(content);
        if refs.is_empty() {
            return content.to_string();
        }

        let mut out = cleaned;
        let trimmed = out.trim();
        if !trimmed.is_empty() {
            out = trimmed.to_string();
            out.push_str("\n\n");
        } else {
            out.clear();
        }

        for (index, source_ref) in refs.iter().enumerate() {
            if index > 0 {
                out.push_str("\n\n");
            }
            out.push_str(&self.replacement_for(source_ref));
        }
        out
    }

    /// 改写 OpenAI JSON 消息数组中的用户字符串 content。
    pub fn rewrite_json_messages(&self, messages: &mut [serde_json::Value]) {
        for message in messages.iter_mut() {
            let Some(obj) = message.as_object_mut() else {
                continue;
            };
            if obj.get("role").and_then(serde_json::Value::as_str) != Some("user") {
                continue;
            }
            let Some(content) = obj.get("content").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if parse_image_markers(content).1.is_empty()
                && !content.contains("[VW_IMAGE:")
                && !content.contains(DATA_IMAGE_PREFIX)
            {
                continue;
            }
            // 已是 VW_IMAGE 引用时，若需切回/保持引用，仍按 source 映射处理 data URI。
            let rewritten = self.rewrite_content(content);
            obj.insert("content".to_string(), serde_json::Value::String(rewritten));
        }
    }

    /// 改写 ChatMessage 列表。
    pub fn rewrite_chat_messages(&self, messages: &mut [ChatMessage]) {
        for message in messages.iter_mut() {
            if message.role != "user" {
                continue;
            }
            if parse_image_markers(&message.content).1.is_empty() {
                // 可能已是 data URI 标记；仍尝试按已知 source/data_uri 替换。
                if !message.content.contains(DATA_IMAGE_PREFIX)
                    && !message.content.contains("[VW_IMAGE:")
                {
                    continue;
                }
            }
            message.content = self.rewrite_user_content_preserving_refs(&message.content);
        }
    }

    /// 供应商边界检查：禁止历史/成功后消息携带图片字节或本地路径标记。
    pub fn assert_provider_messages_safe(
        &self,
        messages: &[serde_json::Value],
    ) -> Result<(), String> {
        if self.should_attach_image_bytes() {
            return Ok(());
        }
        for message in messages {
            let Some(content) = message
                .as_object()
                .and_then(|obj| obj.get("content"))
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            if content.contains(DATA_IMAGE_PREFIX) {
                return Err("provider messages must not contain data:image after first success or for history".into());
            }
            let (_, refs) = parse_image_markers(content);
            for reference in refs {
                if reference.starts_with('/')
                    || reference.starts_with("http://")
                    || reference.starts_with("https://")
                {
                    return Err(
                        "provider messages must not contain local/remote image paths after first success or for history"
                            .into(),
                    );
                }
            }
        }
        Ok(())
    }

    /// 同边界检查，面向 ChatMessage。
    pub fn assert_chat_messages_safe(&self, messages: &[ChatMessage]) -> Result<(), String> {
        let json: Vec<serde_json::Value> = messages
            .iter()
            .map(|message| {
                serde_json::json!({
                    "role": message.role,
                    "content": message.content,
                })
            })
            .collect();
        self.assert_provider_messages_safe(&json)
    }

    /// 将待重看 image_id 对应的引用文本临时替换为压缩模型输入标记。
    pub fn attach_pending_reloads(&self, content: &str, pending_ids: &[String]) -> String {
        let mut out = content.to_string();
        for image in &self.images {
            if !pending_ids.iter().any(|id| id == image.image_id.as_str()) {
                continue;
            }
            if out.contains(&image.reference_text) {
                out = out.replace(
                    &image.reference_text,
                    &format!("{IMAGE_MARKER_PREFIX}{}]", image.model_input_data_uri),
                );
            }
        }
        out
    }

    /// 将 JSON 消息中的待重看引用替换为压缩图。
    pub fn attach_pending_reloads_to_json(
        &self,
        messages: &mut [serde_json::Value],
        pending_ids: &[String],
    ) {
        if pending_ids.is_empty() {
            return;
        }
        for message in messages.iter_mut() {
            let Some(obj) = message.as_object_mut() else {
                continue;
            };
            if obj.get("role").and_then(serde_json::Value::as_str) != Some("user") {
                continue;
            }
            let Some(content) = obj.get("content").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let rewritten = self.attach_pending_reloads(content, pending_ids);
            obj.insert("content".to_string(), serde_json::Value::String(rewritten));
        }
    }

    /// 将 ChatMessage 中的待重看引用替换为压缩图。
    pub fn attach_pending_reloads_to_chat(
        &self,
        messages: &mut [ChatMessage],
        pending_ids: &[String],
    ) {
        if pending_ids.is_empty() {
            return;
        }
        for message in messages.iter_mut() {
            if message.role != "user" {
                continue;
            }
            message.content = self.attach_pending_reloads(&message.content, pending_ids);
        }
    }

    /// 测试辅助：公开恢复引用文本路径。
    pub fn rewrite_user_content_preserving_refs_public(&self, content: &str) -> String {
        self.rewrite_user_content_preserving_refs(content)
    }

    fn replacement_for(&self, source_ref: &str) -> String {
        let Some(image) = self.find_image(source_ref) else {
            // 未知引用：保留原标记，避免静默丢图。
            return format!("{IMAGE_MARKER_PREFIX}{source_ref}]");
        };
        if image.is_current && self.should_attach_image_bytes() {
            format!("{IMAGE_MARKER_PREFIX}{}]", image.model_input_data_uri)
        } else {
            image.reference_text.clone()
        }
    }

    fn find_image(&self, source_ref: &str) -> Option<&TrackedImage> {
        self.images.iter().find(|image| {
            image.source_ref == source_ref
                || image.model_input_data_uri == source_ref
                || image.image_id.as_str() == source_ref
        })
    }

    fn rewrite_user_content_preserving_refs(&self, content: &str) -> String {
        // 若内容已是引用文本（含 VW_IMAGE），在首次成功后保持；首次成功前当前图需 data URI。
        if !parse_image_markers(content).1.is_empty() {
            return self.rewrite_content(content);
        }
        if self.should_attach_image_bytes() {
            let mut out = content.to_string();
            for image in self.images.iter().filter(|image| image.is_current) {
                if out.contains(&image.reference_text) {
                    out = out.replace(
                        &image.reference_text,
                        &format!("{IMAGE_MARKER_PREFIX}{}]", image.model_input_data_uri),
                    );
                }
            }
            return out;
        }
        let mut out = content.to_string();
        for image in &self.images {
            let marker = format!("{IMAGE_MARKER_PREFIX}{}]", image.model_input_data_uri);
            if out.contains(&marker) {
                out = out.replace(&marker, &image.reference_text);
            }
            let source_marker = format!("{IMAGE_MARKER_PREFIX}{}]", image.source_ref);
            if out.contains(&source_marker) {
                out = out.replace(&source_marker, &image.reference_text);
            }
        }
        out
    }
}

#[allow(clippy::too_many_arguments)]
async fn persist_and_analyze(
    store: &ImageAssetStore,
    scope: &ImageAssetScope,
    source_ref: &str,
    is_current: bool,
    total_refs: usize,
    config: &MultimodalConfig,
    model_ref: &str,
    analysis_cache: &ImageAnalysisCache,
    extractor: &dyn ImageTextExtractor,
) -> Result<TrackedImage> {
    let materialized = materialize_image_reference(source_ref, config, total_refs.max(1)).await?;
    tracked_from_materialized(
        store,
        scope,
        source_ref,
        is_current,
        materialized,
        model_ref,
        analysis_cache,
        extractor,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn tracked_from_materialized(
    store: &ImageAssetStore,
    scope: &ImageAssetScope,
    source_ref: &str,
    is_current: bool,
    materialized: MaterializedImage,
    model_ref: &str,
    analysis_cache: &ImageAnalysisCache,
    extractor: &dyn ImageTextExtractor,
) -> Result<TrackedImage> {
    let image_id = store.put(
        scope,
        &materialized.original_bytes,
        &materialized.original_mime,
        &materialized.model_input_bytes,
        &materialized.model_input_mime,
    )?;
    let analysis = match analysis_cache
        .analyze_or_load(store, scope, &image_id, model_ref, extractor)
        .await
    {
        Ok(analysis) => analysis,
        Err(error) if is_exhausted_recoverable_analysis_error(&error) => ImageAnalysisResult {
            version: 1,
            image_id: image_id.clone(),
            visible_text: String::new(),
            visual_summary: ANALYSIS_UNAVAILABLE_TEXT.to_string(),
            model_ref: model_ref.to_string(),
            prompt_version: ANALYSIS_PROMPT_VERSION.to_string(),
        },
        Err(error) => return Err(error),
    };
    let reference_text = format_image_reference_text(&image_id, &analysis);
    Ok(TrackedImage {
        image_id,
        is_current,
        source_ref: source_ref.to_string(),
        model_input_data_uri: materialized.model_input_data_uri(),
        reference_text,
        analysis,
    })
}

fn is_exhausted_recoverable_analysis_error(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<MultimodalError>(),
        Some(
            MultimodalError::OcrModelUnavailable { .. }
                | MultimodalError::OcrExtractionFailed { .. }
                | MultimodalError::EmptyOcrOutput { .. }
        )
    )
}

/// 从内容快速判断是否仍含图片字节或本地路径标记。
pub fn content_contains_image_bytes(content: &str) -> bool {
    if content.contains(DATA_IMAGE_PREFIX) {
        return true;
    }
    let (_, refs) = parse_image_markers(content);
    refs.iter().any(|reference| {
        reference.starts_with('/')
            || reference.starts_with("http://")
            || reference.starts_with("https://")
    })
}
