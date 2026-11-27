use super::*;
use crate::message::SettingsMessage as S;
use crate::state::CronJobDraft;
use desktop_ui_controls::system_settings_common::{settings_text_input, settings_toggle};
use iced::widget::column;

pub(super) fn view<'a>(app: &ViewData<'_>, draft: &'a CronJobDraft) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let mut fields = column![field_row_with_help(
        app,
        "开启结果投递",
        "运行结束后通过所选通道发送任务结果。",
        move || {
            settings_toggle(draft.delivery_enabled)
                .label(i18n.__("启用"))
                .on_toggle(|enabled| {
                    Message::Settings(S::CronJobEditDeliveryEnabledToggled(enabled))
                })
                .into()
        },
    )];
    if draft.delivery_enabled {
        let wecom = draft.delivery_via_wecom;
        fields = fields.push(settings_divider()).push(field_row_with_help(
            app,
            "投递方式",
            "选择企微通知实例，或填写当前任务的群 Webhook。",
            move || {
                row![
                    segment_button(
                        i18n.__("定时任务企业微信通知"),
                        wecom,
                        Message::Settings(S::CronJobEditDeliveryWeComSelected(true))
                    ),
                    segment_button(
                        i18n.__("群 Webhook"),
                        !wecom,
                        Message::Settings(S::CronJobEditDeliveryWeComSelected(false))
                    ),
                ]
                .spacing(8)
                .wrap()
                .into()
            },
        ));
        if wecom {
            let choices = app.wecom_notifications.clone();
            let selected = choices
                .iter()
                .find(|choice| Some(&choice.id) == draft.delivery_wecom_instance.as_ref())
                .cloned();
            fields = fields.push(settings_divider()).push(field_row_with_help(
                app,
                "接收通知的实例",
                "选择一个实例，任务结果会发给它绑定的会话。",
                move || {
                    if choices.is_empty() {
                        return text(i18n.__("请先在通道设置中新增通知实例并绑定接收会话。")).into();
                    }
                    editor_select(
                        choices.iter().map(|choice| choice.id.clone()),
                        draft.delivery_wecom_instance.clone(),
                        |id| Message::Settings(S::CronJobEditDeliveryWeComInstanceSelected(id)),
                    )
                    .format_labels(|id| {
                        choices.iter().find(|choice| &choice.id == id)
                            .map(|choice| choice.label.clone())
                            .unwrap_or_else(|| i18n.__("当前通知实例不可用").to_owned())
                    })
                    .placeholder(i18n.__("请选择一个企微通知实例"))
                    .width(Length::Fill)
                    .into()
                },
            ));
            if let Some(selected) = selected {
                fields = fields
                    .push(settings_divider())
                    .push(binding_controls(i18n, selected, draft.submitting));
            }
        } else {
            let placeholder = if draft.delivery_webhook_configured {
                i18n.__("已配置，留空保留；输入新值替换")
            } else {
                i18n.__("粘贴企业微信群机器人的 Webhook")
            };
            fields = fields.push(settings_divider()).push(field_row_with_help(
                app,
                "本任务的群 Webhook",
                "粘贴企业微信群机器人的 Webhook",
                move || {
                    settings_text_input(placeholder, draft.delivery_webhook.as_str())
                        .secure(true)
                        .on_input(|value| {
                            Message::Settings(S::CronJobEditDeliveryWebhookChanged(value.into()))
                        })
                        .width(Length::Fill)
                        .into()
                },
            ));
        }
        fields = fields.push(settings_divider()).push(field_row_with_help(
            app,
            "尽力投递",
            "开启后失败记入历史但不改变任务状态；关闭后投递失败会将任务标记为失败。",
            move || {
                checkbox(draft.delivery_best_effort)
                    .label(i18n.__("投递失败不影响任务状态"))
                    .on_toggle(|value| {
                        Message::Settings(S::CronJobEditDeliveryBestEffortToggled(value))
                    })
                    .style(settings_checkbox_style)
                    .into()
            },
        ));
    }
    let mut content = column![settings_group_card(fields)].spacing(12).width(Length::Fill);
    if let Err(error) = crate::delivery::validate(draft) {
        content =
            content.push(settings_error_banner(desktop_i18n::settings::message(i18n, &error)));
    }
    content.into()
}

fn binding_controls<'a>(
    i18n: I18n,
    selected: WeComNotification,
    submitting: bool,
) -> Element<'a, Message> {
    let status = match (selected.bound, selected.connected) {
        (_, Some(false)) => "定时任务机器人尚未连接",
        (Some(true), _) => "接收会话已绑定",
        (Some(false), _) => "接收会话尚未绑定",
        _ => "请在通道中连接定时任务企微机器人并绑定接收会话",
    };
    let request = |start| Message::WeComBindingRequested { instance: selected.id.clone(), start };
    let available = !selected.loading && !submitting;
    let bind_label = if selected.bound == Some(true) {
        "重新绑定接收会话"
    } else {
        "绑定接收会话"
    };
    let mut actions = row![
        button(text(i18n.__(bind_label)).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding(SETTINGS_CONTROL_PADDING)
            .style(rounded_action_btn_style)
            .on_press_maybe((available && selected.connected != Some(false)).then(|| request(true))),
        button(text(i18n.__("检查连接状态")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding(SETTINGS_CONTROL_PADDING)
            .style(rounded_action_btn_style)
            .on_press_maybe(available.then(|| request(false))),
    ].spacing(8);
    let mut content = column![text(i18n.__(status))].spacing(8);
    if let Some(code) = selected.pairing_code {
        content = content.push(text(i18n.__("请在 5 分钟内私聊专用机器人，发送下方绑定指令，然后检查连接状态。")));
        content = content.push(text(code.clone()));
        actions = actions.push(secondary_button(i18n.__("复制绑定指令"), Message::CopyCode(code)));
    }
    content.push(actions.wrap()).padding([14, 0]).into()
}
