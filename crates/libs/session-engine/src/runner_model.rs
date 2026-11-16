//! Session runner implementation; instantiated with the concrete IO host.
use super::*;

pub fn request_provider_retry_policy<H: RunnerHost>(req: &Request<H>) -> (u32, u64) {
    (req.reliability.provider_retries, req.reliability.provider_backoff_ms)
}

pub fn request_full_access_enabled(options: &Value) -> bool {
    options.get("full_access").and_then(Value::as_bool).unwrap_or(false)
}

pub fn with_request_model_context<H: RunnerHost>(
    mut context: ToolUseContext<H::CtxHost>,
    request_model: Option<&str>,
) -> ToolUseContext<H::CtxHost> {
    let Some(request_model) = request_model.map(str::trim).filter(|value| !value.is_empty()) else {
        return context;
    };
    if let Some((provider, model)) = request_model.split_once('/')
        && !provider.trim().is_empty()
        && !model.trim().is_empty()
    {
        context =
            context.with_provider(provider.trim().to_string()).with_model(model.trim().to_string());
    } else {
        context = context.with_model(request_model.to_string());
    }
    context
}

pub fn is_auto_model_request(model: Option<&str>) -> bool {
    model.map(str::trim).is_none_or(|model| model.is_empty() || model.eq_ignore_ascii_case("auto"))
}

pub fn request_needs_image_input<H: RunnerHost>(req: &Request<H>) -> bool {
    use agent_multimodal::parse_image_markers;

    !parse_image_markers(&req.query).1.is_empty()
        || req.history.iter().any(|message| !parse_image_markers(&message.content).1.is_empty())
}

pub fn resolve_request_model<H: RunnerHost>(
    req: &Request<H>,
    config: &agent_config::runtime_schema::Config,
) -> Result<ResolvedRequestModel, String> {
    let acp_requested = is_acp_request(&req.options);
    let cli_local = acp_requested
        && H::acp_model_source(&req.options)?
            == vw_api_acp_model::acp_model_source::AcpModelSource::CliLocal;
    if cli_local {
        let model = req
            .options
            .get("acp_agent")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .unwrap_or("acp");
        return Ok(ResolvedRequestModel::new(
            format!("acp/{model}"),
            model_types::ModelLimit { context: 8192, input: None, output: 4096 },
        ));
    }

    let providers = H::providers().map_err(|error| error.to_string())?;
    let candidates = providers
        .values()
        .flat_map(|info| info.models.values())
        .filter(|model| !H::is_acp_model(model))
        .cloned()
        .collect::<Vec<_>>();
    let selected = if is_auto_model_request(req.model.as_deref()) {
        let needs_image_input = request_needs_image_input::<H>(req);
        let agent_key = req.options.get("agent").and_then(Value::as_str).map(str::trim);
        let soft_preference = agent_key
            .and_then(|key| config.agents.get(key))
            .filter(|agent| !agent.provider.trim().is_empty() && !agent.model.trim().is_empty())
            .map(|agent| model_selection::ModelReference::new(&agent.provider, &agent.model));
        let candidates_for_selection = candidates
            .iter()
            .map(|model| model_selection::ModelCandidate { provider_id: &model.provider_id, model })
            .collect::<Vec<_>>();
        let decision = model_selection::select_auto_model(
            &model_selection::AutoModelSelectionRequest {
                message: &req.query,
                required_context: 0,
                needs_tools: true,
                needs_image_input,
                needs_attachments: needs_image_input,
                soft_preference,
                query_classification: config.query_classification.clone(),
                model_routes: config.model_routes.clone(),
            },
            &candidates_for_selection,
        )
        .map_err(|error| error.to_string())?;
        candidates
            .into_iter()
            .find(|model| {
                model.provider_id == decision.model.provider && model.id == decision.model.model
            })
            .ok_or_else(|| "Auto 模型选择结果不在 active 候选中".to_string())?
    } else {
        let requested = req.model.as_deref().expect("non-auto request has a model").trim();
        let matches = candidates
            .into_iter()
            .filter(|candidate| {
                if let Some((provider, model)) = requested.split_once('/') {
                    candidate.provider_id == provider && candidate.id == model
                } else {
                    candidate.id == requested
                }
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [model] => model.clone(),
            [] => return Err(format!("未找到模型：{requested}")),
            _ => return Err(format!("模型ID存在歧义：{requested}，请使用 provider/model")),
        }
    };

    Ok(ResolvedRequestModel::new(
        format!("{}/{}", selected.provider_id, selected.id),
        selected.limit,
    ))
}

pub fn enabled_identity_files_for_agent(
    config: &agent_config::runtime_schema::Config,
    agent_key: Option<&str>,
) -> Vec<String> {
    let agent_key = agent_key.map(str::trim).filter(|key| !key.is_empty()).unwrap_or("main");
    config
        .agents
        .get(agent_key)
        .map(|agent| agent.enabled_identity_files.clone())
        .unwrap_or_default()
}

pub fn ensure_persistent_request_message_id<H: RunnerHost>(
    req: &mut Request<H>,
) -> Result<(), String> {
    if req.persist_app_session_artifacts && req.assistant_message_id.is_none() {
        req.assistant_message_id = Some(H::message_id().map_err(|error| error.to_string())?);
    }
    Ok(())
}

#[cfg(test)]
#[path = "runner_model_tests.rs"]
mod tests;
