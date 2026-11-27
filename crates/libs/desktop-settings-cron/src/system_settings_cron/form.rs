//! 新增和编辑共用的任务表单，保存边界位于固定弹窗页脚。
use super::*;
use crate::message::SettingsMessage as S;
use crate::state::{CronEditorTab, CronJobDraft};
use desktop_ui_controls::system_settings_common::settings_responsive_row;
use iced::widget::column;

pub fn view<'a>(app: &ViewData<'a>, editing: bool) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let draft = if editing { &app.cron_settings.edit_draft } else { &app.cron_settings.add_draft };
    let form: Element<'_, Message> = match draft.editor_tab {
        CronEditorTab::Basic => column![
            text_field(i18n.__("输入任务名称"), &draft.name, |name| Message::Settings(
                S::CronJobEditNameChanged(name)
            )),
            composer::view(app, draft, editing),
            schedule::frequency_form(app, draft, editing),
        ]
        .spacing(12)
        .width(Length::Fill)
        .into(),
        CronEditorTab::Advanced => advanced_fields(app, draft),
        CronEditorTab::Delivery => super::delivery::view(app, draft),
        CronEditorTab::Preview => schedule::preview(i18n, draft, editing),
    };

    let submitting = draft.submitting;
    form.map(move |message| {
        if submitting {
            Message::None
        } else if editing {
            message
        } else {
            for_add(message)
        }
    })
}

fn advanced_fields<'a>(app: &ViewData<'_>, draft: &'a CronJobDraft) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let mut fields = column![
        field_row_with_help(
            app,
            "任务类型",
            "Shell 执行命令，Agent 执行提示词任务。",
            move || row![
                segment_button(
                    "Agent",
                    draft.job_type == CronAddJobType::Agent,
                    Message::Settings(S::CronJobEditJobTypeChanged(CronAddJobType::Agent))
                ),
                segment_button(
                    "Shell",
                    draft.job_type == CronAddJobType::Shell,
                    Message::Settings(S::CronJobEditJobTypeChanged(CronAddJobType::Shell))
                ),
            ]
            .spacing(8)
            .into(),
        ),
    ];
    if draft.job_type == CronAddJobType::Agent {
        fields = fields
            .push(settings_divider())
            .push(
                container(settings_responsive_row(
                    i18n.__("任务池"),
                    i18n.__("开启后触发时投递到项目任务池，由任务看板接管执行。"),
                    move || {
                        checkbox(draft.task_pool)
                            .label(i18n.__("投递到任务池"))
                            .on_toggle(|v| Message::Settings(S::CronJobEditTaskPoolToggled(v)))
                            .style(settings_checkbox_style)
                    },
                ))
                .padding([10, 0])
                .width(Length::Fill),
            )
            .push(settings_divider())
            .push(field_row_with_help(
                app,
                "唤醒",
                "保留唤醒标记，供桌面侧识别需要关注的任务。",
                move || {
                    checkbox(draft.wake)
                        .label(i18n.__("任务运行时标记为需要关注"))
                        .on_toggle(|v| Message::Settings(S::CronJobEditWakeToggled(v)))
                        .style(settings_checkbox_style)
                        .into()
                },
            ));
    }
    fields = fields.push(settings_divider()).push(field_row_with_help(
        app,
        "执行后删除",
        "适合指定时间的一次性任务。",
        move || {
            checkbox(draft.delete_after_run)
                .label(i18n.__("运行成功后删除任务"))
                .on_toggle(|v| Message::Settings(S::CronJobEditDeleteAfterRunToggled(v)))
                .style(settings_checkbox_style)
                .into()
        },
    ));
    settings_group_card(fields).into()
}

fn for_add(message: Message) -> Message {
    use message::SettingsMessage as S;
    match message {
        Message::Settings(S::CronJobEditAgentChanged(v)) => {
            Message::Settings(S::CronAddAgentChanged(v))
        }
        Message::Settings(S::CronJobEditCommandEditorAction(v)) => {
            Message::Settings(S::CronAddCommandEditorAction(v))
        }
        Message::Settings(S::CronJobEditDeleteAfterRunToggled(v)) => {
            Message::Settings(S::CronAddDeleteAfterRunToggled(v))
        }
        Message::Settings(S::CronJobEditDeliveryBestEffortToggled(v)) => {
            Message::Settings(S::CronAddDeliveryBestEffortToggled(v))
        }
        Message::Settings(S::CronJobEditDeliveryWebhookChanged(v)) => {
            Message::Settings(S::CronAddDeliveryWebhookChanged(v))
        }
        Message::Settings(S::CronJobEditDeliveryEnabledToggled(v)) => {
            Message::Settings(S::CronAddDeliveryEnabledToggled(v))
        }
        Message::Settings(S::CronJobEditDeliveryWeComInstanceSelected(id)) => {
            Message::Settings(S::CronAddDeliveryWeComInstanceSelected(id))
        }
        Message::Settings(S::CronJobEditDeliveryWeComSelected(v)) => {
            Message::Settings(S::CronAddDeliveryWeComSelected(v))
        }
        Message::Settings(S::CronJobEditFullAccessToggled(v)) => {
            Message::Settings(S::CronAddFullAccessToggled(v))
        }
        Message::Settings(S::CronJobEditJobTypeChanged(v)) => {
            Message::Settings(S::CronAddJobTypeChanged(v))
        }
        Message::Settings(S::CronJobEditNameChanged(v)) => {
            Message::Settings(S::CronAddNameChanged(v))
        }
        Message::Settings(S::CronJobEditProjectPathChanged(v)) => {
            Message::Settings(S::CronAddProjectPathChanged(v))
        }
        Message::Settings(S::CronJobEditPromptEditorAction(v)) => {
            Message::Settings(S::CronAddPromptEditorAction(v))
        }
        Message::Settings(S::CronJobEditTaskPoolToggled(v)) => {
            Message::Settings(S::CronAddTaskPoolToggled(v))
        }
        Message::Settings(S::CronJobEditWakeToggled(v)) => {
            Message::Settings(S::CronAddWakeToggled(v))
        }
        other => other,
    }
}
