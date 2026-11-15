use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use tui_input::key_hint;
use tui_render::renderable::Renderable;
use tui_text::wrapping::RtOptions;
use tui_text::wrapping::adaptive_wrap_lines;

/// 显示待提交工具及在操作期间正在等待的后续输入项的 Widget。
/// Widget that displays pending steers plus follow-up inputs held while a turn is in progress.
///
/// The widget renders pending steers first, then rejected steers that will be
/// resubmitted at end of turn, then ordinary queued user messages. Pending
/// steers explain that they will be submitted after the next tool/result
/// boundary unless the user invokes the interrupt binding to send them
/// immediately. The edit hint at the bottom only appears when there are actual
/// queued user inputs to pop back into the composer. Because some terminals
/// intercept certain modifier-key combinations, the displayed binding is
/// configurable via [`set_edit_binding`](Self::set_edit_binding).
pub struct PendingInputPreview {
    pub pending_steers: Vec<String>,
    pub rejected_steers: Vec<String>,
    pub queued_messages: Vec<String>,
    /// 键组合在提示行中显示。默认使用 Alt+Up，但在终端中可能无法实现该组合。
    /// Key combination rendered in the hint line.  Defaults to Alt+Up but may
    /// be overridden for terminals where that chord is unavailable.
    edit_binding: Option<key_hint::ShortcutHint>,
    /// 用于立即中断并发送航向的键组合
    /// Key combination rendered for immediately interrupting and sending steers.
    interrupt_binding: Option<key_hint::ShortcutHint>,
}

const PREVIEW_LINE_LIMIT: usize = 3;

impl PendingInputPreview {
    pub fn new() -> Self {
        Self {
            pending_steers: Vec::new(),
            rejected_steers: Vec::new(),
            queued_messages: Vec::new(),
            edit_binding: Some(key_hint::alt(KeyCode::Up).into()),
            interrupt_binding: Some(key_hint::plain(KeyCode::Esc).into()),
        }
    }

    /// 将提示行底部的 queued-messages 列表中的键绑定替换。调用者还需负责关联相应的按键事件处理器。
    /// Replace the keybinding shown in the hint line at the bottom of the
    /// queued-messages list.  The caller is responsible for also wiring the
    /// corresponding key event handler.
    pub fn set_edit_binding(&mut self, binding: Option<key_hint::ShortcutHint>) {
        self.edit_binding = binding;
    }

    pub fn set_interrupt_binding(&mut self, binding: Option<key_hint::ShortcutHint>) {
        self.interrupt_binding = binding;
    }

    /// 从读写的 v3 快照表面中替换队列后的跟进文本。
    /// Replace queued follow-up text from the read-only v3 surface snapshot.
    pub fn set_queued_messages(&mut self, messages: Vec<String>) {
        self.queued_messages = messages;
    }

    pub fn clear(&mut self) {
        self.pending_steers.clear();
        self.rejected_steers.clear();
        self.queued_messages.clear();
    }

    fn push_truncated_preview_lines(
        lines: &mut Vec<Line<'static>>,
        wrapped: Vec<Line<'static>>,
        overflow_line: Line<'static>,
    ) {
        let wrapped_len = wrapped.len();
        lines.extend(wrapped.into_iter().take(PREVIEW_LINE_LIMIT));
        if wrapped_len > PREVIEW_LINE_LIMIT {
            lines.push(overflow_line);
        }
    }

    fn push_section_header(lines: &mut Vec<Line<'static>>, width: u16, header: Line<'static>) {
        let mut spans = vec!["• ".dim()];
        spans.extend(header.spans);
        lines.extend(adaptive_wrap_lines(
            std::iter::once(Line::from(spans)),
            RtOptions::new(width as usize).subsequent_indent(Line::from("  ".dim())),
        ));
    }

    fn as_renderable(&self, width: u16) -> Box<dyn Renderable> {
        if (self.pending_steers.is_empty()
            && self.rejected_steers.is_empty()
            && self.queued_messages.is_empty())
            || width < 4
        {
            return Box::new(());
        }

        let mut lines = vec![];

        if !self.pending_steers.is_empty() {
            let mut header = vec!["Messages to be submitted after next tool call".into()];
            if let Some(interrupt_binding) = self.interrupt_binding {
                header.extend(vec![
                    " (press ".dim(),
                    interrupt_binding.into(),
                    " to interrupt and send immediately)".dim(),
                ]);
            }
            Self::push_section_header(&mut lines, width, Line::from(header));

            for steer in &self.pending_steers {
                let wrapped = adaptive_wrap_lines(
                    steer.lines().take(PREVIEW_LINE_LIMIT + 1).map(|line| Line::from(line.dim())),
                    RtOptions::new(width as usize)
                        .initial_indent(Line::from("  ↳ ".dim()))
                        .subsequent_indent(Line::from("    ")),
                );
                Self::push_truncated_preview_lines(&mut lines, wrapped, Line::from("    …".dim()));
            }
        }

        if !self.rejected_steers.is_empty() {
            if !lines.is_empty() {
                lines.push(Line::from(""));
            }
            Self::push_section_header(
                &mut lines,
                width,
                "Messages to be submitted at end of turn".into(),
            );

            for steer in &self.rejected_steers {
                let wrapped = adaptive_wrap_lines(
                    steer.lines().take(PREVIEW_LINE_LIMIT + 1).map(|line| Line::from(line.dim())),
                    RtOptions::new(width as usize)
                        .initial_indent(Line::from("  ↳ ".dim()))
                        .subsequent_indent(Line::from("    ")),
                );
                Self::push_truncated_preview_lines(&mut lines, wrapped, Line::from("    …".dim()));
            }
        }

        if !self.queued_messages.is_empty() {
            if !lines.is_empty() {
                lines.push(Line::from(""));
            }
            Self::push_section_header(&mut lines, width, "Queued follow-up inputs".into());

            for message in &self.queued_messages {
                let wrapped = adaptive_wrap_lines(
                    message
                        .lines()
                        .take(PREVIEW_LINE_LIMIT + 1)
                        .map(|line| Line::from(line.dim().italic())),
                    RtOptions::new(width as usize)
                        .initial_indent(Line::from("  ↳ ".dim()))
                        .subsequent_indent(Line::from("    ")),
                );
                Self::push_truncated_preview_lines(
                    &mut lines,
                    wrapped,
                    Line::from("    …".dim().italic()),
                );
            }
        }

        if !self.queued_messages.is_empty()
            && let Some(edit_binding) = self.edit_binding
        {
            lines.push(
                Line::from(vec![
                    "    ".into(),
                    edit_binding.into(),
                    " edit last queued message".into(),
                ])
                .dim(),
            );
        }

        Paragraph::new(lines).into()
    }
}

impl Renderable for PendingInputPreview {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        self.as_renderable(area.width).render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.as_renderable(width).desired_height(width)
    }
}

#[cfg(test)]
#[path = "pending_input_preview_tests.rs"]
mod tests;
