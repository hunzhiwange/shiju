pub fn validate(draft: &crate::state::CronJobDraft) -> Result<(), String> {
    if draft.delivery_enabled && draft.delivery_via_wecom && draft.delivery_wecom_instance.is_none() {
        return Err("请选择一个企微通知实例".into());
    }
    if draft.delivery_enabled
        && !draft.delivery_via_wecom
        && !draft.delivery_webhook_configured
        && draft.delivery_webhook.as_str().trim().is_empty()
    {
        return Err("请在当前任务中填写群机器人 Webhook".into());
    }
    Ok(())
}
