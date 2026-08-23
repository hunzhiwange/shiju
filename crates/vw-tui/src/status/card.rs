//! VibeWindow adaptation of the copied `/status` card.
//! VibeWindow 对复制而来的 `/status` 卡片所做的适配。
//!
//! 本模块只把桥接层提供的会话快照排版为历史卡片，不主动查询运行时。

use ratatui::prelude::*;
use ratatui::style::Stylize;

use crate::history_cell::HistoryCell;
use crate::history_cell::session::card_inner_width;
use crate::history_cell::session::with_border_with_inner_width;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;

use super::V3SessionDisplayData;
use super::format::FieldFormatter;
use super::format::format_tokens_compact;

const STATUS_MAX_INNER_WIDTH: usize = 76;

/// Copied bordered status card populated exclusively from bridge-owned values.
/// 复制而来的带边框状态卡片，其内容完全取自桥接层拥有的值。
#[derive(Clone, Debug)]
pub struct StatusHistoryCell {
    data: V3SessionDisplayData,
}

impl StatusHistoryCell {
    /// 用一次只读会话快照创建状态卡片。
    #[must_use]
    pub fn new(data: V3SessionDisplayData) -> Self {
        Self { data }
    }

    fn content_lines(&self, inner_width: usize) -> Vec<Line<'static>> {
        // 标签宽度统一由格式化器计算，保证各字段值在同一列开始。
        let labels = ["Model", "Directory", "Gateway", "Permissions", "Session", "Token usage"];
        let formatter = FieldFormatter::from_labels(labels);
        let session = self
            .data
            .session_title
            .as_deref()
            .filter(|title| !title.trim().is_empty())
            .or(self.data.session_id.as_deref())
            .unwrap_or("<new session>");
        let usage = &self.data.token_usage;
        let usage_summary = format!(
            "{} total  ({} input + {} output)",
            format_tokens_compact(usage.total()),
            format_tokens_compact(usage.input),
            format_tokens_compact(usage.output),
        );

        let mut lines = vec![
            Line::from(vec![
                " >_ ".dim(),
                "VibeWindow".bold(),
                " ".dim(),
                format!("(v{})", self.data.version).dim(),
            ]),
            Line::default(),
            formatter.line("Model", vec![self.data.model.clone().into()]),
            formatter.line(
                "Directory",
                vec![self.data.cwd.display().to_string().into()],
            ),
            formatter.line("Gateway", vec![self.data.gateway_endpoint.clone().into()]),
            formatter.line("Permissions", vec![self.data.permissions.clone().into()]),
            formatter.line("Session", vec![session.to_string().into()]),
            Line::default(),
            formatter.line("Token usage", vec![usage_summary.into()]),
        ];
        if usage.cached > 0 || usage.reasoning > 0 {
            lines.push(formatter.continuation(vec![
                format!(
                    "{} cached + {} reasoning",
                    format_tokens_compact(usage.cached),
                    format_tokens_compact(usage.reasoning),
                )
                .dim(),
            ]));
        }
        lines
            .into_iter()
            .map(|line| truncate_line_with_ellipsis_if_overflow(line, inner_width))
            .collect()
    }
}

impl HistoryCell for StatusHistoryCell {
    /// 在窄到无法容纳边框时返回空内容，否则按最大宽度截断并加框。
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let Some(inner_width) = card_inner_width(width, STATUS_MAX_INNER_WIDTH) else {
            return Vec::new();
        };
        with_border_with_inner_width(self.content_lines(inner_width), inner_width)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::V3TokenUsage;

    // 构造覆盖会话标题及各类令牌计数的稳定展示数据。
    fn fixture() -> V3SessionDisplayData {
        V3SessionDisplayData {
            version: "0.4.0".to_string(),
            cwd: PathBuf::from("/workspace/vibe-window"),
            gateway_endpoint: "127.0.0.1:34112".to_string(),
            model: "openai/gpt-5.6".to_string(),
            permissions: "Plan · workspace write".to_string(),
            session_id: Some("session-14".to_string()),
            session_title: Some("Port welcome surfaces".to_string()),
            token_usage: V3TokenUsage {
                input: 1_024,
                output: 512,
                cached: 128,
                reasoning: 64,
            },
            resumed: true,
        }
    }

    #[test]
    // 验证卡片只显示 v3 桥接数据，不混入服务限额等外部状态。
    fn status_indicator_card_uses_vibe_data_without_service_limits() {
        let cell = StatusHistoryCell::new(fixture());
        let rendered = cell
            .display_lines(80)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!(rendered);
    }

    #[test]
    // 验证窄终端下字段仍按卡片布局折叠并保持可读。
    fn status_indicator_card_folds_at_narrow_width() {
        let cell = StatusHistoryCell::new(fixture());
        let rendered = cell
            .display_lines(36)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!(rendered);
    }
}
