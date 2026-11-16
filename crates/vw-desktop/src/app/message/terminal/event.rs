use crate::app::{App, Message};
use iced::Task;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn focus_active_terminal(app: &mut App) -> Task<Message> {
    let mut focused = app.focus_area == crate::app::FocusArea::Terminal;
    let mut effects = Vec::new();
    let task = desktop_terminal::updates::event::focus_active_terminal(
        &mut app.terminal,
        &mut focused,
        &mut effects,
    );
    if focused {
        app.focus_area = crate::app::FocusArea::Terminal;
    }
    let tasks = vec![task.map(Message::Terminal)];
    for effect in effects {
        if let desktop_terminal::updates::Effect::Focus = effect {
            for tab in &mut app.preview_tabs {
                tab.editor.inner.lose_focus();
            }
        }
    }
    Task::batch(tasks)
}
