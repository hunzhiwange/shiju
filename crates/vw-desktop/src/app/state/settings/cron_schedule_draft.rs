//! 强类型时间草稿；时区换算及下一次执行由网关负责。
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime, Timelike};
use vw_gateway_client::{CronJobDto, CronPreviewRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalUnit {
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CronScheduleRule {
    At {
        date: NaiveDate,
        time: NaiveTime,
    },
    /// 保留既有绝对时间，避免在桌面重复实现 DST 转换。
    AbsoluteAt(String),
    Every {
        amount: u64,
        unit: IntervalUnit,
    },
    Daily {
        time: NaiveTime,
    },
    Weekly {
        weekdays: Vec<u32>,
        time: NaiveTime,
    },
    Biweekly {
        anchor: NaiveDate,
        weekdays: Vec<u32>,
        time: NaiveTime,
    },
    Monthly {
        days: Vec<u32>,
        time: NaiveTime,
    },
    /// 2 月 29 日只在闰年触发；每月 29—31 日在短月跳过。
    Yearly {
        month: u32,
        day: u32,
        time: NaiveTime,
    },
    Advanced(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronScheduleDraft {
    pub rule: CronScheduleRule,
    /// 显式展示的 IANA 名称，由网关校验并解析。
    pub timezone: String,
    /// None 保留原值，Some(None) 清除，Some(Some(date)) 设置包含当日的日期边界。
    pub start_date: Option<Option<NaiveDate>>,
    pub end_date: Option<Option<NaiveDate>>,
    /// 编辑回读的原始 UTC 边界，日期补丁未改动时用于准确预览。
    pub starts_at: Option<DateTime<FixedOffset>>,
    pub ends_at: Option<DateTime<FixedOffset>>,
}

impl Default for CronScheduleDraft {
    fn default() -> Self {
        Self {
            rule: CronScheduleRule::Daily { time: NaiveTime::from_hms_opt(9, 0, 0).unwrap() },
            timezone: "UTC".into(),
            start_date: None,
            end_date: None,
            starts_at: None,
            ends_at: None,
        }
    }
}

fn sorted(values: &[u32], max: u32) -> Result<Vec<u32>, String> {
    if values.is_empty() || values.iter().any(|v| !(1..=max).contains(v)) {
        return Err("请选择合法且非空的日期集合".into());
    }
    let mut values = values.to_vec();
    values.sort_unstable();
    values.dedup();
    Ok(values)
}
const WEEKDAYS: [&str; 7] = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];

impl CronScheduleDraft {
    pub fn request(&self) -> Result<CronPreviewRequest, String> {
        if self.timezone.trim().is_empty() {
            return Err("时区不能为空".into());
        }
        if let (Some(Some(start)), Some(Some(end))) = (self.start_date, self.end_date)
            && start > end
        {
            return Err("有效期结束日期不能早于开始日期".into());
        }
        let mut r = CronPreviewRequest {
            starts_at: self
                .start_date
                .is_none()
                .then(|| self.starts_at.map(|v| Some(v.to_rfc3339())))
                .flatten(),
            ends_at: self
                .end_date
                .is_none()
                .then(|| self.ends_at.map(|v| Some(v.to_rfc3339())))
                .flatten(),
            timezone: Some(self.timezone.clone()),
            start_date: self.start_date.map(|v| v.map(|d| d.to_string())),
            end_date: self.end_date.map(|v| v.map(|d| d.to_string())),
            ..Default::default()
        };
        let cron = |time: &NaiveTime, tail: String| -> Result<String, String> {
            if time.second() != 0 || time.nanosecond() != 0 {
                return Err("日历规则仅支持时分".into());
            }
            Ok(format!("{} {} {tail}", time.minute(), time.hour()))
        };
        r.schedule_kind = "cron".into();
        match &self.rule {
            CronScheduleRule::At { date, time } => {
                if time.second() != 0 || time.nanosecond() != 0 {
                    return Err("单次时间仅支持时分".into());
                }
                r.schedule_kind = "at".into();
                r.local_at = Some(format!("{date}T{}", time.format("%H:%M")));
            }
            CronScheduleRule::AbsoluteAt(at) => {
                r.schedule_kind = "at".into();
                r.at = Some(at.clone());
            }
            CronScheduleRule::Every { amount, unit } => {
                let multiplier = match unit {
                    IntervalUnit::Milliseconds => 1,
                    IntervalUnit::Seconds => 1000,
                    IntervalUnit::Minutes => 60000,
                    IntervalUnit::Hours => 3600000,
                    IntervalUnit::Days => 86400000,
                };
                let ms = amount
                    .checked_mul(multiplier)
                    .filter(|v| *v > 0)
                    .ok_or("固定间隔为零或溢出")?;
                r.schedule_kind = "every".into();
                r.every_ms = Some(ms);
            }
            CronScheduleRule::Daily { time } => r.schedule = Some(cron(time, "* * *".into())?),
            CronScheduleRule::Weekly { weekdays, time } => {
                let days = sorted(weekdays, 7)?
                    .iter()
                    .map(|v| WEEKDAYS[*v as usize - 1])
                    .collect::<Vec<_>>()
                    .join(",");
                r.schedule = Some(cron(time, format!("* * {days}"))?);
            }
            CronScheduleRule::Biweekly { anchor, weekdays, time } => {
                if time.second() != 0 || time.nanosecond() != 0 {
                    return Err("双周规则仅支持时分".into());
                }
                r.schedule_kind = "biweekly".into();
                r.anchor_date = Some(anchor.to_string());
                r.weekdays = Some(sorted(weekdays, 7)?);
                r.hour = Some(time.hour());
                r.minute = Some(time.minute());
            }
            CronScheduleRule::Monthly { days, time } => {
                let days =
                    sorted(days, 31)?.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
                r.schedule = Some(cron(time, format!("{days} * *"))?);
            }
            CronScheduleRule::Yearly { month, day, time } => {
                if NaiveDate::from_ymd_opt(2000, *month, *day).is_none() {
                    return Err("每年月日组合不存在".into());
                }
                r.schedule = Some(cron(time, format!("{day} {month} *"))?);
            }
            CronScheduleRule::Advanced(expression) => {
                if expression.trim().is_empty() {
                    return Err("Cron 表达式不能为空".into());
                }
                r.schedule = Some(expression.clone());
            }
        }
        if matches!(self.rule, CronScheduleRule::At { .. }) {
            // 切换到本地单次规则时清除旧的周期有效期，避免隐藏窗口阻断新时间。
            r.start_date = Some(None);
            r.end_date = Some(None);
            r.starts_at = None;
            r.ends_at = None;
        }
        if (r.start_date.is_some() || r.end_date.is_some())
            && (r.starts_at.is_some() || r.ends_at.is_some())
        {
            return Err("修改已有有效期时，请同时设置或清除起止日期".into());
        }
        Ok(r)
    }

    pub fn from_request(r: &CronPreviewRequest) -> Result<Self, String> {
        let parse_date = |value: &str| value.parse::<NaiveDate>().map_err(|e| e.to_string());
        let rule = match r.schedule_kind.as_str() {
            "at" => {
                if let Some(local) = &r.local_at {
                    let (date, time) = local.split_once('T').ok_or("单次日期时间无效")?;
                    CronScheduleRule::At {
                        date: parse_date(date)?,
                        time: NaiveTime::parse_from_str(time, "%H:%M")
                            .map_err(|e| e.to_string())?,
                    }
                } else {
                    CronScheduleRule::AbsoluteAt(r.at.clone().ok_or("缺少单次时间")?)
                }
            }
            "every" => CronScheduleRule::Every {
                amount: r.every_ms.ok_or("缺少间隔")?,
                unit: IntervalUnit::Milliseconds,
            },
            "biweekly" => CronScheduleRule::Biweekly {
                anchor: parse_date(r.anchor_date.as_deref().ok_or("缺少双周锚点")?)?,
                weekdays: r.weekdays.clone().ok_or("缺少星期")?,
                time: NaiveTime::from_hms_opt(
                    r.hour.ok_or("缺少小时")?,
                    r.minute.ok_or("缺少分钟")?,
                    0,
                )
                .ok_or("时间无效")?,
            },
            "cron" => parse_expression(r.schedule.as_deref().unwrap_or_default()),
            _ => return Err("不支持的时间规则".into()),
        };
        Ok(Self {
            starts_at: r
                .starts_at
                .as_ref()
                .and_then(|v| v.as_deref())
                .map(DateTime::parse_from_rfc3339)
                .transpose()
                .map_err(|e| e.to_string())?,
            ends_at: r
                .ends_at
                .as_ref()
                .and_then(|v| v.as_deref())
                .map(DateTime::parse_from_rfc3339)
                .transpose()
                .map_err(|e| e.to_string())?,
            rule,
            timezone: r.timezone.clone().unwrap_or_else(|| "UTC".into()),
            start_date: r
                .start_date
                .as_ref()
                .map(|v| v.as_deref().map(parse_date).transpose())
                .transpose()?,
            end_date: r
                .end_date
                .as_ref()
                .map(|v| v.as_deref().map(parse_date).transpose())
                .transpose()?,
        })
    }

    pub fn from_job(job: &CronJobDto) -> Result<Self, String> {
        // 兼容旧 DTO 缺失 kind 及既有 legacy 回读路径，不把未知新规则伪装成 Cron。
        let kind = match job.schedule_kind.trim().to_ascii_lowercase().as_str() {
            "" | "legacy" if job.every_ms.is_some() => "every".to_string(),
            "" | "legacy" if job.at.as_ref().is_some_and(|at| !at.trim().is_empty()) => {
                "at".to_string()
            }
            "" | "legacy" => "cron".to_string(),
            "指定时间" => "at".to_string(),
            "固定间隔" => "every".to_string(),
            kind => kind.to_string(),
        };
        Self::from_request(&CronPreviewRequest {
            starts_at: job.starts_at.clone().map(Some),
            ends_at: job.ends_at.clone().map(Some),
            schedule_kind: kind,
            schedule: Some(job.expression.clone()),
            at: job.at.clone(),
            every_ms: job.every_ms,
            timezone: job.timezone.clone(),
            anchor_date: job.anchor_date.clone(),
            weekdays: job.weekdays.clone(),
            hour: job.hour,
            minute: job.minute,
            ..Default::default()
        })
    }
}

fn parse_expression(expression: &str) -> CronScheduleRule {
    let advanced = || CronScheduleRule::Advanced(expression.into());
    let fields: Vec<_> = expression.split_whitespace().collect();
    let fields = match fields.as_slice() {
        ["0", rest @ ..] if rest.len() == 5 => rest,
        other => other,
    };
    let [m, h, d, month, week] = fields else {
        return advanced();
    };
    let Ok(minute) = m.parse::<u32>() else {
        return advanced();
    };
    let Ok(hour) = h.parse::<u32>() else {
        return advanced();
    };
    let Some(time) = NaiveTime::from_hms_opt(hour, minute, 0) else {
        return advanced();
    };
    let parse_days = |value: &str, max| -> Option<Vec<u32>> {
        let values = value.split(',').map(str::parse::<u32>).collect::<Result<Vec<_>, _>>().ok()?;
        sorted(&values, max).ok()
    };
    match (*d, *month, *week) {
        ("*", "*", "*") => CronScheduleRule::Daily { time },
        ("*", "*", days) => {
            let Some(weekdays) = days
                .split(',')
                .map(|d| WEEKDAYS.iter().position(|v| *v == d).map(|v| v as u32 + 1))
                .collect::<Option<Vec<_>>>()
            else {
                return advanced();
            };
            CronScheduleRule::Weekly { weekdays, time }
        }
        (days, "*", "*") => match parse_days(days, 31) {
            Some(days) => CronScheduleRule::Monthly { days, time },
            None => advanced(),
        },
        (day, month, "*") => match (day.parse::<u32>(), month.parse::<u32>()) {
            (Ok(day), Ok(month)) if NaiveDate::from_ymd_opt(2000, month, day).is_some() => {
                CronScheduleRule::Yearly { month, day, time }
            }
            _ => advanced(),
        },
        _ => advanced(),
    }
}

#[cfg(test)]
#[path = "cron_schedule_draft_tests.rs"]
mod tests;
