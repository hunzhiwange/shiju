//! 智能体资料草稿消息与保存事务。
use super::*;

pub(super) fn update(app: &mut App, message: AgentsMessage) -> Task<Message> {
    match message {
        AgentsMessage::ProfileOpen(key) => {
            if !catalog_can_write(app) {
                return Task::none();
            }
            app.agents_settings.profile_modal =
                app.agents_settings.entries.iter().find(|entry| entry.key == key).cloned();
            if let Some(draft) = app.agents_settings.profile_modal.as_mut() {
                draft.description_editor = text_editor::Content::with_text(&draft.description);
                app.agents_settings.active_prompt_tab = AGENT_PROMPT_SYSTEM_TAB.to_string();
                app.agents_settings.workspace_identity_root_path = None;
                app.agents_settings.selected_agent = key;
                app.agents_settings.active_detail_tab =
                    crate::app::state::AGENT_DETAIL_BASIC_TAB.to_string();
                return Task::batch([
                    refresh_tools_task(),
                    refresh_skill_catalog(app),
                    super::super::mcp::update(
                        app,
                        SettingsMessage::Mcp(
                            crate::app::message::settings::McpMessage::InspectRefresh,
                        ),
                    ),
                ]);
            }
            Task::none()
        }
        AgentsMessage::ProfileCancel => {
            if !app.agents_settings.profile_saving {
                app.agents_settings.profile_modal = None;
            }
            Task::none()
        }
        AgentsMessage::ProfileSaved(result) => {
            if !app.agents_settings.profile_saving {
                return Task::none();
            }
            app.agents_settings.profile_saving = false;
            match result {
                Ok(()) => {
                    if let Some(draft) = app.agents_settings.profile_modal.take() {
                        if app.agents_settings.profile_deleting {
                            app.agents_settings.entries.retain(|entry| entry.key != draft.key);
                            app.agents_settings.selected_agent = MAIN_AGENT_KEY.to_string();
                        } else if let Some(entry) = find_entry_mut(app, &draft.key) {
                            *entry = draft;
                        } else {
                            app.agents_settings.entries.push(draft);
                        }
                    }
                    app.agents_settings.profile_deleting = false;
                    app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "智能体资料已保存",
                        vec![],
                    )
                }
                Err(error) => app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "保存智能体配置失败：%s",
                    vec![error.into()],
                ),
            }
        }
        AgentsMessage::ProfileSave | AgentsMessage::ProfileDelete => {
            if app.agents_settings.profile_saving || !app.agents_settings.config_ready {
                return Task::none();
            }
            let Some(draft) = app.agents_settings.profile_modal.as_ref() else {
                return Task::none();
            };
            let deleting = matches!(message, AgentsMessage::ProfileDelete);
            let fixed = builtin_agent_spec(&draft.key).is_some() || draft.key.starts_with("acp-");
            if deleting && fixed {
                return Task::none();
            }
            if !deleting && draft.label.trim().is_empty() {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "智能体名称不能为空",
                    vec![],
                );
            }
            if !deleting {
                let error =
                    crate::app::components::system_settings_agents::defaults::validation_error(
                        app, draft,
                    );
                if let Some(error) = error {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "保存智能体配置失败：%s",
                        vec![error.into()],
                    );
                }
            }
            let mut value = if deleting {
                serde_json::Value::Null
            } else {
                serde_json::json!({
                    "label": draft.label, "description": draft.description,
                    "recommended_prompts": draft.recommended_prompts, "runtime": draft.runtime,
                    "system_prompt": draft.system_prompt_editor.text(),
                    "defaults": crate::app::components::system_settings_agents::defaults::patch(&draft.defaults),
                })
            };
            // JSON Merge Patch must remove the ACP-only field when returning to native.
            // Custom runtime patches are validated after merge by the gateway.
            if !fixed
                && matches!(draft.runtime, vw_config_types::agent::AgentRuntime::Native)
                && !deleting
            {
                value["runtime"]["backend"] = serde_json::Value::Null;
            }
            let mut patch = if deleting {
                serde_json::json!({"agents": {draft.key.clone(): value}})
            } else {
                match build_agents_settings_patch(std::slice::from_ref(draft)) {
                    Ok(patch) => patch,
                    Err(error) => return catalog_error(app, error),
                }
            };
            if !deleting {
                patch["agents"][&draft.key]
                    .as_object_mut()
                    .unwrap()
                    .extend(value.as_object_mut().unwrap().clone());
                // A single-agent edit must not remove unrelated legacy entries.
                patch["agents"].as_object_mut().unwrap().remove("general");
            }
            app.agents_settings.profile_saving = true;
            app.agents_settings.profile_deleting = deleting;
            Task::perform(persist_agents_patch(patch), |result| {
                Message::Settings(SettingsMessage::Agents(AgentsMessage::ProfileSaved(result)))
            })
        }
        AgentsMessage::ProfileModelChanged(_)
        | AgentsMessage::ProfileDefaultsChanged(_)
        | AgentsMessage::ProfileNameChanged(_)
        | AgentsMessage::ProfileDescriptionAction(_)
        | AgentsMessage::ProfileRuntimeChanged(_)
        | AgentsMessage::ProfilePromptChanged(_, _)
        | AgentsMessage::ProfilePromptAdd
        | AgentsMessage::ProfilePromptRemove(_)
        | AgentsMessage::ProfilePromptMove(_, _) => {
            if app.agents_settings.profile_saving {
                return Task::none();
            }
            let Some(draft) = app.agents_settings.profile_modal.as_mut() else {
                return Task::none();
            };
            match message {
                AgentsMessage::ProfileDefaultsChanged(value) => draft.defaults = value,
                AgentsMessage::ProfileModelChanged(value) => {
                    match &value {
                        vw_config_types::agent::AgentDefault::Fixed(model) => {
                            draft.provider = model.provider.clone();
                            draft.model = model.model.clone();
                        }
                        vw_config_types::agent::AgentDefault::Inherit => {
                            draft.provider.clear();
                            draft.model.clear();
                            draft.force_model_selection = false;
                        }
                    }
                    draft.defaults.model = value;
                }
                AgentsMessage::ProfileNameChanged(value) => draft.label = value,
                AgentsMessage::ProfileDescriptionAction(action) => {
                    draft.description_editor.perform(action);
                    draft.description = draft.description_editor.text();
                }
                AgentsMessage::ProfileRuntimeChanged(value) => {
                    if builtin_agent_spec(&draft.key).is_none()
                        && !draft.key.starts_with("acp-")
                        && !app.agents_settings.entries.iter().any(|entry| entry.key == draft.key)
                    {
                        draft.runtime = if value == "native" {
                            vw_config_types::agent::AgentRuntime::Native
                        } else if app.acp_agents.contains(&value) {
                            vw_config_types::agent::AgentRuntime::Acp { backend: value }
                        } else {
                            return Task::none();
                        };
                    }
                }
                AgentsMessage::ProfilePromptAdd => draft.recommended_prompts.push(String::new()),
                AgentsMessage::ProfilePromptChanged(index, value) => {
                    if let Some(prompt) = draft.recommended_prompts.get_mut(index) {
                        *prompt = value;
                    }
                }
                AgentsMessage::ProfilePromptRemove(index) => {
                    if index < draft.recommended_prompts.len() {
                        draft.recommended_prompts.remove(index);
                    }
                }
                AgentsMessage::ProfilePromptMove(from, to) => {
                    if from < draft.recommended_prompts.len()
                        && to < draft.recommended_prompts.len()
                    {
                        draft.recommended_prompts.swap(from, to);
                    }
                }
                _ => unreachable!(),
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
