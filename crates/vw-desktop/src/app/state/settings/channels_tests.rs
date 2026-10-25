//! 渠道设置测试，覆盖多实例字段分类、文本编辑器刷新和项目目录规范化。
//! 重建输入状态时应保留静态编辑器并清除已删除实例留下的陈旧草稿。

use super::*;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("channels_tests"));
}

#[allow(dead_code)]
fn values(prefix: &str) -> Vec<String> {
    vec![format!("{prefix}-1"), format!("{prefix}-2")]
}

#[allow(dead_code)]
fn group(prefix: &str) -> vw_config_types::channels::GroupReplyConfig {
    vw_config_types::channels::GroupReplyConfig { mode: None, allowed_sender_ids: values(prefix) }
}

#[test]
fn default_channels_state_matches_ui_defaults() {
    let state = ChannelsSettingsState::default();

    assert_eq!(state.project_dir(), None);
    assert!(state.cli);
    assert_eq!(state.message_timeout_secs, 300);
    assert_eq!(state.active_tab, ChannelsSettingsTab::Global);
    assert!(state.save_error.is_none());
    assert!(state.text_inputs.is_empty());
    assert!(state.text_editors.is_empty());
}

#[test]
fn project_dir_trims_blank_input_and_preserves_non_blank_path() {
    let mut state = ChannelsSettingsState {
        project_dir_input: "   ".to_string(),
        ..ChannelsSettingsState::default()
    };

    assert_eq!(state.project_dir(), None);

    state.project_dir_input = "  /tmp/vibe-window  ".to_string();
    assert_eq!(state.project_dir(), Some(PathBuf::from("/tmp/vibe-window")));
}

#[test]
fn refresh_text_inputs_inserts_empty_group_reply_when_config_omits_group_reply() {
    let mut state = ChannelsSettingsState {
        telegram: Some(vw_config_types::channels::TelegramConfig {
            bot_token: "token".to_string(),
            allowed_users: Vec::new(),
            stream_mode: vw_config_types::channels::StreamMode::Off,
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        }),
        ..ChannelsSettingsState::default()
    };

    state.refresh_text_inputs();

    assert_eq!(state.text_inputs["telegram.allowed_users"], "");
    assert_eq!(state.text_inputs["telegram.group_reply.allowed_sender_ids"], "");
    assert_eq!(state.text_editors["telegram.group_reply.allowed_sender_ids"].text(), "");
}

#[test]
fn refresh_text_inputs_inserts_empty_group_reply_for_other_optional_group_configs() {
    let mut state = ChannelsSettingsState {
        discord: Some(vw_config_types::channels::DiscordConfig {
            bot_token: "token".to_string(),
            guild_id: None,
            allowed_users: Vec::new(),
            listen_to_bots: false,
            mention_only: false,
            group_reply: None,
        }),
        slack: Some(vw_config_types::channels::SlackConfig {
            bot_token: "token".to_string(),
            app_token: None,
            channel_id: None,
            allowed_users: Vec::new(),
            group_reply: None,
        }),
        mattermost: Some(vw_config_types::channels::MattermostConfig {
            url: "https://mattermost.example".to_string(),
            bot_token: "token".to_string(),
            channel_id: None,
            allowed_users: Vec::new(),
            thread_replies: None,
            mention_only: None,
            group_reply: None,
        }),
        lark: Some(vw_config_types::channels::LarkConfig {
            app_id: "app".to_string(),
            app_secret: "secret".to_string(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: Vec::new(),
            mention_only: false,
            group_reply: None,
            use_feishu: false,
            receive_mode: vw_config_types::channels::LarkReceiveMode::Websocket,
            port: None,
            draft_update_interval_ms: 3000,
            max_draft_edits: 20,
        }),
        feishu: Some(vw_config_types::channels::FeishuConfig {
            app_id: "app".to_string(),
            app_secret: "secret".to_string(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: Vec::new(),
            group_reply: None,
            receive_mode: vw_config_types::channels::LarkReceiveMode::Websocket,
            port: None,
            draft_update_interval_ms: 3000,
            max_draft_edits: 20,
        }),
        ..ChannelsSettingsState::default()
    };
    state.feishu_instances.push(vw_config_types::channels::FeishuInstanceConfig {
        id: "cn".to_string(),
        config: state.feishu.clone().expect("feishu fixture"),
    });

    state.refresh_text_inputs();

    for key in [
        "discord.group_reply.allowed_sender_ids",
        "slack.group_reply.allowed_sender_ids",
        "mattermost.group_reply.allowed_sender_ids",
        "lark.group_reply.allowed_sender_ids",
        "feishu.group_reply.allowed_sender_ids",
        "feishu_instances.0.group_reply.allowed_sender_ids",
    ] {
        assert_eq!(state.text_inputs[key], "", "unexpected input for {key}");
        assert_eq!(state.text_editors[key].text(), "", "unexpected editor for {key}");
    }
}

fn project_settings() -> ChannelsSettingsState {
    ChannelsSettingsState::default()
}

#[test]
fn channel_default_project_selection_isolated_by_instance() {
    let mut state = project_settings();
    state.begin_project_choices("tenant", "feishu:team");
    state.finish_project_choices(
        "tenant",
        "feishu:team",
        1,
        Ok(vec![("p1".into(), "中文项目".into())]),
    );
    state.select_default_project("tenant", "feishu:team", Some("p1")).unwrap();
    state.begin_project_choices("tenant", "qq:default");
    state.finish_project_choices(
        "tenant",
        "qq:default",
        2,
        Ok(vec![("p2".into(), "另一项目".into())]),
    );
    state.select_default_project("tenant", "qq:default", Some("p2")).unwrap();
    assert_eq!(state.instance_default_projects["feishu:team"], "p1");
    assert_eq!(state.instance_default_projects["qq:default"], "p2");
}

#[test]
fn channel_default_project_unknown_selection_preserves_binding() {
    let mut state = project_settings();
    state.instance_default_projects.insert("qq:default".into(), "previous".into());
    state.begin_project_choices("tenant", "qq:default");
    state.finish_project_choices("tenant", "qq:default", 1, Ok(vec![("p1".into(), "项目".into())]));
    assert!(state.select_default_project("tenant", "qq:default", Some("missing")).is_err());
    assert_eq!(state.instance_default_projects["qq:default"], "previous");
}

#[test]
fn channel_default_project_stale_or_cross_tenant_result_ignored() {
    let mut state = project_settings();
    state.begin_project_choices("tenant", "qq:default");
    for (tenant, key, generation) in
        [("other", "qq:default", 1), ("tenant", "feishu:team", 1), ("tenant", "qq:default", 0)]
    {
        assert!(!state.finish_project_choices(
            tenant,
            key,
            generation,
            Ok(vec![("bad".into(), "错误".into())])
        ));
    }
    assert!(state.select_default_project("other", "qq:default", None).is_err());
    assert!(state.project_choices.is_empty());
}

#[test]
fn channel_default_project_load_failure_keeps_deleted_binding_and_unbinds_explicitly() {
    let mut state = project_settings();
    state.instance_default_projects.insert("qq:default".into(), "deleted".into());
    state.begin_project_choices("tenant", "qq:default");
    state.finish_project_choices("tenant", "qq:default", 1, Err("加载失败".into()));
    assert_eq!(state.instance_default_projects["qq:default"], "deleted");
    assert!(state.project_choices_error.is_some());
    state.select_default_project("tenant", "qq:default", None).unwrap();
    assert!(!state.instance_default_projects.contains_key("qq:default"));
}
