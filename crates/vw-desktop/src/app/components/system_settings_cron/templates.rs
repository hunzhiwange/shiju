//! 内置任务模板只预填草稿，由用户确认时间、模型与投递后保存。
use super::*;
use crate::app::state::{CronJobDraft, CronScheduleDraft, CronScheduleRule};
use iced::widget::column;

pub(crate) struct CronTemplate {
    pub title: &'static str,
    pub prompt: &'static str,
    expression: &'static str,
}

pub(crate) const TEMPLATES: &[CronTemplate] = &[
    CronTemplate {
        title: "每日 AI 新闻推送",
        prompt: "关注当天 AI 领域的重要动态，侧重 AI coding 与具身智能。搜索并核实来源，整理重点与原文链接。",
        expression: "0 9 * * *",
    },
    CronTemplate {
        title: "每日 5 个英语单词",
        prompt: "推荐 5 个高频实用英语单词，包含词义、音标、例句和记忆提示。",
        expression: "0 9 * * *",
    },
    CronTemplate {
        title: "每日儿童睡前故事",
        prompt: "生成一个 3–5 分钟可读的温和睡前故事，情节完整并附简短寓意。",
        expression: "0 21 * * *",
    },
    CronTemplate {
        title: "每周工作周报",
        prompt: "汇总当前项目本周的 PR 与 Issue 进展，输出关键变更、待处理事项与下周建议。无法访问仓库时明确说明。",
        expression: "0 17 * * 5",
    },
    CronTemplate {
        title: "经典电影推荐",
        prompt: "推荐一部高分经典电影，简要介绍剧情梗概、亮点与推荐理由，避免剧透。",
        expression: "0 20 * * 6",
    },
    CronTemplate {
        title: "历史上的今天",
        prompt: "从科技、电影、音乐等领域挑选一件今天发生过的有趣事件，核实日期并附来源。",
        expression: "0 9 * * *",
    },
    CronTemplate {
        title: "每日一个为什么",
        prompt: "抛出一个有趣问题，先提问再解答，语气轻松、通俗易懂。",
        expression: "0 9 * * *",
    },
    CronTemplate {
        title: "父母联系提醒",
        prompt: "提醒我给家人打电话或发消息，简单问候近况。",
        expression: "0 10 * * 0",
    },
    CronTemplate {
        title: "体检预约提醒",
        prompt: "提醒我确认体检时间、准备证件，并核对医院的体检注意事项。请在保存前调整为实际预约日期和时间。",
        expression: "0 7 * * *",
    },
    CronTemplate {
        title: "面试准备提醒",
        prompt: "提醒我复习大模型面试内容，并生成 3 个练习问题及参考答案。",
        expression: "0 */2 * * 1-5",
    },
    CronTemplate {
        title: "会议前准备",
        prompt: "提醒我整理会议议题、目标、待确认问题和关键结论。请在保存前调整为实际会议提醒时间。",
        expression: "0 9 * * 1-5",
    },
    CronTemplate {
        title: "可爱萌宠手机壁纸",
        prompt: "随机从水彩、卡通、像素、油画、剪纸、黏土、写实七种风格中选择一种，生成一张 9:16 竖版萌宠手机壁纸；如果没有图像生成工具，请提供可直接使用的绘图提示词。",
        expression: "0 9 * * *",
    },
];

impl CronTemplate {
    pub(crate) fn apply(&self, draft: &mut CronJobDraft) {
        draft.name = self.title.into();
        draft.prompt = self.prompt.into();
        draft.prompt_editor = text_editor::Content::with_text(self.prompt);
        draft.schedule = self.expression.into();
        draft.frequency = crate::app::state::CronFrequency::from_expression(self.expression);
        if matches!(self.title, "体检预约提醒" | "会议前准备") {
            let date = chrono::Local::now().date_naive().succ_opt().expect("next calendar day");
            let time = chrono::NaiveTime::from_hms_opt(9, 0, 0).expect("valid reminder time");
            draft.schedule_kind = crate::app::state::CronAddScheduleKind::At;
            draft.schedule.clear();
            draft.delete_after_run = true;
            draft.schedule_draft = Some(CronScheduleDraft {
                rule: CronScheduleRule::At { date, time },
                ..Default::default()
            });
            return;
        }
        draft.schedule_draft = Some(CronScheduleDraft {
            rule: CronScheduleRule::Advanced(self.expression.into()),
            ..Default::default()
        });
    }
}

pub(super) fn overlay<'a>(_app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let close = Message::Settings(message::SettingsMessage::CronTemplatesToggled(false));
    let header =
        settings_modal_header(text("从模板添加").size(20), settings_close_button(close.clone()));
    let mut cards = column![].spacing(12);
    for (pair_index, pair) in TEMPLATES.chunks(2).enumerate() {
        let mut line = row![].spacing(12).width(Length::Fill);
        for (index, template) in pair.iter().enumerate() {
            line = line.push(
                button(
                    column![
                        text(template.title).size(15),
                        text(template.prompt).size(12).style(settings_muted_text_style)
                    ]
                    .spacing(8),
                )
                .padding(16)
                .width(Length::FillPortion(1))
                .style(rounded_action_btn_style)
                .on_press(Message::Settings(
                    message::SettingsMessage::CronTemplateSelected(pair_index * 2 + index),
                )),
            );
        }
        cards = cards.push(line);
    }
    let body =
        column![text("选择模板后可修改内容和时间，确认后保存。").size(13), cards].spacing(12);
    let card =
        settings_modal_layout(header, body, None).style(cron_modal_style).width(860).height(620);
    settings_modal_overlay(Some(dialog), close, container(card).padding([10, 12]))
}
