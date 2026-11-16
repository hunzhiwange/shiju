//! 当前草稿的菜单转换，不执行剪贴板或焦点操作。
use desktop_session::message::chat::input::context_menus::{self, InputContextMenuState};
use iced::widget::text_editor;
#[derive(Clone, Copy)]
pub enum Action {
    Copy,
    Cut,
    SelectAll,
}
pub enum Effect {
    None,
    Copy(String),
    Focus,
}
pub struct Outcome {
    pub effect: Effect,
    pub sync_editor: bool,
}
pub fn update(
    menu: &mut InputContextMenuState,
    editor: &mut text_editor::Content,
    action: Action,
) -> Outcome {
    menu.close();
    match action {
        Action::Copy => {
            let selected = context_menus::copy_input_selection(editor);
            Outcome {
                effect: if selected.is_empty() { Effect::None } else { Effect::Copy(selected) },
                sync_editor: false,
            }
        }
        Action::Cut => {
            let selected = context_menus::cut_input_selection(editor);
            let changed = !selected.is_empty();
            Outcome {
                effect: if changed { Effect::Copy(selected) } else { Effect::None },
                sync_editor: changed,
            }
        }
        Action::SelectAll => {
            context_menus::select_all_input(editor);
            Outcome { effect: Effect::Focus, sync_editor: true }
        }
    }
}
