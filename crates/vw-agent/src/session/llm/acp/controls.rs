//! Read and change controls on an already connected ACP session.
use super::{ACP_CLIENT_CACHE, CachedAcpClient};
use agent_client_protocol::{
    SessionConfigKind, SessionConfigOptionCategory, SessionConfigSelectOptions,
};
use std::sync::Arc;
use vw_api_types::acp_session::{
    AcpChoice, AcpOptionKind, AcpSessionOption, AcpSessionOptions, AcpSessionSelection,
};

pub(super) fn is_model_config(option: &agent_client_protocol::SessionConfigOption) -> bool {
    option.category == Some(SessionConfigOptionCategory::Model) || option.id.0.as_ref() == "model"
}

fn managed_effort_id(agent: &str) -> Option<&'static str> {
    match agent {
        "codex" => Some("reasoning_effort"),
        "opencode" | "claude" => Some("effort"),
        _ => None,
    }
}

pub(super) fn is_managed_effort_config(
    agent: &str,
    option: &agent_client_protocol::SessionConfigOption,
) -> bool {
    managed_effort_id(agent) == Some(option.id.0.as_ref())
        && match agent {
            "claude" => matches!(
                option.category.as_ref(),
                Some(SessionConfigOptionCategory::Other(category)) if category == "effort"
            ),
            _ => option.category == Some(SessionConfigOptionCategory::ThoughtLevel),
        }
}

fn is_permission_config_id(id: &str) -> bool {
    // Protocol mode and the separate Hermes control found in the fixed-runtime matrix.
    matches!(id, "mode" | "edit_approval_policy")
}

pub(super) fn is_permission_config(option: &agent_client_protocol::SessionConfigOption) -> bool {
    is_permission_config_id(option.id.0.as_ref())
        || match &option.category {
            Some(SessionConfigOptionCategory::Mode) => true,
            Some(SessionConfigOptionCategory::Other(category)) => {
                matches!(category.as_str(), "permission" | "permissions" | "approval" | "sandbox")
            }
            _ => false,
        }
}

fn reject_public_permission_selection(
    kind: &AcpOptionKind,
    info: Option<&vw_acp::SessionCapabilities>,
) -> Result<(), String> {
    let permission = match kind {
        AcpOptionKind::Mode => true,
        AcpOptionKind::Config { id } => {
            is_permission_config_id(id)
                || info.and_then(|info| info.config_options.as_ref()).is_some_and(|options| {
                    options
                        .iter()
                        .any(|option| option.id.0.as_ref() == id && is_permission_config(option))
                })
        }
        AcpOptionKind::Model => false,
    };
    if permission {
        return Err(
            "Native ACP permission controls are private; use the session full_access setting"
                .into(),
        );
    }
    Ok(())
}

fn native_advertised(info: vw_acp::SessionCapabilities) -> AcpSessionOptions {
    let mut options = Vec::new();
    let config_options = info.config_options.unwrap_or_default();
    let has_category =
        |category| config_options.iter().any(|option| option.category.as_ref() == Some(&category));
    if let Some(models) = info.models
        && !config_options.iter().any(is_model_config)
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

fn public_advertised(mut info: vw_acp::SessionCapabilities) -> AcpSessionOptions {
    info.modes = None;
    if let Some(options) = info.config_options.as_mut() {
        options.retain(|option| !is_permission_config(option));
    }
    native_advertised(info)
}

fn native_read_only(info: vw_acp::SessionCapabilities) -> AcpSessionOptions {
    let model_options = info
        .config_options
        .iter()
        .flatten()
        .filter(|option| is_model_config(option))
        .map(|option| option.id.to_string())
        .collect::<Vec<_>>();
    let mut options = public_advertised(info);
    for option in &mut options.options {
        let locked = match &option.kind {
            AcpOptionKind::Model => true,
            AcpOptionKind::Config { id } => model_options.contains(id),
            AcpOptionKind::Mode => false,
        };
        if locked {
            option.kind = AcpOptionKind::Model;
            option.choices.retain(|choice| choice.id == option.current);
        }
    }
    options
}

fn advertised(mut info: vw_acp::SessionCapabilities) -> AcpSessionOptions {
    info.models = None;
    if let Some(options) = info.config_options.as_mut() {
        options.retain(|option| !is_model_config(option));
    }
    public_advertised(info)
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
                native_read_only(info)
            }
        })
        .unwrap_or_default();
    options.acp_model_source = source;
    options.selected_model = selection.map(|value| {
        let mut model = value.model.clone();
        if value.has_started()
            && let Some((provider, id)) = model.provider_id.as_ref().zip(model.model.as_mut())
        {
            *id = format!("{provider}/{id}");
        }
        model
    });
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
    if options.selected_model.is_none() && !options.connected {
        use vw_api_types::acp_model_source::{AcpModelSource, AcpModelSourceCapability};
        options.acp_model_source = if options.model_source_capabilities.iter().any(|capability| {
            matches!(
                capability,
                AcpModelSourceCapability::Supported { source: AcpModelSource::SessionModel }
            )
        }) {
            AcpModelSource::SessionModel
        } else {
            AcpModelSource::CliLocal
        };
    }
}

fn annotate_access(
    options: &mut AcpSessionOptions,
    client: &vw_acp::AcpClient,
    record: &vw_acp::SessionRecord,
) {
    use vw_acp::{AccessPolicyReason, FullAccessSupport};
    use vw_api_types::acp_session::{AcpAccessStatus, AcpAccessSupport};
    let (requested, support, explanation) =
        match client.session_access_policy(&record.acp_session_id) {
            Err(_) => (
                client.access_policy(None, None).full_access_requested,
                AcpAccessSupport::Failed,
                "权限同步尚未确认或会话已断开；请刷新或重试",
            ),
            Ok(policy) => {
                let (support, explanation) = match policy.support {
                    FullAccessSupport::Native => {
                        (AcpAccessSupport::Native, "当前原生权限控制已确认；仍受后端与组织策略约束")
                    }
                    FullAccessSupport::Partial => (
                        AcpAccessSupport::Partial,
                        "仅部分权限生效；原生工具与客户端权限边界不同，仍可能要求审批",
                    ),
                    FullAccessSupport::ClientOnly => (
                        AcpAccessSupport::ClientOnly,
                        "仅客户端工具审批自动批准；后端原生免审批未验证，仍可能要求确认",
                    ),
                    FullAccessSupport::Unavailable => (
                        AcpAccessSupport::Unavailable,
                        match policy.reason {
                            AccessPolicyReason::DeniedByHost => "执行主机策略拒绝工具权限",
                            _ => "按工作区安全边界审批；后端原生工具可能有额外限制",
                        },
                    ),
                };
                (policy.full_access_requested, support, explanation)
            }
        };
    options.access_status = Some(AcpAccessStatus {
        requested,
        backend: client.agent_name().into(),
        version: client.agent_version(),
        support,
        explanation: explanation.into(),
    });
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

// Managed launches release their process and relay at turn end.
// The request stores only its advertised effort control for use between turns.
async fn parked_effort_record(
    saved: &super::lifecycle::Selection,
    directory: &str,
) -> Result<Option<vw_acp::SessionRecord>, String> {
    if !saved.has_started()
        || managed_effort_id(&saved.agent).is_none()
        || saved.model.source != vw_api_types::acp_model_source::AcpModelSource::SessionModel
    {
        return Ok(None);
    }
    let record = vw_acp::resolve_session_record(&saved.record_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok((record.vwacp_record_id == saved.record_id
        && record.cwd == directory
        && record.closed != Some(true)
        && record.agent_command.starts_with("vw-session-model:"))
    .then_some(record))
}

fn stored_effort<'a>(
    agent: &str,
    state: &'a vw_acp::SessionAcpxState,
) -> Option<&'a agent_client_protocol::SessionConfigOption> {
    state.config_options.iter().flatten().find(|option| is_managed_effort_config(agent, option))
}

fn parked_effort_options(
    saved: &super::lifecycle::Selection,
    record: &vw_acp::SessionRecord,
) -> AcpSessionOptions {
    let mut projected = project_options(None, Some(saved));
    let Some(state) = &record.vwacp else { return projected };
    let Some(option) = stored_effort(&saved.agent, state) else { return projected };
    let mut options = native_advertised(vw_acp::SessionCapabilities {
        session_id: record.acp_session_id.clone(),
        models: None,
        modes: None,
        config_options: Some(vec![option.clone()]),
    })
    .options;
    let desired =
        state.desired_config_options.as_ref().and_then(|values| values.get(option.id.0.as_ref()));
    for option in &mut options {
        if let Some(desired) =
            desired.filter(|value| option.choices.iter().any(|choice| &choice.id == *value))
        {
            option.current.clone_from(desired);
        }
    }
    projected.options = options;
    projected
}

pub(crate) async fn read(session_id: &str, directory: &str) -> Result<AcpSessionOptions, String> {
    let selection = super::lifecycle::Store::open(session_id, directory)
        .and_then(|store| store.read())
        .map_err(|e| e.to_string())?;
    let connection = connected(session_id, directory, selection.as_ref()).await?;
    let info = connection
        .as_ref()
        .and_then(|(record, client)| client.client.session_capabilities(&record.acp_session_id));
    let mut options = project_options(info, selection.as_ref());
    if let Some((record, client)) = connection.as_ref() {
        annotate_access(&mut options, &client.client, record);
    } else if let Some(saved) = selection.as_ref()
        && let Some(record) = parked_effort_record(saved, directory).await?
    {
        options = parked_effort_options(saved, &record);
    }
    Ok(options)
}

pub(crate) async fn select(
    session_id: &str,
    directory: &str,
    selection: AcpSessionSelection,
) -> Result<AcpSessionOptions, String> {
    reject_public_permission_selection(&selection.kind, None)?;
    let _session_guard = super::lifecycle::try_lock(session_id, directory)?;
    let saved = super::lifecycle::Store::open(session_id, directory)
        .and_then(|store| store.read())
        .map_err(|e| e.to_string())?;
    let Some((record, client)) = connected(session_id, directory, saved.as_ref()).await? else {
        let AcpOptionKind::Config { id } = &selection.kind else {
            return Err("ACP session is not connected".into());
        };
        let saved = saved.as_ref().ok_or("ACP session is not connected")?;
        let mut record =
            parked_effort_record(saved, directory).await?.ok_or("ACP session is not connected")?;
        let option = record
            .vwacp
            .as_ref()
            .and_then(|state| stored_effort(&saved.agent, state))
            .filter(|option| option.id.0.as_ref() == id)
            .ok_or("ACP option is unavailable for the current model source")?;
        let id =
            vw_acp::resolve_config_value(Some(std::slice::from_ref(option)), id, &selection.value)?;
        let original = record.clone();
        vw_acp::set_desired_config_option(&mut record, &id, &selection.value);
        if let Err(error) = vw_acp::write_session_record(&record).await {
            if let Err(restore) = vw_acp::write_session_record(&original).await {
                return Err(format!(
                    "Failed to save ACP selection: {error}; could not restore its saved value: {restore}"
                ));
            }
            return Err(format!("Failed to save ACP selection: {error}"));
        }
        return Ok(parked_effort_options(saved, &record));
    };
    let _guard = client
        .prompt_lock
        .try_lock()
        .map_err(|_| "ACP session is busy; retry after the current response")?;
    // Reload under the prompt lock so this write cannot overwrite a completed prompt's record.
    let mut record =
        vw_acp::resolve_session_record(&record.vwacp_record_id).await.map_err(|e| e.to_string())?;
    let capabilities = client.client.session_capabilities(&record.acp_session_id);
    reject_public_permission_selection(&selection.kind, capabilities.as_ref())?;
    prepare_access_restore(&client.client, &mut record);
    let original_record = record.clone();
    let is_model = match &selection.kind {
        AcpOptionKind::Model => true,
        AcpOptionKind::Config { id } => capabilities
            .as_ref()
            .and_then(|info| info.config_options.as_ref())
            .is_some_and(|options| {
                options.iter().any(|option| option.id.0.as_ref() == id && is_model_config(option))
            }),
        AcpOptionKind::Mode => false,
    };
    if is_model {
        return Err("changing a session model requires a new session".into());
    }
    let visible = project_options(capabilities, saved.as_ref());
    if !visible.options.iter().any(|option| {
        option.kind == selection.kind
            && option.choices.iter().any(|choice| choice.id == selection.value)
    }) {
        return Err("ACP option is unavailable for the current model source".into());
    }
    let previous = visible
        .options
        .into_iter()
        .find(|option| option.kind == selection.kind)
        .map(|option| AcpSessionSelection { kind: option.kind, value: option.current });
    apply_selection(&client.client, &mut record, selection).await?;
    if let Err(error) = vw_acp::write_session_record(&record).await {
        if let Some(previous) = previous
            && let Err(rollback) = apply_selection(&client.client, &mut record, previous).await
        {
            return Err(format!(
                "Failed to save ACP selection: {error}; backend rollback also failed: {rollback}. Refresh the session options."
            ));
        }
        // The repository may have replaced the record before its index write failed.
        if let Err(restore) = vw_acp::write_session_record(&original_record).await {
            return Err(format!(
                "Failed to save ACP selection: {error}; could not restore its saved value: {restore}"
            ));
        }
        return Err(format!("Failed to save ACP selection: {error}"));
    }
    let mut options =
        project_options(client.client.session_capabilities(&record.acp_session_id), saved.as_ref());
    annotate_access(&mut options, &client.client, &record);
    Ok(options)
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

/// Keep a non-elevated baseline in desired preferences, separate from readback.
/// Revocation also sanitizes legacy records that saved an elevated mode.
pub(super) fn prepare_access_restore(
    client: &vw_acp::AcpClient,
    record: &mut vw_acp::SessionRecord,
) {
    let Some(state) = record.vwacp.as_ref() else { return };
    let mode = state
        .desired_mode_id
        .as_ref()
        .map(|value| client.restricted_permission_value("mode", value).to_owned())
        .or_else(|| {
            state
                .current_mode_id
                .as_ref()
                .map(|value| client.restricted_permission_value("mode", value).to_owned())
        });
    let mut desired = vw_acp::get_desired_config_options(Some(state));
    for option in state.config_options.iter().flatten() {
        if (option.category == Some(SessionConfigOptionCategory::Mode)
            || matches!(option.id.0.as_ref(), "mode" | "edit_approval_policy"))
            && let SessionConfigKind::Select(select) = &option.kind
        {
            desired.entry(option.id.to_string()).or_insert_with(|| {
                client
                    .restricted_permission_value(
                        &option.id.to_string(),
                        &select.current_value.to_string(),
                    )
                    .to_owned()
            });
        }
    }
    if let Some(mode) = mode {
        vw_acp::set_desired_mode_id(record, Some(&mode));
    }
    for (id, value) in desired {
        vw_acp::set_desired_config_option(
            record,
            &id,
            client.restricted_permission_value(&id, &value),
        );
    }
}
