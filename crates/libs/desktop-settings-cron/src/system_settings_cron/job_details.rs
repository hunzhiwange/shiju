//! Read-only view of a job and the effects of deleting it.
use super::*;
use crate::state::CronDeleteTarget;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use iced::widget::column;
use message::{ListMessage as Action, SettingsMessage as S};

pub(super) fn overlays<'a, M: Clone + 'static>(
    app: &ViewData<'a>,
    base: Element<'a, M>,
    map: fn(Message) -> M,
) -> Element<'a, M> {
    let state = app.cron_settings;
    let tr = I18n::new(app.desktop_locale);
    if let Some(target) = &state.delete_target {
        let close = Message::Settings(S::CronList(Action::CancelDelete));
        let (title, selection) = match target {
            CronDeleteTarget::Job(id) => (
                tr.__("确认删除定时任务？"),
                state.jobs.iter().find(|job| job.id == *id).map(|job| job_name(tr, job).to_owned()),
            ),
            CronDeleteTarget::Batch(ids) => (
                tr.__("确认删除选中的定时任务？"),
                desktop_i18n::__!(tr, "已选中 %u 项", ids.len()).ok(),
            ),
        };
        let header = settings_modal_header(
            text(title).size(18).font(list::bold()),
            settings_close_button(close.clone()),
        );
        let mut content = column![header].spacing(16).width(Length::Fill);
        if let Some(selection) = selection {
            content = content.push(SingleLineLabel::new(selection, 14.0));
        }
        let actions = row![
            iced::widget::Space::new().width(Length::Fill),
            secondary_button(tr.__("取消"), close.clone()),
            button(text(tr.__("删除")).size(13))
                .padding([8, 16])
                .style(danger_action_btn_style)
                .on_press(Message::Settings(S::CronList(Action::ConfirmDelete))),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        content = content
            .push(
                text(tr.__("删除后，所选任务将停止按计划执行，其运行记录也会一并删除。"))
                    .size(14)
                    .style(settings_muted_text_style),
            )
            .push(actions);
        let card = settings_modal_card(content).style(cron_modal_style).width(470);
        return desktop_ui_controls::system_settings_common::settings_modal_overlay_with_escape(
            base,
            map(close),
            Element::<Message>::from(container(card).padding([10, 12])).map(map),
        );
    }
    let Some(id) = &state.open_job_id else { return base };
    let Some(job) = state.jobs.iter().find(|job| job.id == *id) else { return base };
    let close = Message::Settings(S::CronList(Action::CloseTask));
    let delete = (!state.deleting_jobs).then(|| Message::Settings(S::CronJobDelete(id.clone())));
    let title = SingleLineLabel::new(job_name(tr, job).to_owned(), 19.0).font(list::bold());
    let header = settings_modal_header(title, settings_close_button(close.clone()));
    let controls = row![
        button(text(tr.__("删除")).size(13))
            .padding(SETTINGS_CONTROL_PADDING)
            .style(danger_action_btn_style)
            .on_press_maybe(delete),
        iced::widget::Space::new().width(Length::Fill),
        secondary_button(
            tr.__("查看运行记录"),
            Message::Settings(S::CronList(Action::OpenHistory(id.clone())))
        ),
        primary_button(tr.__("编辑"), Message::Settings(S::CronJobEditStarted(id.clone()))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(header, job_details(app, job), Some(controls.into()))
        .style(cron_modal_style)
        .width(660)
        .height(540);
    desktop_ui_controls::system_settings_common::settings_modal_overlay_with_escape(
        base,
        map(close),
        Element::<Message>::from(container(card).padding([10, 12])).map(map),
    )
}

fn job_name(tr: I18n, job: &CronJobDto) -> &str {
    let title = job_title(job);
    if title == "未命名任务" { tr.__(title) } else { title }
}

fn job_details<'a>(app: &ViewData<'a>, job: &'a CronJobDto) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let is_agent = job_is_agent(job);
    let type_label = if is_agent { "Agent" } else { "Shell" };
    let last_status = job
        .last_status
        .as_deref()
        .map(|status| match status {
            "running" => i18n.__("运行中"),
            "ok" => i18n.__("成功"),
            "error" => i18n.__("失败"),
            other => other,
        })
        .unwrap_or(i18n.__("未运行"));
    let next_run = if list::job_has_ended(job, chrono::Utc::now()) {
        i18n.__("已结束").into()
    } else {
        format_cron_datetime(&job.next_run, app.system_timezone)
    };
    let last_run = i18n
        .__(&format_optional_cron_datetime(job.last_run.as_deref(), app.system_timezone))
        .to_owned();
    let delivery_label = if job.delivery_mode == "announce" {
        match job.delivery_channel.as_deref() {
            Some("wecom") => "企业微信智能机器人",
            Some("wecom_webhook") => "企业微信群机器人",
            Some("wecom_cron") => "定时任务企业微信通知",
            _ => "投递",
        }
    } else {
        "不投递"
    };
    let status_label = if job.enabled { "启用" } else { "已暂停" };
    let mut meta = column![
        row![
            text("ID").size(12).style(settings_muted_text_style),
            text(job.id.clone()).size(12),
        ]
        .spacing(10),
        row![
            text(i18n.__("概况")).size(12).style(settings_muted_text_style),
            settings_value_badge(type_label),
            settings_value_badge(i18n.__(status_label)),
            settings_value_badge(i18n.__(delivery_label)),
            settings_value_badge(last_status),
        ]
        .spacing(10)
        .align_y(Alignment::Center).wrap(),
        row![
            text(i18n.__("表达式")).size(12).style(settings_muted_text_style),
            text(job.expression.clone()).size(12),
        ]
        .spacing(10),
        row![text(i18n.__("下次运行")).size(12).style(settings_muted_text_style), text(next_run).size(12),]
            .spacing(10),
        row![text(i18n.__("上次运行")).size(12).style(settings_muted_text_style), text(last_run).size(12),]
            .spacing(10),
        row![
            text(i18n.__("项目")).size(12).style(settings_muted_text_style),
            text(job_project_label(app, job)).size(12).wrapping(iced::widget::text::Wrapping::WordOrGlyph).width(Length::Fill),
        ]
        .spacing(10),
        row![
            text(i18n.__("执行选项")).size(12).style(settings_muted_text_style),
            text(desktop_i18n::__!(i18n,
                "代理：%s / ACP：%s / 模型：%s / 回退：%u / 完全访问：%s / 任务池：%s / 唤醒：%s / 执行后删除：%s",
                job.agent.as_deref().unwrap_or(i18n.__("默认")),
                job.acp_agent
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(i18n.__("未使用")),
                job.model.as_deref().unwrap_or(i18n.__("默认")),
                job.fallbacks.len(),
                if job.full_access { i18n.__("是") } else { i18n.__("否") },
                if job.task_pool { i18n.__("是") } else { i18n.__("否") },
                if job.wake { i18n.__("是") } else { i18n.__("否") },
                if job.delete_after_run { i18n.__("是") } else { i18n.__("否") },
            ).expect("valid cron execution summary template"))
            .size(12),
        ]
        .spacing(10),
        settings_divider(),
        text(if is_agent { job.prompt.clone().unwrap_or_default() } else { job.command.clone() })
            .size(13).wrapping(iced::widget::text::Wrapping::WordOrGlyph),
    ]
    .spacing(7);

    if let Some(last_output) =
        job.last_output.as_deref().map(str::trim).filter(|value| !value.is_empty())
    {
        meta = meta.push(
            row![
                text(i18n.__("最近历史")).size(12).style(settings_muted_text_style),
                text(preview_lines(last_output, RUN_HISTORY_PREVIEW_LINES))
                    .size(12)
                    .wrapping(iced::widget::text::Wrapping::Word)
                    .width(Length::Fill),
            ]
            .spacing(10),
        );
    }

    meta.into()
}
