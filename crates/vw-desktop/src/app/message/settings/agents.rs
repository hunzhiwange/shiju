//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use super::types::AgentsLoaded;
use crate::app::i18n::settings_models::messages::Feedback;
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
                source_label: Feedback::owned(source_label, vec![]),
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
    Task::batch([refresh_providers_task(), refresh_tools_task()])
}

// Load independently: provider failures must not hide tools, and WASM cannot use
// the legacy synchronous gateway shim (which returns an empty default).
fn refresh_tools_task() -> Task<Message> {
    Task::perform(async { crate::app::config::gateway_client()?.tools_list().await }, |result| {
        Message::Settings(SettingsMessage::Agents(AgentsMessage::ToolsLoaded(result)))
    })
}

fn refresh_providers_task() -> Task<Message> {
    Task::perform(
        async move {
            let providers = load_gateway_providers().await?;
            let provider_summaries = summarize_providers(providers.clone());
            let provider_models = summarize_models(providers);
            Ok((provider_summaries, provider_models, Vec::new()))
        },
        |res: AgentsLoaded| Message::Settings(SettingsMessage::Agents(AgentsMessage::Loaded(res))),
    )
}

pub(super) fn config_task(_project_path: Option<String>) -> Task<Message> {
    Task::perform(async { load_catalog_config().await.map(Box::new) }, |result| {
        Message::Settings(SettingsMessage::Agents(AgentsMessage::ConfigLoaded {
            project_path: None,
            result,
        }))
    })
}

fn refresh_skill_catalog(app: &mut App) -> Task<Message> {
    let directory = app.agents_settings.skill_directory.clone();
    app.agents_settings.skill_catalog_generation += 1;
    let generation = app.agents_settings.skill_catalog_generation;
    app.agents_settings.skill_catalog_loading = true;
    app.agents_settings.skill_catalog_error = None;
    app.agents_settings.skill_catalog.clear();
    Task::perform(
        async move {
            let result = crate::app::session_gateway::gateway_skills_async(directory.as_deref())
                .await
                .map(super::skills::map_catalog_items);
            (directory, result)
        },
        move |(directory, result)| {
            Message::Settings(SettingsMessage::Agents(AgentsMessage::SkillCatalogLoaded {
                directory,
                generation,
                result,
            }))
        },
    )
}

#[derive(serde::Deserialize)]
struct CatalogAgent {
    key: String,
    #[serde(flatten)]
    definition: vw_config_types::agent::DelegateAgentConfig,
}

async fn load_catalog_config() -> Result<vw_config_types::config::Config, String> {
    let mut config: vw_config_types::config::Config =
        serde_json::from_value(crate::app::config::load_global_config_value_async().await?)
            .map_err(|error| error.to_string())?;
    let value = crate::app::config::gateway_client()?.agent_profiles(None, true).await?;
    merge_catalog_profiles(&mut config, value)?;
    Ok(config)
}

pub(in crate::app) fn merge_catalog_profiles(
    config: &mut vw_config_types::config::Config,
    value: serde_json::Value,
) -> Result<(), String> {
    let catalog: Vec<CatalogAgent> =
        serde_json::from_value(value).map_err(|error| format!("智能体目录响应无效：{error}"))?;
    for mut agent in catalog {
        // The server owns official identities and backend bindings, including disabled presets.
        if let Some(original) = config.agents.get(&agent.key) {
            agent.definition.api_key = original.api_key.clone();
            agent.definition.options = original.options.clone();
        }
        config.agents.insert(agent.key, agent.definition);
    }
    Ok(())
}

fn apply_config(app: &mut App, config: &vw_config_types::config::Config) {
    let mut entries = crate::app::state::agent_settings_entries_from_config(config);
    crate::app::state::apply_default_allowed_tools(
        &mut entries,
        &app.agents_settings.available_tools,
    );
    if let Some(registered_tools) = app.agents_settings.registered_tools.as_deref() {
        crate::app::state::retain_available_allowed_tools(&mut entries, registered_tools);
    }
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
    if app.agents_settings.profile_modal.as_ref().is_some_and(|draft| draft.key == agent_key) {
        return app.agents_settings.profile_modal.as_mut();
    }
    app.agents_settings.entries.iter_mut().find(|entry| entry.key == agent_key)
}

fn tool_matches_any(tool_id: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| tool_id == *candidate)
}

fn tool_in_preset(tool_id: &str, preset_key: &str) -> bool {
    match preset_key {
        "minimal" => tool_matches_any(
            tool_id,
            &["ls", "read", "file_read", "pdf_read", "grep", "glob", "memory_recall", "question"],
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
            agent.insert("allowed_tools".to_string(), serde_json::json!(allowed_tools));
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
                "allowed_tools": allowed_tools,
                "skill_policy": entry.skill_policy,
                "allowed_skills": allowed_skills,
                "enabled_identity_files": enabled_identity_files,
            })
            .as_object()
            .expect("agent settings patch is an object")
            .clone();
        }
        agent.insert("compact_context".to_string(), serde_json::json!(entry.compact_context));
        agent.insert(
            "max_history_messages".to_string(),
            serde_json::json!(entry.max_history_messages.clamp(1, 1000)),
        );
        agent.insert("parallel_tools".to_string(), serde_json::json!(entry.parallel_tools));
        agent.insert("max_iterations".to_string(), serde_json::json!(entry.max_iterations.max(1)));
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
    }

    Ok(serde_json::Value::Object(patch))
}

async fn persist_agents_patch(patch: serde_json::Value) -> Result<(), String> {
    crate::app::config::patch_full_agent_config_async(patch).await
}

fn persist_agents_settings(app: &mut App) -> Task<Message> {
    if !app.agents_settings.config_ready {
        app.agents_settings.save_error =
            Some("Gateway 配置尚未加载完成，已拒绝保存默认快照。请刷新后重试。".to_string());
        return Task::none();
    }

    if app.agents_settings.save_in_flight {
        return Task::none();
    }

    #[cfg(not(target_arch = "wasm32"))]
    app.note_startup_setting_edit("agents_settings");
    app.agents_settings.save_in_flight = true;
    app.agents_settings.dirty = false;
    app.agents_settings.save_error = None;
    let entries = app.agents_settings.entries.clone();
    let agents_task = crate::app::config::spawn_gateway_task("agent", async move {
        let patch = build_agents_settings_patch(&entries)?;
        persist_agents_patch(patch).await
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
    match result {
        Ok(()) => {
            app.agents_settings.save_error = None;
            if app.agents_settings.dirty {
                app.show_localized_toast(
                    crate::app::state::ToastKind::Success,
                    "已保存提交的配置；后续修改尚未保存",
                    vec![],
                )
            } else {
                app.show_localized_toast(
                    crate::app::state::ToastKind::Success,
                    "智能体配置已保存，将用于下一次请求",
                    vec![],
                )
            }
        }
        Err(error) => {
            app.agents_settings.dirty = true;
            let message = format!("保存智能体配置失败：{error}");
            app.agents_settings.save_error = Some(message.clone());
            app.agents_settings.save_error_translation =
                Some(crate::app::i18n::format::Message::new(
                    "保存智能体配置失败：%s",
                    vec![error.clone().into()],
                ));
            app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "保存智能体配置失败：%s",
                vec![error.into()],
            )
        }
    }
}

fn mark_agents_settings_dirty(app: &mut App) -> Task<Message> {
    if !app.agents_settings.config_ready {
        app.agents_settings.save_error =
            Some("Gateway 配置尚未加载完成，已拒绝保存默认快照。请刷新后重试。".to_string());
        return Task::none();
    }
    if app.agents_settings.profile_modal.is_none() {
        app.agents_settings.dirty = true;
    }
    Task::none()
}

fn catalog_can_write(app: &App) -> bool {
    let settings = &app.agents_settings;
    settings.config_ready
        && !settings.save_in_flight
        && !settings.profile_saving
        && !settings.dirty
        && settings.profile_modal.is_none()
}

fn catalog_error(app: &mut App, error: String) -> Task<Message> {
    app.show_localized_toast(
        crate::app::state::ToastKind::Error,
        "保存智能体配置失败：%s",
        vec![error.into()],
    )
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Agents(message) = message else {
        return Task::none();
    };

    if app.agents_settings.catalog_busy
        && !matches!(
            message,
            AgentsMessage::CatalogToggled { .. } | AgentsMessage::CatalogCopied(_)
        )
    {
        return Task::none();
    }
    if app.agents_settings.profile_saving && !matches!(message, AgentsMessage::ProfileSaved(_)) {
        return Task::none();
    }
    let invert_skill_selection =
        matches!(&message, AgentsMessage::AllowedSkillsInvertSelection(_, _));
    match message {
        AgentsMessage::CatalogTabSelected(official) => {
            app.agents_settings.catalog_official = official;
            Task::none()
        }
        AgentsMessage::CatalogQueryChanged(query) => {
            app.agents_settings.catalog_query = query;
            Task::none()
        }
        AgentsMessage::CatalogRuntimeSettings(backend) => {
            app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Acp;
            super::acp::update(app, SettingsMessage::Acp(super::AcpMessage::EditAgent(backend)))
        }
        AgentsMessage::CatalogToggle(key, enabled) => {
            if let Some(draft) = app.agents_settings.profile_modal.as_mut() {
                if draft.key == key && draft.can_disable {
                    draft.enabled = enabled;
                }
                return Task::none();
            }
            if !catalog_can_write(app)
                || key == MAIN_AGENT_KEY
                || !app.agents_settings.entries.iter().any(|entry| entry.key == key)
            {
                return Task::none();
            }
            app.agents_settings.catalog_busy = true;
            let patch = serde_json::json!({"agents": {&key: {"enabled": enabled}}});
            Task::perform(persist_agents_patch(patch), move |result| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::CatalogToggled {
                    key: key.clone(),
                    enabled,
                    result,
                }))
            })
        }
        AgentsMessage::CatalogToggled { key, enabled, result } => {
            if !app.agents_settings.catalog_busy {
                return Task::none();
            }
            app.agents_settings.catalog_busy = false;
            match result {
                Ok(()) => {
                    if let Some(entry) = find_entry_mut(app, &key) {
                        entry.enabled = enabled;
                    }
                    Task::none()
                }
                Err(error) => catalog_error(app, error),
            }
        }
        AgentsMessage::CatalogCopy(key) => {
            if !catalog_can_write(app)
                || !app.agents_settings.entries.iter().any(|entry| entry.key == key)
            {
                return Task::none();
            }
            app.agents_settings.catalog_busy = true;
            Task::perform(
                async move {
                    crate::app::config::gateway_client()?.agent_profile_copy(None, &key).await
                },
                |result| {
                    Message::Settings(SettingsMessage::Agents(AgentsMessage::CatalogCopied(result)))
                },
            )
        }
        AgentsMessage::CatalogCopied(result) => {
            if !app.agents_settings.catalog_busy {
                return Task::none();
            }
            app.agents_settings.catalog_busy = false;
            match result.and_then(|value| {
                serde_json::from_value::<CatalogAgent>(value).map_err(|error| error.to_string())
            }) {
                Ok(agent)
                    if !agent.definition.builtin
                        && !app
                            .agents_settings
                            .entries
                            .iter()
                            .any(|entry| entry.key == agent.key) =>
                {
                    app.agents_settings.selected_agent = agent.key.clone();
                    app.agents_settings.entries.push(DelegateAgentSettingsEntry::from_config(
                        &agent.key,
                        Some(agent.definition),
                    ));
                    app.agents_settings.catalog_official = false;
                    app.agents_settings.catalog_query.clear();
                    Task::none()
                }
                Ok(_) => catalog_error(app, "复制目录返回重复或内置身份".into()),
                Err(error) => catalog_error(app, error),
            }
        }
        AgentsMessage::CatalogLaunch(key) => {
            if !catalog_can_write(app)
                || app.new_session_creating
                || !app
                    .agents_settings
                    .entries
                    .iter()
                    .any(|entry| entry.key == key && entry.can_start_session())
            {
                return Task::none();
            }
            let Some(project) = app.project_path.clone().filter(|path| !path.trim().is_empty())
            else {
                return catalog_error(app, "请先打开项目，再创建新会话".into());
            };
            let draft =
                crate::app::message::project::open_session_draft(app, project.clone(), project);
            let selection = crate::app::message::chat::session::update(
                app,
                crate::app::message::chat::ChatMessage::SessionAgentSelected(Some(key)),
            );
            app.show_system_settings = false;
            app.show_settings = false;
            app.screen = crate::app::Screen::Project;
            Task::batch([draft, selection])
        }
        AgentsMessage::Save => persist_agents_settings(app),
        AgentsMessage::DiscardChanges => {
            if app.agents_settings.save_in_flight {
                return Task::none();
            }
            app.agents_settings.dirty = false;
            update(app, SettingsMessage::Agents(AgentsMessage::Refresh))
        }
        AgentsMessage::Refresh => {
            if app.agents_settings.dirty || app.agents_settings.save_in_flight {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "请先保存或放弃智能体配置修改，再刷新",
                    vec![],
                );
            }
            app.agents_settings.loading = true;
            app.agents_settings.config_ready = false;
            app.agents_settings.registered_tools = None;
            app.agents_settings.save_error = None;
            app.agents_settings.project_path = None;
            Task::batch(vec![refresh_task(), config_task(app.agents_settings.project_path.clone())])
        }
        AgentsMessage::Loaded(res) => {
            // Catalog normalization is part of loading, not a user edit.
            match res {
                Ok((providers, provider_models, available_tools)) => {
                    if !app.agents_settings.dirty && !app.agents_settings.save_in_flight {
                        app.agents_settings.save_error = None;
                    }
                    app.agents_settings.providers = providers;
                    app.agents_settings.provider_models = provider_models;
                    if !available_tools.is_empty() {
                        return update(
                            app,
                            SettingsMessage::Agents(AgentsMessage::ToolsLoaded(Ok(
                                available_tools,
                            ))),
                        );
                    }
                }
                Err(error) => {
                    app.agents_settings.save_error = Some(error);
                }
            }
            Task::none()
        }
        AgentsMessage::ToolsLoaded(result) => {
            match result {
                Ok(available_tools) => {
                    if available_tools.is_empty() {
                        return Task::none();
                    }
                    let registered_tools =
                        crate::app::state::normalize_available_tool_catalog(available_tools);
                    let available_tools = crate::app::state::filter_mcp_from_available_tools(
                        registered_tools.clone(),
                    );
                    if !app.agents_settings.dirty && !app.agents_settings.save_in_flight {
                        if app.agents_settings.available_tools.is_empty()
                            && !available_tools.is_empty()
                        {
                            crate::app::state::apply_default_allowed_tools(
                                &mut app.agents_settings.entries,
                                &available_tools,
                            );
                        }
                        crate::app::state::retain_available_allowed_tools(
                            &mut app.agents_settings.entries,
                            &registered_tools,
                        );
                    }
                    app.agents_settings.available_tools = available_tools;
                    app.agents_settings.registered_tools = Some(registered_tools);
                }
                Err(error) => {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "加载工具列表失败：%s",
                        vec![error.into()],
                    );
                }
            }
            Task::none()
        }
        AgentsMessage::ConfigLoaded { project_path, result } => {
            if app.agents_settings.dirty || app.agents_settings.save_in_flight {
                return Task::none();
            }
            if project_path.is_some() {
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
        AgentsMessage::ProjectSelected(directory) => update(
            app,
            SettingsMessage::Agents(AgentsMessage::SkillDirectorySelected(Some(directory))),
        ),
        AgentsMessage::SkillDirectorySelected(directory) => {
            app.agents_settings.skill_directory = directory;
            refresh_skill_catalog(app)
        }
        AgentsMessage::SkillCatalogRefresh => refresh_skill_catalog(app),
        AgentsMessage::SkillDirectoryScopeChanged(scope) => {
            app.agents_settings.skill_directory_scope = scope;
            Task::none()
        }
        AgentsMessage::SkillCatalogLoaded { directory, generation, result } => {
            if directory != app.agents_settings.skill_directory
                || generation != app.agents_settings.skill_catalog_generation
            {
                return Task::none();
            }
            app.agents_settings.skill_catalog_loading = false;
            match result {
                Ok(catalog) => app.agents_settings.skill_catalog = catalog,
                Err(error) => app.agents_settings.skill_catalog_error = Some(error),
            }
            Task::none()
        }
        AgentsMessage::ProjectChanged(_) => {
            // Application workspace changes never reload or retarget global definitions.
            app.agents_settings.project_path = None;
            Task::none()
        }
        AgentsMessage::ProjectConfigLoaded { .. } => Task::none(),
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
        AgentsMessage::AddAgentOpened => {
            if catalog_can_write(app) {
                app.agents_settings.new_agent_key_input.clear();
                app.agents_settings.profile_entering_key = true;
            }
            Task::none()
        }
        AgentsMessage::AddAgentKeyChanged(value) => {
            app.agents_settings.new_agent_key_input = value;
            if !app.agents_settings.profile_entering_key {
                app.agents_settings.save_error = None;
            }
            Task::none()
        }
        AgentsMessage::AddAgentRequested => {
            if !catalog_can_write(app) {
                return Task::none();
            }
            let entering_key = app.agents_settings.profile_entering_key;
            let key = normalize_agent_key(&app.agents_settings.new_agent_key_input);
            if key.is_empty() {
                if entering_key {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "智能体 key 不能为空，只允许字母、数字、-、_。",
                        vec![],
                    );
                }
                app.agents_settings.save_error =
                    Some("智能体 key 不能为空，只允许字母、数字、-、_。".to_string());
                return Task::none();
            }
            if app.agents_settings.entries.iter().any(|entry| entry.key == key) {
                if entering_key {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "智能体 %s 已存在。",
                        vec![key.into()],
                    );
                }
                app.agents_settings.save_error = Some(format!("智能体 {key} 已存在。"));
                app.agents_settings.save_error_translation = Some(
                    crate::app::i18n::format::Message::new("智能体 %s 已存在。", vec![key.into()]),
                );
                return Task::none();
            }

            let mut entry = DelegateAgentSettingsEntry::from_config(&key, None);
            crate::app::state::apply_default_allowed_tools(
                std::slice::from_mut(&mut entry),
                &app.agents_settings.available_tools,
            );
            app.agents_settings.active_prompt_tab = AGENT_PROMPT_SYSTEM_TAB.to_string();
            app.agents_settings.workspace_identity_root_path = None;
            app.agents_settings.profile_modal = Some(entry);
            app.agents_settings.profile_entering_key = false;
            app.agents_settings.new_agent_key_input.clear();
            app.agents_settings.selected_agent = key;
            app.agents_settings.active_detail_tab =
                crate::app::state::AGENT_DETAIL_BASIC_TAB.to_string();
            if !entering_key {
                app.agents_settings.save_error = None;
            }
            let catalog = if app.acp_agents.is_empty() && !app.acp_settings.loading {
                super::acp::update(app, SettingsMessage::Acp(super::AcpMessage::Refresh))
            } else {
                Task::none()
            };
            Task::batch([refresh_skill_catalog(app), catalog])
        }
        AgentsMessage::DetailTabSelected(tab_key) => {
            let acp_draft = app.agents_settings.profile_modal.as_ref().is_some_and(|draft| {
                matches!(draft.runtime, vw_config_types::agent::AgentRuntime::Acp { .. })
            });
            app.agents_settings.active_detail_tab =
                if acp_draft && tab_key == crate::app::state::AGENT_DETAIL_TOOLS_TAB {
                    crate::app::state::AGENT_DETAIL_BASIC_TAB.to_string()
                } else {
                    tab_key
                };
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
            mark_agents_settings_dirty(app)
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
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::ProviderCleared(agent_key) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.provider.clear();
                entry.model.clear();
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::ModelChanged(agent_key, model) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.model = model;
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::ModelCleared(agent_key) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.model.clear();
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::ForceModelSelectionToggled(agent_key, force_model_selection) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.force_model_selection = false;
                if let vw_config_types::agent::AgentDefault::Fixed(model) = &entry.defaults.model {
                    entry.provider = model.provider.clone();
                    entry.model = model.model.clone();
                    entry.force_model_selection = force_model_selection;
                }
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::EnabledToggled(agent_key, enabled) => {
            if let Some(entry) = find_entry_mut(app, &agent_key)
                && entry.key != MAIN_AGENT_KEY
            {
                entry.enabled = enabled;
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::AllowAsSubagentToggled(agent_key, allow_as_subagent) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.allow_as_subagent = allow_as_subagent;
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::MaxParallelSubagentsChanged(agent_key, max_parallel_subagents) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.max_parallel_subagents = max_parallel_subagents.clamp(1, 32);
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::SystemPromptAction(agent_key, action) => {
            let should_persist = matches!(action, text_editor::Action::Edit(_));
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.system_prompt_editor.perform(action);
            }
            if should_persist {
                app.agents_settings.save_error = None;
                return mark_agents_settings_dirty(app);
            }
            Task::none()
        }
        AgentsMessage::ApiKeyChanged(agent_key, api_key) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.api_key_input = api_key;
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::TemperatureChanged(agent_key, temperature) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.temperature = temperature.clamp(0.0, 2.0);
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::CompactContextToggled(agent_key, value) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.compact_context = value;
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::MaxHistoryMessagesChanged(agent_key, value) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.max_history_messages = value.clamp(1, 1000);
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::ParallelToolsToggled(agent_key, value) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.parallel_tools = value;
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::MaxDepthChanged(agent_key, max_depth) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.max_depth = max_depth.clamp(1, 32);
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::AgenticToggled(agent_key, agentic) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.agentic = agentic;
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
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
            mark_agents_settings_dirty(app)
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
            mark_agents_settings_dirty(app)
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
            mark_agents_settings_dirty(app)
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
            mark_agents_settings_dirty(app)
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
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::AllowedSkillToggled(agent_key, skill_id, enabled) => {
            let available = app
                .agents_settings
                .skill_catalog
                .iter()
                .any(|skill| skill.id == skill_id && skill.installed && skill.enabled);
            if let Some(entry) = find_entry_mut(app, &agent_key)
                && let vw_config_types::agent::AgentDefault::Fixed(ids) = &mut entry.defaults.skills
            {
                if enabled
                    && (!available
                        || (entry.skill_policy
                            == vw_config_types::agent::DelegateCapabilityPolicy::Strict
                            && !entry.allowed_skills.contains(&skill_id)))
                {
                    return catalog_error(
                        app,
                        "Skill is unavailable or outside the strict skill boundary".into(),
                    );
                }
                ids.retain(|id| id != &skill_id);
                if enabled {
                    ids.push(skill_id);
                }
                ids.sort();
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::AllowedSkillsSelectAll(agent_key, scope)
        | AgentsMessage::AllowedSkillsInvertSelection(agent_key, scope) => {
            let invert = invert_skill_selection;
            let available = app
                .agents_settings
                .skill_catalog
                .iter()
                .filter(|skill| skill.installed && skill.enabled)
                .filter(|skill| match scope {
                    crate::app::state::SkillsDirectoryScope::Project => skill.source == "workspace",
                    crate::app::state::SkillsDirectoryScope::Ancestor => skill.source == "ancestor",
                    crate::app::state::SkillsDirectoryScope::Global => skill.source == "global",
                    crate::app::state::SkillsDirectoryScope::Bundled => skill.source == "bundled",
                    crate::app::state::SkillsDirectoryScope::All => true,
                })
                .map(|skill| skill.id.clone())
                .collect::<Vec<_>>();
            if let Some(entry) = find_entry_mut(app, &agent_key)
                && let vw_config_types::agent::AgentDefault::Fixed(ids) = &mut entry.defaults.skills
            {
                for id in available {
                    if entry.skill_policy
                        == vw_config_types::agent::DelegateCapabilityPolicy::Strict
                        && !entry.allowed_skills.contains(&id)
                    {
                        continue;
                    }
                    if invert && ids.contains(&id) {
                        ids.retain(|selected| selected != &id);
                    } else if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
                ids.sort();
                ids.dedup();
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        AgentsMessage::MaxIterationsChanged(agent_key, max_iterations) => {
            if let Some(entry) = find_entry_mut(app, &agent_key) {
                entry.max_iterations = max_iterations.max(1);
            }
            app.agents_settings.save_error = None;
            mark_agents_settings_dirty(app)
        }
        message => profile::update(app, message),
    }
}

#[cfg(test)]
#[path = "agents_tests.rs"]
mod tests;

#[path = "agents_profile.rs"]
mod profile;
