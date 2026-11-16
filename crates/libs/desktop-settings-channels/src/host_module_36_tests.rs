//! Channel state and QR updates use the real domain update entry.
use crate::messages::ChannelsMessage;
use crate::update::{Effect, Services};
use config_channel_services::LarkReceiveMode;
use gateway_channel_qr::{
    ChannelQrFetchResponseDto, ChannelQrStatusDto, ChannelQrStatusResponseDto,
};
use std::sync::Arc;
fn update(
    state: &mut crate::state::ChannelsSettingsState,
    message: ChannelsMessage,
) -> iced::Task<Effect> {
    let config = desktop_config::ConfigAccess::new(
        gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 9),
        None,
    );
    let services =
        Services { config, client: Err("fixture unavailable".into()), project_choices: Vec::new() };
    crate::update::updates::update(state, &services, message)
}
#[test]
fn channel_refresh_clears_error_and_tab_selection_updates_state() {
    let mut state = crate::state::ChannelsSettingsState::default();
    state.save_error = Some("old".to_string());

    let _ = update(&mut state, ChannelsMessage::Refresh);
    assert!(state.save_error.is_none());

    let _ = update(
        &mut state,
        ChannelsMessage::TabSelected(crate::state::ChannelsSettingsTab::Telegram),
    );
    assert_eq!(state.active_tab, crate::state::ChannelsSettingsTab::Telegram);
}

#[test]
fn channel_scalar_updates_store_values_and_clamp_timeout() {
    let mut state = crate::state::ChannelsSettingsState::default();

    let _ = update(&mut state, ChannelsMessage::CliToggled(true));
    let _ = update(&mut state, ChannelsMessage::ProjectDirChanged(" /tmp/work ".to_string()));
    let _ = update(&mut state, ChannelsMessage::MessageTimeoutSecsChanged(0));

    assert!(state.cli);
    assert_eq!(state.project_dir_input, " /tmp/work ");
    assert_eq!(state.message_timeout_secs, 1);
    assert!(state.save_error.is_none());
}

#[test]
fn channel_text_update_mirrors_input_and_applies_field() {
    let mut state = crate::state::ChannelsSettingsState::default();
    let _ = update(&mut state, ChannelsMessage::EnabledToggled("telegram".to_string(), true));

    let _ = update(
        &mut state,
        ChannelsMessage::TextChanged("telegram.bot_token".to_string(), "token".to_string()),
    );

    assert_eq!(state.text_inputs["telegram.bot_token"], "token");
    assert_eq!(state.telegram.as_ref().expect("telegram enabled").bot_token, "token");
}

#[test]
fn channel_text_editor_edit_updates_editor_mirror_and_channel_field() {
    let mut state = crate::state::ChannelsSettingsState::default();
    let key = "telegram.allowed_users".to_string();
    let _ = update(&mut state, ChannelsMessage::EnabledToggled("telegram".to_string(), true));
    state.save_error = Some("old".to_string());

    let _ = update(
        &mut state,
        ChannelsMessage::TextEditorAction(
            key.clone(),
            text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new("alice\nbob".to_string()))),
        ),
    );

    assert_eq!(state.text_editors[&key].text(), "alice\nbob");
    assert_eq!(state.text_inputs[&key], "alice\nbob");
    assert_eq!(
        state.telegram.as_ref().expect("telegram enabled").allowed_users,
        vec!["alice".to_string(), "bob".to_string()]
    );
    assert!(state.save_error.is_none());
}

// 提交与删除：覆盖保存、安装或移除操作前后的状态衔接。
// 这里关注校验、任务载荷和结果反馈，不绑定底层实现。
#[test]
fn channel_text_editor_non_edit_does_not_persist_or_change_text() {
    let mut state = crate::state::ChannelsSettingsState::default();
    let key = "telegram.allowed_users".to_string();
    let _ = update(&mut state, ChannelsMessage::EnabledToggled("telegram".to_string(), true));
    state.save_error = Some("keep".to_string());

    let _ = update(
        &mut state,
        ChannelsMessage::TextEditorAction(key.clone(), text_editor::Action::SelectAll),
    );

    assert_eq!(state.text_editors[&key].text(), "");
    assert_eq!(state.text_inputs[&key], "");
    assert_eq!(state.save_error.as_deref(), Some("keep"));
}

#[test]
fn channel_text_editor_missing_key_ignores_content_but_edit_still_persists() {
    let mut state = crate::state::ChannelsSettingsState::default();
    let key = "missing.editor".to_string();
    state.save_error = Some("old".to_string());

    let _ = update(
        &mut state,
        ChannelsMessage::TextEditorAction(
            key.clone(),
            text_editor::Action::Edit(text_editor::Edit::Insert('x')),
        ),
    );

    assert!(!state.text_editors.contains_key(&key));
    assert!(!state.text_inputs.contains_key(&key));
    assert!(state.save_error.is_none());
}

#[test]
fn channel_typed_updates_apply_bool_number_and_receive_mode_normalization() {
    let mut state = crate::state::ChannelsSettingsState::default();
    for channel in ["telegram", "lark"] {
        let _ = update(&mut state, ChannelsMessage::EnabledToggled(channel.to_string(), true));
    }

    let _ =
        update(&mut state, ChannelsMessage::BoolToggled("telegram.mention_only".to_string(), true));
    let _ = update(
        &mut state,
        ChannelsMessage::NumberChanged("telegram.draft_update_interval_ms".to_string(), 20),
    );
    let _ = update(
        &mut state,
        ChannelsMessage::ReceiveModeChanged("lark.receive_mode".to_string(), "webhook".to_string()),
    );

    let telegram = state.telegram.as_ref().expect("telegram enabled");
    assert!(telegram.mention_only);
    assert_eq!(telegram.draft_update_interval_ms, 100);
    assert_eq!(state.lark.as_ref().expect("lark enabled").receive_mode, LarkReceiveMode::Webhook);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn channel_cli_toggle_rejects_disabling_the_last_entry() {
    let mut state = crate::state::ChannelsSettingsState::default();

    let _ = update(&mut state, ChannelsMessage::CliToggled(false));

    assert!(!state.cli);
    assert!(state.save_error.as_deref().unwrap_or("").contains("至少保留一个"));
}

#[test]
fn channel_qr_flow_stops_on_close_and_fills_confirmed_credentials() {
    let mut state = crate::state::ChannelsSettingsState::default();
    let channel = "qq".to_string();

    let _ = update(&mut state, ChannelsMessage::QrRequested(channel.clone()));
    let generation = state.qr_flow.generation;
    let _ = update(
        &mut state,
        ChannelsMessage::QrFetched {
            channel: channel.clone(),
            generation,
            result: Ok(ChannelQrFetchResponseDto {
                channel: channel.clone(),
                qrcode_png_base64: "cG5n".to_string(),
                handle: "one-time-handle".to_string(),
                expires_at: "2026-10-20T12:00:00Z".to_string(),
            }),
        },
    );
    assert_eq!(state.qr_flow.channel.as_deref(), Some("qq"));
    assert!(state.qr_flow.qrcode_image.is_some());
    assert!(state.qq.is_none(), "fetching must not fill credentials");

    let _ = update(&mut state, ChannelsMessage::QrClosed);
    let stale = ChannelQrStatusResponseDto {
        status: ChannelQrStatusDto::Authorized,
        credentials: Some(serde_json::Map::from_iter([
            ("app_id".to_string(), serde_json::Value::String("stale-id".to_string())),
            ("app_secret".to_string(), serde_json::Value::String("stale-secret".to_string())),
        ])),
    };
    let _ = update(
        &mut state,
        ChannelsMessage::QrPolled { channel: channel.clone(), generation, result: Ok(stale) },
    );
    assert!(state.qq.is_none(), "closing must invalidate an in-flight poll");

    let next_generation = state.qr_flow.generation;
    state.qr_flow.channel = Some(channel.clone());
    state.qr_flow.handle = Some("next-handle".to_string());
    let confirmed = ChannelQrStatusResponseDto {
        status: ChannelQrStatusDto::Authorized,
        credentials: Some(serde_json::Map::from_iter([
            ("app_id".to_string(), serde_json::Value::String("confirmed-id".to_string())),
            ("app_secret".to_string(), serde_json::Value::String("confirmed-secret".to_string())),
        ])),
    };
    let _ = update(
        &mut state,
        ChannelsMessage::QrPolled { channel, generation: next_generation, result: Ok(confirmed) },
    );

    let qq = state.qq.as_ref().expect("confirmed credentials should fill form");
    assert_eq!(qq.app_id, "confirmed-id");
    assert_eq!(qq.app_secret, "confirmed-secret");
    assert_eq!(qq.allowed_users, ["*"]);
    assert_eq!(state.text_inputs["qq.allowed_users"], "*");
    assert!(!state.qr_flow.poll_in_flight);
}

#[test]
fn channel_qr_authorization_defaults_empty_allowed_users_and_preserves_restrictions() {
    for channel in ["dingtalk", "feishu", "qq"] {
        for existing_users in [None, Some(Vec::new()), Some(vec!["specific-user".to_string()])] {
            let mut state = crate::state::ChannelsSettingsState::default();
            let credentials = serde_json::Map::from_iter(
                ["client_id", "client_secret", "app_id", "app_secret"]
                    .map(|key| (key.to_string(), serde_json::Value::String(format!("qr-{key}")))),
            );
            if let Some(users) = existing_users.as_ref() {
                state.qr_flow.channel = Some(channel.into());
                let generation = state.qr_flow.generation;
                let _ = update(
                    &mut state,
                    ChannelsMessage::QrPolled {
                        channel: channel.into(),
                        generation,
                        result: Ok(ChannelQrStatusResponseDto {
                            status: ChannelQrStatusDto::Authorized,
                            credentials: Some(credentials.clone()),
                        }),
                    },
                );
                match channel {
                    "dingtalk" => state.dingtalk.as_mut().unwrap().allowed_users = users.clone(),
                    "feishu" => state.feishu.as_mut().unwrap().allowed_users = users.clone(),
                    "qq" => state.qq.as_mut().unwrap().allowed_users = users.clone(),
                    _ => unreachable!(),
                }
            }
            state.qr_flow.channel = Some(channel.to_string());
            let generation = state.qr_flow.generation;
            let _ = update(
                &mut state,
                ChannelsMessage::QrPolled {
                    channel: channel.to_string(),
                    generation,
                    result: Ok(ChannelQrStatusResponseDto {
                        status: ChannelQrStatusDto::Authorized,
                        credentials: Some(credentials),
                    }),
                },
            );

            let config = crate::update::persist::build_channels_config_from_settings(&state)
                .expect("authorized settings should be saveable");
            let allowed_users = match channel {
                "dingtalk" => config.dingtalk.unwrap().allowed_users,
                "feishu" => config.feishu.unwrap().allowed_users,
                "qq" => config.qq.unwrap().allowed_users,
                _ => unreachable!(),
            };
            let expected = existing_users
                .filter(|users| !users.is_empty())
                .unwrap_or_else(|| vec!["*".to_string()]);
            assert_eq!(allowed_users, expected, "{channel}");
            assert_eq!(state.text_inputs[&format!("{channel}.allowed_users")], expected.join(", "));
            assert!(state.qr_flow.credentials_ready);
        }
    }
}

#[test]
fn channel_qr_flow_ignores_failure_until_scan_is_observed() {
    let mut state = crate::state::ChannelsSettingsState::default();
    let channel = "wechat".to_string();
    state.qr_flow.channel = Some(channel.clone());
    state.qr_flow.handle = Some("pending-handle".to_string());
    let generation = state.qr_flow.generation;

    let failed =
        ChannelQrStatusResponseDto { status: ChannelQrStatusDto::Failed, credentials: None };
    let _ = update(
        &mut state,
        ChannelsMessage::QrPolled {
            channel: channel.clone(),
            generation,
            result: Ok(failed.clone()),
        },
    );
    assert_eq!(state.qr_flow.channel.as_deref(), Some("wechat"));
    assert_eq!(state.qr_flow.status, Some(ChannelQrStatusDto::Pending));

    let _ = update(
        &mut state,
        ChannelsMessage::QrPolled {
            channel: channel.clone(),
            generation,
            result: Ok(ChannelQrStatusResponseDto {
                status: ChannelQrStatusDto::Scanned,
                credentials: None,
            }),
        },
    );
    assert!(state.qr_flow.scan_observed);

    let _ =
        update(&mut state, ChannelsMessage::QrPolled { channel, generation, result: Ok(failed) });
    assert!(state.qr_flow.channel.is_none());
}

#[test]
fn project_choices_uses_managed_projects_names_and_order() {
    let projects = vec!["/data/two".into(), "/data/one".into()];
    let edits = vec!["项目二".into(), "项目一".into()];
    assert_eq!(
        crate::update::projects::project_choices(&projects, &edits, []),
        vec![("/data/two".into(), "项目二".into()), ("/data/one".into(), "项目一".into()),]
    );
}

#[test]
fn project_choices_uses_directory_name_when_managed_name_is_blank() {
    let projects = vec!["/data/one".into()];
    let edits = vec![" ".into()];
    assert_eq!(
        crate::update::projects::project_choices(&projects, &edits, []),
        vec![("/data/one".into(), "one".into())]
    );
}

#[test]
fn channel_execution_binding_is_removed_with_channel_instance() {
    let mut state = crate::state::ChannelsSettingsState::default();
    state.instance_execution.insert(
        "wechat:default".into(),
        config_channels::ChannelExecutionConfig { agent: Some("acp-codex".into()), model: None },
    );
    state.wechat = None;
    crate::update::projects::retain_configured_bindings(&mut state);
    assert!(!state.instance_execution.contains_key("wechat:default"));
}

use crate::state::ChannelExecutionChoices;
use config_agent_profile::AgentModelReference;
#[test]
fn channel_execution_loading_provider_changes_clear_models_and_keep_unified_agent() {
    let mut state = crate::state::ChannelsSettingsState::default();
    state.active_tab = crate::state::ChannelsSettingsTab::WeChat;
    state.wechat = Some(config_channel_services::WeChatConfig {
        bot_token: "fixture-token".into(),
        base_url: "https://example.test".into(),
    });
    let key = "wechat:default".to_owned();
    let endpoint = gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 9);
    let scope = serde_json::to_string(endpoint.tenant_scope()).unwrap();
    let services = Services {
        config: desktop_config::ConfigAccess::new(endpoint.clone(), None),
        client: vw_gateway_client::GatewayClient::new(endpoint),
        project_choices: Vec::new(),
    };
    let a = AgentModelReference { provider: "a".into(), model: "model-a".into() };
    let b = AgentModelReference { provider: "b".into(), model: "model-b".into() };
    let generation = state.begin_execution_choices(&scope, &key);
    state.finish_execution_choices(
        &scope,
        &key,
        generation,
        Ok(ChannelExecutionChoices {
            agents: vec![("acp-codex".into(), "Codex".into())],
            models: vec![a.clone(), b.clone()],
        }),
    );
    state.instance_execution.insert(
        key.clone(),
        ChannelExecutionConfig { agent: Some("acp-codex".into()), model: Some(a.clone()) },
    );
    let _ = crate::update::execution::select_provider(
        &mut state,
        &services,
        key.clone(),
        Some("a".into()),
    );
    assert_eq!(state.instance_execution[&key].model, Some(a.clone()));
    let _ = crate::update::execution::select_provider(
        &mut state,
        &services,
        key.clone(),
        Some("missing".into()),
    );
    assert_eq!(state.instance_execution[&key].model, Some(a));
    let _ = crate::update::execution::select_provider(
        &mut state,
        &services,
        key.clone(),
        Some("b".into()),
    );
    assert_eq!(state.execution_provider[&key], "b");
    assert!(state.instance_execution[&key].model.is_none());
    assert_eq!(state.instance_execution[&key].agent.as_deref(), Some("acp-codex"));
    let _ =
        crate::update::execution::select_model(&mut state, &services, key.clone(), Some(b.clone()));
    assert_eq!(state.instance_execution[&key].model, Some(b));
    let _ = crate::update::execution::select_provider(&mut state, &services, key.clone(), None);
    assert!(!state.execution_provider.contains_key(&key));
    assert!(state.instance_execution[&key].model.is_none());
}
