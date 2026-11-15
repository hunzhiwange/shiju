//! 智能体资料草稿消息与保存事务。
use super::*;

pub(super) fn update(app: &mut Update<'_>, message: AgentsMessage) -> Task<Message> {
    match message {
        AgentsMessage::ProfileOpen(key) => {
            if !catalog_can_write(app) || app.state.profile_entering_key {
                return Task::none();
            }
            app.state.profile_modal =
                app.state.entries.iter().find(|entry| entry.key == key).cloned();
            if let Some(draft) = app.state.profile_modal.as_mut() {
                draft.description_editor = text_editor::Content::with_text(&draft.description);
                app.state.active_prompt_tab = AGENT_PROMPT_SYSTEM_TAB.to_string();
                app.state.workspace_identity_root_path = None;
                app.state.selected_agent = key;
                app.state.active_detail_tab =
                    crate::state::AGENT_DETAIL_BASIC_TAB.to_string();
                return Task::batch([
                    refresh_tools_task(app.access.clone(), app.client.clone()),
                    refresh_skill_catalog(app),
                    app.effect(Effect::RefreshMcp),
                ]);
            }
            Task::none()
        }
        AgentsMessage::ProfileCancel => {
            if !app.state.profile_saving {
                if app.state.profile_entering_key {
                    app.state.profile_entering_key = false;
                    app.state.new_agent_key_input.clear();
                }
                app.state.profile_modal = None;
            }
            Task::none()
        }
        AgentsMessage::ProfileSaved(result) => {
            if !app.state.profile_saving {
                return Task::none();
            }
            app.state.profile_saving = false;
            match result {
                Ok(()) => {
                    if let Some(draft) = app.state.profile_modal.take() {
                        if app.state.profile_deleting {
                            app.state.entries.retain(|entry| entry.key != draft.key);
                            app.state.selected_agent = MAIN_AGENT_KEY.to_string();
                        } else if let Some(entry) = find_entry_mut(app, &draft.key) {
                            *entry = draft;
                        } else {
                            app.state.entries.push(draft);
                        }
                    }
                    app.state.profile_deleting = false;
                    app.show_localized_toast(
                        ToastKind::Success,
                        "智能体资料已保存",
                        vec![],
                    )
                }
                Err(error) => app.show_localized_toast(
                    ToastKind::Error,
                    "保存智能体配置失败：%s",
                    vec![error.into()],
                ),
            }
        }
        AgentsMessage::ProfileSave | AgentsMessage::ProfileDelete => {
            if app.state.profile_saving || !app.state.config_ready {
                return Task::none();
            }
            let Some(draft) = app.state.profile_modal.as_ref() else {
                return Task::none();
            };
            let deleting = matches!(message, AgentsMessage::ProfileDelete);
            let fixed = builtin_agent_spec(&draft.key).is_some() || draft.key.starts_with("acp-");
            if deleting && fixed {
                return Task::none();
            }
            if !deleting && draft.label.trim().is_empty() {
                return app.show_localized_toast(
                    ToastKind::Error,
                    "智能体名称不能为空",
                    vec![],
                );
            }
            if !deleting {
                let error = app.validation_error.clone();
                if let Some(error) = error {
                    return app.show_localized_toast(
                        ToastKind::Error,
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
                    "defaults": crate::defaults::patch(&draft.defaults),
                })
            };
            // JSON Merge Patch must remove the ACP-only field when returning to native.
            // Custom runtime patches are validated after merge by the gateway.
            if !fixed
                && matches!(draft.runtime, config_agent_profile::AgentRuntime::Native)
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
            app.state.profile_saving = true;
            app.state.profile_deleting = deleting;
            Task::perform(persist_agents_patch(app.access.clone(), app.client.clone(), patch), |result| {
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
            if app.state.profile_saving {
                return Task::none();
            }
            let Some(draft) = app.state.profile_modal.as_mut() else {
                return Task::none();
            };
            match message {
                AgentsMessage::ProfileDefaultsChanged(value) => draft.defaults = value,
                AgentsMessage::ProfileModelChanged(value) => {
                    match &value {
                        config_agent_profile::AgentDefault::Fixed(model) => {
                            draft.provider = model.provider.clone();
                            draft.model = model.model.clone();
                        }
                        config_agent_profile::AgentDefault::Inherit => {
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
                        && !app.state.entries.iter().any(|entry| entry.key == draft.key)
                    {
                        draft.runtime = if value == "native" {
                            config_agent_profile::AgentRuntime::Native
                        } else if app.acp_agents.contains(&value) {
                            config_agent_profile::AgentRuntime::Acp { backend: value }
                        } else {
                            return Task::none();
                        };
                        if matches!(draft.runtime, config_agent_profile::AgentRuntime::Acp { .. })
                            && app.state.active_detail_tab
                                == crate::state::AGENT_DETAIL_TOOLS_TAB
                        {
                            app.state.active_detail_tab =
                                crate::state::AGENT_DETAIL_BASIC_TAB.to_string();
                        }
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
