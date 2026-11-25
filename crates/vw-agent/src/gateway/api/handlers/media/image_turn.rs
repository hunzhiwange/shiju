//! Execute an explicitly selected image-understanding model for one chat turn.

use agent_gateway::api::media::{
    MAX_MEDIA_TEXT_BYTES, MEDIA_MODEL_SELECTION_OPTION, MediaModelSelection, MediaSessionQuery,
};
use agent_multimodal::{MultimodalError, materialize_image_reference, parse_image_markers};
use serde_json::{Value, json};
use session_engine::{processor_clock::now_ms, processor_events::StreamEvent};
use session_port::llm_raw::LlmRawArtifact;
use tokio_util::sync::CancellationToken;

use super::{
    Code, Identity, MediaFile, MediaOperation, WORKERS, deployments, failure_code, validate_inputs,
};

pub(in crate::gateway::api::handlers) fn selection(
    options: &Value,
) -> Option<Result<MediaModelSelection, Code>> {
    let value = options.get(MEDIA_MODEL_SELECTION_OPTION)?;
    if value.get("operation") != Some(&json!(MediaOperation::ImageUnderstand)) {
        return None;
    }
    Some(
        serde_json::from_value::<MediaModelSelection>(value.clone())
            .map_err(|_| Code::InvalidRequest)
            .and_then(|selection| {
                selection.validate()?;
                Ok(selection)
            }),
    )
}

pub(in crate::gateway::api::handlers) fn run(
    selection: Result<MediaModelSelection, Code>,
    request: &crate::session::processor::Request,
    mut emit: impl FnMut(StreamEvent) -> bool,
) {
    if let Err(error) = execute_turn(selection, request, &mut emit) {
        emit(StreamEvent::Error(error));
    }
}

fn execute_turn(
    selection: Result<MediaModelSelection, Code>,
    request: &crate::session::processor::Request,
    emit: &mut impl FnMut(StreamEvent) -> bool,
) -> Result<(), String> {
    let selection = selection.map_err(|code| code.message())?;
    if session_engine::helpers::is_acp_request(&request.options)
        || request.options.get("desktop_resume_history_only").and_then(Value::as_bool) == Some(true)
        || !request.persist_app_session_artifacts
    {
        return Err(Code::InvalidRequest.message().into());
    }
    let turn_id = request.assistant_message_id.as_ref().ok_or(Code::InvalidRequest.message())?;
    let context = crate::tools::current_tool_use_context().ok_or(Code::Unauthorized.message())?;
    if context.session() != request.session.as_str() {
        return Err(Code::Forbidden.message().into());
    }
    let (identity, runtime) =
        Identity::from_tool_context(&context).map_err(|code| code.message())?;
    let (prompt, sources) = parse_image_markers(&request.query);
    if sources.is_empty() {
        return Err("请先添加要分析的图片。".into());
    }
    let cancel = request.abort_token.clone().unwrap_or_default();
    let created_ms = now_ms();
    let (model, output) = runtime
        .block_on(async {
            tokio::select! {
                biased;
                () = cancel.cancelled() => Err(Code::Conflict),
                result = understand(identity, &request.session, selection, &prompt, &sources, &cancel) => result,
            }
        })
        .map_err(|code| code.message())?;

    let usage = session_port::ui_types::TokenUsage::default();
    // Keep an inspectable trajectory without persisting the provider-bound image bytes.
    let raw = LlmRawArtifact::from_full_raw_rounds(
        turn_id.as_str(),
        &[json!({
            "session_id": request.session,
            "step_index": 0,
            "model": model,
            "system": [],
            "request": { "tools": [], "config": { "operation": MediaOperation::ImageUnderstand } },
            "messages": [{ "role": "user", "content": prompt, "image_count": sources.len() }],
            "message_sources": [null],
            "output": { "text": output, "finish_reason": "stop" },
        })],
    )
    .map_err(|_| Code::Unavailable.message())?;
    if emit(StreamEvent::StepStart { step_index: 0, created_ms, model: Some(model.clone()) })
        && emit(StreamEvent::Delta(output))
        && emit(StreamEvent::LlmRawArtifact(Box::new(raw)))
        && emit(StreamEvent::StepFinish {
            step_index: 0,
            finished_ms: now_ms(),
            usage: usage.clone(),
            finish_reason: Some("stop".into()),
            model: Some(model),
        })
    {
        emit(StreamEvent::Done(usage));
    }
    Ok(())
}

async fn understand(
    identity: Identity,
    session: &str,
    selection: MediaModelSelection,
    prompt: &str,
    sources: &[String],
    cancel: &CancellationToken,
) -> Result<(String, String), Code> {
    let _worker = WORKERS.try_acquire().map_err(|_| Code::Backpressure)?;
    let context = identity.session(MediaSessionQuery { session_id: session.into() }).await?;
    let directory = std::path::Path::new(&context.directory);
    let config = crate::config::get_project_for_tenant(&identity.tenant, directory)
        .await
        .map_err(|_| Code::Unavailable)?;
    let (max_images, _) = config.multimodal.effective_limits();
    if sources.len() > max_images || prompt.len() > MAX_MEDIA_TEXT_BYTES {
        return Err(Code::PayloadTooLarge);
    }

    let routes = deployments::routes(
        identity.tenant,
        context.directory,
        Some((selection.deployment_id.clone(), selection.model_id.clone())),
    )
    .await?;
    let (route, _) = deployments::select(
        routes,
        selection.operation,
        true,
        Some(&selection.deployment_id),
        Some(&selection.model_id),
    )?;
    let mut files = Vec::with_capacity(sources.len());
    for source in sources {
        let image = materialize_image_reference(source, &config.multimodal, sources.len())
            .await
            .map_err(|error| match error.downcast_ref::<MultimodalError>() {
                Some(MultimodalError::ImageTooLarge { .. }) => Code::PayloadTooLarge,
                Some(MultimodalError::RemoteFetchDisabled { .. }) => Code::Forbidden,
                Some(MultimodalError::ImageSourceNotFound { .. }) => Code::NotFound,
                _ => Code::UnsupportedMedia,
            })?;
        files.push(MediaFile { mime: image.model_input_mime, bytes: image.model_input_bytes });
    }
    let text = (!prompt.is_empty()).then(|| prompt.to_owned());
    validate_inputs(selection.operation, text.as_deref(), &files)?;
    let output = route.endpoint.execute(text, files, cancel, |_| {}).await.map_err(failure_code)?;
    let output = output.text.ok_or(Code::UpstreamFailed)?;
    Ok((format!("{}/{}", route.provider_id, route.model.model_id), output))
}
