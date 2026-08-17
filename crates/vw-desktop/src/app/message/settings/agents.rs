//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use super::types::AgentsLoaded;
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::{
    App, Message,
    state::{
        AGENT_PROMPT_SYSTEM_TAB, DelegateAgentSettingsEntry, MAIN_AGENT_KEY, ModelSummary,
        ProviderModelsSummary, ProviderSummary, WorkspaceIdentityFileState,
    },
};
use iced::Task;
use iced::widget::text_editor;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::time::UNIX_EPOCH;
use vw_config_types::agent::builtin_agent_spec;
use vw_shared::provider::types as provider_types;
use vw_shared::provider::types::Info;

use super::util::{is_provider_connected, load_gateway_providers};

fn summarize_providers(providers: std::collections::HashMap<String, Info>) -> Vec<ProviderSummary> {
    let mut out = providers
        .into_values()
        .map(|provider| {
            let (source_label, connected) = match provider.source {
                provider_types::ProviderSource::Api => ("API 密钥", true),
                provider_types::ProviderSource::Env => ("环境变量", true),
                provider_types::ProviderSource::Config => {
                    ("配置", is_provider_connected(&provider))
                }
                provider_types::ProviderSource::Custom => {
                    ("内置", is_provider_connected(&provider))
                }
            };
            ProviderSummary {
                id: provider.id,
                name: provider.name,
                source_label: source_label.to_string(),
                connected,
            }
        })
        .collect::<Vec<_>>();
    out.sort_by(|left, right| left.name.cmp(&right.name).then_with(|| left.id.cmp(&right.id)));
    out
}

fn summarize_models(
    providers: std::collections::HashMap<String, Info>,
) -> Vec<ProviderModelsSummary> {
    let mut out = providers
        .into_values()
        .map(|provider| {
            let mut models = provider.models.into_values().collect::<Vec<_>>();
            models = provider_types::sort(models);
            ProviderModelsSummary {
                id: provider.id,
                name: provider.name,
                models: models
                    .into_iter()
                    .map(|model| {
                        let detail =
                            serde_json::to_value(&model).unwrap_or(serde_json::Value::Null);
                        ModelSummary {
                            id: model.id,
                            name: model.name,
                            enabled: model.status == "active",
                            toolcall: model.capabilities.toolcall,
                            attachment: model.capabilities.input.image,
                            context_limit: model.limit.context,
                            detail,
                        }
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    out.sort_by(|left, right| left.name.cmp(&right.name).then_with(|| left.id.cmp(&right.id)));
    out
}

fn refresh_task() -> Task<Message> {
    Task::perform(
        async move {
            let providers = load_gateway_providers().await?;
            let provider_summaries = summarize_providers(providers.clone());
            let provider_models = summarize_models(providers);
            let available_tools = crate::app::config::load_tools_list_via_gateway();
            Ok((provider_summaries, provider_models, available_tools))
        },
        |res: AgentsLoaded| Message::Settings(SettingsMessage::Agents(AgentsMessage::Loaded(res))),
    )
}

fn project_config_task(project_path: String) -> Task<Message> {
    let response_project_path = project_path.clone();
    Task::perform(
        async move {
            crate::app::config::load_project_agent_config_async(&project_path).await.map(Box::new)
        },
        move |result| {
            Message::Settings(SettingsMessage::Agents(AgentsMessage::ProjectConfigLoaded {
                project_path: response_project_path.clone(),
                result,
            }))
        },
    )
}

pub(super) fn config_task(project_path: Option<String>) -> Task<Message> {
    let response_project_path = project_path.clone();
    Task::perform(
        async move {
            match project_path {
                Some(project_path) => {
                    crate::app::config::load_project_agent_config_async(&project_path).await
                }
                None => crate::app::config::load_full_agent_config_async().await,
            }
            .map(Box::new)
        },
        move |result| {
            Message::Settings(SettingsMessage::Agents(AgentsMessage::ConfigLoaded {
                project_path: response_project_path.clone(),
                result,
            }))
        },
    )
}

fn apply_config(app: &mut App, config: &vw_config_types::config::Config) {
    let mut entries = crate::app::state::agent_settings_entries_from_config(config);
    crate::app::state::apply_default_allowed_tools(
        &mut entries,
        &app.agents_settings.available_tools,
    );
    app.agents_settings.entries = entries;
    if !app
        .agents_settings
        .entries
        .iter()
        .any(|entry| entry.key == app.agents_settings.selected_agent)
    {
        app.agents_settings.selected_agent = MAIN_AGENT_KEY.to_string();
    }
}

fn find_entry_mut<'a>(
    app: &'a mut App,
    agent_key: &str,
) -> Option<&'a mut DelegateAgentSettingsEntry> {
    app.agents_settings.entries.iter_mut().find(|entry| entry.key == agent_key)
}

fn tool_matches_any(tool_id: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| tool_id == *candidate)
}

fn tool_in_preset(tool_id: &str, preset_key: &str) -> bool {
    match preset_key {
        "minimal" => tool_matches_any(
            tool_id,
            &[
                "ls",
                "read",
                "file_read",
                "pdf_read",
                "grep",
                "glob",
                "lsp",
                "memory_recall",
                "question",
            ],
        ),
        "coding" => {
            tool_in_preset(tool_id, "minimal")
                || tool_matches_any(
                    tool_id,
                    &["write", "file_write", "apply_patch", "bash", "shell", "git_operations"],
                )
        }
        "research" => {
            tool_in_preset(tool_id, "minimal")
                || tool_matches_any(
                    tool_id,
                    &[
                        "browser",
                        "browser_open",
                        "http_request",
                        "web_fetch",
                        "web_search",
                        "websearch",
                        "web_search_tool",
                        "screenshot",
                    ],
                )
        }
        "collab" => {
            tool_in_preset(tool_id, "minimal")
                || tool_matches_any(
                    tool_id,
                    &[
                        "AgentTool",
                        "schedule",
                        "cron_add",
                        "cron_list",
                        "cron_remove",
                        "cron_update",
                        "cron_run",
                        "memory_store",
                        "memory_forget",
                        "todoread",
                        "todowrite",
                    ],
                )
        }
        "full" => true,
        _ => false,
    }
}

fn tools_for_preset(available_tools: &[String], preset_key: &str) -> Vec<String> {
    let mut tools = available_tools
        .iter()
        .filter(|tool_id| tool_in_preset(tool_id.as_str(), preset_key))
        .cloned()
        .collect::<Vec<_>>();
    tools.sort();
    tools.dedup();
    tools
}

fn normalize_agent_key(raw: &str) -> String {
    raw.trim().chars().filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_').collect()
}

fn find_workspace_file_mut<'a>(
    app: &'a mut App,
    file_name: &str,
) -> Option<&'a mut WorkspaceIdentityFileState> {
    app.agents_settings
        .workspace_identity_files
        .iter_mut()
        .find(|entry| entry.file_name == file_name)
}

#[cfg(test)]
fn bundled_workspace_identity_path(file_name: &str) -> String {
    format!("assets/agent/{file_name}")
}

fn bundled_workspace_identity_content(file_name: &str) -> Option<&'static str> {
    match file_name {
        "AGENTS.md" => Some(include_str!("../../../../../../assets/agent/AGENTS.md")),
        "SOUL.md" => Some(include_str!("../../../../../../assets/agent/SOUL.md")),
        "TOOLS.md" => Some(include_str!("../../../../../../assets/agent/TOOLS.md")),
        "IDENTITY.md" => Some(include_str!("../../../../../../assets/agent/IDENTITY.md")),
        "USER.md" => Some(include_str!("../../../../../../assets/agent/USER.md")),
        "HEARTBEAT.md" => Some(include_str!("../../../../../../assets/agent/HEARTBEAT.md")),
        "BOOTSTRAP.md" => Some(include_str!("../../../../../../assets/agent/BOOTSTRAP.md")),
        "MEMORY.md" => Some(include_str!("../../../../../../assets/agent/MEMORY.md")),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn workspace_identity_file_metadata(
    root_directory: &str,
    file_name: &str,
) -> (Option<u64>, Option<u64>) {
    let path = PathBuf::from(root_directory).join(file_name);
    let Ok(metadata) = std::fs::metadata(path) else {
        return (None, None);
    };

    let size_bytes = metadata.is_file().then_some(metadata.len());
    let modified_at_ms = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| u64::try_from(duration.as_millis()).ok());
    (size_bytes, modified_at_ms)
}

#[cfg(target_arch = "wasm32")]
fn workspace_identity_file_metadata(
    _root_directory: &str,
    _file_name: &str,
) -> (Option<u64>, Option<u64>) {
    (None, None)
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_workspace_identity_root(agent_key: &str) -> Option<PathBuf> {
    let normalized_agent_key = agent_key.trim();
    let suffix = if normalized_agent_key == "main" {
        String::new()
    } else {
        format!("-{normalized_agent_key}")
    };
    std::env::var_os("HOME").map(PathBuf::from).map(|home| {
        vw_config_types::paths::home_config_dir(home).join(format!("workspace{suffix}"))
    })
}

fn load_workspace_identity_file_task(agent_key: String, file_name: String) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    let response_workspace_root_path =
        resolve_workspace_identity_root(&agent_key).map(|path| path.to_string_lossy().into_owned());
    #[cfg(target_arch = "wasm32")]
    let response_workspace_root_path: Option<String> = None;
    let request_workspace_root_path = response_workspace_root_path.clone();
    let response_agent_key = agent_key.clone();
    let response_file_name = file_name.clone();

    Task::perform(
        async move {
            let client = crate::app::config::gateway_client()?;
            let response = client
                .file_read_in_directory(
                    request_workspace_root_path.as_deref(),
                    Some(&agent_key),
                    &file_name,
                )
                .await?;

            let (size_bytes, modified_at_ms) =
                workspace_identity_file_metadata(&response.root_directory, &file_name);
            Ok((response.root_directory, response.content, size_bytes, modified_at_ms))
        },
        move |result| {
            let (workspace_root_path, result) = match result {
                Ok((workspace_root_path, content, size_bytes, modified_at_ms)) => {
                    (Some(workspace_root_path), Ok((content, size_bytes, modified_at_ms)))
                }
                Err(error) => (response_workspace_root_path.clone(), Err(error)),
            };
            Message::Settings(SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityLoaded {
                agent_key: response_agent_key.clone(),
                file_name: response_file_name.clone(),
                workspace_root_path,
                result,
            }))
        },
    )
}

fn restore_workspace_identity_default_task(agent_key: String, file_name: String) -> Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    let response_workspace_root_path =
        resolve_workspace_identity_root(&agent_key).map(|path| path.to_string_lossy().into_owned());
    #[cfg(target_arch = "wasm32")]
    let response_workspace_root_path: Option<String> = None;
    let request_workspace_root_path = response_workspace_root_path.clone();
    let response_agent_key = agent_key.clone();
    let response_file_name = file_name.clone();

    Task::perform(
        async move {
            let bundled_content = bundled_workspace_identity_content(&file_name)
                .map(str::to_string)
                .ok_or_else(|| format!("未找到 {} 的默认模板。", file_name))?;

            if bundled_content.trim().is_empty() {
                return Err(format!("未找到 {} 的默认模板。", file_name));
            }

            let client = crate::app::config::gateway_client()?;

            let response = client
                .file_write_in_directory(
                    request_workspace_root_path.as_deref(),
                    Some(&agent_key),
                    &file_name,
                    &bundled_content,
                    true,
                )
                .await?;
            let (size_bytes, modified_at_ms) =
                workspace_identity_file_metadata(&response.root_directory, &file_name);
            Ok((bundled_content, size_bytes, modified_at_ms))
        },
        move |result| {
            Message::Settings(SettingsMessage::Agents(
                AgentsMessage::WorkspaceIdentityDefaultRestored {
                    agent_key: response_agent_key.clone(),
                    file_name: response_file_name.clone(),
                    workspace_root_path: response_workspace_root_path.clone(),
                    result,
                },
            ))
        },
    )
}

fn save_workspace_identity_file_task(
    agent_key: String,
    file_name: String,
    text: String,
) -> Task<Message> {
    let response_file_name = file_name.clone();
    Task::perform(
        async move {
            #[cfg(not(target_arch = "wasm32"))]
            let workspace_root = resolve_workspace_identity_root(&agent_key)
                .map(|path| path.to_string_lossy().into_owned());
            #[cfg(target_arch = "wasm32")]
            let workspace_root: Option<String> = None;

            let client = crate::app::config::gateway_client()?;
            client
                .file_write_in_directory(
                    workspace_root.as_deref(),
                    Some(&agent_key),
                    &file_name,
                    &text,
                    true,
                )
                .await
                .map(|response| {
                    let (size_bytes, modified_at_ms) =
                        workspace_identity_file_metadata(&response.root_directory, &file_name);
                    (size_bytes, modified_at_ms)
                })
        },
        move |result| match result {
            Ok((size_bytes, modified_at_ms)) => {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::WorkspaceIdentitySaved {
                    file_name: response_file_name.clone(),
                    size_bytes,
                    modified_at_ms,
                    result: Ok(()),
                }))
            }
            Err(error) => {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::WorkspaceIdentitySaved {
                    file_name: response_file_name.clone(),
                    size_bytes: None,
                    modified_at_ms: None,
                    result: Err(error),
                }))
            }
        },
    )
}

fn build_agents_settings_patch(
    entries: &[DelegateAgentSettingsEntry],
) -> Result<serde_json::Value, String> {
    let mut agents = serde_json::Map::with_capacity(entries.len());
    for entry in entries {
        let provider = entry.provider.trim().to_string();
        let model = entry.model.trim().to_string();
        let system_prompt = entry.system_prompt_editor.text().trim().to_string();
        let api_key = entry.api_key_input.trim().to_string();
        let mut allowed_tools = entry
            .allowed_tools
            .iter()
            .map(|tool| tool.trim().to_string())
            .filter(|tool| !tool.is_empty())
            .collect::<Vec<_>>();
        allowed_tools.sort();
        allowed_tools.dedup();
        let allowed_skills = entry
            .allowed_skills
            .iter()
            .map(|skill| skill.trim().to_string())
            .filter(|skill| !skill.is_empty())
            .collect::<Vec<_>>();
        let mut enabled_identity_files = entry
            .enabled_identity_files
            .iter()
            .map(|file_name| file_name.trim().to_string())
            .filter(|file_name| !file_name.is_empty())
            .collect::<Vec<_>>();
        enabled_identity_files.sort();
        enabled_identity_files.dedup();
        let mut agent = serde_json::Map::new();
        if builtin_agent_spec(&entry.key).is_some() {
            if entry.can_disable {
                agent.insert("enabled".to_string(), serde_json::Value::Bool(entry.enabled));
            }
            agent.insert("provider".to_string(), serde_json::Value::String(provider));
            agent.insert("model".to_string(), serde_json::Value::String(model));
            agent.insert(
                "force_model_selection".to_string(),
                serde_json::Value::Bool(entry.force_model_selection),
            );
            agent.insert(
                "system_prompt".to_string(),
                (!system_prompt.is_empty())
                    .then_some(system_prompt)
                    .map_or(serde_json::Value::Null, serde_json::Value::String),
            );
            agent.insert(
                "api_key".to_string(),
                (!api_key.is_empty())
                    .then_some(api_key)
                    .map_or(serde_json::Value::Null, serde_json::Value::String),
            );
            agent.insert(
                "temperature".to_string(),
                serde_json::json!(entry.temperature.clamp(0.0, 2.0)),
            );
            if entry.key == MAIN_AGENT_KEY {
                agent.insert(
                    "max_parallel_subagents".to_string(),
                    serde_json::json!(entry.max_parallel_subagents.clamp(1, 32)),
                );
            }
            if builtin_agent_spec(&entry.key).is_some_and(|spec| spec.allow_as_subagent) {
                agent.insert("allowed_tools".to_string(), serde_json::json!(allowed_tools));
            }
            agent.insert("tool_policy".to_string(), serde_json::json!(entry.tool_policy));
            agent.insert("allowed_skills".to_string(), serde_json::json!(allowed_skills));
            agent.insert("skill_policy".to_string(), serde_json::json!(entry.skill_policy));
            agent.insert(
                "enabled_identity_files".to_string(),
                serde_json::json!(enabled_identity_files),
            );
        } else {
            agent = serde_json::json!({
                "enabled": if entry.key == MAIN_AGENT_KEY { true } else { entry.enabled },
                "allow_as_subagent": entry.allow_as_subagent,
                "max_parallel_subagents": entry.max_parallel_subagents.clamp(1, 32),
                "provider": provider,
                "model": model,
                "force_model_selection": entry.force_model_selection,
                "system_prompt": (!system_prompt.is_empty()).then_some(system_prompt),
                "api_key": (!api_key.is_empty()).then_some(api_key),
                "temperature": entry.temperature.clamp(0.0, 2.0),
                "max_depth": entry.max_depth.clamp(1, 32),
                "agentic": entry.agentic,
                "tool_policy": entry.tool_policy,
                "allowed_tools": allowed_tools,
                "skill_policy": entry.skill_policy,
                "allowed_skills": allowed_skills,
                "enabled_identity_files": enabled_identity_files,
                "max_iterations": entry.max_iterations.max(1),
            })
            .as_object()
            .expect("agent settings patch is an object")
            .clone();
        }
        agents.insert(entry.key.clone(), serde_json::Value::Object(agent));
    }
    agents.insert("general".to_string(), serde_json::Value::Null);

    let mut patch = serde_json::Map::new();
    patch.insert("agents".to_string(), serde_json::Value::Object(agents));

    if let Some(main) = entries.iter().find(|entry| entry.key == MAIN_AGENT_KEY) {
        let provider = main.provider.trim();
        let model = main.model.trim();
        patch.insert(
            "default_provider".to_string(),
            if provider.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(provider.to_string())
            },
        );
        if !provider.is_empty() && !model.is_empty() {
            patch.insert(
                "default_model".to_string(),
                serde_json::Value::String(format!("{provider}/{model}")),
            );
        } else {
            patch.insert("default_model".to_string(), serde_json::Value::Null);
        }
        if let Some(temperature) =
            serde_json::Number::from_f64(main.temperature.clamp(0.0, 2.0) as f64)
        {
            patch.insert("default_temperature".to_string(), serde_json::Value::Number(temperature));
        }
        let tool_dispatcher = main.tool_dispatcher.trim();
        let tool_dispatcher = if tool_dispatcher.is_empty() { "auto" } else { tool_dispatcher };
        patch.insert(
            "agent".to_string(),
            serde_json::json!({
                "compact_context": main.compact_context,
                "max_tool_iterations": main.max_tool_iterations.max(1),
                "max_history_messages": main.max_history_messages.clamp(1, 1000),
                "parallel_tools": main.parallel_tools,
                "tool_dispatcher": tool_dispatcher,
            }),
        );
    }

    Ok(serde_json::Value::Object(patch))
}

async fn persist_agents_patch(
    project_path: Option<String>,
    patch: serde_json::Value,
) -> Result<(), String> {
    match project_path {
        Some(project_path) => {
            crate::app::config::patch_project_agent_config_async(&project_path, patch).await
        }
        None => crate::app::config::patch_full_agent_config_async(patch).await,
    }
}

fn persist_agents_settings(app: &mut App) -> Task<Message> {
    if !app.agents_settings.config_ready {
        app.agents_settings.save_error =
            Some("Gateway 配置尚未加载完成，已拒绝保存默认快照。请刷新后重试。".to_string());
        return Task::none();
    }

    if app.agents_settings.save_in_flight {
        app.agents_settings.save_pending = true;
        return Task::none();
    }

    app.agents_settings.save_in_flight = true;
    app.agents_settings.save_pending = false;
    let entries = app.agents_settings.entries.clone();
    let project_path = app.agents_settings.project_path.clone();
    let agents_task = crate::app::config::spawn_gateway_task("agent", async move {
        let patch = build_agents_settings_patch(&entries)?;
        persist_agents_patch(project_path, patch).await
    });

    let compat_cleanup_task = crate::app::config::update_agents_compat_registry_async(|agents| {
        agents.clear();
    });

    Task::batch(vec![agents_task, compat_cleanup_task])
}

pub(super) fn handle_agents_config_saved(
    app: &mut App,
    result: Result<(), String>,
) -> Task<Message> {
    app.agents_settings.save_in_flight = false;
    app.agents_settings.save_error =
        result.err().map(crate::app::config::server_config_unreachable_error);

    if app.agents_settings.save_pending { persist_agents_settings(app) } else { Task::none() }
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Agents(message) = message else {
        return Task::none();
    };

    match message {
        AgentsMessage::Refresh => {
            app.agents_settings.loading = true;
            app.agents_settings.config_ready = false;
            app.agents_settings.save_error = None;
            app.agents_settings.project_path = app.project_path.clone();
            Task::batch(vec![refresh_task(), config_task(app.agents_settings.project_path.clone())])
        }
        AgentsMessage::Loaded(res) => {
            match res {
                Ok((providers, provider_models, available_tools)) => {
                    app.agents_settings.providers = providers;
                    app.agents_settings.provider_models = provider_models;
                    let available_tools =
                        crate::app::state::filter_mcp_from_available_tools(available_tools);
                    if app.agents_settings.available_tools.is_empty() && !available_tools.is_empty()
                    {
                        crate::app::state::apply_default_allowed_tools(
                            &mut app.agents_settings.entries,
                            &available_tools,
                        );
                    }
                    if !available_tools.is_empty() || app.agents_settings.available_tools.is_empty()
                    {
                        app.agents_settings.available_tools = available_tools;
                    }
                }
                Err(error) => {
                    app.agents_settings.save_error = Some(error);
                }
            }
            Task::none()
        }
        AgentsMessage::ConfigLoaded { project_path, result } => {
            if app.agents_settings.project_path != project_path {
                return Task::none();
            }

            app.agents_settings.loading = false;
            match result {
                Ok(config) => {
                    apply_config(app, config.as_ref());
                    app.agents_settings.config_ready = true;
                }
                Err(error) => {
                    app.agents_settings.config_ready = false;
                    app.agents_settings.save_error = Some(error);
                }
            }
            Task::none()
        }
        AgentsMessage::ProjectSelected(project_path) => {
            if app.project_path.as_deref() == Some(project_path.as_str()) {
                return crate::app::projects::project_settings_refresh_task(project_path);
            }
            app.open_project_and_index(project_path)
        }
        AgentsMessage::ProjectChanged(project_path) => {
            app.agents_settings.project_path = Some(project_path.clone());
            app.agents_settings.loading = true;
            app.agents_settings.config_ready = false;
            app.agents_settings.save_error = None;
            project_config_task(project_path)
        }
        AgentsMessage::ProjectConfigLoaded { project_path, result } => {
            if app.agents_settings.project_path.as_deref() != Some(project_path.as_str()) {
                return Task::none();
            }

            app.agents_settings.loading = false;
            match result {
                Ok(config) => {
                    apply_config(app, config.as_ref());
                    app.agents_settings.config_ready = true;
                    app.agents_settings.save_error = None;
                }
                Err(error) => {
                    app.agents_settings.config_ready = false;
                    app.agents_settings.save_error = Some(error);
                }
            }
            Task::none()
        }
        AgentsMessage::SelectAgent(agent_key) => {
            app.agents_settings.selected_agent = agent_key.clone();
            app.agents_settings.save_error = None;
            if app.agents_settings.active_prompt_tab == AGENT_PROMPT_SYSTEM_TAB {
                Task::none()
            } else {
                load_workspace_identity_file_task(
                    agent_key,
                    app.agents_settings.active_prompt_tab.clone(),
                )
            }
        }
        AgentsMessage::AddAgentKeyChanged(value) => {
            app.agents_settings.new_agent_key_input = value;
            app.agents_settings.save_error = None;
            Task::none()
        }
        AgentsMessage::AddAgentRequested => {
            let key = normalize_agent_key(&app.agents_settings.new_agent_key_input);
            if key.is_empty() {
                app.agents_settings.save_error =
                    Some("智能体 key 不能为空，只允许字母、数字、-、_。".to_string());
                return Task::none();
            }
            if app.agents_settings.entries.iter().any(|entry| entry.key == key) {
                app.agents_settings.save_error = Some(format!("智能体 {key} 已存在。"));
                return Task::none();
            }

            let mut entry = DelegateAgentSettingsEntry::from_config(&key, None);
            crate::app::state::apply_default_allowed_tools(
                std::slice::from_mut(&mut entry),
                &app.agents_settings.available_tools,
            );
            app.agents_settings.entries.push(entry);
            app.agents_settings.new_agent_key_input.clear();
            app.agents_settings.selected_agent = key;
            app.agents_settings.save_error = None;
            let selected_agent = app.agents_settings.selected_agent.clone();
            let active_prompt_tab = app.agents_settings.active_prompt_tab.clone();

            let mut tasks = vec![persist_agents_settings(app)];
            if active_prompt_tab != AGENT_PROMPT_SYSTEM_TAB {
                tasks.push(load_workspace_identity_file_task(selected_agent, active_prompt_tab));
            }
            Task::batch(tasks)
        }
        AgentsMessage::DetailTabSelected(tab_key) => {
            app.agents_settings.active_detail_tab = tab_key;
            app.agents_settings.save_error = None;
            Task::none()
        }
        AgentsMessage::PromptTabSelected(tab_key) => {
            app.agents_settings.active_prompt_tab = tab_key.clone();
            app.agents_settings.save_error = None;
            if tab_key == AGENT_PROMPT_SYSTEM_TAB {
                Task::none()
            } else {
                load_workspace_identity_file_task(
                    app.agents_settings.selected_agent.clone(),
                    tab_key,
                )
            }
        }
        AgentsMessage::WorkspaceIdentityLoaded {
            agent_key,
            file_name,
            workspace_root_path,
            result,
        } => {
            if agent_key != app.agents_settings.selected_agent {
                return Task::none();
            }
            app.agents_settings.workspace_identity_root_path = workspace_root_path;
            match result {
                Ok((content, size_bytes, modified_at_ms)) => {
                    if let Some(file) = find_workspace_file_mut(app, &file_name) {
                        file.editor = text_editor::Content::with_text(&content);
                        file.size_bytes = size_bytes;
                        file.modified_at_ms = modified_at_ms;
                    }
                    app.agents_settings.save_error = None;
                }
                Err(error) => {
                    app.agents_settings.save_error = Some(error);
                }
            }
            Task::none()
        }
        AgentsMessage::WorkspaceIdentityRestoreDefaultRequested(file_name) => {
            app.agents_settings.save_error = None;
            restore_workspace_identity_default_task(
                app.agents_settings.selected_agent.clone(),
                file_name,
            )
        }
        AgentsMessage::WorkspaceIdentityEnabledToggled { agent_key, file_name, enabled } => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                if enabled {
                    if !entry
                        .enabled_identity_files
                        .iter()
                        .any(|enabled_file| enabled_file == &file_name)
                    {
                        entry.enabled_identity_files.push(file_name);
                        entry.enabled_identity_files.sort();
                    }
                } else {
                    entry.enabled_identity_files.retain(|enabled_file| enabled_file != &file_name);
                }
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::WorkspaceIdentityDefaultRestored {
            agent_key,
            file_name,
            workspace_root_path,
            result,
        } => {
            if agent_key != app.agents_settings.selected_agent {
                return Task::none();
            }

            app.agents_settings.workspace_identity_root_path = workspace_root_path;
            match result {
                Ok((content, size_bytes, modified_at_ms)) => {
                    if let Some(file) = find_workspace_file_mut(app, &file_name) {
                        file.editor = text_editor::Content::with_text(&content);
                        file.size_bytes = size_bytes;
                        file.modified_at_ms = modified_at_ms;
                    }
                    app.agents_settings.save_error = None;
                }
                Err(error) => {
                    app.agents_settings.save_error = Some(error);
                }
            }
            Task::none()
        }
        AgentsMessage::WorkspaceIdentityEditorAction(file_name, action) => {
            let should_persist = matches!(action, text_editor::Action::Edit(_));
            let mut pending_save = None;
            if let Some(file) = find_workspace_file_mut(app, &file_name) {
                file.editor.perform(action);
                if should_persist {
                    pending_save = Some(file.editor.text());
                }
            }
            if let Some(text) = pending_save {
                app.agents_settings.save_error = None;
                return save_workspace_identity_file_task(
                    app.agents_settings.selected_agent.clone(),
                    file_name,
                    text,
                );
            }
            Task::none()
        }
        AgentsMessage::WorkspaceIdentitySaved { file_name, size_bytes, modified_at_ms, result } => {
            if let Err(error) = result {
                app.agents_settings.save_error = Some(error);
            } else {
                if let Some(file) = find_workspace_file_mut(app, &file_name) {
                    file.size_bytes = size_bytes;
                    file.modified_at_ms = modified_at_ms;
                }
                app.agents_settings.save_error = None;
            }
            Task::none()
        }
        AgentsMessage::ProviderChanged(agent_key, provider) => {
            if let Some(entry) = find_entry_mut(app, &agent_key)
                && entry.provider != provider
            {
                entry.provider = provider;
                entry.model.clear();
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::ProviderCleared(agent_key) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.provider.clear();
                entry.model.clear();
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::ModelChanged(agent_key, model) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.model = model;
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::ModelCleared(agent_key) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.model.clear();
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::ForceModelSelectionToggled(agent_key, force_model_selection) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.force_model_selection = force_model_selection;
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::EnabledToggled(agent_key, enabled) => {
            if let Some(entry) = find_entry_mut(app, &agent_key)
                && entry.key != MAIN_AGENT_KEY
            {
                entry.enabled = enabled;
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::AllowAsSubagentToggled(agent_key, allow_as_subagent) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.allow_as_subagent = allow_as_subagent;
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::MaxParallelSubagentsChanged(agent_key, max_parallel_subagents) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.max_parallel_subagents = max_parallel_subagents.clamp(1, 32);
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::SystemPromptAction(agent_key, action) => {
            let should_persist = matches!(action, text_editor::Action::Edit(_));
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.system_prompt_editor.perform(action);
            }
            if should_persist {
                app.agents_settings.save_error = None;
                return persist_agents_settings(app);
            }
            Task::none()
        }
        AgentsMessage::ApiKeyChanged(agent_key, api_key) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.api_key_input = api_key;
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::TemperatureChanged(agent_key, temperature) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.temperature = temperature.clamp(0.0, 2.0);
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::CompactContextToggled(agent_key, value) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.compact_context = value;
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::MaxToolIterationsChanged(agent_key, value) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.max_tool_iterations = value.max(1);
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::MaxHistoryMessagesChanged(agent_key, value) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.max_history_messages = value.clamp(1, 1000);
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::ParallelToolsToggled(agent_key, value) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.parallel_tools = value;
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::ToolDispatcherChanged(agent_key, value) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.tool_dispatcher = value;
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::MaxDepthChanged(agent_key, max_depth) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.max_depth = max_depth.clamp(1, 32);
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::AgenticToggled(agent_key, agentic) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.agentic = agentic;
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::ToolPolicyChanged(agent_key, tool_policy) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.tool_policy = tool_policy;
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::AllowedToolToggled(agent_key, tool_id, enabled) => {
            if !enabled && vw_config_types::tools::is_builtin_required_tool_id(&tool_id) {
                return Task::none();
            }
            let available_tools = app.agents_settings.available_tools.clone();
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                if enabled {
                    if !entry.allowed_tools.iter().any(|tool| tool == &tool_id) {
                        entry.allowed_tools.push(tool_id);
                        entry.allowed_tools.sort();
                    }
                } else {
                    entry.allowed_tools.retain(|tool| tool != &tool_id);
                    entry.allowed_tools = crate::app::state::merge_builtin_required_tools(
                        &entry.allowed_tools,
                        &available_tools,
                    );
                }
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::AllowedToolsSelectAll(agent_key) => {
            let available_tools = app.agents_settings.available_tools.clone();
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.allowed_tools = crate::app::state::merge_builtin_required_tools(
                    &available_tools,
                    &available_tools,
                );
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::AllowedToolsInvertSelection(agent_key) => {
            let available_tools = app.agents_settings.available_tools.clone();
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                let inverted = available_tools
                    .iter()
                    .filter(|tool| !entry.allowed_tools.iter().any(|selected| selected == *tool))
                    .cloned()
                    .collect::<Vec<_>>();
                entry.allowed_tools =
                    crate::app::state::merge_builtin_required_tools(&inverted, &available_tools);
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::AllowedToolsApplyPreset(agent_key, preset_key) => {
            let available_tools = app.agents_settings.available_tools.clone();
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                let preset_tools = tools_for_preset(&available_tools, &preset_key);
                entry.allowed_tools = crate::app::state::merge_builtin_required_tools(
                    &preset_tools,
                    &available_tools,
                );
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::ToolsListTabSelected(tab_key) => {
            app.agents_settings.active_tools_list_tab = tab_key;
            Task::none()
        }
        AgentsMessage::SkillPolicyChanged(agent_key, skill_policy) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.skill_policy = skill_policy;
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::AllowedSkillToggled(agent_key, skill_id, enabled) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                if enabled {
                    if !entry.allowed_skills.iter().any(|skill| skill == &skill_id) {
                        entry.allowed_skills.push(skill_id);
                        entry.allowed_skills.sort();
                    }
                } else {
                    entry.allowed_skills.retain(|skill| skill != &skill_id);
                }
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::AllowedSkillsSelectAll(agent_key, scope) => {
            let available_skills = app
                .skills_settings
                .catalog
                .iter()
                .filter(|skill| match scope {
                    crate::app::state::SkillsDirectoryScope::Project => skill.source == "workspace",
                    crate::app::state::SkillsDirectoryScope::Ancestor => skill.source == "ancestor",
                    crate::app::state::SkillsDirectoryScope::Global => skill.source == "global",
                    crate::app::state::SkillsDirectoryScope::Bundled => skill.source == "bundled",
                    crate::app::state::SkillsDirectoryScope::All => true,
                })
                .filter(|skill| skill.enabled)
                .map(|skill| skill.id.clone())
                .collect::<Vec<_>>();
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                for skill in available_skills {
                    if !entry.allowed_skills.iter().any(|selected| selected == &skill) {
                        entry.allowed_skills.push(skill);
                    }
                }
                entry.allowed_skills.sort();
                entry.allowed_skills.dedup();
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::AllowedSkillsInvertSelection(agent_key, scope) => {
            let available_skills = app
                .skills_settings
                .catalog
                .iter()
                .filter(|skill| match scope {
                    crate::app::state::SkillsDirectoryScope::Project => skill.source == "workspace",
                    crate::app::state::SkillsDirectoryScope::Ancestor => skill.source == "ancestor",
                    crate::app::state::SkillsDirectoryScope::Global => skill.source == "global",
                    crate::app::state::SkillsDirectoryScope::Bundled => skill.source == "bundled",
                    crate::app::state::SkillsDirectoryScope::All => true,
                })
                .filter(|skill| skill.enabled)
                .map(|skill| skill.id.clone())
                .collect::<Vec<_>>();
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                for skill in available_skills {
                    if entry.allowed_skills.iter().any(|selected| selected == &skill) {
                        entry.allowed_skills.retain(|selected| selected != &skill);
                    } else {
                        entry.allowed_skills.push(skill);
                    }
                }
                entry.allowed_skills.sort();
                entry.allowed_skills.dedup();
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
        AgentsMessage::MaxIterationsChanged(agent_key, max_iterations) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.max_iterations = max_iterations.max(1);
            }
            app.agents_settings.save_error = None;
            persist_agents_settings(app)
        }
    }
}

#[cfg(test)]
#[path = "agents_tests.rs"]
mod tests;
