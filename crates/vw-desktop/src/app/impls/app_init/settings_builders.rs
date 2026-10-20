//! 组织桌面应用初始化阶段的 settings_builders.rs 逻辑。
//! 本模块把启动输入、配置加载和初始状态装配拆开，便于定位启动失败路径。

/// 模块内可见函数，执行 build_channels_settings 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn build_channels_settings(
    channels_cfg: &vw_config_types::channels::ChannelsConfig,
) -> super::state::ChannelsSettingsState {
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
        wecom: channels_cfg.wecom.clone(),
        wechat: channels_cfg.wechat.clone(),
        nostr: channels_cfg.nostr.clone(),
        clawdtalk: channels_cfg.clawdtalk.clone(),
        message_timeout_secs: channels_cfg.message_timeout_secs.min(u32::MAX as u64) as u32,
        active_tab: super::state::ChannelsSettingsTab::Global,
        save_error: None,
        qr_flow: Default::default(),
    };
    channels_settings.refresh_text_inputs();
    channels_settings
}
