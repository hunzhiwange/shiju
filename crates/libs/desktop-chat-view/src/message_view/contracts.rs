use session_ui_types::ProcessingTiming;

pub const TODO_CONTINUATION_TEXT: &str = "TODO 尚未完成，正在继续推进";
pub const COMPACTION_STARTED_TEXT: &str = "正在压缩会话";
pub const COMPACTION_COMPLETED_TEXT: &str = "压缩完成";
pub const COMPACTION_FAILED_TEXT: &str = "会话压缩失败";
pub const COMPACTION_CANCELLED_TEXT: &str = "会话压缩已取消";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionStatusCardContract {
    pub horizontal_alignment: iced::alignment::Horizontal,
    pub vertical_alignment: iced::alignment::Vertical,
    pub uses_default_font: bool,
}

pub fn compaction_status_card_contract(content: &str) -> Option<CompactionStatusCardContract> {
    matches!(
        content,
        COMPACTION_STARTED_TEXT
            | COMPACTION_COMPLETED_TEXT
            | COMPACTION_FAILED_TEXT
            | COMPACTION_CANCELLED_TEXT
    )
    .then_some(CompactionStatusCardContract {
        horizontal_alignment: iced::alignment::Horizontal::Center,
        vertical_alignment: iced::alignment::Vertical::Center,
        uses_default_font: true,
    })
}
pub fn message_footer_padding(is_user: bool) -> iced::Padding {
    if is_user { [0, 6].into() } else { iced::Padding::ZERO }
}

pub fn processing_duration_label(
    i18n: desktop_i18n::I18n,
    timing: Option<&ProcessingTiming>,
    live_elapsed_ms: Option<u64>,
) -> Option<String> {
    let timing = timing?;
    let elapsed_ms = if let Some(completed_ms) = timing.completed_ms {
        timing
            .processing_elapsed_ms
            .unwrap_or_else(|| completed_ms.saturating_sub(timing.started_ms))
    } else {
        live_elapsed_ms?
    };
    let total_seconds = elapsed_ms / 1_000;
    let hours = total_seconds / 3_600;
    let minutes = total_seconds % 3_600 / 60;
    let seconds = total_seconds % 60;
    let duration = if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    };
    Some(i18n.format("已处理 %s", &[duration.into()]).expect("valid duration template"))
}
