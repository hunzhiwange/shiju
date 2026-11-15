//! 输入菜单对当前会话编辑器的局部操作。
use iced::widget::text_editor;

pub fn copy_input_selection(editor: &text_editor::Content) -> String {
    editor.selection().unwrap_or_default()
}

pub fn cut_input_selection(editor: &mut text_editor::Content) -> String {
    let selected = copy_input_selection(editor);
    if !selected.is_empty() {
        editor.perform(text_editor::Action::Edit(text_editor::Edit::Backspace));
    }
    selected
}

pub fn select_all_input(editor: &mut text_editor::Content) {
    editor.perform(text_editor::Action::SelectAll);
}

#[derive(Debug, Clone, Default)]
pub struct InputContextMenuState {
    pub open: bool,
    pub position: Option<(f32, f32)>,
}

impl InputContextMenuState {
    pub fn open(&mut self, x: f32, y: f32) {
        self.open = true;
        self.position = Some((x, y));
    }

    pub fn close(&mut self) {
        self.open = false;
        self.position = None;
    }
}

#[derive(Debug, Clone, Default)]
pub struct MessageContextMenuState {
    pub target: Option<u64>,
    pub position: Option<(f32, f32)>,
    pub text: String,
}

impl MessageContextMenuState {
    pub fn open(&mut self, target: u64, x: f32, y: f32, text: String) {
        self.target = Some(target);
        self.position = Some((x, y));
        self.text = text;
    }

    pub fn close(&mut self) {
        self.target = None;
        self.position = None;
        self.text.clear();
    }

    pub fn take_text(&mut self) -> String {
        self.target = None;
        self.position = None;
        std::mem::take(&mut self.text)
    }
}
