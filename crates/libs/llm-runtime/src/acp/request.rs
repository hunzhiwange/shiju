//! ACP 流式请求执行入口。
//!
//! 本模块负责把一次 VibeWindow LLM 请求映射为 ACP prompt 运行：解析代理配置、
//! 查找或创建 ACP 会话、转发 ACP 输出事件，并在会话变化等可恢复错误上执行一次明确重试。

use std::sync::Arc;
use serde_json::Value;
use tokio::sync::mpsc;
use acp_agent_registry::{EMPLOYEE_ACP_BACKEND, EMPLOYEE_ACP_CREDENTIAL_ENV, normalize_agent_name};
use acp_types::{AcpJsonRpcMessage, PromptRequest, SessionStrategy};
use acp_lifecycle::apply_lifecycle_snapshot_to_record;
use acp_storage::write_session_record;

use agent_auth as auth;
use agent_config as config;
use model_types as provider;
use message_types as message;
use session_ui_types as ui_types;
use super::host::{Host, SkillLease};
use super::options::AcpReplayStrategy;
use super::session::CachedAcpClientEntry;
use crate::{Error, StreamEvent};

use super::config::{lookup_acp_command, normalize_acp_agent_config, parse_acp_options};
use super::replay::{build_request_prompt, parse_recent_count, parse_replay_strategy};
use super::session::{
    acp_session_name, close_cached_acp_client_if_unshared, find_or_create_cached_session_record,
    get_cached_acp_client_with_approval, should_abort,
};
use super::updates::{AcpToolUpdates, forward_acp_message};
use super::{acp_option_error, is_acp_session_changed_error, to_api_error};

/// 执行一次 ACP 流式请求并把结果转发给上层事件消费者。
///
/// 参数说明：
/// - `model`：当前模型配置，用于解析对应 ACP 代理。
/// - `merged_options`：运行时选项，包含 cwd、ACP 权限、会话模式和历史策略等。
/// - `chat_messages`：OpenAI 风格的本地会话消息。
/// - `session_id`：VibeWindow 本地会话 id，用于生成稳定 ACP 会话名。
/// - `abort`：可选取消信号；触发后会尽力取消 ACP 端会话。
/// - `on_event`：流式事件回调，接收文本增量、推理增量、完成和错误事件。
///
/// 返回值为 `Ok(())` 表示请求完成并已发送 `Done` 事件；错误会保留为上层统一
/// `Error`。配置缺失、空 prompt、ACP 调用失败和取消都会作为错误返回。
pub async fn do_stream_request_acp<H: Host>(
    model: &provider::Model,
    provider_info: &provider::Info,
    auth_info: Option<&auth::Info>,
    merged_options: &Value,
    chat_messages: &Value,
    session_id: &str,
    abort: Option<&tokio::sync::watch::Receiver<bool>>,
    on_event: &mut impl FnMut(StreamEvent),
) -> Result<(), Error> {
    if should_abort(abort) {
        tracing::debug!(
            target: "vw_agent",
            model = %model.api.id,
            "ACP stream aborted before request start"
        );
        on_event(StreamEvent::Error(message::AssistantError::MessageAbortedError {
            message: "aborted".to_string(),
        }));
        return Err(Error::Aborted);
    }

    use vw_api_acp_model::acp_model_source::{AcpModelSource, AcpModelSourceCapability};
    let cfg = H::config().await.map_err(to_api_error)?;
    let (acp_agent_name, acp_cfg) = lookup_acp_command(&cfg, model, merged_options)
        .ok_or_else(|| to_api_error(format!("acp command not configured for model {}", model.api.id)))?;
    let employee = normalize_agent_name(&acp_agent_name) == EMPLOYEE_ACP_BACKEND;
    let (acp_agent_name, acp_cfg, employee_text, employee_stop, source) = if employee {
        let launch = H::employee_acp_launch(session_id)
            .ok_or_else(|| acp_option_error("Employee ACP requires an authorized session binding"))?;
        let stop = launch.stop.clone();
        (
            EMPLOYEE_ACP_BACKEND.to_owned(),
            employee_launch_config(launch, H::employee_acp_executable().map_err(to_api_error)?)?,
            Some(super::replay::employee_text(chat_messages)?),
            stop,
            AcpModelSource::CliLocal,
        )
    } else {
        let source = H::model_source(merged_options).map_err(acp_option_error)?;
        if source == AcpModelSource::SessionModel {
            validate_session_model(model, provider_info, auth_info)?;
        }
        (acp_agent_name, acp_cfg, None, None, source)
    };

    let tool_context = H::tool_context();
    let desktop_context = tool_context.as_ref().and_then(H::desktop_context);
    let requested_cwd = merged_options
        .get("cwd")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(std::path::PathBuf::from);
    let (cwd, desktop_access) = if let Some(context) = desktop_context.as_ref() {
        let (cwd, root) =
            super::session::desktop_workspace(context, session_id, requested_cwd.as_deref())?;
        (cwd, Some((root, context.full_access)))
    } else {
        let cwd = requested_cwd
            .or_else(|| std::env::current_dir().ok())
            .ok_or_else(|| to_api_error("failed to resolve cwd for acp session"))?;
        (cwd, None)
    };
    let worktree_use =
        H::worktree_lease(&cwd).map_err(to_api_error)?;
    let cwd_text = cwd.to_string_lossy().into_owned();
    let _session_guard = super::lifecycle::try_lock(&H::tenant(), session_id, &cwd_text).map_err(to_api_error)?;
    let selection_store =
        H::open_selection(session_id, &cwd_text).map_err(to_api_error)?;
    let previous_selection = selection_store.read().map_err(to_api_error)?;
    let mut fork_replay =
        H::fork_replay(session_id, &cwd_text).await.map_err(to_api_error)?;
    let fork_history_needed = fork_replay.as_ref().is_some_and(|state| state.needs_history());
    if employee && fork_replay.is_some() {
        return Err(acp_option_error("Employee ACP does not support local history forks"));
    }
    if !employee && source == AcpModelSource::CliLocal
        && !super::model_source::drivers::capabilities(&acp_agent_name).iter().any(|capability| {
            matches!(capability, AcpModelSourceCapability::Supported { source: AcpModelSource::CliLocal })
        })
    {
        return Err(acp_option_error(
            "This CLI has no verified entry point for the selected model source",
        ));
    }
    if !employee {
        H::reject_expanded(&acp_agent_name, chat_messages)?;
    }
    let selected_model = vw_api_acp_model::acp_model_source::AcpSessionModelSelection {
        source,
        provider_id: (source == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel)
            .then(|| model.provider_id.clone()),
        model: (source == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel)
            .then(|| model.id.clone()),
    };
    let locked_selection = previous_selection.as_ref().filter(|selection| selection.has_started());
    if locked_selection
        .is_some_and(|old| old.agent != acp_agent_name || old.model != selected_model)
    {
        return Err(to_api_error("changing a session agent or model requires a new session"));
    }
    let source_changed = previous_selection
        .as_ref()
        .is_some_and(|old| old.model.source != source || old.agent != acp_agent_name);
    let selected_generation = previous_selection
        .as_ref()
        .map_or(0, |old| old.generation)
        .checked_add(u64::from(source_changed))
        .ok_or_else(|| to_api_error("ACP source generation exhausted"))?;
    let identity = super::lifecycle::session_identity(&H::tenant(), session_id, &cwd_text);
    let mut normalized_acp_cfg = normalize_acp_agent_config(&acp_agent_name, &acp_cfg);
    // Older model selection writes could clear the anchor of a running session.
    let recovered = if !employee && previous_selection.is_some() && locked_selection.is_none() {
        let launch = match source {
            vw_api_acp_model::acp_model_source::AcpModelSource::CliLocal => {
                super::recovery::Launch::Local {
                    agent: &acp_agent_name,
                    config: &normalized_acp_cfg,
                }
            }
            vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel => {
                let project = H::project_id()
                    .ok_or_else(|| to_api_error("ACP launch requires project context"))?;
                super::recovery::Launch::Managed(super::model_source::launch_identity(
                    H::tenant(), project,
                    &acp_agent_name,
                    model,
                    "",
                ))
            }
        };
        super::recovery::recover_session(
            session_id,
            &identity,
            &cwd_text,
            selected_generation,
            &launch,
            |agent, config| H::scanner(agent, config, None).map_err(|error| anyhow::anyhow!(error.to_string())),
        )
        .await
        .map_err(to_api_error)?
    } else {
        None
    };
    let generation = recovered.as_ref().map_or(selected_generation, |session| session.generation);
    let scoped_session = format!("{session_id}:acp-source-{generation}:{identity}");
    let mut cfg = cfg;
    let (mut parsed_options, chat_messages) = if employee {
        (
            super::options::ParsedAcpOptions::<H::Skills> {
                permission_mode: Some(acp_types::PermissionMode::DenyAll),
                non_interactive_permissions: Some(acp_types::NonInteractivePermissionPolicy::Deny),
                ..Default::default()
            },
            Value::Null,
        )
    } else {
        let mut parsed = parse_acp_options::<H::Skills>(merged_options, &acp_agent_name, &normalized_acp_cfg)?;
        H::bind_agent(session_id, &mut cfg).await.map_err(to_api_error)?;
        let messages = H::agent_defaults(&cfg, merged_options, &acp_agent_name, &cwd, &mut parsed, chat_messages).await?;
        (parsed, messages)
    };
    validate_linux_session_mcp(&acp_agent_name, source, !parsed_options.mcp_servers.is_empty())?;
    let mut restricted_auggie = false;
    let mut restricted_hermes = false;
    let mut restricted_droid = false;
    let mut restricted_kilocode = false;
    let mut restricted_pi = false;
    let mut restricted_opencode = false;
    let mut restricted_openclaw = false;
    if !employee && let Some((root, full_grant)) = desktop_access {
        super::session::bind_desktop_access(&mut parsed_options, root, full_grant);
        restricted_auggie = acp_agent_name == "auggie"
            && !parsed_options.full_access
            && parsed_options.permission_mode == Some(acp_types::PermissionMode::ApproveReads)
            && merged_options.get("disable_tools").and_then(Value::as_bool) != Some(true)
            && cfg!(all(target_os = "macos", target_arch = "aarch64"))
            && source == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel;
        restricted_droid = acp_agent_name == "droid"
            && parsed_options.permission_mode == Some(acp_types::PermissionMode::ApproveReads)
            && merged_options.get("disable_tools").and_then(Value::as_bool) != Some(true)
            && cfg!(all(target_os = "macos", target_arch = "aarch64"))
            && source == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel;
        restricted_hermes = acp_agent_name == "hermes"
            && parsed_options.permission_mode == Some(acp_types::PermissionMode::ApproveReads)
            && merged_options.get("disable_tools").and_then(Value::as_bool) != Some(true)
            && cfg!(all(target_os = "macos", target_arch = "aarch64"))
            && source == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel;
        restricted_openclaw = acp_agent_name == "openclaw"
            && super::skill_mounts::openclaw::restricted_launch(
                source,
                parsed_options.permission_mode,
                parsed_options.full_access,
                parsed_options.non_interactive_permissions.is_some(),
                merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true),
            );
        restricted_opencode = acp_agent_name == "opencode"
            && !parsed_options.full_access
            && cfg!(all(target_os = "macos", target_arch = "aarch64"))
            && source == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel;
        restricted_pi = super::skill_mounts::pi::restricted_launch(
            &acp_agent_name, source, parsed_options.permission_mode,
            merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true),
        );
        restricted_kilocode = acp_agent_name == "kilocode"
            && !parsed_options.full_access
            && parsed_options.permission_mode == Some(acp_types::PermissionMode::ApproveReads)
            && merged_options.get("disable_tools").and_then(Value::as_bool) != Some(true)
            && cfg!(all(target_os = "macos", target_arch = "aarch64"))
            && source == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel;
        if !restricted_droid && !restricted_auggie && !restricted_kilocode && !restricted_hermes && !restricted_opencode && !restricted_pi && !restricted_openclaw {
            acp_client::AcpClient::ensure_desktop_native_access(
                &acp_agent_name,
                parsed_options.full_access,
            )
            .map_err(to_api_error)?;
        }
    }
    if acp_agent_name == "auggie" {
        if restricted_auggie {
            acp_agent_registry::verify_auggie_restricted_executable(&normalized_acp_cfg.command)
                .map_err(to_api_error)?;
        } else if !parsed_options.skill_mounts.is_empty() {
            return Err(acp_option_error("Auggie selected skills require its restricted session-model launch"));
        }
    }
    if acp_agent_name == "droid" {
        if restricted_droid {
            acp_agent_registry::verify_droid_restricted_executable(&normalized_acp_cfg.command)
                .map_err(to_api_error)?;
            if recovered.is_some() || fork_replay.is_some() {
                return Err(acp_option_error("Restricted Droid requires a new session"));
            }
        } else if !parsed_options.skill_mounts.is_empty() {
            return Err(acp_option_error("Droid selected skills require its restricted session-model launch"));
        }
    }
    parsed_options.runtime_identity = Some(scoped_session.clone());
    if !employee {
        parsed_options.skill_view =
            H::prepare_skills(&parsed_options.skill_mounts, &cwd, &scoped_session, &cfg)?;
    }
    if acp_agent_name == "openclaw" {
        use super::model_source::drivers::openclaw::RESTRICTED_ENV;
        if normalized_acp_cfg.env.keys().any(|key| key.eq_ignore_ascii_case(RESTRICTED_ENV)) {
            return Err(acp_option_error("Reserved OpenClaw launch environment"));
        }
        if restricted_openclaw {
            if recovered.is_some() || fork_replay.is_some() {
                return Err(acp_option_error("Restricted OpenClaw requires an owned new session"));
            }
            normalized_acp_cfg.env.insert(RESTRICTED_ENV.into(), "1".into());
        }
        H::scanner(
            &acp_agent_name,
            &mut normalized_acp_cfg,
            parsed_options.skill_view.as_ref(),
        )?;
        if source == vw_api_acp_model::acp_model_source::AcpModelSource::CliLocal
            && parsed_options.skill_view.is_some()
        {
            return Err(acp_option_error(
                "OpenClaw skill mounts require an owned session-model Gateway; cli_local Gateway isolation is unverified",
            ));
        }
    }
    if acp_agent_name == "copilot"
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        normalized_acp_cfg.env.remove(super::model_source::drivers::copilot::PROJECT_SKILLS_ENV);
    }
    if acp_agent_name == "kimi"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error(
            "Kimi skill mounts require session_model with native tools enabled",
        ));
    }
    if acp_agent_name == "grok"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error(
            "Grok Build skill mounts require session_model with native tools enabled",
        ));
    }
    let cursor_skill_reads_disabled = acp_agent_name == "cursor"
        && (parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true));
    if acp_agent_name == "cursor"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || cursor_skill_reads_disabled)
    {
        return Err(acp_option_error(
            "Cursor skill mounts require session_model with native tools enabled",
        ));
    }
    if acp_agent_name == "junie"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error("Junie skill mounts require session_model with native tools enabled"));
    }
    if acp_agent_name == "qoder"
        && parsed_options.skill_view.is_some()
        && source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
    {
        return Err(acp_option_error("Qoder skill mounts require session_model"));
    }
    if acp_agent_name == "trae"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error(
            "TRAE skill mounts require session_model with native tools enabled",
        ));
    }
    if acp_agent_name == "corust-agent"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error(
            "Corust skill mounts require session_model with native tools enabled",
        ));
    }
    if acp_agent_name == "cortex-code"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error(
            "Cortex skill mounts require session_model with native tools enabled",
        ));
    }
    if acp_agent_name == "sigit"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error(
            "siGit skill mounts require session_model with native tools enabled",
        ));
    }
    if acp_agent_name == "glm-acp-agent"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error("GLM skill mounts require session_model with native tools enabled"));
    }
    if !employee {
        H::supplier(&acp_agent_name, parsed_options.skill_view.as_ref())?;
    }
    if acp_agent_name == "deepagents"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error(
            "DeepAgents skill mounts require session_model with native tools enabled",
        ));
    }
    if acp_agent_name == "dirac"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error(
            "Dirac skill mounts require session_model with native tools enabled",
        ));
    }
    if acp_agent_name == "omp"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error("OMP skill mounts require session_model with native reads enabled"));
    }
    if acp_agent_name == "mimo-code"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error("MiMo Code skill mounts require session_model with native reads enabled"));
    }
    if acp_agent_name == "nova"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error(
            "Nova skill mounts require session_model with native tools enabled",
        ));
    }
    let managed_autohand = acp_agent_name == "autohand"
        && cfg!(all(target_os = "macos", target_arch = "aarch64"))
        && source == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
        && super::skill_mounts::autohand::matches_managed_entry(&normalized_acp_cfg.command);
    let autohand_selected = acp_agent_name == "autohand" && parsed_options.skill_view.is_some();
    let autohand_mcp = managed_autohand && !parsed_options.mcp_servers.is_empty();
    if acp_agent_name == "autohand" && normalized_acp_cfg.env.keys().any(|key| {
        key.eq_ignore_ascii_case(super::skill_mounts::autohand::MANAGED)
            || key.eq_ignore_ascii_case(super::skill_mounts::autohand::ROOT)
            || key.eq_ignore_ascii_case(super::skill_mounts::autohand::WORKSPACE)
    }) {
        return Err(acp_option_error("Reserved AutoHand skill and MCP launch controls"));
    }
    if (autohand_selected || autohand_mcp) && (!managed_autohand
        || parsed_options.full_access
        || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
        || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true)
        || recovered.is_some() || fork_replay.is_some())
    {
        return Err(acp_option_error("AutoHand requires a reviewed session_model launch with host approvals"));
    }
    if acp_agent_name == "stakpak"
        && parsed_options.skill_view.is_some()
        && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error("Stakpak skill mounts require session_model with native reads enabled"));
    }
    if !employee {
        H::validate_mounts(&acp_agent_name, !parsed_options.skill_mounts.is_empty())?;
    }

    if acp_agent_name == "dimcode"
        && (!parsed_options.mcp_servers.is_empty() || parsed_options.skill_view.is_some())
    {
        if parsed_options.mcp_servers.iter().any(|server| {
            matches!(server, agent_client_protocol::McpServer::Stdio(_))
        }) {
            return Err(acp_option_error(
                "DimCode does not support ACP stdio MCP; select an HTTP or SSE server",
            ));
        }
        if source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
            || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true)
        {
            return Err(acp_option_error(
                "DimCode skills and MCP require session_model with enabled tool approvals",
            ));
        }
        acp_agent_registry::verify_reviewed_agent_executable("dimcode", &normalized_acp_cfg.command)
            .map_err(acp_option_error)?;
    }

    let previous_native = if !employee && source == vw_api_acp_model::acp_model_source::AcpModelSource::CliLocal
        && (recovered.is_some() || locked_selection.is_some())
    {
        let state = if let Some(recovered) = &recovered {
            recovered.record.vwacp.clone()
        } else if let Some(previous) = locked_selection {
            acp_storage::resolve_session_record(&previous.record_id).await.map_err(to_api_error)?.vwacp
        } else {
            None
        };
        let model = acp_access_policy::session_mode_preference::get_desired_model_id(state.as_ref());
        parsed_options.session_options.get_or_insert_with(Default::default).model = model;
        state
    } else {
        None
    };

    let vtcode_selected = acp_agent_name == "vtcode" && parsed_options.skill_view.is_some();
    let vtcode_reads = parsed_options.permission_mode != Some(acp_types::PermissionMode::DenyAll)
        && merged_options.get("disable_tools").and_then(Value::as_bool) != Some(true)
        && recovered.is_none()
        && fork_replay.is_none();
    if vtcode_selected {
        if !vtcode_reads
            || source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
        {
            return Err(acp_option_error("VT Code skills require a new managed session with reads enabled"));
        }
        acp_agent_registry::verify_vtcode_skill_executable(&normalized_acp_cfg.command)
            .map_err(acp_option_error)?;
    }
    let vtcode_mcp = acp_agent_name == "vtcode" && !parsed_options.mcp_servers.is_empty();
    if vtcode_mcp && (source != vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
        || parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
        || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true)
        || recovered.is_some() || fork_replay.is_some())
    {
        return Err(acp_option_error("VT Code MCP requires a new managed session with tools enabled"));
    }
    let iflow_managed = cfg!(all(target_os = "linux", target_env = "gnu",
        any(target_arch = "x86_64", target_arch = "aarch64")))
        && acp_agent_name == "iflow" && source == AcpModelSource::SessionModel;
    let iflow_mcp = iflow_managed && !parsed_options.mcp_servers.is_empty();
    let iflow_projection_allowed = iflow_managed
        && !parsed_options.full_access
        && parsed_options.non_interactive_permissions.is_none()
        && parsed_options.permission_mode != Some(acp_types::PermissionMode::DenyAll)
        && merged_options.get("disable_tools").and_then(Value::as_bool) != Some(true);
    if iflow_mcp && !iflow_projection_allowed {
        return Err(acp_option_error("iFlow Linux MCP requires session_model and interactive tool approvals"));
    }
    let mimo_mcp = acp_agent_name == "mimo-code"
        && source == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel
        && !parsed_options.mcp_servers.is_empty();
    if mimo_mcp && (parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
        || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error("MiMo managed MCP requires enabled tool approvals"));
    }
    if restricted_opencode
        && (parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true))
    {
        return Err(acp_option_error("Restricted OpenCode requires native read tools and approvals"));
    }
    let auggie_mcp = restricted_auggie && !parsed_options.mcp_servers.is_empty();
    let droid_mcp = restricted_droid && !parsed_options.mcp_servers.is_empty();
    let hermes_mcp = restricted_hermes && !parsed_options.mcp_servers.is_empty();
    let openclaw_mcp = if acp_agent_name == "openclaw" && !restricted_openclaw {
        if !parsed_options.mcp_servers.is_empty()
            && merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true)
        {
            return Err(acp_option_error("OpenClaw MCP requires tools to be enabled"));
        }
        super::model_source::drivers::openclaw::project_mcp(&parsed_options, source)
            .map_err(to_api_error)?
    } else {
        serde_json::Map::new()
    };
    // The profile owns the relay, token and private files for this request only.
    // CLI-local execution preserves the user's original configuration.
    let mut launch_profile = match source {
        vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel => {
            let cancellation = async {
                if let Some(mut receiver) = abort.cloned() {
                    loop {
                        if *receiver.borrow() {
                            return;
                        }
                        if receiver.changed().await.is_err() {
                            break;
                        }
                    }
                }
                std::future::pending::<()>().await;
            };
            let prepare = if iflow_projection_allowed || mimo_mcp || vtcode_mcp || hermes_mcp || auggie_mcp || droid_mcp || autohand_mcp {
                H::prepare_stdio_mcp_launch(&acp_agent_name, &normalized_acp_cfg, model, &scoped_session, &cwd, &parsed_options.mcp_servers)
            } else if openclaw_mcp.is_empty() {
                H::prepare_launch(&acp_agent_name, &normalized_acp_cfg, model, &scoped_session)
            } else {
                H::prepare_openclaw_mcp_launch(&normalized_acp_cfg, model, &scoped_session, &openclaw_mcp)
            };
            // Dropping preparation also drops its owned Gateway during startup.
            let profile = tokio::select! {
                result = prepare => result.map_err(to_api_error)?,
                () = cancellation => {
                    on_event(StreamEvent::Error(message::AssistantError::MessageAbortedError { message: "aborted".to_string() }));
                    return Err(Error::Aborted);
                }
            };
            Some(profile)
        }
        vw_api_acp_model::acp_model_source::AcpModelSource::CliLocal => None,
    };
    if acp_agent_name == "harn" {
        let enabled = parsed_options.permission_mode != Some(acp_types::PermissionMode::DenyAll)
            && merged_options.get("disable_tools").and_then(Value::as_bool) != Some(true);
        if let Some(profile) = launch_profile.as_mut() {
            super::skill_mounts::harn::configure(profile, enabled, parsed_options.skill_view.as_ref())?;
        } else if parsed_options.skill_view.is_some() {
            return Err(acp_option_error("Harn selected skills require the reviewed session-model launch"));
        }
    }
    if acp_agent_name == "amp-acp" {
        let enabled = parsed_options.permission_mode != Some(acp_types::PermissionMode::DenyAll)
            && merged_options.get("disable_tools").and_then(Value::as_bool) != Some(true);
        if let Some(profile) = launch_profile.as_mut() {
            super::skill_mounts::amp_acp::configure(profile, enabled, parsed_options.skill_view.as_ref())?;
        } else if parsed_options.skill_view.is_some() {
            return Err(acp_option_error("Amp selected skills require the reviewed session-model launch"));
        }
    }
    if acp_agent_name == "poolside" {
        let enabled = parsed_options.permission_mode != Some(acp_types::PermissionMode::DenyAll)
            && merged_options.get("disable_tools").and_then(Value::as_bool) != Some(true);
        if let Some(profile) = launch_profile.as_mut() {
            super::skill_mounts::poolside::configure(profile, enabled, parsed_options.skill_view.as_ref())?;
        } else if parsed_options.skill_view.is_some() {
            return Err(acp_option_error("Poolside skills require the reviewed session-model launch"));
        }
    }
    if restricted_pi {
        let profile = launch_profile.as_mut().ok_or_else(|| acp_option_error("Missing restricted Pi launch"))?;
        super::skill_mounts::pi::configure_restricted(profile).map_err(to_api_error)?;
    }
    let auggie_sandbox = if restricted_auggie {
        let profile = launch_profile.as_mut().ok_or_else(|| acp_option_error("Missing restricted Auggie launch"))?;
        Some(super::skill_mounts::auggie::sandbox(profile, &cwd, parsed_options.skill_view.as_ref()).map_err(to_api_error)?)
    } else {
        None
    };
    let droid_sandbox = if restricted_droid {
        let profile = launch_profile.as_ref().ok_or_else(|| acp_option_error("Missing restricted Droid launch"))?;
        Some(super::skill_mounts::droid::sandbox(profile, &cwd, parsed_options.skill_view.as_ref()).map_err(to_api_error)?)
    } else {
        None
    };
    let hermes_sandbox = if restricted_hermes {
        let profile = launch_profile.as_ref().ok_or_else(|| acp_option_error("Missing restricted Hermes launch"))?;
        Some(super::skill_mounts::hermes::sandbox(profile, &cwd, parsed_options.skill_view.as_ref()).map_err(to_api_error)?)
    } else {
        None
    };
    if acp_agent_name == "aionrs" && parsed_options.skill_view.is_some() {
        let profile = launch_profile.as_ref().ok_or_else(|| {
            acp_option_error("aionrs selected skills require the reviewed session-model launch")
        })?;
        if parsed_options.permission_mode == Some(acp_types::PermissionMode::DenyAll)
            || merged_options.get("disable_tools").and_then(Value::as_bool) == Some(true)
        {
            return Err(acp_option_error("aionrs selected skills require tool approval"));
        }
        super::skill_mounts::aionrs::publish(
            profile.directory(),
            &cwd,
            parsed_options.skill_view.as_ref(),
        )?;
    }
    if acp_agent_name == "dimcode"
        && parsed_options.permission_mode != Some(acp_types::PermissionMode::DenyAll)
        && merged_options.get("disable_tools").and_then(Value::as_bool) != Some(true)
        && let Some(profile) = launch_profile.as_mut()
    {
        super::skill_mounts::dimcode::configure(profile, parsed_options.skill_view.as_ref())?;
    }
    if acp_agent_name == "sigit" && let Some(profile) = launch_profile.as_mut() {
        super::skill_mounts::sigit::configure(profile, parsed_options.skill_view.as_ref())?;
    }
    if acp_agent_name == "stakpak" && let Some(profile) = launch_profile.as_ref() {
        super::skill_mounts::stakpak::publish(profile, parsed_options.skill_view.as_ref())?;
    }
    if acp_agent_name == "junie" && let Some(profile) = launch_profile.as_ref() {
        super::skill_mounts::junie::publish(profile, parsed_options.skill_view.as_ref())?;
    }
    if acp_agent_name == "qoder" && let Some(profile) = launch_profile.as_ref() {
        super::skill_mounts::qoder::publish(profile.directory(), parsed_options.skill_view.as_ref())?;
    }
    if acp_agent_name == "trae"
        && let Some(profile) = launch_profile.as_ref()
    {
        super::skill_mounts::trae::publish(profile.directory(), parsed_options.skill_view.as_ref())?;
    }
    if acp_agent_name == "corust-agent"
        && let Some(profile) = launch_profile.as_ref()
    {
        super::skill_mounts::corust_agent::publish(
            profile.directory(),
            &cwd,
            parsed_options.skill_view.as_ref(),
        )?;
    }
    if acp_agent_name == "cortex-code"
        && let Some(profile) = launch_profile.as_ref()
    {
        super::skill_mounts::cortex_code::publish(
            profile.directory(),
            &cwd,
            parsed_options.skill_view.as_ref(),
        )?;
    }
    if (autohand_selected || autohand_mcp) && let Some(profile) = launch_profile.as_mut() {
        super::skill_mounts::autohand::configure(profile, &cwd, parsed_options.skill_view.as_ref())?;
    }
    let openclaw_sandbox = if restricted_openclaw {
        let profile = launch_profile.as_ref().ok_or_else(|| acp_option_error("Missing restricted OpenClaw launch"))?;
        Some(super::skill_mounts::openclaw::restrict(
            profile, &cwd, parsed_options.skill_view.as_ref(), &parsed_options.mcp_servers,
        ).map_err(to_api_error)?)
    } else {
        None
    };
    if mimo_mcp || !openclaw_mcp.is_empty() || restricted_openclaw {
        // The request-owned launch now owns the projected MCP connections.
        parsed_options.mcp_servers.clear();
    }
    if iflow_mcp && launch_profile.as_ref().is_none_or(|profile| profile.managed_mcp.is_none()) {
        return Err(acp_option_error("Missing reviewed iFlow Linux MCP projection"));
    }
    if let Some((servers, _)) = launch_profile.as_ref().and_then(|profile| profile.managed_mcp.as_ref()) {
        parsed_options.mcp_servers = servers.clone();
    }
    // Managed SessionModel requests are metered by the owned relay. Never bill
    // their ACP session snapshots a second time.
    let mut request_usage = std::collections::HashMap::new();
    if acp_agent_name == "glm-acp-agent"
        && super::skill_mounts::glm_acp_agent::supports_session_view_target()
        && (parsed_options.skill_view.is_none() || cfg!(target_os = "macos"))
        && let Some(profile) = launch_profile.as_mut()
    {
        super::skill_mounts::glm_acp_agent::configure(
            profile,
            &cwd,
            parsed_options.skill_view.as_ref(),
        )?;
    }
    let mut normalized_acp_cfg = launch_profile
        .as_ref()
        .map(|profile| profile.config().clone())
        .unwrap_or(normalized_acp_cfg);
    if acp_agent_name == "vtcode" && let Some(profile) = launch_profile.as_ref() {
        super::skill_mounts::vtcode::configure(
            profile, &mut normalized_acp_cfg, vtcode_reads, parsed_options.skill_view.as_ref(),
        )?;
    }
    if acp_agent_name == "omp" && let Some(profile) = launch_profile.as_ref() {
        super::skill_mounts::omp::configure(
            profile,
            &mut normalized_acp_cfg,
            parsed_options.skill_view.as_ref(),
        )?;
    }
    if acp_agent_name == "mimo-code" && let Some(profile) = launch_profile.as_ref() {
        super::skill_mounts::mimo_code::configure(
            profile,
            &mut normalized_acp_cfg,
            parsed_options.skill_view.as_ref(),
        )?;
    }
    if cursor_skill_reads_disabled && launch_profile.is_some() {
        normalized_acp_cfg
            .env
            .insert("SHIJU_ACP_CURSOR_DISABLE_SKILL_READS".into(), "1".into());
    }
    let command = normalized_acp_cfg.command.trim();
    if command.is_empty() {
        tracing::warn!(
            target: "vw_agent",
            model = %model.api.id,
            acp_agent = %acp_agent_name,
            "ACP command is empty"
        );
        return Err(Error::Api(message::AssistantError::Unknown {
            message: format!("acp command is empty for model {}", model.api.id),
        }));
    }

    // Server-owned fork anchors and recovered sessions take precedence over desktop rebuild hints.
    let requested_force_new_session = !employee && fork_replay.is_none()
        && recovered.is_none()
        && (restricted_droid || restricted_openclaw || vtcode_mcp || vtcode_selected || autohand_selected || autohand_mcp || merged_options.get("acp_force_new_session").and_then(Value::as_bool).unwrap_or(false));
    let replay_strategy = if fork_history_needed {
        AcpReplayStrategy::Full
    } else {
        parse_replay_strategy(merged_options)
    };
    let replay_recent_count = parse_recent_count(merged_options);
    let initial_prompt_len = employee_text.as_ref().map_or_else(
        || build_request_prompt(
            &chat_messages,
            requested_force_new_session || fork_history_needed,
            replay_strategy,
            replay_recent_count,
        ).len(),
        String::len,
    );
    tracing::info!(
        target: "vw_agent",
        model = %model.api.id,
        acp_agent = %acp_agent_name,
        command,
        args_count = normalized_acp_cfg.args.len(),
        prompt_len = initial_prompt_len,
        "resolved ACP request configuration"
    );
    let acp_agent_name = acp_agent_name.to_string();
    if !employee && acp_agent_name != "openclaw" {
        H::scanner(
            &acp_agent_name,
            &mut normalized_acp_cfg,
            parsed_options.skill_view.as_ref(),
        )?;
    }
    let opencode_sandbox = if restricted_opencode {
        let profile = launch_profile.as_ref().ok_or_else(|| acp_option_error("Missing restricted OpenCode launch"))?;
        Some(super::skill_mounts::opencode::sandbox(
            profile, &mut normalized_acp_cfg, &cwd, parsed_options.skill_view.as_ref(),
        ).map_err(to_api_error)?)
    } else {
        None
    };
    let kilocode_sandbox = if restricted_kilocode {
        super::skill_mounts::kilocode::validate_mcp(&parsed_options.mcp_servers)?;
        let profile = launch_profile.as_ref().ok_or_else(|| acp_option_error("Missing restricted Kilocode launch"))?;
        Some(super::skill_mounts::kilocode::sandbox(
            profile, &mut normalized_acp_cfg, &cwd, parsed_options.skill_view.as_ref(),
        ).map_err(to_api_error)?)
    } else {
        None
    };
    let close_client_after_request =
        employee || parse_close_client_after_request(merged_options)? || launch_profile.is_some();
    let cwd_text = cwd.to_string_lossy().into_owned();
    let walk_boundary = acp_storage::find_git_repository_root(&cwd_text)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd_text.clone());
    let session_name = acp_session_name(&scoped_session);
    // Managed launches must never replay an advertised native model/config over the Provider.
    if source == vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel {
        if let Some(options) = parsed_options.session_options.as_mut() {
            options.model = None;
        }
    }
    let mut force_new_session = requested_force_new_session;
    let mut recovered_session_change = false;
    loop {
        let prompt = if let Some(text) = &employee_text {
            acp_types::prompt_content::text_prompt(text.clone())
        } else {
            super::content::build_request_content(
                &chat_messages,
                force_new_session || fork_history_needed,
                replay_strategy,
                replay_recent_count,
                build_request_prompt,
            )
            .map_err(to_api_error)?
        };
        if prompt.is_empty() {
            tracing::warn!(
                target: "vw_agent",
                model = %model.api.id,
                acp_agent = %acp_agent_name,
                "ACP prompt is empty"
            );
            return Err(Error::Api(message::AssistantError::Unknown {
                message: "acp prompt is empty".to_string(),
            }));
        }

        if let Some(view) = &parsed_options.skill_view {
            view.validate().map_err(to_api_error)?;
        }
        super::session::stop_incompatible_cached_clients::<H>(
            &acp_agent_name,
            &normalized_acp_cfg,
            &cwd,
            &parsed_options,
        )
        .await?;
        let cached_client_entry =
            get_cached_acp_client_with_approval::<H>(
                &acp_agent_name, &normalized_acp_cfg, &cwd, &parsed_options, auggie_sandbox.clone(), hermes_sandbox.clone(), droid_sandbox.clone(), kilocode_sandbox.clone(), opencode_sandbox.clone(), openclaw_sandbox.clone(),
                launch_profile.as_ref().and_then(|profile| profile.managed_mcp.as_ref()).map(|(_, approve)| approve.clone()),
            );
        let _request_cleanup = super::session::RequestClientCleanup::new(&cached_client_entry);
        let cached_client = &cached_client_entry.client;
        if let Some(lease) = &worktree_use {
            let mut held = cached_client.worktree_use.lock();
            if held.is_none() {
                *held = Some(Arc::clone(lease));
            }
        }
        // 同一个 ACP 客户端可能被多个本地请求复用；串行化 prompt 可以避免代理端
        // 把并发输入混到同一会话流里。
        let _prompt_guard = cached_client.prompt_lock.lock().await;
        if let Some(profile) = launch_profile.as_ref() {
            profile.validate_managed_mcp().map_err(to_api_error)?;
        }
        let record = if employee && let Some(selection) = locked_selection {
            acp_storage::resolve_session_record(&selection.record_id).await
                .map_err(to_api_error)
                .and_then(|record| {
                    if record.vwacp_record_id == selection.record_id
                        && normalized_acp_cfg.env.get(super::model_source::profile::REFERENCE_ENV)
                            .is_some_and(|reference| reference == &record.agent_command)
                    {
                        Ok(record)
                    } else {
                        Err(acp_option_error("Employee ACP session binding changed"))
                    }
                })
        } else if let Some(recovered) = &recovered {
            Ok(recovered.record.clone())
        } else {
            find_or_create_cached_session_record::<H>(
                cached_client.as_ref(),
                &acp_agent_name,
                &normalized_acp_cfg,
                &cwd,
                &walk_boundary,
                session_name.clone(),
                &parsed_options,
                force_new_session,
            )
            .await
        };
        let mut record = match record {
            Ok(record) => record,
            Err(err) => {
                close_client_after_request_if_needed(
                    close_client_after_request,
                    &cached_client_entry,
                )
                .await;
                return Err(err);
            }
        };

        if let Some(state) = &fork_replay
            && let Err(error) = state.validate_record(&record)
        {
            close_client_after_request_if_needed(true, &cached_client_entry).await;
            return Err(to_api_error(error));
        }

        let acp_session_id = record.acp_session_id.clone();
        // session/load replays remote history. Complete it before attaching this
        // turn's output channel so historical updates cannot become a new reply.
        if employee && !cached_client.client.has_reusable_session(&acp_session_id)
            && let Err(error) = cached_client.client.load_session(acp_session_id.clone(), &cwd).await
        {
            close_client_after_request_if_needed(true, &cached_client_entry).await;
            return Err(to_api_error(format!("could not restore Employee ACP session: {error}")));
        }
        if !employee {
            let bound_model_id =
                launch_profile.as_ref().and_then(|profile| profile.session_model_id.as_deref());
            if let Some(model_id) = bound_model_id {
                // Managed launch identity takes precedence over a saved native selection.
                acp_access_policy::session_mode_preference::set_desired_model_id(&mut record, Some(model_id));
            }
            // Capture the non-elevated preference before replaying controls. Automatic
            // native access sync must never become the saved user preference.
            if acp_access_policy::session_mode_preference::get_desired_mode_id(record.vwacp.as_ref()).is_none() {
                acp_access_policy::session_mode_preference::set_desired_mode_id(&mut record, parsed_options.session_mode.as_deref());
            }
            let saved_config = acp_access_policy::session_mode_preference::get_desired_config_options(record.vwacp.as_ref());
            for (id, value) in &parsed_options.session_config_options {
                if !saved_config.contains_key(id)
                    && matches!(id.as_str(), "mode" | "edit_approval_policy")
                {
                    acp_access_policy::session_mode_preference::set_desired_config_option(&mut record, id, value);
                }
            }
            super::controls::prepare_access_restore(&cached_client.client, &mut record);
            if !super::controls::has_saved_config_category(
                &record,
                agent_client_protocol::SessionConfigOptionCategory::Mode,
            ) && let Some(mode_id) = acp_access_policy::session_mode_preference::get_desired_mode_id(record.vwacp.as_ref())
                .or_else(|| parsed_options.session_mode.clone())
            {
                if let Err(err) = apply_default_mode(
                    &cached_client.client,
                    &acp_session_id,
                    &cwd,
                    cached_client.client.restored_permission_value("mode", &mode_id).to_owned(),
                )
                .await
                {
                    close_client_after_request_if_needed(
                        close_client_after_request,
                        &cached_client_entry,
                    )
                    .await;
                    return Err(err);
                }
            }
            if (bound_model_id.is_some()
                || (launch_profile.is_none()
                    && !super::controls::has_saved_config_category(
                        &record,
                        agent_client_protocol::SessionConfigOptionCategory::Model,
                    )))
                && let Some(model_id) =
                    acp_access_policy::session_mode_preference::get_desired_model_id(record.vwacp.as_ref()).or_else(|| {
                        parsed_options
                            .session_options
                            .as_ref()
                            .and_then(|options| options.model.clone())
                    })
                && !cached_client
                    .client
                    .session_capabilities(&acp_session_id)
                    .and_then(|info| info.models)
                    .is_some_and(|models| models.current_model_id.0.as_ref() == model_id)
            {
                if let Err(err) = cached_client
                    .client
                    .set_session_model(acp_session_id.clone(), &cwd, model_id)
                    .await
                    .map_err(to_api_error)
                {
                    close_client_after_request_if_needed(
                        close_client_after_request,
                        &cached_client_entry,
                    )
                    .await;
                    return Err(err);
                }
            }
            let mut config_options: std::collections::BTreeMap<_, _> =
                parsed_options.session_config_options.iter().cloned().collect();
            if super::controls::has_saved_config_category(
                &record,
                agent_client_protocol::SessionConfigOptionCategory::ThoughtLevel,
            ) {
                config_options.retain(|id, _| {
                    !matches!(id.as_str(), "reasoning_effort" | "thought_level" | "effort")
                });
            }
            let info = cached_client.client.session_capabilities(&acp_session_id);
            // Typed reasoning options can map to a vendor config ID. Do not let a
            // desktop request write that ID if the adapter advertises it as a permission control.
            if parsed_options.desktop_workspace.is_some()
                && info.as_ref().and_then(|info| info.config_options.as_ref()).is_some_and(|options| {
                    options.iter().any(|option| {
                        config_options.contains_key(option.id.0.as_ref())
                            && super::controls::is_permission_config(option)
                    })
                })
            {
                close_client_after_request_if_needed(close_client_after_request, &cached_client_entry)
                    .await;
                return Err(super::acp_option_error(
                    "ACP request options cannot change native permissions",
                ));
            }
            let native_config = |id: &str| {
                info.as_ref().and_then(|info| info.config_options.as_ref()).and_then(|options| {
                    options.iter().find(|option| {
                        option.id.0.as_ref() == id && super::controls::is_model_config(option)
                    })
                })
            };
            if locked_selection.is_some() || recovered.is_some() {
                config_options.retain(|id, _| id != "model" && native_config(id).is_none());
            }
            config_options.extend(acp_access_policy::session_mode_preference::get_desired_config_options(record.vwacp.as_ref()));
            if let Some(state) = previous_native.as_ref() {
                for option in state
                    .config_options
                    .iter()
                    .flatten()
                    .filter(|option| super::controls::is_model_config(option))
                {
                    let agent_client_protocol::SessionConfigKind::Select(select) = &option.kind else {
                        continue;
                    };
                    let id = option.id.to_string();
                    let value = state
                        .desired_config_options
                        .as_ref()
                        .and_then(|values| values.get(&id))
                        .cloned()
                        .unwrap_or_else(|| select.current_value.to_string());
                    config_options.insert(id, value);
                }
            }
            if launch_profile.is_some() {
                config_options.retain(|id, _| id != "model" && native_config(id).is_none());
            }
            for (config_id, value) in config_options {
                if let Some(option) = native_config(&config_id)
                    && let agent_client_protocol::SessionConfigKind::Select(select) = &option.kind
                    && select.current_value.0.as_ref() == value
                {
                    continue;
                }
                let value =
                    cached_client.client.restored_permission_value(&config_id, &value).to_owned();
                if let Err(err) = cached_client
                    .client
                    .set_session_config_option(acp_session_id.clone(), &cwd, config_id, value)
                    .await
                    .map_err(to_api_error)
                {
                    close_client_after_request_if_needed(
                        close_client_after_request,
                        &cached_client_entry,
                    )
                    .await;
                    return Err(err);
                }
            }

            if launch_profile.is_none()
                && let Some(info) = cached_client.client.session_capabilities(&acp_session_id)
            {
                if acp_access_policy::session_mode_preference::get_desired_model_id(record.vwacp.as_ref()).is_none()
                    && let Some(models) = info.models.as_ref()
                    && !info.config_options.iter().flatten().any(super::controls::is_model_config)
                {
                    acp_access_policy::session_mode_preference::set_desired_model_id(&mut record, Some(models.current_model_id.0.as_ref()));
                }
                for option in info
                    .config_options
                    .iter()
                    .flatten()
                    .filter(|option| super::controls::is_model_config(option))
                {
                    if let agent_client_protocol::SessionConfigKind::Select(select) = &option.kind {
                        acp_access_policy::session_mode_preference::set_desired_config_option(
                            &mut record,
                            option.id.0.as_ref(),
                            select.current_value.0.as_ref(),
                        );
                    }
                }
                acp_access_policy::session_mode_preference::sync_config_options(&mut record, info.config_options);
            }
        }
        apply_lifecycle_snapshot_to_record(
            &mut record,
            &cached_client.client.get_agent_lifecycle_snapshot(),
        );
        if let Err(err) = write_session_record(&record).await.map_err(to_api_error) {
            close_client_after_request_if_needed(close_client_after_request, &cached_client_entry)
                .await;
            return Err(err);
        }

        let selection = super::lifecycle::Selection {
            agent: acp_agent_name.clone(),
            model: selected_model.clone(),
            generation,
            record_id: record.vwacp_record_id.clone(),
        };
        if recovered.is_none()
            && let Err(error) = selection_store.commit(&selection)
        {
            close_client_after_request_if_needed(true, &cached_client_entry).await;
            return Err(to_api_error(error));
        }

        if let Some(state) = &mut fork_replay
            && let Err(error) = state.begin(&record)
        {
            close_client_after_request_if_needed(true, &cached_client_entry).await;
            return Err(to_api_error(error));
        }

        if employee_stop.as_ref().is_some_and(|stop| stop.is_cancelled()) {
            close_client_after_request_if_needed(true, &cached_client_entry).await;
            on_event(StreamEvent::Done {
                finish_reason: Some("cancelled".into()),
                usage: ui_types::TokenUsage::default(),
            });
            return Ok(());
        }

        let (acp_message_tx, mut acp_message_rx) = {
            let (tx, rx) = mpsc::unbounded_channel::<AcpJsonRpcMessage>();
            (tx, rx)
        };
        // output_tx 只在本轮请求期间暴露给 ACP 输出回调。请求结束或取消时立即清空，
        // 避免后续代理消息被投递到已经失效的消费者。
        *cached_client.output_tx.lock() = Some(acp_message_tx.clone());

        let prompt_client = cached_client.client.clone();
        let prompt_cwd = cwd.clone();
        let prompt_text = prompt.clone();
        let prompt_session_id = record.acp_session_id.clone();
        if let Some(profile) = launch_profile.as_ref() {
            profile.validate_managed_mcp().map_err(to_api_error)?;
        }
        let approval = H::approval(&cached_client.permission_confirmation, session_id);
        let mut acp_task = tokio::spawn(async move {
            let mut on_prompt_event = |_event: acp_types::PromptEvent| {};
            prompt_client
                .run_prompt(
                    PromptRequest::from_content(prompt_cwd, prompt_text)
                        // 历史由 ACP 对端保存；恢复失败不能静默换成没有历史的新会话。
                        .with_session_strategy(SessionStrategy::ResumeOrLoad(prompt_session_id)),
                    &mut on_prompt_event,
                )
                .await
                .map_err(to_api_error)
        });

        let mut acp_error: Option<Error> = None;
        let mut delta_count = 0usize;
        let mut tool_updates = AcpToolUpdates::default();
        let mut latest_usage = ui_types::TokenUsage::default();
        let mut finish_reason = None;
        let mut abort_rx = abort.cloned();
        let stop_session = record.acp_session_id.clone();
        let request_employee_stop = async {
            let Some(stop) = &employee_stop else {
                return std::future::pending::<()>().await;
            };
            stop.cancelled().await;
            // Stop may arrive before the actor has registered its prompt.
            while !cached_client.client.cancel(&stop_session).await.unwrap_or(false) {
                tokio::time::sleep(std::time::Duration::from_millis(15)).await;
            }
        };
        tokio::pin!(request_employee_stop);
        let mut employee_stop_sent = false;
        loop {
            if should_abort(abort) {
                drop(approval);
                *cached_client.output_tx.lock() = None;
                let _ = cached_client.client.cancel(record.acp_session_id.clone()).await;
                acp_task.abort();
                let _ = acp_task.await;
                on_event(StreamEvent::Error(message::AssistantError::MessageAbortedError {
                    message: "aborted".to_string(),
                }));
                close_client_after_request_if_needed(
                    close_client_after_request,
                    &cached_client_entry,
                )
                .await;
                return Err(Error::Aborted);
            }
            if let Some(abort_rx) = abort_rx.as_mut() {
                tokio::select! {
                    _ = &mut request_employee_stop, if !employee_stop_sent => {
                        employee_stop_sent = true;
                    }
                    changed = abort_rx.changed() => {
                        match changed {
                            Ok(_) if *abort_rx.borrow() => {
                                drop(approval);
                                *cached_client.output_tx.lock() = None;
                                let _ = cached_client.client.cancel(record.acp_session_id.clone()).await;
                                acp_task.abort();
                                let _ = acp_task.await;
                                on_event(StreamEvent::Error(message::AssistantError::MessageAbortedError {
                                    message: "aborted".to_string(),
                                }));
                                close_client_after_request_if_needed(
                                    close_client_after_request,
                                    &cached_client_entry,
                                )
                                .await;
                                return Err(Error::Aborted);
                            }
                            Ok(_) | Err(_) => {}
                        }
                    }
                    maybe_message = acp_message_rx.recv() => {
                        if let Some(message) = maybe_message {
                            if !employee && launch_profile.is_none()
                                && let Some((id, usage)) = super::updates::request_usage(&message)
                            {
                                if request_usage.len() < 4096 || request_usage.contains_key(&id) {
                                    request_usage.insert(id, usage);
                                } else {
                                    tracing::warn!("ACP request usage observation limit reached");
                                }
                            }
                            if !employee || super::updates::extract_delta_from_acp_message(&message).is_some() {
                                forward_acp_message(&message, on_event, &mut latest_usage, &mut delta_count, &mut tool_updates, H::tool_input_preview);
                            }
                        }
                    }
                    joined = &mut acp_task => {
                        *cached_client.output_tx.lock() = None;
                        match joined {
                            Ok(Ok(result)) => {
                                finish_reason = result.finish_reason.clone();
                                if let Some(usage) = result.usage {
                                    latest_usage = ui_types::TokenUsage {
                                        input_tokens: usage.input_tokens,
                                        output_tokens: usage.output_tokens,
                                        cached_tokens: usage.cached_tokens,
                                        reasoning_tokens: usage.reasoning_tokens,
                                    };
                                }
                                if result.session_id != record.acp_session_id {
                                    record.acp_session_id = result.session_id;
                                }
                            }
                            Ok(Err(err)) => {
                                acp_error = Some(to_api_error(err));
                            }
                            Err(err) => {
                                let api_err = to_api_error(err.to_string());
                                acp_error = Some(api_err);
                            }
                        }
                        while let Ok(message) = acp_message_rx.try_recv() {
                            if !employee && launch_profile.is_none()
                                && let Some((id, usage)) = super::updates::request_usage(&message)
                            {
                                if request_usage.len() < 4096 || request_usage.contains_key(&id) {
                                    request_usage.insert(id, usage);
                                } else {
                                    tracing::warn!("ACP request usage observation limit reached");
                                }
                            }
                            if !employee || super::updates::extract_delta_from_acp_message(&message).is_some() {
                                forward_acp_message(&message, on_event, &mut latest_usage, &mut delta_count, &mut tool_updates, H::tool_input_preview);
                            }
                        }
                        break;
                    }
                }
            } else {
                tokio::select! {
                    _ = &mut request_employee_stop, if !employee_stop_sent => {
                        employee_stop_sent = true;
                    }
                    maybe_message = acp_message_rx.recv() => {
                        if let Some(message) = maybe_message {
                            if !employee && launch_profile.is_none()
                                && let Some((id, usage)) = super::updates::request_usage(&message)
                            {
                                if request_usage.len() < 4096 || request_usage.contains_key(&id) {
                                    request_usage.insert(id, usage);
                                } else {
                                    tracing::warn!("ACP request usage observation limit reached");
                                }
                            }
                            if !employee || super::updates::extract_delta_from_acp_message(&message).is_some() {
                                forward_acp_message(&message, on_event, &mut latest_usage, &mut delta_count, &mut tool_updates, H::tool_input_preview);
                            }
                        }
                    }
                    joined = &mut acp_task => {
                        *cached_client.output_tx.lock() = None;
                        match joined {
                            Ok(Ok(result)) => {
                                finish_reason = result.finish_reason.clone();
                                if let Some(usage) = result.usage {
                                    latest_usage = ui_types::TokenUsage {
                                        input_tokens: usage.input_tokens,
                                        output_tokens: usage.output_tokens,
                                        cached_tokens: usage.cached_tokens,
                                        reasoning_tokens: usage.reasoning_tokens,
                                    };
                                }
                                if result.session_id != record.acp_session_id {
                                    record.acp_session_id = result.session_id;
                                }
                            }
                            Ok(Err(err)) => {
                                acp_error = Some(to_api_error(err));
                            }
                            Err(err) => {
                                let api_err = to_api_error(err.to_string());
                                acp_error = Some(api_err);
                            }
                        }
                        while let Ok(message) = acp_message_rx.try_recv() {
                            if !employee && launch_profile.is_none()
                                && let Some((id, usage)) = super::updates::request_usage(&message)
                            {
                                if request_usage.len() < 4096 || request_usage.contains_key(&id) {
                                    request_usage.insert(id, usage);
                                } else {
                                    tracing::warn!("ACP request usage observation limit reached");
                                }
                            }
                            if !employee || super::updates::extract_delta_from_acp_message(&message).is_some() {
                                forward_acp_message(&message, on_event, &mut latest_usage, &mut delta_count, &mut tool_updates, H::tool_input_preview);
                            }
                        }
                        break;
                    }
                }
            }
        }

        drop(approval);
        *cached_client.output_tx.lock() = None;
        if fork_history_needed && finish_reason.as_deref() == Some("cancelled") {
            acp_error = Some(to_api_error(
                "fork history delivery is uncertain after a cancelled ACP prompt; create a new fork",
            ));
        }
        if let Some(err) = acp_error {
            if !employee && !fork_history_needed
                && !recovered_session_change
                && is_acp_session_changed_error(&err)
            {
                // 会话漂移不是丢弃历史的授权。只重试一次恢复原记录，失败明确上抛。
                recovered_session_change = true;
                force_new_session = false;
                tracing::warn!(
                    target: "vw_agent",
                    model = %model.api.id,
                    acp_agent = %acp_agent_name,
                    "ACP session changed during prompt; retrying the original session"
                );
                close_client_after_request_if_needed(
                    close_client_after_request,
                    &cached_client_entry,
                )
                .await;
                continue;
            }
            close_client_after_request_if_needed(close_client_after_request, &cached_client_entry)
                .await;
            return Err(err);
        }

        let now = acp_storage::iso_now();
        if let Some(profile) = launch_profile.as_ref()
            && profile.model_api.as_ref().is_some_and(|api| match acp_agent_name.as_str() {
                "codex" | "claude" => {
                    super::model_source::drivers::launch_route(&acp_agent_name, profile)
                        .is_ok_and(|route| !route.conversion_required)
                }
                "opencode" => model_protocol::endpoint_urls::chat_url(api).is_ok(),
                "copilot" => {
                    profile.model_reasoning_options.is_some()
                        && model_protocol::endpoint_urls::chat_url(api).is_ok()
                }
                _ => false,
            })
        {
            let options = cached_client
                .client
                .session_capabilities(&acp_session_id)
                .and_then(|info| info.config_options)
                .unwrap_or_default();
            let effort = options
                .into_iter()
                .filter(|option| {
                    profile.model_supports_reasoning
                        && super::controls::is_managed_effort_config(&acp_agent_name, option)
                })
                .collect();
            acp_access_policy::session_mode_preference::sync_config_options(&mut record, Some(effort));
        }
        record.last_used_at = now.clone();
        record.last_prompt_at = Some(now);
        record.closed = Some(false);
        apply_lifecycle_snapshot_to_record(
            &mut record,
            &cached_client.client.get_agent_lifecycle_snapshot(),
        );
        if let Err(err) = write_session_record(&record).await.map_err(to_api_error) {
            close_client_after_request_if_needed(close_client_after_request, &cached_client_entry)
                .await;
            return Err(err);
        }

        // Keep the original recovery path available if the CLI refuses to load
        // the saved session or the user cancels the first recovery attempt.
        if recovered.is_some()
            && let Err(error) = selection_store.commit(&selection)
        {
            close_client_after_request_if_needed(true, &cached_client_entry).await;
            return Err(to_api_error(error));
        }

        if let Some(state) = &mut fork_replay
            && let Err(error) = state.complete(&record)
        {
            close_client_after_request_if_needed(true, &cached_client_entry).await;
            return Err(to_api_error(error));
        }

        for (id, usage) in request_usage.drain() {
            H::record_usage(&model.provider_id, &model.api.id, &acp_session_id, &acp_agent_name, &id, usage);
        }
        on_event(StreamEvent::Done { finish_reason, usage: latest_usage });

        tracing::info!(
            target: "vw_agent",
            model = %model.api.id,
            acp_agent = %acp_agent_name,
            delta_count,
            "forwarding ACP deltas to stream consumer"
        );
        close_client_after_request_if_needed(close_client_after_request, &cached_client_entry)
            .await;
        return Ok(());
    }
}

pub(super) fn validate_linux_session_mcp(
    agent: &str,
    source: vw_api_acp_model::acp_model_source::AcpModelSource,
    selected: bool,
) -> Result<(), Error> {
    use vw_api_acp_model::acp_model_source::AcpModelSource;
    let backend = acp_agent_registry::agent_registry::desktop_permission_agent(agent);
    if selected
        && (backend.is_some_and(acp_agent_registry::linux_native_only)
            || (cfg!(target_os = "linux") && backend == Some("pi"))
            || (cfg!(target_os = "linux") && backend == Some("glm-acp-agent")
                && source == AcpModelSource::SessionModel)
            || (cfg!(target_os = "linux") && backend == Some("iflow")
                && (source != AcpModelSource::SessionModel
                    || !cfg!(all(target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64"))))))
    {
        return Err(acp_option_error(format!("{agent}: no verified Linux session MCP launch")));
    }
    Ok(())
}

/// Validate the captured client model before touching a CLI process. Credentials stay
/// in the Provider/auth snapshot; CLI-local execution does not consult this boundary.
pub fn validate_session_model(
    model: &provider::Model,
    provider_info: &provider::Info,
    auth_info: Option<&auth::Info>,
) -> Result<(), Error> {
    if model.status != "active" || is_acp_model(model) {
        return Err(to_api_error("ACP session_model requires an active non-ACP Provider/model"));
    }
    if model.provider_id != provider_info.id {
        return Err(to_api_error("ACP session_model Provider identity mismatch"));
    }
    let has_credentials =
        provider_info.key.as_deref().is_some_and(|value| is_usable_copilot_token(value.trim()))
            || provider_info.env.iter().any(|key| {
                agent_env::get(key)
                    .as_deref()
                    .is_some_and(|value| is_usable_copilot_token(value.trim()))
            })
            || match auth_info {
                Some(auth::Info::Api(info)) => is_usable_copilot_token(info.key.trim()),
                // Refresh remains the responsibility of the existing Provider transport.
                Some(auth::Info::Oauth(info)) => {
                    !info.access.trim().is_empty() || !info.refresh.trim().is_empty()
                }
                Some(auth::Info::Wellknown(info)) => !info.token.trim().is_empty(),
                None => false,
            };
    if !has_credentials {
        return Err(Error::Api(message::AssistantError::ProviderAuthError {
            provider_id: model.provider_id.clone(),
            message: "ACP session_model requires Provider credentials".to_string(),
        }));
    }
    Ok(())
}

fn employee_launch_config(
    launch: super::host::EmployeeAcpLaunch,
    executable: std::path::PathBuf,
) -> Result<config::schema::AcpAgentConfig, Error> {
    let command = executable
        .to_str()
        .filter(|_| executable.is_absolute())
        .ok_or_else(|| acp_option_error("Employee ACP requires an absolute ShiJu executable"))?;
    let state_dir = launch
        .state_dir
        .to_str()
        .filter(|_| launch.state_dir.is_absolute())
        .ok_or_else(|| acp_option_error("Employee ACP requires an absolute state directory"))?;
    if launch.employee_id.trim().is_empty()
        || launch.account_scope.trim().is_empty()
        || launch.credential.trim().is_empty()
        || launch.credential.contains('\0')
    {
        return Err(acp_option_error("Employee ACP launch binding is incomplete"));
    }
    let kind = match launch.kind {
        super::host::EmployeeAcpKind::Private => "private",
        super::host::EmployeeAcpKind::Shared => "shared",
    };
    let binding = serde_json::json!([launch.account_scope, kind, launch.employee_id]);
    let reference = format!(
        "vw-employee-acp:{}",
        super::model_source::fingerprint(binding.to_string().as_bytes())
    );
    Ok(config::schema::AcpAgentConfig {
        command: command.into(),
        args: vec![
            EMPLOYEE_ACP_BACKEND.into(),
            "--employee-id".into(),
            launch.employee_id,
            "--employee-kind".into(),
            kind.into(),
            "--state-dir".into(),
            state_dir.into(),
            "--account-scope".into(),
            launch.account_scope,
        ],
        env: [
            (super::model_source::profile::REFERENCE_ENV.into(), reference),
            (EMPLOYEE_ACP_CREDENTIAL_ENV.into(), launch.credential),
        ]
        .into(),
    })
}

async fn close_client_after_request_if_needed(should_close: bool, entry: &CachedAcpClientEntry) {
    if should_close {
        close_cached_acp_client_if_unshared(entry).await;
    }
}

fn parse_close_client_after_request(merged_options: &Value) -> Result<bool, Error> {
    let Some(value) = merged_options.get("acp_close_client_after_request") else {
        return Ok(false);
    };
    value
        .as_bool()
        .ok_or_else(|| super::acp_option_error("acp_close_client_after_request must be a boolean"))
}

fn is_usable_copilot_token(value: &str) -> bool {
    !value.is_empty() && value != auth::OAUTH_DUMMY_KEY
}

/// Validate advertised choices before requesting a mode, including permissive peers.
pub async fn apply_default_mode(
    client: &acp_client::AcpClient,
    session_id: &str,
    cwd: &std::path::Path,
    mode: String,
) -> Result<(), Error> {
    // The actor restores the original session and validates advertised modes before setting it.
    // A new connection has no cached capabilities until that restoration finishes.
    if client.session_capabilities(session_id).is_none() {
        return client
            .set_session_mode(session_id.to_string(), cwd, mode)
            .await
            .map_err(to_api_error);
    }
    let supported =
        client.session_capabilities(session_id).and_then(|info| info.modes).is_some_and(|modes| {
            modes.available_modes.iter().any(|candidate| candidate.id.to_string() == mode)
        });
    if !supported {
        return Err(super::acp_option_error(format!("ACP session mode is not advertised: {mode}")));
    }
    client.set_session_mode(session_id.to_string(), cwd, mode).await.map_err(to_api_error)
}


fn is_acp_model(model: &provider::Model) -> bool {
    model.provider_id == "acp" || matches!(model.api.adapter.trim(), "acp" | "agent-client-protocol" | "agent_client_protocol")
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod request_tests;
