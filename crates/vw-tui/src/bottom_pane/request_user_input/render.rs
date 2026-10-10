//! 本模块集中实现 `render` 在底部面板中的职责。
//! 关键声明 `render_ui_at`、`text`、`request_user_input_render_preserves_chinese_options_and_focus_hint` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。
//! 空数据、越界索引与不可用选项按现有分支保持可预期的回退。
//! 布局计算以当前可用区域为边界，避免向小终端尺寸外绘制。
//! 键盘输入只在当前界面状态内解释，未匹配输入继续遵循上层约定。

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget, Wrap};

use crate::bottom_pane::scroll_state::ScrollState;
use crate::bottom_pane::selection_popup_common::{render_menu_surface, render_rows};
use crate::render::renderable::Renderable;

use super::{RequestUserInputOverlay, TIP_SEPARATOR};

impl RequestUserInputOverlay {
    pub(super) fn render_ui_at(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let content = render_menu_surface(area, buf);
        if content.is_empty() {
            return;
        }
        if self.confirm_unanswered {
            Paragraph::new(vec![
                Line::from(super::UNANSWERED_CONFIRM_TITLE.bold()),
                Line::from(format!("{} unanswered question(s)", self.unanswered_count())),
                Line::from("Enter to proceed | Esc to go back".dim()),
            ])
            .wrap(Wrap { trim: false })
            .render(content, buf);
            return;
        }

        let sections = self.layout_sections(content);
        for (offset, line) in sections.question_lines.iter().enumerate() {
            let y = sections.question_area.y.saturating_add(offset as u16);
            if y >= sections.question_area.bottom() {
                break;
            }
            Paragraph::new(Line::from(line.clone()).cyan()).render(
                Rect::new(sections.question_area.x, y, sections.question_area.width, 1),
                buf,
            );
        }

        if self.has_options() && !sections.options_area.is_empty() {
            let rows = self.option_rows();
            let state = ScrollState { selected_idx: self.selected_option_index(), scroll_top: 0 };
            render_rows(sections.options_area, buf, &rows, &state, rows.len().max(1), "No options");
        }

        if self.notes_ui_visible() && !sections.notes_area.is_empty() {
            self.composer.render(sections.notes_area, buf);
        }

        let tips = self.footer_tip_lines(sections.footer_area.width);
        for (row, tips) in tips.into_iter().enumerate() {
            if row >= sections.footer_area.height as usize {
                break;
            }
            let mut spans = Vec::new();
            for (index, tip) in tips.into_iter().enumerate() {
                if index > 0 {
                    spans.push(TIP_SEPARATOR.into());
                }
                if tip.highlight {
                    spans.push(tip.text.cyan().bold().not_dim());
                } else {
                    spans.push(tip.text.dim());
                }
            }
            Paragraph::new(Line::from(spans)).render(
                Rect::new(
                    sections.footer_area.x,
                    sections.footer_area.y.saturating_add(row as u16),
                    sections.footer_area.width,
                    1,
                ),
                buf,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibe::action::{V3UserInputOption, V3UserInputQuestion, V3UserInputRequest};

    fn text(buf: &Buffer, area: Rect) -> String {
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    // 验证 `request_user_input_render_preserves_chinese_options_and_focus_hint` 场景的触发条件与预期结果。
    #[test]
    fn request_user_input_render_preserves_chinese_options_and_focus_hint() {
        let request = V3UserInputRequest::new(
            "render-1",
            vec![V3UserInputQuestion::new(
                "mode",
                "模式",
                "请选择工作方式",
                vec![
                    V3UserInputOption::new("安全", "只读检查"),
                    V3UserInputOption::new("快速", "允许更多操作"),
                ],
            )],
        );
        let overlay = RequestUserInputOverlay::new(request);
        let area = Rect::new(0, 0, 52, overlay.desired_height(52).max(10));
        let mut buf = Buffer::empty(area);
        overlay.render(area, &mut buf);
        let rendered = text(&buf, area);
        let compact = rendered.replace(' ', "");
        assert!(compact.contains("请选择工作方式"), "{rendered}");
        assert!(compact.contains("安全"));
        assert!(rendered.contains("enter to submit"));
    }
}
