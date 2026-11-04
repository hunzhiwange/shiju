//! Read and change controls on an already connected ACP session.
use super::{ACP_CLIENT_CACHE, CachedAcpClient};
use agent_client_protocol::{
    SessionConfigKind, SessionConfigOptionCategory, SessionConfigSelectOptions,
};
use std::sync::Arc;
use vw_api_types::acp_session::{
    AcpChoice, AcpOptionKind, AcpSessionOption, AcpSessionOptions, AcpSessionSelection,
};

fn native_advertised(info: vw_acp::SessionCapabilities) -> AcpSessionOptions {
    let mut options = Vec::new();
    let config_options = info.config_options.unwrap_or_default();
    let has_category =
        |category| config_options.iter().any(|option| option.category.as_ref() == Some(&category));
    if let Some(models) = info.models
        && !has_category(SessionConfigOptionCategory::Model)
    {
        options.push(AcpSessionOption {
            kind: AcpOptionKind::Model,
            name: "模型".into(),
            current: models.current_model_id.to_string(),
            choices: models
                .available_models
                .into_iter()
                .map(|m| AcpChoice { id: m.model_id.to_string(), name: m.name })
                .collect(),
        });
    }
    if let Some(modes) = info.modes
        && !has_category(SessionConfigOptionCategory::Mode)
    {
        options.push(AcpSessionOption {
            kind: AcpOptionKind::Mode,
            name: "模式".into(),
            current: modes.current_mode_id.to_string(),
            choices: modes
                .available_modes
                .into_iter()
                .map(|m| AcpChoice { id: m.id.to_string(), name: m.name })
                .collect(),
        });
    }
    for option in config_options {
        let SessionConfigKind::Select(select) = option.kind else { continue };
        let values = match select.options {
            SessionConfigSelectOptions::Ungrouped(values) => values,
            SessionConfigSelectOptions::Grouped(groups) => {
                groups.into_iter().flat_map(|g| g.options).collect()
            }
            _ => continue,
        };
        options.push(AcpSessionOption {
            kind: AcpOptionKind::Config { id: option.id.to_string() },
            name: option.name,
            current: select.current_value.to_string(),
            choices: values
                .into_iter()
                .map(|v| AcpChoice { id: v.value.to_string(), name: v.name })
                .collect(),
        });
    }
    options.retain(|option| !option.choices.is_empty());
    AcpSessionOptions { connected: true, options, ..Default::default() }
}

fn advertised(mut info: vw_acp::SessionCapabilities) -> AcpSessionOptions {
    info.models = None;
    if let Some(options) = info.config_options.as_mut() {
        options
            .retain(|option| option.category.as_ref() != Some(&SessionConfigOptionCategory::Model));
    }
    native_advertised(info)
}

fn project_options(
    info: Option<vw_acp::SessionCapabilities>,
    selection: Option<&super::lifecycle::Selection>,
) -> AcpSessionOptions {
    use vw_api_types::acp_model_source::AcpModelSource;
    let source = selection.map_or(
        if info.is_some() { AcpModelSource::CliLocal } else { AcpModelSource::SessionModel },
        |value| value.model.source,
    );
    let mut options = info
        .map(|info| {
            if source == AcpModelSource::SessionModel {
                advertised(info)
            } else {
                native_advertised(info)
            }
        })
        .unwrap_or_default();
    options.acp_model_source = source;
    options.selected_model = selection.map(|value| value.model.clone());
    if let Some(selection) = selection {
        annotate_model_source(&mut options, &selection.agent);
    }
    options
}

pub(crate) fn annotate_model_source(options: &mut AcpSessionOptions, agent: &str) {
    options.verified_cli_release =
        super::model_source::drivers::verified_release(agent).map(str::to_owned);
    options.verified_platform =
        options.verified_cli_release.as_ref().map(|_| "Darwin arm64".into());
    options.model_source_capabilities = super::model_source::drivers::capabilities(agent);
}

async fn connected(
    session_id: &str,
    directory: &str,
    saved: Option<&super::lifecycle::Selection>,
) -> Result<Option<(vw_acp::SessionRecord, Arc<CachedAcpClient>)>, String> {
    let name = super::session::acp_session_name(session_id);
    let records = vw_acp::list_sessions().await.map_err(|e| e.to_string())?;
    let cache = ACP_CLIENT_CACHE.lock();
    for record in records {
        if saved
            .map_or(record.name != name, |selection| record.vwacp_record_id != selection.record_id)
            || record.cwd != directory
            || record.closed == Some(true)
        {
            continue;
        }
        if let Some(client) = cache.values().find(|c| {
            c.tenant == crate::session::ui_store::current_tenant_scope()
                && record.pid.is_some()
                && c.client.get_agent_lifecycle_snapshot().pid == record.pid
                && c.client.session_capabilities(&record.acp_session_id).is_some()
        }) {
            return Ok(Some((record, client.clone())));
        }
    }
    Ok(None)
}

pub(crate) async fn read(session_id: &str, directory: &str) -> Result<AcpSessionOptions, String> {
    let selection = super::lifecycle::Store::open(session_id, directory)
        .and_then(|store| store.read())
        .map_err(|e| e.to_string())?;
    let info = if let Some((record, client)) =
        connected(session_id, directory, selection.as_ref()).await?
    {
        client.client.session_capabilities(&record.acp_session_id)
    } else {
        None
    };
    Ok(project_options(info, selection.as_ref()))
}

pub(crate) async fn select(
    session_id: &str,
    directory: &str,
    selection: AcpSessionSelection,
) -> Result<AcpSessionOptions, String> {
    let _session_guard = super::lifecycle::try_lock(session_id, directory)?;
    let saved = super::lifecycle::Store::open(session_id, directory)
        .and_then(|store| store.read())
        .map_err(|e| e.to_string())?;
    let (record, client) = connected(session_id, directory, saved.as_ref())
        .await?
        .ok_or("ACP session is not connected")?;
    let _guard = client
        .prompt_lock
        .try_lock()
        .map_err(|_| "ACP session is busy; retry after the current response")?;
    // Reload under the prompt lock so this write cannot overwrite a completed prompt's record.
    let mut record =
        vw_acp::resolve_session_record(&record.vwacp_record_id).await.map_err(|e| e.to_string())?;
    let original_record = record.clone();
    let visible =
        project_options(client.client.session_capabilities(&record.acp_session_id), saved.as_ref());
    if !visible.options.iter().any(|option| {
        option.kind == selection.kind
            && option.choices.iter().any(|choice| choice.id == selection.value)
    }) {
        return Err("ACP option is unavailable for the current model source".into());
    }
    let previous = client
        .client
        .session_capabilities(&record.acp_session_id)
        .map(native_advertised)
        .and_then(|options| {
            options.options.into_iter().find(|option| option.kind == selection.kind)
        })
        .map(|option| AcpSessionSelection { kind: option.kind, value: option.current });
    apply_selection(&client.client, &mut record, selection).await?;
    if let Err(error) = vw_acp::write_session_record(&record).await {
        if let Some(previous) = previous {
            if let Err(rollback) = apply_selection(&client.client, &mut record, previous).await {
                return Err(format!(
                    "Failed to save ACP selection: {error}; backend rollback also failed: {rollback}. Refresh the session options."
                ));
            }
        }
        // The repository may have replaced the record before its index write failed.
        if let Err(restore) = vw_acp::write_session_record(&original_record).await {
            return Err(format!(
                "Failed to save ACP selection: {error}; could not restore its saved value: {restore}"
            ));
        }
        return Err(format!("Failed to save ACP selection: {error}"));
    }
    Ok(project_options(client.client.session_capabilities(&record.acp_session_id), saved.as_ref()))
}

async fn apply_selection(
    client: &vw_acp::AcpClient,
    record: &mut vw_acp::SessionRecord,
    selection: AcpSessionSelection,
) -> Result<(), String> {
    let directory = record.cwd.clone();
    let info =
        client.session_capabilities(&record.acp_session_id).ok_or("ACP session is disconnected")?;
    let options = native_advertised(info);
    if !options
        .options
        .iter()
        .any(|o| o.kind == selection.kind && o.choices.iter().any(|v| v.id == selection.value))
    {
        return Err("ACP backend did not advertise this option value".into());
    }
    match selection.kind {
        AcpOptionKind::Mode => {
            client
                .set_session_mode(&record.acp_session_id, &directory, &selection.value)
                .await
                .map_err(|e| e.to_string())?;
            vw_acp::set_desired_mode_id(record, Some(&selection.value));
            if let Some(state) = record.vwacp.as_mut() {
                state.current_mode_id = Some(selection.value);
            }
        }
        AcpOptionKind::Model => {
            client
                .set_session_model(&record.acp_session_id, &directory, &selection.value)
                .await
                .map_err(|e| e.to_string())?;
            vw_acp::set_desired_model_id(record, Some(&selection.value));
            vw_acp::set_current_model_id(record, Some(&selection.value));
        }
        AcpOptionKind::Config { id } => {
            let response = client
                .set_session_config_option(
                    &record.acp_session_id,
                    &directory,
                    &id,
                    &selection.value,
                )
                .await
                .map_err(|e| e.to_string())?;
            vw_acp::set_desired_config_option(record, &id, &selection.value);
            vw_acp::sync_config_options(record, Some(response.config_options));
        }
    }
    Ok(())
}

/// A confirmed config selector takes precedence over the legacy mode/model/effort fields.
pub(super) fn has_saved_config_category(
    record: &vw_acp::SessionRecord,
    category: SessionConfigOptionCategory,
) -> bool {
    let Some(state) = record.vwacp.as_ref() else {
        return false;
    };
    state.config_options.iter().flatten().any(|option| {
        option.category.as_ref() == Some(&category)
            && state
                .desired_config_options
                .as_ref()
                .is_some_and(|values| values.contains_key(option.id.0.as_ref()))
    })
}

#[cfg(test)]
#[path = "controls_tests.rs"]
mod tests;
