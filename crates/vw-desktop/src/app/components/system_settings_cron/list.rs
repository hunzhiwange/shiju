//! Current jobs own their scroll viewport so batch actions remain visible.
use super::*;
use crate::app::components::system_settings_common::settings_toggle;
use crate::app::state::{CronFrequency, CronListFilter, CronScheduleDraft};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, column, responsive, scrollable};

fn card_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: iced::Border {
            color: theme.palette().text.scale_alpha(0.12),
            width: 1.0,
            radius: 16.0.into(),
        },
        ..Default::default()
    }
}

fn summary(job: &CronJobDto) -> String {
    let source =
        if job_is_agent(job) { job.prompt.as_deref().unwrap_or_default() } else { &job.command };
    let normalized = source.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = normalized.chars();
    let mut result: String = chars.by_ref().take(160).collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}

fn schedule_label(app: &App, job: &CronJobDto) -> String {
    let tr = I18n::new(app.desktop_locale);
    let timezone = job.timezone.as_deref().unwrap_or("UTC");
    match job.schedule_kind.as_str() {
        "at" => format!(
            "{} · {} · {}",
            tr.__("指定时间"),
            format_cron_datetime(job.at.as_deref().unwrap_or(&job.expression)),
            tr.__("本地时间")
        ),
        "every" => format!(
            "{} · {}",
            tr.__("固定间隔"),
            job.every_ms.map(|ms| format!("{ms} ms")).unwrap_or_else(|| job.expression.clone())
        ),
        "biweekly" => match CronScheduleDraft::from_job(job) {
            Ok(draft) => {
                format!("{} · {}", super::schedule::rule_summary(&draft.rule), draft.timezone)
            }
            Err(error) => format!("{} · {error}", tr.__("每双周")),
        },
        "cron" | "" => {
            let fields: Vec<_> = job.expression.split_whitespace().collect();
            let fields = match fields.as_slice() {
                ["0", rest @ ..] if rest.len() == 5 => rest,
                other => other,
            };
            let frequency = CronFrequency::from_expression(&fields.join(" "));
            if frequency == CronFrequency::Daily {
                format!("{} {:0>2}:{:0>2} · {timezone}", tr.__("每天"), fields[1], fields[0])
            } else if frequency == CronFrequency::Advanced {
                format!("{} · {timezone}", job.expression)
            } else {
                format!("{} · {} · {timezone}", tr.__(&frequency.to_string()), job.expression)
            }
        }
        _ => job.expression.clone(),
    }
}

pub(super) fn job_row<'a>(app: &'a App, job: &'a CronJobDto) -> Element<'a, Message> {
    let s = &app.cron_settings;
    let tr = I18n::new(app.desktop_locale);
    let selected = s.selected_job_ids.iter().any(|id| id == &job.id);
    let details_expanded = s.expanded_job_ids.iter().any(|id| id == &job.id);
    let id_for_select = job.id.clone();
    let id_for_toggle = job.id.clone();
    let id_for_edit = job.id.clone();
    let id_for_runs = job.id.clone();
    let id_for_delete = job.id.clone();
    let status_label = if job.enabled {
        crate::app::i18n::I18n::new(app.desktop_locale).__("启用")
    } else {
        crate::app::i18n::I18n::new(app.desktop_locale).__("禁用")
    };
    let is_agent = job_is_agent(job);
    let type_label = if is_agent { "Agent" } else { "Shell" };
    let last_status = job
        .last_status
        .as_deref()
        .unwrap_or(crate::app::i18n::I18n::new(app.desktop_locale).__("未运行"));
    let ended = job_has_ended(job, chrono::Utc::now());
    let next_run = if ended { "已结束".into() } else { format_cron_datetime(&job.next_run) };
    let last_run = crate::app::i18n::I18n::new(app.desktop_locale)
        .__(&format_optional_cron_datetime(job.last_run.as_deref()))
        .to_string();
    let delivery_label = if job.delivery_mode == "announce" {
        match job.delivery_channel.as_deref() {
            Some("wecom") => "企业微信智能机器人",
            Some("wecom_webhook") => "企业微信群机器人",
            _ => "投递",
        }
    } else {
        "不投递"
    };

    let mut header = row![].spacing(10).align_y(Alignment::Center);
    if s.batch_mode {
        header = header.push(
            checkbox(selected)
                .on_toggle(move |v| {
                    Message::Settings(message::SettingsMessage::CronJobSelectionToggled(
                        id_for_select.clone(),
                        v,
                    ))
                })
                .style(settings_checkbox_style),
        );
    }
    header = header
        .push(
            text(job_title(job))
                .size(16)
                .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() })
                .width(Length::Fill),
        )
        .push(settings_toggle(job.enabled).on_toggle(move |v| {
            Message::Settings(message::SettingsMessage::CronJobEnabledChanged(
                id_for_toggle.clone(),
                v,
            ))
        }));
    let next = if ended {
        tr.__("已结束").to_string()
    } else if job.enabled {
        format!(
            "{} · {} · {}",
            tr.__("下次运行"),
            format_cron_datetime(&job.next_run),
            tr.__("本地时间")
        )
    } else {
        tr.__("已暂停").to_string()
    };
    let frequency = schedule_label(app, job);
    let timing = responsive(move |size| {
        let frequency = text(frequency.clone())
            .size(12)
            .style(settings_muted_text_style)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph);
        let next = text(next.clone())
            .size(12)
            .style(settings_muted_text_style)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph);
        if size.width >= 620.0 {
            row![frequency.width(Length::Fill), next].spacing(12).into()
        } else {
            column![frequency, next].spacing(8).into()
        }
    })
    .height(Length::Shrink);
    let mut meta = column![
        row![
            text("ID").size(12).style(settings_muted_text_style),
            text(job.id.clone()).size(12),
        ]
        .spacing(10),
        row![
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("概况")).size(12).style(settings_muted_text_style),
            settings_value_badge(type_label),
            settings_value_badge(status_label),
            settings_value_badge(delivery_label),
            settings_value_badge(last_status),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        row![
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("表达式")).size(12).style(settings_muted_text_style),
            text(job.expression.clone()).size(12),
        ]
        .spacing(10),
        row![text(crate::app::i18n::I18n::new(app.desktop_locale).__("下次运行")).size(12).style(settings_muted_text_style), text(next_run).size(12),]
            .spacing(10),
        row![text(crate::app::i18n::I18n::new(app.desktop_locale).__("上次运行")).size(12).style(settings_muted_text_style), text(last_run).size(12),]
            .spacing(10),
        row![
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("项目")).size(12).style(settings_muted_text_style),
            text(job_project_label(app, job)).size(12).wrapping(iced::widget::text::Wrapping::WordOrGlyph).width(Length::Fill),
        ]
        .spacing(10),
        row![
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("执行选项")).size(12).style(settings_muted_text_style),
            text(crate::__!(crate::app::i18n::I18n::new(app.desktop_locale),
                "代理：%s / ACP：%s / 模型：%s / 回退：%u / 完全访问：%s / 任务池：%s / 唤醒：%s / 执行后删除：%s",
                job.agent.as_deref().unwrap_or(crate::app::i18n::I18n::new(app.desktop_locale).__("默认")),
                job.acp_agent
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(crate::app::i18n::I18n::new(app.desktop_locale).__("未使用")),
                job.model.as_deref().unwrap_or(crate::app::i18n::I18n::new(app.desktop_locale).__("默认")),
                job.fallbacks.len(),
                if job.full_access { crate::app::i18n::I18n::new(app.desktop_locale).__("是") } else { crate::app::i18n::I18n::new(app.desktop_locale).__("否") },
                if job.task_pool { crate::app::i18n::I18n::new(app.desktop_locale).__("是") } else { crate::app::i18n::I18n::new(app.desktop_locale).__("否") },
                if job.wake { crate::app::i18n::I18n::new(app.desktop_locale).__("是") } else { crate::app::i18n::I18n::new(app.desktop_locale).__("否") },
                if job.delete_after_run { crate::app::i18n::I18n::new(app.desktop_locale).__("是") } else { crate::app::i18n::I18n::new(app.desktop_locale).__("否") },
            ).expect("valid cron execution summary template"))
            .size(12),
        ]
        .spacing(10),
        text(if is_agent { job.prompt.clone().unwrap_or_default() } else { job.command.clone() })
            .size(12),
    ]
    .spacing(7);

    if let Some(last_output) =
        job.last_output.as_deref().map(str::trim).filter(|value| !value.is_empty())
    {
        meta = meta.push(
            row![
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("最近历史"))
                    .size(12)
                    .style(settings_muted_text_style),
                text(preview_lines(last_output, RUN_HISTORY_PREVIEW_LINES))
                    .size(12)
                    .wrapping(iced::widget::text::Wrapping::Word)
                    .width(Length::Fill),
            ]
            .spacing(10),
        );
    }

    let toggle_label = if job.enabled {
        crate::app::i18n::I18n::new(app.desktop_locale).__("禁用")
    } else {
        crate::app::i18n::I18n::new(app.desktop_locale).__("启用")
    };
    let actions = row![
        small_button(
            crate::app::i18n::I18n::new(app.desktop_locale).__("编辑"),
            Message::Settings(message::SettingsMessage::CronJobEditStarted(id_for_edit)),
            false
        ),
        small_button(
            crate::app::i18n::I18n::new(app.desktop_locale).__("历史"),
            Message::Settings(message::SettingsMessage::CronJobRunsOpen(id_for_runs)),
            false
        ),
        small_button(
            toggle_label,
            Message::Settings(message::SettingsMessage::CronJobEnabledChanged(
                job.id.clone(),
                !job.enabled
            )),
            false
        ),
        small_button(
            crate::app::i18n::I18n::new(app.desktop_locale).__("删除"),
            Message::Settings(message::SettingsMessage::CronJobDelete(id_for_delete)),
            true
        ),
    ]
    .spacing(8);

    let details = small_button(
        tr.__(if details_expanded { "收起" } else { "展开" }),
        Message::Settings(message::SettingsMessage::CronJobDetailsToggled(
            job.id.clone(),
            !details_expanded,
        )),
        false,
    );
    let excerpt = container(
        text(summary(job))
            .size(13)
            .style(settings_muted_text_style)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
    )
    .width(Length::Fill)
    .max_height(33.8)
    .clip(true);
    let mut content = column![header, excerpt, settings_divider(), timing, details]
        .spacing(12)
        .width(Length::Fill);
    if details_expanded {
        content = content.push(meta).push(actions);
    }
    container(content).padding(18).width(Length::Fill).style(card_style).into()
}

pub(super) fn jobs_tab(app: &App) -> Element<'_, Message> {
    let tr = I18n::new(app.desktop_locale);
    let s = &app.cron_settings;
    let mut groups = row![].spacing(8);
    for (filter, label) in [
        (CronListFilter::All, "全部"),
        (CronListFilter::Enabled, "已启用"),
        (CronListFilter::Paused, "已暂停"),
    ] {
        groups = groups.push(
            button(text(format!("{} ({})", tr.__(label), s.job_count(filter))).size(13))
                .padding([8, 14])
                .style(move |theme: &Theme, status| {
                    cron_segment_style(theme, status, filter == s.list_filter)
                })
                .on_press(Message::Settings(message::SettingsMessage::CronListFilterChanged(
                    filter,
                ))),
        );
    }
    let toolbar = responsive(move |size| {
        let heading = settings_group_heading_with_help(
            tr,
            &app.app_theme,
            tr.__("当前任务"),
            tr.__("默认展示已配置的定时任务，并支持单项或批量管理。"),
        );
        let actions = row![
            small_button(
                tr.__("刷新"),
                Message::Settings(message::SettingsMessage::CronJobsRefresh),
                false
            ),
            small_button(
                tr.__(if s.batch_mode { "取消批量管理" } else { "批量管理" }),
                Message::Settings(message::SettingsMessage::CronBatchModeToggled(!s.batch_mode)),
                false
            ),
            small_button(
                "从模板新建",
                Message::Settings(message::SettingsMessage::CronTemplatesToggled(true)),
                false
            ),
            primary_button(
                tr.__("新建定时任务"),
                Message::Settings(message::SettingsMessage::CronTabSelected(CronSettingsTab::Add))
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if size.width >= 650.0 {
            row![container(heading).width(Length::Fill), actions]
                .spacing(12)
                .align_y(Alignment::Center)
                .into()
        } else {
            column![heading, actions.wrap()].spacing(8).into()
        }
    })
    .height(Length::Shrink);
    let mut cards = column![].spacing(12).width(Length::Fill);
    if s.jobs_loading {
        cards = cards
            .push(text(tr.__("正在加载定时任务...")).size(13).style(settings_muted_text_style));
    } else if s.jobs.is_empty() {
        cards =
            cards.push(text(tr.__("当前没有定时任务。")).size(13).style(settings_muted_text_style));
    } else if s.visible_jobs().next().is_none() {
        cards =
            cards.push(text(tr.__("当前分组暂无任务")).size(13).style(settings_muted_text_style));
    } else {
        let jobs: Vec<_> = s.visible_jobs().collect();
        for pair in jobs.chunks(2) {
            let mut line = row![].spacing(12).width(Length::Fill);
            for job in pair {
                line = line.push(container(job_row(app, job)).width(Length::FillPortion(1)));
            }
            if pair.len() == 1 {
                line = line.push(Space::new().width(Length::FillPortion(1)));
            }
            cards = cards.push(line);
        }
    }
    let list = scrollable(container(cards).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill);
    let mut page =
        column![toolbar, groups.wrap(), list].spacing(14).width(Length::Fill).height(Length::Fill);
    if s.batch_mode {
        let count = s.selected_job_ids.len();
        let visible_count = s.visible_jobs().count();
        let all_selected =
            visible_count > 0 && s.visible_jobs().all(|job| s.selected_job_ids.contains(&job.id));
        let select_label =
            crate::__!(tr, "全选 %u 个任务", visible_count).expect("valid cron selection template");
        let selected_label = format!("{}: {count}", tr.__("已选"));
        let controls = responsive(move |size| {
            let selection = row![
                checkbox(all_selected)
                    .label(select_label.clone())
                    .on_toggle(|v| Message::Settings(
                        message::SettingsMessage::CronJobsSelectAllToggled(v)
                    ))
                    .style(settings_checkbox_style),
                text(selected_label.clone()).size(12).style(settings_muted_text_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            let mut actions = row![].spacing(8);
            for (template, action, danger) in [
                ("启用已选(%u)", message::SettingsMessage::CronSelectedJobsEnable, false),
                ("禁用已选(%u)", message::SettingsMessage::CronSelectedJobsDisable, false),
                ("删除已选(%u)", message::SettingsMessage::CronSelectedJobsDelete, true),
            ] {
                let label = crate::__!(tr, template, count).expect("valid cron batch template");
                let btn = button(text(label).size(12)).padding([8, 12]).style(if danger {
                    danger_action_btn_style
                } else {
                    rounded_action_btn_style
                });
                actions = actions
                    .push(btn.on_press_maybe((count > 0).then_some(Message::Settings(action))));
            }
            if size.width >= 650.0 {
                row![selection, Space::new().width(Length::Fill), actions]
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .into()
            } else {
                column![selection, actions.wrap()].spacing(10).into()
            }
        })
        .height(Length::Shrink);
        let footer = column![settings_divider(), controls].spacing(12);
        page = page.push(container(footer).padding([12, 7]).width(Length::Fill));
    }
    page.into()
}

pub(super) fn job_has_ended(job: &CronJobDto, now: chrono::DateTime<chrono::Utc>) -> bool {
    job.ends_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|end| end <= now)
}
