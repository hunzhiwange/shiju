//! 常用执行频率与 Cron 表达式之间的保守映射。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronFrequency {
    Minutes,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Advanced,
}

impl CronFrequency {
    pub(crate) const ALL: [Self; 6] =
        [Self::Minutes, Self::Hourly, Self::Daily, Self::Weekly, Self::Monthly, Self::Advanced];

    pub(crate) fn default_expression(self) -> &'static str {
        match self {
            Self::Minutes => "*/5 * * * *",
            Self::Hourly => "0 * * * *",
            Self::Daily => "0 9 * * *",
            Self::Weekly => "0 9 * * MON",
            Self::Monthly => "0 9 1 * *",
            Self::Advanced => "",
        }
    }

    pub(crate) fn from_expression(expression: &str) -> Self {
        let fields: Vec<_> = expression.split_whitespace().collect();
        let number = |value: &str, max: u8| {
            value.parse::<u8>().is_ok_and(|n| n <= max && n.to_string() == value)
        };
        match fields.as_slice() {
            [m, "*", "*", "*", "*"]
                if m.strip_prefix("*/").is_some_and(|n| number(n, 59) && n != "0") =>
            {
                Self::Minutes
            }
            [m, "*", "*", "*", "*"] if number(m, 59) => Self::Hourly,
            [m, h, "*", "*", "*"] if number(m, 59) && number(h, 23) => Self::Daily,
            [m, h, "*", "*", d]
                if number(m, 59)
                    && number(h, 23)
                    && ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"].contains(d) =>
            {
                Self::Weekly
            }
            [m, h, d, "*", "*"] if number(m, 59) && number(h, 23) && number(d, 31) && *d != "0" => {
                Self::Monthly
            }
            _ => Self::Advanced,
        }
    }
}

impl std::fmt::Display for CronFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Minutes => "分钟间隔",
            Self::Hourly => "每小时",
            Self::Daily => "每天",
            Self::Weekly => "每周",
            Self::Monthly => "每月",
            Self::Advanced => "高级",
        })
    }
}

#[cfg(test)]
#[path = "cron_frequency_tests.rs"]
mod tests;
