//! 本模块集中实现 `pending_thread_approvals` 在底部面板中的职责。
//! 关键声明 `PendingThreadApprovals`、`new`、`set_threads`、`is_empty` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。


use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::render::renderable::Renderable;
use crate::wrapping::RtOptions;
use crate::wrapping::adaptive_wrap_lines;

/// 列出未激活线程的待审批请求列表
/// Widget that lists inactive threads with outstanding approval requests.
pub(crate) struct PendingThreadApprovals {
    threads: Vec<String>,
}

impl PendingThreadApprovals {
    pub(crate) fn new() -> Self {
        Self {
            threads: Vec::new(),
        }
    }

    pub(crate) fn set_threads(&mut self, threads: Vec<String>) -> bool {
        if self.threads == threads {
            return false;
        }
        self.threads = threads;
        true
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.threads.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn threads(&self) -> &[String] {
        &self.threads
    }

    fn as_renderable(&self, width: u16) -> Box<dyn Renderable> {
        if self.threads.is_empty() || width < 4 {
            return Box::new(());
        }

        let mut lines = Vec::new();
        for thread in self.threads.iter().take(3) {
            let wrapped = adaptive_wrap_lines(
                std::iter::once(Line::from(format!("Approval needed in {thread}"))),
                RtOptions::new(width as usize)
                    .initial_indent(Line::from(vec!["  ".into(), "!".red().bold(), " ".into()]))
                    .subsequent_indent(Line::from("    ")),
            );
            lines.extend(wrapped);
        }

        if self.threads.len() > 3 {
            lines.push(Line::from("    ...".dim().italic()));
        }

        lines.push(
            Line::from(vec![
                "    ".into(),
                "/subagents".cyan().bold(),
                " to switch threads".dim(),
            ])
            .dim(),
        );

        Paragraph::new(lines).into()
    }
}

impl Renderable for PendingThreadApprovals {
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
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use pretty_assertions::assert_eq;

    fn snapshot_rows(widget: &PendingThreadApprovals, width: u16) -> String {
        let height = widget.desired_height(width);
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        widget.render(Rect::new(0, 0, width, height), &mut buf);

        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    // 验证 `desired_height_empty` 场景的触发条件与预期结果。
    #[test]
    fn desired_height_empty() {
        let widget = PendingThreadApprovals::new();
        assert_eq!(widget.desired_height(/*width*/ 40), 0);
    }

    // 验证 `render_single_thread_snapshot` 场景的触发条件与预期结果。
    #[test]
    fn render_single_thread_snapshot() {
        let mut widget = PendingThreadApprovals::new();
        widget.set_threads(vec!["Robie [explorer]".to_string()]);

        assert_snapshot!(
            snapshot_rows(&widget, /*width*/ 40).replace(' ', "."),
            @r"
        ..!.Approval.needed.in.Robie.[explorer].
        ..../subagents.to.switch.threads........
        "
        );
    }

    // 验证 `render_multiple_threads_snapshot` 场景的触发条件与预期结果。
    #[test]
    fn render_multiple_threads_snapshot() {
        let mut widget = PendingThreadApprovals::new();
        widget.set_threads(vec![
            "Main [default]".to_string(),
            "Robie [explorer]".to_string(),
            "Inspector".to_string(),
            "Extra agent".to_string(),
        ]);

        assert_snapshot!(
            snapshot_rows(&widget, /*width*/ 44).replace(' ', "."),
            @r"
        ..!.Approval.needed.in.Main.[default].......
        ..!.Approval.needed.in.Robie.[explorer].....
        ..!.Approval.needed.in.Inspector............
        ............................................
        ..../subagents.to.switch.threads............
        "
        );
    }
}
