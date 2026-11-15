pub fn validate(
    draft: &crate::state::CronJobDraft,
    availability: crate::system_settings_cron::DeliveryAvailability,
) -> Result<(), String> {
    if !draft.delivery_enabled {
        return Ok(());
    }
    if draft.delivery_to.trim().is_empty() {
        return Err("请输入投递目标".into());
    }
    let channel = draft.delivery_channel.trim().to_ascii_lowercase();
    let configured = match channel.as_str() {
        "wecom" => {
            availability.bot
        }
        "wecom_webhook" => {
            if draft.delivery_to.trim() != "default" {
                return Err("群机器人只支持默认群引用".into());
            }
            availability.webhook
        }
        "telegram" => availability.telegram,
        "discord" => availability.discord,
        "slack" => availability.slack,
        "mattermost" => availability.mattermost,
        "qq" => availability.qq,
        #[cfg(not(target_arch = "wasm32"))]
        "email" => availability.email,
        _ => {
            return Err("不支持的定时任务投递通道；请选择受支持通道或关闭结果投递".into());
        }
    };
    if !configured {
        if channel == "wecom_webhook" {
            return Err("群通知未配置；请在定时任务通知配置中保存 Webhook".into());
        }
        return Err(format!("投递通道 {channel} 未配置；请先在通道设置中配置凭证"));
    }
    Ok(())
}
