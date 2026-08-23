//! 本模块集中实现 `footer` 在底部面板中的职责。
//! 关键声明 `INDENT`、`FooterMode`、`FooterProps`、`compact_tokens` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。
//! 空数据、越界索引与不可用选项按现有分支保持可预期的回退。
//! 布局计算以当前可用区域为边界，避免向小终端尺寸外绘制。


//! VibeWindow 输入编辑器中的纯底部渲染。
//! Pure footer rendering for the VibeWindow composer.
//!
//! The footer receives a read-only [`V3FooterData`] snapshot.  It does not query a model,
//! permission store, directory state, context tracker, Gateway, or runtime handle.

use crate::key_hint;
use crate::render::renderable::Renderable;
use crate::vibe::action::V3FooterData;
use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

const INDENT: &str = "  ";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum FooterMode {
    #[default]
    ComposerEmpty,
    ComposerHasDraft,
    Working,
    ShortcutOverlay,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct FooterProps {
    pub(crate) mode: FooterMode,
    pub(crate) data: V3FooterData,
    pub(crate) queued_count: usize,
    pub(crate) attachment_count: usize,
}

fn compact_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

fn context_spans(
    data: &V3FooterData,
    queued_count: usize,
    attachment_count: usize,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    if let Some(model) = &data.model {
        spans.push(Span::from(model.clone()).dim());
    }
    if let Some(permission) = &data.permission {
        if !spans.is_empty() {
            spans.push(" · ".dim());
        }
        spans.push(Span::from(permission.clone()).dim());
    }
    if let Some(directory) = &data.directory {
        if !spans.is_empty() {
            spans.push(" · ".dim());
        }
        spans.push(Span::from(directory.clone()).dim());
    }
    if let Some(percent) = data.context_window_percent {
        if !spans.is_empty() {
            spans.push(" · ".dim());
        }
        let usage = data
            .context_window_used_tokens
            .map(compact_tokens)
            .map_or_else(|| format!("{percent}% ctx"), |tokens| format!("{percent}% ctx/{tokens}"));
        spans.push(Span::from(usage).dim());
    }
    if queued_count > 0 {
        if !spans.is_empty() {
            spans.push(" · ".dim());
        }
        spans.push(Span::from(format!("{queued_count} queued")).dim());
    }
    if attachment_count > 0 {
        if !spans.is_empty() {
            spans.push(" · ".dim());
        }
        spans.push(Span::from(format!("{attachment_count} attached")).dim());
    }
    spans
}

fn left_line(props: &FooterProps) -> Line<'static> {
    match props.mode {
        FooterMode::ComposerEmpty => {
            Line::from(vec![key_hint::plain(KeyCode::Char('?')).into(), " shortcuts".into()])
        }
        FooterMode::ComposerHasDraft => {
            if props.data.is_task_running {
                Line::from(vec![key_hint::plain(KeyCode::Tab).into(), " queue".into()])
            } else {
                Line::from("Enter send · Shift+Enter newline")
            }
        }
        FooterMode::Working => {
            Line::from(vec![key_hint::plain(KeyCode::Esc).into(), " interrupt".into()])
        }
        FooterMode::ShortcutOverlay => Line::from(
            "Enter send · Tab queue · Esc cancel · ↑/↓ history · Ctrl+G editor · Ctrl+T transcript",
        ),
    }
}

fn fallback_line(props: &FooterProps, width: u16) -> Line<'static> {
    let left = left_line(props);
    let context = context_spans(&props.data, props.queued_count, props.attachment_count);
    let left_width = left.width();
    let context_width: usize =
        context.iter().map(|span| UnicodeWidthStr::width(span.content.as_ref())).sum();
    let available = usize::from(width.saturating_sub(INDENT.len() as u16));
    if left_width + context_width < available.saturating_sub(1) && !context.is_empty() {
        let mut spans = left.spans;
        spans.push("  ".into());
        spans.extend(context);
        return Line::from(spans);
    }
    if left_width <= available {
        return left;
    }
    if props.data.is_task_running {
        return Line::from("Esc interrupt");
    }
    Line::from("Enter send")
}

pub(crate) fn footer_height(props: &FooterProps, width: u16) -> u16 {
    if width < 12 {
        0
    } else if matches!(props.mode, FooterMode::ShortcutOverlay) {
        2
    } else {
        1
    }
}

pub(crate) fn render_footer(props: &FooterProps, area: Rect, buf: &mut Buffer) {
    if area.is_empty() || footer_height(props, area.width) == 0 {
        return;
    }
    let line = fallback_line(props, area.width);
    let line_area = Rect::new(area.x, area.y, area.width, 1);
    Paragraph::new(Line::from({
        let mut spans = vec![Span::from(INDENT)];
        spans.extend(line.spans);
        spans
    }))
    .render(line_area, buf);
    if matches!(props.mode, FooterMode::ShortcutOverlay) && area.height > 1 {
        Paragraph::new(Line::from(vec![
            Span::from("  "),
            Span::from("Ctrl+C interrupt · Ctrl+R history search · Ctrl+Y yank").dim(),
        ]))
        .render(Rect::new(area.x, area.y + 1, area.width, 1), buf);
    }
}

pub(crate) fn footer_line(props: &FooterProps, width: u16) -> Line<'static> {
    fallback_line(props, width)
}

impl Renderable for FooterProps {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        render_footer(self, area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        footer_height(self, width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 验证 `footer_collapses_for_narrow_widths` 场景的触发条件与预期结果。
    #[test]
    fn footer_collapses_for_narrow_widths() {
        let props = FooterProps {
            mode: FooterMode::ComposerHasDraft,
            data: V3FooterData {
                model: Some("model".into()),
                permission: Some("workspace".into()),
                directory: Some("/tmp/project".into()),
                context_window_percent: Some(42),
                context_window_used_tokens: Some(12_345),
                is_task_running: true,
            },
            queued_count: 2,
            attachment_count: 1,
        };
        assert!(footer_line(&props, 40).width() <= 40);
        assert!(footer_line(&props, 80).width() <= 80);
        assert!(footer_line(&props, 120).width() <= 120);
        assert_eq!(footer_height(&props, 10), 0);
    }
}
