//! 本模块集中实现 `layout` 在底部面板中的职责。
//! 关键声明 `LayoutSections`、`desired_height`、`layout_sections` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。

use ratatui::layout::Rect;

use super::RequestUserInputOverlay;

pub(super) struct LayoutSections {
    pub(super) question_area: Rect,
    pub(super) options_area: Rect,
    pub(super) notes_area: Rect,
    pub(super) footer_area: Rect,
    pub(super) question_lines: Vec<String>,
}

impl LayoutSections {
    pub(super) fn desired_height(&self) -> u16 {
        self.question_area
            .height
            .saturating_add(self.options_area.height)
            .saturating_add(self.notes_area.height)
            .saturating_add(self.footer_area.height)
            .saturating_add(2)
    }
}

impl RequestUserInputOverlay {
    pub(super) fn layout_sections(&self, area: Rect) -> LayoutSections {
        let width = area.width.max(1);
        let question_lines = self.wrapped_question_lines(width.saturating_sub(2));
        let question_height = question_lines.len().min(area.height as usize) as u16;
        let mut y = area.y;
        let question_area = Rect::new(area.x, y, area.width, question_height);
        y = y.saturating_add(question_height).saturating_add(1);

        let options_height = if self.has_options() {
            self.options_preferred_height(width).min(area.height.saturating_sub(question_height))
        } else {
            0
        };
        let options_area = Rect::new(area.x, y, area.width, options_height);
        y = y.saturating_add(options_height);

        let notes_height = if self.notes_ui_visible() {
            self.notes_input_height(width).min(area.height.saturating_sub(y.saturating_sub(area.y)))
        } else {
            0
        };
        let notes_area = Rect::new(area.x, y, area.width, notes_height);
        y = y.saturating_add(notes_height);

        let footer_area = Rect::new(
            area.x,
            y,
            area.width,
            self.footer_required_height(width)
                .min(area.height.saturating_sub(y.saturating_sub(area.y))),
        );

        LayoutSections { question_area, options_area, notes_area, footer_area, question_lines }
    }
}
