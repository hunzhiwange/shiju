//! Validate and persist a source choice before the desktop displays it as selected.
use vw_api_types::{
    acp_model_source::{AcpModelSource, AcpModelSourceCapability, AcpModelSourceUpdate},
    acp_session::AcpSessionOptions,
};

pub(crate) async fn select(
    session: &str,
    directory: &str,
    update: AcpModelSourceUpdate,
) -> Result<AcpSessionOptions, String> {
    let _guard = super::lifecycle::try_lock(session, directory)?;
    let store = super::lifecycle::Store::open(session, directory).map_err(|e| e.to_string())?;
    let previous = store.read().map_err(|e| e.to_string())?;
    if previous.as_ref().is_some_and(super::lifecycle::Selection::has_started) {
        return Err("changing a session model requires a new session".into());
    }
    let config = crate::config::get_current().await.map_err(|e| e.to_string())?;
    if vw_acp::resolve_agent_spec(&update.agent).is_none()
        && !config.acp.contains_key(&update.agent)
    {
        return Err("Unknown ACP agent".into());
    }
    let mut options = AcpSessionOptions::default();
    super::controls::annotate_model_source(&mut options, &update.agent);
    if !options.model_source_capabilities.iter().any(|capability| {
        matches!(capability, AcpModelSourceCapability::Supported { source } if *source == update.selection.source)
    }) {
        return Err("This CLI has no verified entry point for the selected model source".into());
    }
    let mut model = update.selection;
    match model.source {
        AcpModelSource::CliLocal => {
            model.provider_id = None;
            model.model = None;
        }
        AcpModelSource::SessionModel => {
            let resolved = super::super::prompt::resolve_model(model.model.clone()).await?;
            if model.provider_id.as_ref().is_some_and(|id| *id != resolved.provider_id) {
                return Err("ACP model Provider identity mismatch".into());
            }
            let provider = crate::provider::provider::get_provider(&resolved.provider_id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or("Provider is unavailable")?;
            let auth = crate::auth::get(&resolved.provider_id);
            super::request::validate_session_model(&resolved, &provider, auth.as_ref())
                .map_err(|e| e.to_string())?;
            let (_, command) = super::config::lookup_acp_command(
                &config,
                &resolved,
                &serde_json::json!({"acp_agent": update.agent}),
            )
            .ok_or("ACP command is unavailable")?;
            let command = super::config::normalize_acp_agent_config(&update.agent, &command);
            // Exercise the same protocol/configuration boundary as a real launch, then
            // revoke its short-lived relay. No prompt or CLI process is started here.
            drop(
                super::model_source::prepare(&update.agent, &command, &resolved, session)
                    .await
                    .map_err(|e| e.to_string())?,
            );
            // Auto is resolved again at each user turn; preserve the user's auto choice.
            if model.model.as_deref() != Some("auto") {
                model.model = Some(format!("{}/{}", resolved.provider_id, resolved.id));
                model.provider_id = Some(resolved.provider_id);
            }
        }
    }
    let changed =
        previous.as_ref().is_none_or(|old| old.agent != update.agent || old.model != model);
    let generation = previous
        .as_ref()
        .map_or(0, |old| old.generation)
        .checked_add(u64::from(changed))
        .ok_or("ACP source generation exhausted")?;
    let record_id = if changed {
        String::new()
    } else {
        previous.as_ref().map(|old| old.record_id.clone()).unwrap_or_default()
    };
    store
        .commit(&super::lifecycle::Selection {
            agent: update.agent,
            model: model.clone(),
            generation,
            record_id,
        })
        .map_err(|e| e.to_string())?;
    options.acp_model_source = model.source;
    options.selected_model = Some(model);
    Ok(options)
}
