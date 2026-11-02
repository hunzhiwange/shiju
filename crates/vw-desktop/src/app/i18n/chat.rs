//! Chat labels are rendered from semantic values with the caller's locale.
use super::I18n;

pub fn elapsed(i18n: I18n, seconds: u64) -> String {
    if seconds >= 60 {
        i18n.format("%u分 %u秒", &[(seconds / 60).into(), (seconds % 60).into()])
    } else {
        i18n.format("%u秒", &[seconds.into()])
    }
    .expect("valid elapsed template")
}

pub fn relative_time(i18n: I18n, bucket: (u8, u64)) -> String {
    let source = match bucket.0 {
        1 => "%u 分钟前",
        2 => "%u 小时前",
        3 => "%u 天前",
        4 => "%u 个月前",
        5 => "%u 年前",
        _ => return i18n.__("刚刚").to_owned(),
    };
    i18n.format(source, &[bucket.1.into()]).expect("valid relative time template")
}

/// Map stable trajectory field identifiers at the desktop display boundary.
pub fn trajectory_label(i18n: I18n, label: &str) -> String {
    let source = match label {
        "SYSTEM" => "系统",
        "USER" => "用户",
        "CONTEXT" => "上下文",
        "MODEL" => "模型",
        "TOOL" => "工具",
        "System Prompt" => "系统提示词",
        "Tools" => "工具",
        "Summary" => "摘要",
        "Preview" => "预览",
        "Raw" => "原始数据",
        "Source" => "来源",
        "Options" => "选项",
        "Usage" => "用量",
        "Timing" => "计时",
        "Payload" => "请求载荷",
        "Result" => "结果",
        "Schema" => "结构定义",
        "Status" => "状态",
        "Duration" => "耗时",
        "Reasoning" => "推理",
        "Content" => "内容",
        "Tool" => "工具",
        "Call ID" => "调用 ID",
        "Arguments" => "参数",
        "Input tokens" => "输入 Token",
        "Output tokens" => "输出 Token",
        "Cached tokens" => "缓存 Token",
        "Reasoning tokens" => "推理 Token",
        "Started" => "开始时间",
        "Timing source" => "计时来源",
        "No system prompt in this request" => "此请求未记录系统提示词",
        "No tools in this request" => "此请求没有工具",
        "No content" => "没有内容",
        "Source not recorded" => "未记录来源",
        "Options not recorded" => "未记录选项",
        "Usage not recorded" => "未记录用量",
        "No payload captured" => "未捕获请求载荷",
        "No result captured" => "未捕获结果",
        "Schema unavailable" => "结构定义不可用",
        "This tab is not available for this record" => "此记录不支持该选项卡",
        "Raw payload not recorded" => "未记录原始载荷",
        "Completed" => "已完成",
        "Running" => "运行中",
        "Session timestamps" => "会话时间戳",
        "Not available" => "不可用",
        "Not recorded" => "未记录",
        "Unknown" => "未知",
        "User" => "用户",
        _ => return label.to_string(),
    };
    i18n.__(source).to_string()
}

/// Cached exploration facts, independent of the language in which they are displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExploreCounts {
    pub reads: usize,
    pub searches: usize,
    pub listings: usize,
    pub others: usize,
}
impl ExploreCounts {
    pub fn render(self, i18n: I18n) -> String {
        let mut parts = Vec::new();
        for (source, count) in [
            ("%u 次读取", self.reads),
            ("%u 次搜索", self.searches),
            ("%u 次列出", self.listings),
            ("%u 次其他", self.others),
        ] {
            if count > 0 {
                parts.push(
                    i18n.format(source, &[count.into()]).expect("valid exploration count template"),
                );
            }
        }
        if parts.is_empty() {
            i18n.__("暂无").to_string()
        } else {
            parts.into_iter().reduce(|left, right| format!("{left}，{right}")).unwrap_or_default()
        }
    }
}

/// Message metadata retains its model and timestamp instead of a rendered fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MessageMeta {
    Model { model: String, time_ms: Option<u64> },
    TaskSource,
}
impl MessageMeta {
    pub(crate) fn render(&self, i18n: I18n) -> String {
        match self {
            Self::TaskSource => i18n
                .__(vw_shared::session::session_utils::TASK_SESSION_SOURCE_DISPLAY_LABEL)
                .to_string(),
            Self::Model { model, time_ms: None } => model.clone(),
            Self::Model { model, time_ms: Some(timestamp) } => {
                let time =
                    crate::app::components::chat_panel::utils::format_chat_time_label(*timestamp);
                let time = if time == "刚刚" { i18n.__("刚刚") } else { &time };
                format!("{model} · {time}")
            }
        }
    }
    pub(crate) fn cached_bytes(&self) -> usize {
        match self {
            Self::Model { model, .. } => model.len(),
            Self::TaskSource => 0,
        }
    }
}
