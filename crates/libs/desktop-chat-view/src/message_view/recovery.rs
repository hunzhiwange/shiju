pub fn model_recovery_text(event: &gateway_chat_stream::GatewayModelRecoveryEvent) -> String {
    use gateway_chat_stream::GatewayModelRecoveryPhase;
    match event.phase {
        GatewayModelRecoveryPhase::Waiting => format!(
            "正在重试 {}/{}，等待 {}ms，原因：{}",
            event.attempt, event.max_attempts, event.wait_ms, event.reason
        ),
        GatewayModelRecoveryPhase::Stopped => {
            let attempts = if event.attempt <= 1 {
                "未重试".to_string()
            } else {
                format!("已尝试 {} 次（重试 {} 次）", event.attempt, event.attempt - 1)
            };
            format!("已停止，{attempts}，原因：{}。{}", event.reason, event.next_action)
        }
        GatewayModelRecoveryPhase::Recovered => "模型调用已恢复".to_string(),
        GatewayModelRecoveryPhase::Cancelled => "已取消重试".to_string(),
    }
}

pub fn localized_model_recovery_text(
    i18n: desktop_i18n::I18n,
    event: &gateway_chat_stream::GatewayModelRecoveryEvent,
) -> String {
    use gateway_chat_stream::GatewayModelRecoveryPhase;
    match event.phase {
        GatewayModelRecoveryPhase::Waiting => i18n
            .format(
                "正在重试 %u/%u，等待 %ums，原因：%s",
                &[
                    event.attempt.into(),
                    event.max_attempts.into(),
                    event.wait_ms.into(),
                    event.reason.clone().into(),
                ],
            )
            .expect("valid recovery template"),
        GatewayModelRecoveryPhase::Stopped => {
            let attempts = if event.attempt <= 1 {
                i18n.__("未重试").to_string()
            } else {
                i18n.format(
                    "已尝试 %u 次（重试 %u 次）",
                    &[event.attempt.into(), (event.attempt - 1).into()],
                )
                .expect("valid attempt template")
            };
            i18n.format(
                "已停止，%s，原因：%s。%s",
                &[attempts.into(), event.reason.clone().into(), event.next_action.clone().into()],
            )
            .expect("valid stopped template")
        }
        GatewayModelRecoveryPhase::Recovered => i18n.__("模型调用已恢复").to_string(),
        GatewayModelRecoveryPhase::Cancelled => i18n.__("已取消重试").to_string(),
    }
}

pub fn recovery_notice_text(
    i18n: desktop_i18n::I18n,
    active_request_id: Option<u64>,
    recovery: Option<(u64, &gateway_chat_stream::GatewayModelRecoveryEvent)>,
) -> Option<String> {
    let (id, event) = recovery?;
    if active_request_id? != id
        || event.phase != gateway_chat_stream::GatewayModelRecoveryPhase::Waiting
    {
        return None;
    }
    Some(localized_model_recovery_text(i18n, event))
}
pub fn recovery_notice_view<'a, M: 'a>(label: Option<String>) -> iced::Element<'a, M> {
    use iced::widget::{container, text};
    use iced::{Length, Theme};
    let Some(label) = label else { return iced::widget::Space::new().height(0).into() };
    container(text(label).size(super::styles::MESSAGE_META_TEXT_SIZE).style(|theme: &Theme| {
        iced::widget::text::Style {
            color: Some(super::styles::compaction_status_text_color(theme)),
        }
    }))
    .padding([8, 14])
    .width(Length::Fill)
    .style(super::styles::compaction_status_card_style)
    .into()
}
