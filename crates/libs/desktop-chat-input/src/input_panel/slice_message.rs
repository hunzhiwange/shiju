#[derive(Debug, Clone)]
pub enum Message {
    RemoveFileReference(String),
    FileReferenceHoverChanged(Option<usize>),
    FileSearchSelect(String),
    SlashPaletteNavigateUp,
    SlashPaletteNavigateDown,
    SlashPaletteSelectCurrent,
    SlashPaletteClosed,
    FileSearchNavigateUp,
    FileSearchNavigateDown,
    FileSearchSelectCurrent,
    FileSearchInputChanged(String),
    PasteIntoInput,
    SendPressed,
    InputEditorAction(iced::widget::text_editor::Action),
    OpenInputContextMenu { x: f32, y: f32 },
    CloseInputContextMenu,
    CopyInputSelection,
    CutInputSelection,
    SelectAllInput,
    QueueGuide(usize),
    QueueEdit(usize),
    QueueCopy(usize),
    QueueUp(usize),
    QueueDown(usize),
    QueueRemove(usize),
}
pub fn dark_tooltip_content<'a>(label: impl Into<String>) -> iced::Element<'a, Message> {
    iced::widget::container(iced::widget::text(label.into()).size(12).color(iced::Color::WHITE))
        .style(super::styles::tooltip_dark_style)
        .padding([6, 8])
        .into()
}
