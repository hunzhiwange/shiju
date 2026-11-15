/// 将秒数格式化为适合界面展示的简短时长字符串。
pub fn format_duration(secs: i64) -> String {
    if secs <= 0 {
        return String::new();
    }
    if secs < 60 {
        return format!("{}s", secs);
    }
    if secs < 3600 {
        let mins = secs / 60;
        let remaining = secs % 60;
        return if remaining > 0 {
            format!("{}m {}s", mins, remaining)
        } else {
            format!("{}m", mins)
        };
    }
    if secs < 86400 {
        let hours = secs / 3600;
        let remaining = (secs % 3600) / 60;
        return if remaining > 0 {
            format!("{}h {}m", hours, remaining)
        } else {
            format!("{}h", hours)
        };
    }
    if secs < 604800 {
        let days = secs / 86400;
        return if days == 1 { "~1 day".to_string() } else { format!("~{} days", days) };
    }
    let weeks = secs / 604800;
    if weeks == 1 { "~1 week".to_string() } else { format!("~{} weeks", weeks) }
}

/// 按字符长度截断字符串，并在尾部追加省略号。
pub fn truncate(s: &str, len: usize) -> String {
    if s.chars().count() <= len {
        return s.to_string();
    }
    if len == 0 {
        return String::new();
    }
    if len == 1 {
        return "…".to_string();
    }
    let mut out = String::new();
    for ch in s.chars().take(len - 1) {
        out.push(ch);
    }
    out.push('…');
    out
}

/// 将每个单词的首个字母转换为大写形式。
pub fn titlecase(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut start = true;
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            if start {
                for up in ch.to_uppercase() {
                    out.push(up);
                }
                start = false;
            } else {
                out.push(ch);
            }
        } else {
            start = true;
            out.push(ch);
        }
    }
    out
}

/// 在中间截断过长字符串，保留首尾信息。
pub fn truncate_middle(s: &str, max_length: usize) -> String {
    if s.chars().count() <= max_length {
        return s.to_string();
    }
    let ellipsis = "…";
    let ellipsis_len = ellipsis.chars().count();
    if max_length <= ellipsis_len {
        return ellipsis.to_string();
    }
    let keep_start = (max_length - ellipsis_len).div_ceil(2);
    let keep_end = (max_length - ellipsis_len) / 2;
    let start: String = s.chars().take(keep_start).collect();
    let end: String =
        s.chars().rev().take(keep_end).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{}{}{}", start, ellipsis, end)
}

/// 根据数量选择单复数模板并插入计数值。
pub fn pluralize(count: i64, singular: &str, plural: &str) -> String {
    let template = if count == 1 { singular } else { plural };
    template.replace("{}", &count.to_string())
}

/// 将数字压缩为更适合展示的 K 或 M 形式。
pub fn number(num: f64) -> String {
    if num >= 1_000_000.0 {
        format!("{:.1}M", num / 1_000_000.0)
    } else if num >= 1_000.0 {
        format!("{:.1}K", num / 1_000.0)
    } else if num.fract() == 0.0 {
        format!("{}", num as i64)
    } else {
        format!("{}", num)
    }
}

/// 将毫秒数格式化为适合界面展示的时长字符串。
pub fn duration(ms: i64) -> String {
    if ms < 1000 {
        return format!("{}ms", ms);
    }
    if ms < 60000 {
        return format!("{:.1}s", (ms as f64) / 1000.0);
    }
    if ms < 3600000 {
        let minutes = ms / 60000;
        let seconds = (ms % 60000) / 1000;
        return format!("{}m {}s", minutes, seconds);
    }
    if ms < 86400000 {
        let hours = ms / 3600000;
        let minutes = (ms % 3600000) / 60000;
        return format!("{}h {}m", hours, minutes);
    }
    let days = ms / 86400000;
    let hours = (ms % 86400000) / 3600000;
    format!("{}d {}h", days, hours)
}

#[cfg(test)]
#[path = "util_tests.rs"]
mod util_tests;
