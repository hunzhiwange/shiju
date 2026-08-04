//! 组织桌面应用初始化阶段的 settings_builders.rs 逻辑。
//! 本模块把启动输入、配置加载和初始状态装配拆开，便于定位启动失败路径。

/// 模块内可见函数，执行 build_channels_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_channels_settings(
    channels_cfg: &vw_config_types::channels::ChannelsConfig,
) -> super::state::ChannelsSettingsState {
    let channels_telegram_enabled = channels_cfg.telegram.is_some();
    let channels_discord_enabled = channels_cfg.discord.is_some();
    let channels_slack_enabled = channels_cfg.slack.is_some();
    let channels_mattermost_enabled = channels_cfg.mattermost.is_some();
    let channels_webhook_enabled = channels_cfg.webhook.is_some();
    let channels_imessage_enabled = channels_cfg.imessage.is_some();
    let channels_matrix_enabled = channels_cfg.matrix_enabled;
    let channels_signal_enabled = channels_cfg.signal.is_some();
    let channels_whatsapp_enabled = channels_cfg.whatsapp.is_some();
    let channels_linq_enabled = channels_cfg.linq.is_some();
    let channels_wati_enabled = channels_cfg.wati.is_some();
    let channels_nextcloud_talk_enabled = channels_cfg.nextcloud_talk.is_some();
    #[cfg(not(target_arch = "wasm32"))]
    let channels_email_enabled = channels_cfg.email.is_some();
    let channels_irc_enabled = channels_cfg.irc.is_some();
    let channels_lark_enabled = channels_cfg.lark.is_some();
    let channels_feishu_enabled =
        channels_cfg.feishu.is_some() || !channels_cfg.feishu_instances.is_empty();
    let channels_dingtalk_enabled =
        channels_cfg.dingtalk.is_some() || !channels_cfg.dingtalk_instances.is_empty();
    let channels_qq_enabled = channels_cfg.qq.is_some();
    let channels_nostr_enabled = channels_cfg.nostr.is_some();
    let channels_clawdtalk_enabled = channels_cfg.clawdtalk.is_some();

    let mut channels_settings = super::state::ChannelsSettingsState {
        project_dir_input: channels_cfg
            .project_dir
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        text_inputs: std::collections::HashMap::new(),
        text_editors: std::collections::HashMap::new(),
        cli: channels_cfg.cli,
        telegram: channels_cfg.telegram.clone(),
        discord: channels_cfg.discord.clone(),
        slack: channels_cfg.slack.clone(),
        mattermost: channels_cfg.mattermost.clone(),
        webhook: channels_cfg.webhook.clone(),
        imessage: channels_cfg.imessage.clone(),
        matrix: channels_cfg.matrix.clone(),
        matrix_enabled: channels_cfg.matrix_enabled,
        matrix_instances: channels_cfg.matrix_instances.clone(),
        signal: channels_cfg.signal.clone(),
        whatsapp: channels_cfg.whatsapp.clone(),
        linq: channels_cfg.linq.clone(),
        wati: channels_cfg.wati.clone(),
        nextcloud_talk: channels_cfg.nextcloud_talk.clone(),
        #[cfg(not(target_arch = "wasm32"))]
        email: channels_cfg.email.clone(),
        irc: channels_cfg.irc.clone(),
        lark: channels_cfg.lark.clone(),
        feishu: channels_cfg.feishu.clone(),
        feishu_instances: channels_cfg.feishu_instances.clone(),
        dingtalk: channels_cfg.dingtalk.clone(),
        dingtalk_instances: channels_cfg.dingtalk_instances.clone(),
        qq: channels_cfg.qq.clone(),
        nostr: channels_cfg.nostr.clone(),
        clawdtalk: channels_cfg.clawdtalk.clone(),
        message_timeout_secs: channels_cfg.message_timeout_secs.min(u32::MAX as u64) as u32,
        expanded_panels: {
            let mut panels = vec!["feishu".to_string()];
            if channels_telegram_enabled {
                panels.push("telegram".to_string());
            }
            if channels_discord_enabled {
                panels.push("discord".to_string());
            }
            if channels_slack_enabled {
                panels.push("slack".to_string());
            }
            if channels_mattermost_enabled {
                panels.push("mattermost".to_string());
            }
            if channels_webhook_enabled {
                panels.push("webhook".to_string());
            }
            if channels_imessage_enabled {
                panels.push("imessage".to_string());
            }
            if channels_matrix_enabled {
                panels.push("matrix".to_string());
            }
            if channels_signal_enabled {
                panels.push("signal".to_string());
            }
            if channels_whatsapp_enabled {
                panels.push("whatsapp".to_string());
            }
            if channels_linq_enabled {
                panels.push("linq".to_string());
            }
            if channels_wati_enabled {
                panels.push("wati".to_string());
            }
            if channels_nextcloud_talk_enabled {
                panels.push("nextcloud_talk".to_string());
            }
            #[cfg(not(target_arch = "wasm32"))]
            if channels_email_enabled {
                panels.push("email".to_string());
            }
            if channels_irc_enabled {
                panels.push("irc".to_string());
            }
            if channels_lark_enabled {
                panels.push("lark".to_string());
            }
            if channels_feishu_enabled {
                panels.push("feishu".to_string());
            }
            if channels_dingtalk_enabled {
                panels.push("dingtalk".to_string());
            }
            if channels_qq_enabled {
                panels.push("qq".to_string());
            }
            if channels_nostr_enabled {
                panels.push("nostr".to_string());
            }
            if channels_clawdtalk_enabled {
                panels.push("clawdtalk".to_string());
            }
            panels.into_iter().collect()
        },
        save_error: None,
    };
    channels_settings.refresh_text_inputs();
    channels_settings
}

/// 模块内可见函数，执行 build_staff_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_staff_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> super::state::StaffSettingsState {
    super::state::StaffSettingsState::from_config(full_agent_cfg.staff.clone())
}

#[cfg(test)]
#[path = "settings_builders_tests.rs"]
mod settings_builders_tests;
