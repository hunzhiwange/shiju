//! 会话头部与紧凑转录标记。
//!
//! 会话卡片使用产品中立展示数据，不在渲染层读取运行时配置。
//! 标签先对齐再整体截断，使宽字符目录与边框宽度保持一致。
//! 四列空间保留给左右边框及内边距，过窄时直接不绘制残缺卡片。
//! 历史压缩边界使用紧凑单行标记，并按当前宽度安全裁切。
//! 会话标题为空时依次回退到会话标识和新会话占位文本。
//! 卡片只展示传入的模型、目录、网关与权限摘要，不查询外部状态。
//! Session headers and compact transcript markers.

use ratatui::prelude::*;

use crate::V3SessionDisplayData;
use crate::line_truncation::line_width;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::width::display_width;

use super::HistoryCell;

pub(crate) const SESSION_HEADER_MAX_INNER_WIDTH: usize = 56;

pub(crate) fn card_inner_width(width: u16, max_inner_width: usize) -> Option<usize> {
    if width < 4 {
        return None;
    }
    Some(std::cmp::min(usize::from(width.saturating_sub(4)), max_inner_width))
}

pub(crate) fn with_border_with_inner_width(
    lines: Vec<Line<'static>>,
    inner_width: usize,
) -> Vec<Line<'static>> {
    let max_line_width = lines.iter().map(line_width).max().unwrap_or(0);
    let content_width = inner_width.max(max_line_width);
    let border_inner_width = content_width + 2;
    let mut out = Vec::with_capacity(lines.len() + 2);
    out.push(Line::from(format!("╭{}╮", "─".repeat(border_inner_width))).dim());
    for line in lines {
        let used_width = line_width(&line);
        let mut spans = Vec::with_capacity(line.spans.len() + 3);
        spans.push("│ ".dim());
        spans.extend(line);
        if used_width < content_width {
            spans.push(Span::from(" ".repeat(content_width - used_width)).dim());
        }
        spans.push(" │".dim());
        out.push(Line::from(spans));
    }
    out.push(Line::from(format!("╰{}╯", "─".repeat(border_inner_width))).dim());
    out
}

#[derive(Debug)]
enum SessionHistoryKind {
    CompactBoundary(String),
    Header(V3SessionDisplayData),
}

/// 使用产品中立展示数据填充的复制版会话历史界面。
/// Copied session history surface populated by product-neutral display data.
#[derive(Debug)]
pub struct SessionHistoryCell {
    kind: SessionHistoryKind,
}

impl SessionHistoryCell {
    #[must_use]
    pub fn compact_boundary(summary: &str) -> Self {
        Self { kind: SessionHistoryKind::CompactBoundary(summary.to_string()) }
    }

    #[must_use]
    pub fn header(data: V3SessionDisplayData) -> Self {
        Self { kind: SessionHistoryKind::Header(data) }
    }

    fn header_lines(data: &V3SessionDisplayData, width: u16) -> Vec<Line<'static>> {
        let Some(inner_width) = card_inner_width(width, SESSION_HEADER_MAX_INNER_WIDTH) else {
            return Vec::new();
        };
        const MODEL_LABEL: &str = "model:";
        const DIRECTORY_LABEL: &str = "directory:";
        const GATEWAY_LABEL: &str = "gateway:";
        const PERMISSIONS_LABEL: &str = "permissions:";
        const SESSION_LABEL: &str = "session:";
        let label_width =
            [MODEL_LABEL, DIRECTORY_LABEL, GATEWAY_LABEL, PERMISSIONS_LABEL, SESSION_LABEL]
                .into_iter()
                .map(str::len)
                .max()
                .unwrap_or_default();
        let row = |label: &str, value: String| {
            Line::from(vec![format!("{label:<label_width$} ").dim(), Span::from(value)])
        };
        let directory_prefix_width =
            display_width(DIRECTORY_LABEL) + 1 + label_width - DIRECTORY_LABEL.len();
        let directory_width = inner_width.saturating_sub(directory_prefix_width);
        let directory = crate::text_formatting::center_truncate_path(
            data.cwd.display().to_string().as_str(),
            directory_width,
        );
        let session = data
            .session_title
            .as_deref()
            .filter(|title| !title.trim().is_empty())
            .or(data.session_id.as_deref())
            .unwrap_or("<new session>");
        let lines = vec![
            Line::from(vec![
                ">_ ".dim(),
                "ShiJu".bold(),
                " ".dim(),
                format!("(v{})", data.version).dim(),
            ]),
            Line::default(),
            row(MODEL_LABEL, data.model.clone()),
            row(DIRECTORY_LABEL, directory),
            row(GATEWAY_LABEL, data.gateway_endpoint.clone()),
            row(PERMISSIONS_LABEL, data.permissions.clone()),
            row(SESSION_LABEL, session.to_string()),
        ];
        let lines = lines
            .into_iter()
            .map(|line| truncate_line_with_ellipsis_if_overflow(line, inner_width))
            .collect();
        with_border_with_inner_width(lines, inner_width)
    }
}

impl HistoryCell for SessionHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        match &self.kind {
            SessionHistoryKind::Header(data) => Self::header_lines(data, width),
            SessionHistoryKind::CompactBoundary(summary) => {
                let width = usize::from(width).max(1);
                let text = format!("── compacted history: {summary} ──");
                let text = if text.chars().count() > width {
                    text.chars().take(width).collect::<String>()
                } else {
                    text
                };
                vec![Line::from(text).dim()]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::V3TokenUsage;

    fn fixture() -> V3SessionDisplayData {
        V3SessionDisplayData {
            version: "0.4.0".to_string(),
            cwd: PathBuf::from("/workspace/vibe-window"),
            gateway_endpoint: "127.0.0.1:34112".to_string(),
            model: "openai/gpt-5.6".to_string(),
            permissions: "Plan · workspace write".to_string(),
            session_id: Some("session-14".to_string()),
            session_title: Some("Port welcome surfaces".to_string()),
            token_usage: V3TokenUsage { input: 1_024, output: 512, cached: 128, reasoning: 64 },
            resumed: false,
        }
    }

    #[test]
    fn session_header_keeps_copied_border_and_narrow_fold() {
        let cell = SessionHistoryCell::header(fixture());
        let wide = cell
            .display_lines(80)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let narrow = cell
            .display_lines(34)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!("session_header_vibe_wide", wide);
        insta::assert_snapshot!("session_header_vibe_narrow", narrow);
    }

    #[test]
    fn v3_welcome_status_and_session_header_keep_codex_layout_with_vibe_data() {
        let data = fixture();
        let header = SessionHistoryCell::header(data.clone());
        let header_text = header
            .display_lines(80)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(header_text.contains(">_ ShiJu (v0.4.0)"));
        assert!(header_text.contains("model:       openai/gpt-5.6"));
        assert!(header_text.contains("directory:   /workspace/vibe-window"));

        let status = crate::StatusHistoryCell::new(data);
        let status_text = status
            .display_lines(80)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(status_text.contains(">_ ShiJu (v0.4.0)"));
        assert!(status_text.contains("Gateway:"));
        assert!(!status_text.contains("OpenAI"));
        assert!(!status_text.contains("ChatGPT"));
    }
}
