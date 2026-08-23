//! 本模块集中实现 `mod` 在底部面板中的职责。
//! 关键声明 `TIP_SEPARATOR`、`DESIRED_SPACERS_BETWEEN_SECTIONS`、`OTHER_OPTION_LABEL`、`OTHER_OPTION_DESCRIPTION` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。
//! 空数据、越界索引与不可用选项按现有分支保持可预期的回退。
//! 布局计算以当前可用区域为边界，避免向小终端尺寸外绘制。
//! 键盘输入只在当前界面状态内解释，未匹配输入继续遵循上层约定。
//! 取消与关闭路径保持已有返回语义，不暗中确认待处理操作。
//! 选择、滚动与焦点值在使用前由原有逻辑约束到有效范围。


//! 将请求 - 用户输入表面与 VibeWindow 拥有的答案动作复制。 问题、选项标签和当前焦点属于呈现状态。提交事件不构造应用服务器响应，也不创建任何 AppServer 响应。空提交、未回答的问题以及 `Esc` 保持显式路径，以便桥接应用可以应用其自身策略。
//! Copied request-user-input surface with VibeWindow-owned answer actions.
//!
//! Questions, option labels, and the current focus are presentation state.  Submission emits
//! [`V3Action::AnswerUserInput`] and never constructs an app-server response.  Empty submissions,
//! unanswered questions, and `Esc` remain explicit paths so a bridge can apply its own policy.

mod layout;
mod render;

use crate::bottom_pane::ChatComposer;
use crate::bottom_pane::bottom_pane_view::BottomPaneView;
use crate::bottom_pane::scroll_state::ScrollState;
use crate::bottom_pane::selection_popup_common::GenericDisplayRow;
use crate::render::renderable::Renderable;
use crate::vibe::action::{V3Action, V3UserInputAnswer, V3UserInputQuestion, V3UserInputRequest};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

pub(super) const TIP_SEPARATOR: &str = " | ";
pub(super) const DESIRED_SPACERS_BETWEEN_SECTIONS: u16 = 2;
const OTHER_OPTION_LABEL: &str = "None of the above";
const OTHER_OPTION_DESCRIPTION: &str = "Optionally, add details in notes (tab).";
const UNANSWERED_CONFIRM_TITLE: &str = "Submit with unanswered questions?";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Options,
    Notes,
}

#[derive(Clone, Debug, Default)]
struct AnswerState {
    selected_idx: Option<usize>,
    committed: bool,
    notes_visible: bool,
    notes: String,
}

#[derive(Clone, Debug)]
pub(super) struct FooterTip {
    pub(super) text: String,
    pub(super) highlight: bool,
}

impl FooterTip {
    fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), highlight: false }
    }

    fn highlighted(text: impl Into<String>) -> Self {
        Self { text: text.into(), highlight: true }
    }
}

pub(crate) struct RequestUserInputOverlay {
    request: V3UserInputRequest,
    composer: ChatComposer,
    answers: Vec<AnswerState>,
    current_idx: usize,
    focus: Focus,
    confirm_unanswered: bool,
    done: bool,
}

impl RequestUserInputOverlay {
    #[must_use]
    pub(crate) fn new(request: V3UserInputRequest) -> Self {
        let answers = request
            .questions
            .iter()
            .map(|question| AnswerState {
                selected_idx: (!question.options.is_empty()).then_some(0),
                notes_visible: question.options.is_empty(),
                ..AnswerState::default()
            })
            .collect();
        let focus = if request.questions.first().is_some_and(|question| question.options.is_empty())
        {
            Focus::Notes
        } else {
            Focus::Options
        };
        let mut overlay = Self {
            request,
            composer: ChatComposer::with_data(
                "Type your answer (optional)".into(),
                Default::default(),
            ),
            answers,
            current_idx: 0,
            focus,
            confirm_unanswered: false,
            done: false,
        };
        overlay.sync_composer();
        overlay
    }

    #[must_use]
    pub(crate) fn is_complete(&self) -> bool {
        self.done
    }

    #[must_use]
    pub(crate) fn request(&self) -> &V3UserInputRequest {
        &self.request
    }

    fn current_question(&self) -> Option<&V3UserInputQuestion> {
        self.request.questions.get(self.current_idx)
    }

    fn current_answer(&self) -> Option<&AnswerState> {
        self.answers.get(self.current_idx)
    }

    fn current_answer_mut(&mut self) -> Option<&mut AnswerState> {
        self.answers.get_mut(self.current_idx)
    }

    fn has_options(&self) -> bool {
        self.current_question().is_some_and(|question| !question.options.is_empty())
    }

    fn options_len(&self) -> usize {
        let Some(question) = self.current_question() else {
            return 0;
        };
        question.options.len() + usize::from(question.allow_other && !question.options.is_empty())
    }

    fn option_label(&self, idx: usize) -> Option<String> {
        let question = self.current_question()?;
        question.options.get(idx).map(|option| option.label.clone()).or_else(|| {
            (idx == question.options.len() && question.allow_other && !question.options.is_empty())
                .then(|| OTHER_OPTION_LABEL.to_string())
        })
    }

    fn option_description(&self, idx: usize) -> Option<String> {
        let question = self.current_question()?;
        question.options.get(idx).map(|option| option.description.clone()).or_else(|| {
            (idx == question.options.len() && question.allow_other && !question.options.is_empty())
                .then(|| OTHER_OPTION_DESCRIPTION.to_string())
        })
    }

    fn selected_option_index(&self) -> Option<usize> {
        self.current_answer().and_then(|answer| answer.selected_idx)
    }

    fn sync_composer(&mut self) {
        let notes = self.current_answer().map(|answer| answer.notes.clone()).unwrap_or_default();
        self.composer.clear();
        if !notes.is_empty() {
            self.composer.handle_paste(notes);
        }
    }

    fn save_notes(&mut self) {
        let notes = self.composer.text().to_string();
        if let Some(answer) = self.current_answer_mut() {
            answer.notes = notes;
            if answer.notes_visible && !answer.notes.is_empty() {
                answer.committed = false;
            }
        }
    }

    fn move_question(&mut self, delta: isize) {
        if self.request.questions.is_empty() {
            return;
        }
        self.save_notes();
        let len = self.request.questions.len() as isize;
        self.current_idx = (self.current_idx as isize + delta).rem_euclid(len) as usize;
        self.focus = if self.has_options() { Focus::Options } else { Focus::Notes };
        self.sync_composer();
    }

    fn select_option(&mut self, idx: usize) {
        if idx >= self.options_len() {
            return;
        }
        if let Some(answer) = self.current_answer_mut() {
            answer.selected_idx = Some(idx);
            answer.committed = false;
        }
    }

    fn next_or_submit(&mut self) -> V3Action {
        self.save_notes();
        let answered = self
            .current_answer()
            .is_some_and(|answer| answer.selected_idx.is_some() || !answer.notes.trim().is_empty());
        if let Some(answer) = self.current_answer_mut() {
            answer.committed = answered;
        }
        if self.current_idx + 1 < self.request.questions.len() {
            self.move_question(1);
            return V3Action::None;
        }
        if self.unanswered_count() > 0 && !self.confirm_unanswered {
            self.confirm_unanswered = true;
            return V3Action::None;
        }
        self.submit_answers()
    }

    fn unanswered_count(&self) -> usize {
        self.answers.iter().filter(|answer| !answer.committed).count()
    }

    fn submit_answers(&mut self) -> V3Action {
        self.save_notes();
        let answers = self
            .request
            .questions
            .iter()
            .zip(&self.answers)
            .map(|(question, answer)| {
                let mut values = answer
                    .selected_idx
                    .and_then(|idx| self.option_label_for(question, idx))
                    .into_iter()
                    .collect::<Vec<_>>();
                if !answer.notes.trim().is_empty() {
                    values.push(format!("user_note: {}", answer.notes.trim()));
                }
                V3UserInputAnswer { question_id: question.id.clone(), values }
            })
            .collect();
        self.done = true;
        V3Action::AnswerUserInput { request_id: self.request.id.clone(), answers }
    }

    fn option_label_for(&self, question: &V3UserInputQuestion, idx: usize) -> Option<String> {
        question.options.get(idx).map(|option| option.label.clone()).or_else(|| {
            (idx == question.options.len() && question.allow_other && !question.options.is_empty())
                .then(|| OTHER_OPTION_LABEL.to_string())
        })
    }

    fn option_rows(&self) -> Vec<GenericDisplayRow> {
        (0..self.options_len())
            .filter_map(|idx| {
                let label = self.option_label(idx)?;
                let selected = self.selected_option_index() == Some(idx);
                let prefix = if selected { '›' } else { ' ' };
                let prefix_label = format!("{prefix} {}. ", idx + 1);
                Some(GenericDisplayRow {
                    name: format!("{prefix_label}{label}"),
                    description: self.option_description(idx),
                    wrap_indent: Some(UnicodeWidthStr::width(prefix_label.as_str())),
                    ..Default::default()
                })
            })
            .collect()
    }

    fn wrapped_question_lines(&self, width: u16) -> Vec<String> {
        let Some(question) = self.current_question() else {
            return Vec::new();
        };
        let text = if question.header.is_empty() {
            question.question.clone()
        } else {
            format!("{}: {}", question.header, question.question)
        };
        textwrap::wrap(&text, width.max(1) as usize)
            .into_iter()
            .map(|line| line.into_owned())
            .collect()
    }

    fn notes_ui_visible(&self) -> bool {
        !self.has_options() || self.current_answer().is_some_and(|answer| answer.notes_visible)
    }

    fn notes_input_height(&self, width: u16) -> u16 {
        self.composer.desired_height(width.max(1)).max(1)
    }

    fn footer_required_height(&self, _width: u16) -> u16 {
        1
    }

    fn options_required_height(&self, width: u16) -> u16 {
        let rows = self.option_rows();
        if rows.is_empty() {
            0
        } else {
            crate::bottom_pane::selection_popup_common::measure_rows_height(
                &rows,
                &ScrollState { selected_idx: self.selected_option_index(), scroll_top: 0 },
                rows.len(),
                width.max(1),
            )
        }
    }

    fn options_preferred_height(&self, width: u16) -> u16 {
        self.options_required_height(width)
    }

    fn footer_tips(&self) -> Vec<FooterTip> {
        if self.confirm_unanswered {
            return vec![
                FooterTip::highlighted("enter to proceed"),
                FooterTip::new("esc to go back"),
            ];
        }
        let mut tips = Vec::new();
        if self.has_options() && self.selected_option_index().is_some() && !self.notes_ui_visible()
        {
            tips.push(FooterTip::highlighted("tab to add notes"));
        }
        tips.push(FooterTip::highlighted("enter to submit"));
        if self.request.questions.len() > 1 {
            tips.push(FooterTip::new("←/→ to navigate questions"));
        }
        tips.push(FooterTip::new("esc to cancel"));
        tips
    }

    fn footer_tip_lines(&self, width: u16) -> Vec<Vec<FooterTip>> {
        let max_width = width.max(1) as usize;
        let mut lines = vec![Vec::new()];
        let mut used = 0;
        for tip in self.footer_tips() {
            let extra = if lines.last().is_some_and(|line| !line.is_empty()) {
                TIP_SEPARATOR.len() + tip.text.len()
            } else {
                tip.text.len()
            };
            if used > 0 && used + extra > max_width {
                lines.push(Vec::new());
                used = 0;
            }
            used += if used == 0 { tip.text.len() } else { extra };
            lines.last_mut().expect("footer line exists").push(tip);
        }
        lines
    }

    fn confirm_key(&mut self, event: KeyEvent) -> V3Action {
        match event.code {
            KeyCode::Enter => self.submit_answers(),
            KeyCode::Esc => {
                self.confirm_unanswered = false;
                V3Action::None
            }
            _ => V3Action::None,
        }
    }
}

impl BottomPaneView for RequestUserInputOverlay {
    fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        if self.done || event.kind != KeyEventKind::Press {
            return V3Action::None;
        }
        if self.confirm_unanswered {
            return self.confirm_key(event);
        }
        if event.code == KeyCode::Esc {
            if self.focus == Focus::Notes && self.has_options() && self.notes_ui_visible() {
                self.save_notes();
                if let Some(answer) = self.current_answer_mut() {
                    answer.notes.clear();
                    answer.notes_visible = false;
                    answer.committed = false;
                }
                self.focus = Focus::Options;
                self.sync_composer();
                return V3Action::None;
            }
            self.done = true;
            return V3Action::Cancel;
        }
        if self.focus == Focus::Options {
            match event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    let len = self.options_len();
                    if len > 0 {
                        let idx = self.selected_option_index().unwrap_or(0);
                        self.select_option((idx + len - 1) % len);
                    }
                    return V3Action::None;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let len = self.options_len();
                    if len > 0 {
                        let idx = self.selected_option_index().unwrap_or(0);
                        self.select_option((idx + 1) % len);
                    }
                    return V3Action::None;
                }
                KeyCode::Left => {
                    self.move_question(-1);
                    return V3Action::None;
                }
                KeyCode::Right => {
                    self.move_question(1);
                    return V3Action::None;
                }
                KeyCode::Tab => {
                    if self.selected_option_index().is_some() {
                        if let Some(answer) = self.current_answer_mut() {
                            answer.notes_visible = true;
                        }
                        self.focus = Focus::Notes;
                        self.sync_composer();
                    }
                    return V3Action::None;
                }
                KeyCode::Enter => return self.next_or_submit(),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    if let Some(idx) = ch.to_digit(10).and_then(|value| value.checked_sub(1)) {
                        self.select_option(idx as usize);
                    }
                    return V3Action::None;
                }
                _ => {}
            }
        }
        if self.focus == Focus::Notes {
            if event.code == KeyCode::Tab {
                self.save_notes();
                if self.has_options() {
                    if let Some(answer) = self.current_answer_mut() {
                        answer.notes_visible = false;
                    }
                    self.focus = Focus::Options;
                    self.sync_composer();
                }
                return V3Action::None;
            }
            if event.code == KeyCode::Left && event.modifiers.contains(KeyModifiers::CONTROL) {
                self.move_question(-1);
                return V3Action::None;
            }
            if event.code == KeyCode::Right && event.modifiers.contains(KeyModifiers::CONTROL) {
                self.move_question(1);
                return V3Action::None;
            }
            let result = self.composer.handle_key_event(event);
            if matches!(result, V3Action::Submit { .. })
                || (event.code == KeyCode::Enter && event.modifiers.is_empty())
            {
                return self.next_or_submit();
            }
            return V3Action::None;
        }
        V3Action::None
    }
}

impl Renderable for RequestUserInputOverlay {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.render_ui_at(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.layout_sections(Rect::new(0, 0, width, 32)).desired_height()
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        if self.focus == Focus::Notes && self.notes_ui_visible() {
            let sections = self.layout_sections(area);
            self.composer.cursor_pos(sections.notes_area)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibe::action::{V3UserInputOption, V3UserInputQuestion};

    fn request() -> V3UserInputRequest {
        V3UserInputRequest::new(
            "input-1",
            vec![V3UserInputQuestion::new(
                "mode",
                "Mode",
                "Choose a mode",
                vec![
                    V3UserInputOption::new("Safe", "Read-only work"),
                    V3UserInputOption::new("Fast", "Allow broader work"),
                ],
            )],
        )
    }

    // 验证 `request_user_input_emits_data_without_reply_side_effect` 场景的触发条件与预期结果。
    #[test]
    fn request_user_input_emits_data_without_reply_side_effect() {
        let mut overlay = RequestUserInputOverlay::new(request());
        assert_eq!(
            overlay.handle_key_event(KeyEvent::from(KeyCode::Enter)),
            V3Action::AnswerUserInput {
                request_id: "input-1".into(),
                answers: vec![V3UserInputAnswer {
                    question_id: "mode".into(),
                    values: vec!["Safe".into()],
                }],
            }
        );
    }

    // 验证 `unanswered_confirmation_keeps_go_back_path` 场景的触发条件与预期结果。
    #[test]
    fn unanswered_confirmation_keeps_go_back_path() {
        let mut question = request();
        question.questions[0].options.clear();
        let mut overlay = RequestUserInputOverlay::new(question);
        assert_eq!(overlay.handle_key_event(KeyEvent::from(KeyCode::Enter)), V3Action::None);
        assert_eq!(overlay.handle_key_event(KeyEvent::from(KeyCode::Esc)), V3Action::None);
        assert!(!overlay.is_complete());
    }
}
