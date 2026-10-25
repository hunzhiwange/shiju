//! 复制版 Codex 历史界面中的用户与助手消息单元。
//!
//! 用户消息保留原始文本，以便终端宽度变化时重建首行与续行缩进。
//! 助手消息通过 Markdown 渲染器生成富文本，并在定稿后按单宽度缓存。
//! 主视口与转录共用同一消息源码，但可按各自宽度独立生成布局。
//! 远程图像和链接只作为展示元数据保留，不在历史单元中主动访问网络。
//! 缓存仅优化重复滚动渲染，不改变消息内容或链接目的地。
//! 用户文本的前缀与续行缩进按显示宽度计算，兼容宽字符和长链接。
//! 助手消息定稿后才作为稳定源码缓存，实时可变尾部由流式控制器管理。
//! User and assistant message cells from the copied Codex history surface.

use super::{HistoryCell, prefixed_lines};
use crate::style::{proposed_plan_style, user_message_style};
use crate::vibe::event::V3Attachment;
use crate::wrapping::{RtOptions, adaptive_wrap_line};
use ratatui::prelude::*;

#[derive(Debug)]
pub struct UserHistoryCell {
    text: String,
    attachments: Vec<V3Attachment>,
}

impl UserHistoryCell {
    pub fn new(text: String, attachments: Vec<V3Attachment>) -> Self {
        Self { text, attachments }
    }
}

impl HistoryCell for UserHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let content_width = usize::from(width).saturating_sub(2).max(1);
        let mut lines = Vec::new();
        for source in self.text.lines().chain(self.text.is_empty().then_some("")) {
            let line = Line::from(source.to_string());
            lines.extend(
                adaptive_wrap_line(&line, RtOptions::new(content_width))
                    .into_iter()
                    .map(|line| crate::render::line_utils::line_to_static(&line)),
            );
        }
        for attachment in &self.attachments {
            let path =
                attachment.path.as_deref().map(|path| format!(" ({path})")).unwrap_or_default();
            lines.push(Line::from(format!("{}{}", attachment.label, path)));
        }
        prefixed_lines(lines, "▌ ", "  ")
            .into_iter()
            .map(|line| line.style(user_message_style()))
            .collect()
    }
}

#[derive(Debug)]
pub struct AgentMessageCell {
    markdown: String,
}

impl AgentMessageCell {
    pub fn new(markdown: String) -> Self {
        Self { markdown }
    }
}

impl HistoryCell for AgentMessageCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        crate::markdown::append_markdown(
            &self.markdown,
            Some(usize::from(width).saturating_sub(2).max(1)),
            None,
            &mut lines,
        );
        prefixed_lines(lines, "• ", "  ")
    }
}

#[derive(Debug)]
pub struct PlainHistoryCell {
    label: String,
    text: String,
    style: Style,
}

impl PlainHistoryCell {
    pub fn label(label: &str, text: &str) -> Self {
        Self { label: label.to_string(), text: text.to_string(), style: Style::default().dim() }
    }

    pub fn bullet(label: &str, text: &str) -> Self {
        Self { label: label.to_string(), text: text.to_string(), style: proposed_plan_style() }
    }

    pub fn attachment(label: &str, path: Option<&str>) -> Self {
        Self::label(
            "attachment",
            &format!("{label}{}", path.map(|p| format!(" ({p})")).unwrap_or_default()),
        )
    }

    pub fn progress(label: &str, detail: Option<&str>) -> Self {
        Self::label(label, detail.unwrap_or("working"))
    }
}

impl HistoryCell for PlainHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let prefix = format!("{}: ", self.label);
        let content_width =
            usize::from(width).saturating_sub(Line::from(prefix.clone()).width()).max(1);
        let lines = self.text.split('\n').flat_map(|source| {
            let line = Line::from(source.trim_end_matches('\r').to_string());
            adaptive_wrap_line(&line, RtOptions::new(content_width))
                .into_iter()
                .map(|line| crate::render::line_utils::line_to_static(&line))
                .collect::<Vec<_>>()
        });
        lines
            .enumerate()
            .map(|(index, line)| {
                let prefix = if index == 0 { prefix.clone() } else { "  ".to_string() };
                Line::from(vec![
                    Span::styled(prefix, self.style),
                    Span::styled(line.to_string(), self.style),
                ])
            })
            .collect()
    }
}
