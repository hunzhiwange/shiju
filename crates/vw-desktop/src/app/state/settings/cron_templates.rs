//! 常用任务目录。除新闻外提示词为产品拟定文案；Cron 预设均使用 UTC。
use super::{CronAddScheduleKind, CronFrequency, CronJobDraft};
use chrono::{DateTime, LocalResult, TimeZone};
use iced::widget::text_editor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronTemplateId {
    AiNews,
    English,
    Story,
    WeeklyReport,
    Movie,
    History,
    Why,
    Parents,
    Health,
    Interview,
    Meeting,
    Wallpaper,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum CronTemplateSchedule {
    /// UTC Cron 表达式。
    Cron(&'static str),
    TomorrowAt {
        hour: u32,
    },
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct CronTemplate {
    pub(crate) id: CronTemplateId,
    pub(crate) title: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) prompt: &'static str,
    pub(crate) schedule: CronTemplateSchedule,
}
pub(crate) const CRON_TEMPLATES: &[CronTemplate] = &[
    CronTemplate {
        id: CronTemplateId::AiNews,
        title: "每日 AI 新闻推送",
        summary: "关注 AI coding 与具身智能，每天精选重要动态",
        prompt: "关注当天 AI 领域的重要动态，侧重 AI coding 与具身智能方向。筛选 3–5 条有价值的信息，简要说明事件内容及值得关注的原因。结合 memory.md 里之前推送过的内容，尽量不要重复。",
        schedule: CronTemplateSchedule::Cron("0 9 * * *"),
    },
    CronTemplate {
        id: CronTemplateId::English,
        title: "每日 5 个英语单词",
        summary: "每天学习五个实用英语单词",
        prompt: "每天推荐 5 个高频实用英语单词，包含词义、音标、例句和中文翻译，并给出简短记忆提示；可用历史记录存在时尽量避免重复。",
        schedule: CronTemplateSchedule::Cron("0 9 * * *"),
    },
    CronTemplate {
        id: CronTemplateId::Story,
        title: "每日儿童睡前故事",
        summary: "温和有寓意的儿童睡前故事",
        prompt: "生成一篇 3–5 分钟可读的温和儿童睡前故事，情节完整、语言易懂、不含惊吓内容，并附一句简短寓意。",
        schedule: CronTemplateSchedule::Cron("0 21 * * *"),
    },
    CronTemplate {
        id: CronTemplateId::WeeklyReport,
        title: "每周工作周报",
        summary: "汇总项目进展、风险与下周计划",
        prompt: "汇总当前项目本周可访问的 PR 与 Issue 进展，列出关键变更、待办、风险和下周计划。缺少仓库或访问权限时明确说明，不编造进展。",
        schedule: CronTemplateSchedule::Cron("0 17 * * FRI"),
    },
    CronTemplate {
        id: CronTemplateId::Movie,
        title: "经典电影推荐",
        summary: "推荐一部避免剧透的经典电影",
        prompt: "推荐一部高分经典电影，简要介绍剧情梗概、主要亮点和推荐理由，避免关键剧透；可用历史记录存在时尽量避免重复。",
        schedule: CronTemplateSchedule::Cron("0 20 * * *"),
    },
    CronTemplate {
        id: CronTemplateId::History,
        title: "历史上的今天",
        summary: "了解历史上今天发生的有趣事件",
        prompt: "从科技、电影、音乐等领域挑选一件在历史上今天发生的有趣事件，说明年份、背景和意义；优先给出可核验的来源，无法核实时明确说明。",
        schedule: CronTemplateSchedule::Cron("0 9 * * *"),
    },
    CronTemplate {
        id: CronTemplateId::Why,
        title: "每日一个为什么",
        summary: "每天一个有趣问题与通俗解答",
        prompt: "每天抛出一个有趣问题，先提问再解答，语气轻松、通俗易懂，给出贴近日常的例子；可用历史记录存在时尽量避免重复。",
        schedule: CronTemplateSchedule::Cron("0 9 * * *"),
    },
    CronTemplate {
        id: CronTemplateId::Parents,
        title: "父母联系提醒",
        summary: "提醒问候家人并提供聊天话题",
        prompt: "提醒我给家人打电话或发消息，简单问候近况，并给出一个自然的聊天话题。只生成提醒，不自动联系他人。",
        schedule: CronTemplateSchedule::Cron("0 10 * * SUN"),
    },
    CronTemplate {
        id: CronTemplateId::Health,
        title: "体检预约提醒",
        summary: "核对体检预约与机构准备要求",
        prompt: "提醒我确认体检预约时间和地点、准备证件，并核对体检机构提供的准备要求；不自行推断禁食或停药要求。",
        schedule: CronTemplateSchedule::TomorrowAt { hour: 7 },
    },
    CronTemplate {
        id: CronTemplateId::Interview,
        title: "面试准备提醒",
        summary: "复习大模型面试并练习三个问题",
        prompt: "提醒我复习大模型面试内容，并生成 3 个练习问题，覆盖基础原理、工程实践和场景分析，随后给出参考解答要点。",
        schedule: CronTemplateSchedule::Cron("0 9-17/2 * * MON-FRI"),
    },
    CronTemplate {
        id: CronTemplateId::Meeting,
        title: "会议前准备",
        summary: "整理会议议题与准备清单",
        prompt: "提醒我整理会议议题、目标、待确认问题和关键材料，输出一份简短准备清单；具体会议时间由我确认和修改。",
        schedule: CronTemplateSchedule::TomorrowAt { hour: 9 },
    },
    CronTemplate {
        id: CronTemplateId::Wallpaper,
        title: "可爱萌宠手机壁纸",
        summary: "七种风格随机选择的竖版萌宠壁纸",
        prompt: "随机从水彩、卡通、像素、黏土、毛绒、剪纸、写实 7 种风格中选择一种，生成一张 9:16 竖版可爱萌宠手机壁纸。若当前执行代理没有图片生成能力，明确说明能力缺失并提供可用的绘图提示词，不声称已生成图片。",
        schedule: CronTemplateSchedule::Cron("0 9 * * *"),
    },
];
impl CronTemplate {
    /// 注入带时区的当前时间，拒绝不存在或歧义的本地时间。
    pub(crate) fn draft<Tz: TimeZone>(&self, now: DateTime<Tz>) -> Result<CronJobDraft, String> {
        let mut draft = CronJobDraft {
            name: self.title.into(),
            prompt: self.prompt.into(),
            prompt_editor: text_editor::Content::with_text(self.prompt),
            ..Default::default()
        };
        match self.schedule {
            CronTemplateSchedule::Cron(expression) => {
                draft.schedule = expression.into();
                draft.frequency = CronFrequency::from_expression(expression);
            }
            CronTemplateSchedule::TomorrowAt { hour } => {
                let date = now.date_naive().succ_opt().ok_or("次日日期超出范围")?;
                let local = date.and_hms_opt(hour, 0, 0).ok_or("单次时间无效")?;
                let at = match now.timezone().from_local_datetime(&local) {
                    LocalResult::Single(at) => at,
                    LocalResult::Ambiguous(_, _) => {
                        return Err("次日本地时间存在歧义，请手动选择时间".into());
                    }
                    LocalResult::None => return Err("次日本地时间不存在，请手动选择时间".into()),
                };
                if at <= now {
                    return Err("单次时间必须晚于当前时间".into());
                }
                draft.schedule_kind = CronAddScheduleKind::At;
                draft.at = at.to_rfc3339();
            }
        }
        Ok(draft)
    }
}
#[cfg(test)]
#[path = "cron_templates_tests.rs"]
mod tests;
