use super::text_editor_context_menu::{
    SelectionActionOutcome, TextEditorContextMenuMessages, TextEditorContextMenuState, close_menu,
    context_menu, focus_editor_task, is_dark_theme, menu_button_style, open_menu, paste_action,
    paste_task, popover_style, selection_copy_task, selection_cut_task, selection_delete_task,
    wrap_with_context_menu,
};
use iced::widget::{button, text, text_editor};
use iced::{Background, Color, Element, Length, Point, Theme};

#[derive(Clone, Debug, PartialEq, Eq)]
enum TestMessage {
    Close,
    Copy,
    Cut,
    Paste,
    Delete,
    Opened,
    Pasted(String),
}

fn editor_id() -> iced::widget::Id {
    iced::widget::Id::new("text-editor-context-menu-test-editor")
}

fn messages() -> TextEditorContextMenuMessages<TestMessage> {
    TextEditorContextMenuMessages {
        close: TestMessage::Close,
        copy: TestMessage::Copy,
        cut: TestMessage::Cut,
        paste: TestMessage::Paste,
        delete: TestMessage::Delete,
    }
}

fn selected_editor(text: &str) -> text_editor::Content {
    let mut editor = text_editor::Content::with_text(text);
    editor.perform(text_editor::Action::SelectAll);
    editor
}

#[test]
fn context_menu_state_defaults_to_closed_without_anchor() {
    let state = TextEditorContextMenuState::default();

    assert!(!state.open);
    assert_eq!(state.position, None);
}

#[test]
fn open_menu_records_anchor_position() {
    let mut state = TextEditorContextMenuState::default();

    open_menu(&mut state, Point::new(12.5, 34.0));

    assert!(state.open);
    assert_eq!(state.position, Some((12.5, 34.0)));
}

#[test]
fn open_menu_replaces_existing_anchor_position() {
    let mut state = TextEditorContextMenuState { open: true, position: Some((1.0, 2.0)) };

    open_menu(&mut state, Point::new(-8.5, 0.0));

    assert!(state.open);
    assert_eq!(state.position, Some((-8.5, 0.0)));
}

#[test]
fn close_menu_resets_open_state_and_position() {
    let mut state = TextEditorContextMenuState { open: true, position: Some((8.0, 13.0)) };

    close_menu(&mut state);

    assert!(!state.open);
    assert_eq!(state.position, None);
}

#[test]
fn wrap_with_context_menu_builds_closed_content() {
    let content = text("editor").width(Length::Fill);
    let state = TextEditorContextMenuState { open: false, position: Some((1.0, 2.0)) };

    let element: Element<'_, TestMessage> =
        wrap_with_context_menu(content, state, |_| TestMessage::Opened, messages());

    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn wrap_with_context_menu_ignores_open_state_without_position() {
    let content = text("editor").width(Length::Fill);
    let state = TextEditorContextMenuState { open: true, position: None };

    let element: Element<'_, TestMessage> =
        wrap_with_context_menu(content, state, |_| TestMessage::Opened, messages());

    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn wrap_with_context_menu_builds_open_overlay_when_position_exists() {
    let content = text("editor").width(Length::Fill);
    let state = TextEditorContextMenuState { open: true, position: Some((24.0, 48.0)) };

    let element: Element<'_, TestMessage> =
        wrap_with_context_menu(content, state, |_| TestMessage::Opened, messages());

    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn context_menu_builds_action_buttons() {
    let _element: Element<'_, TestMessage> = context_menu(messages());
}

#[test]
fn selection_copy_task_reports_none_for_empty_selection() {
    let editor = text_editor::Content::with_text("copy me");

    let (outcome, _task) = selection_copy_task::<TestMessage>(&editor, &editor_id());

    assert_eq!(outcome, SelectionActionOutcome::None);
    assert_eq!(editor.text(), "copy me");
}

#[test]
fn selection_copy_task_reports_copied_for_selected_text() {
    let editor = selected_editor("copy me");

    let (outcome, _task) = selection_copy_task::<TestMessage>(&editor, &editor_id());

    assert_eq!(outcome, SelectionActionOutcome::Copied);
    assert_eq!(editor.text(), "copy me");
    assert_eq!(editor.selection().as_deref(), Some("copy me"));
}

#[test]
fn selection_cut_task_reports_none_for_empty_selection() {
    let mut editor = text_editor::Content::with_text("cut me");

    let (outcome, _task) = selection_cut_task::<TestMessage>(&mut editor, &editor_id());

    assert_eq!(outcome, SelectionActionOutcome::None);
    assert_eq!(editor.text(), "cut me");
}

#[test]
fn selection_cut_task_removes_selected_text() {
    let mut editor = selected_editor("cut me");

    let (outcome, _task) = selection_cut_task::<TestMessage>(&mut editor, &editor_id());

    assert_eq!(outcome, SelectionActionOutcome::Cut);
    assert_eq!(editor.text(), "");
    assert_eq!(editor.selection(), None);
}

#[test]
fn selection_delete_task_reports_none_for_empty_selection() {
    let mut editor = text_editor::Content::with_text("delete me");

    let (outcome, _task) = selection_delete_task::<TestMessage>(&mut editor, &editor_id());

    assert_eq!(outcome, SelectionActionOutcome::None);
    assert_eq!(editor.text(), "delete me");
}

#[test]
fn selection_delete_task_removes_selected_text_without_copying() {
    let mut editor = selected_editor("delete me");

    let (outcome, _task) = selection_delete_task::<TestMessage>(&mut editor, &editor_id());

    assert_eq!(outcome, SelectionActionOutcome::Deleted);
    assert_eq!(editor.text(), "");
    assert_eq!(editor.selection(), None);
}

#[test]
fn paste_task_batches_clipboard_read_and_focus() {
    let _task = paste_task(&editor_id(), TestMessage::Pasted);
}

#[test]
fn focus_editor_task_targets_editor_id() {
    let _task = focus_editor_task::<TestMessage>(&editor_id());
}

#[test]
fn paste_action_wraps_content_as_text_editor_paste() {
    let action = paste_action("hello".to_string());

    match action {
        text_editor::Action::Edit(text_editor::Edit::Paste(content)) => {
            assert_eq!(&*content, "hello");
        }
        other => panic!("expected paste edit action, got {other:?}"),
    }
}

#[test]
fn paste_action_preserves_empty_multiline_and_unicode_content() {
    for content in ["", "first line\n第二行"] {
        let action = paste_action(content.to_string());

        match action {
            text_editor::Action::Edit(text_editor::Edit::Paste(pasted)) => {
                assert_eq!(&*pasted, content);
            }
            other => panic!("expected paste edit action, got {other:?}"),
        }
    }
}

#[test]
fn is_dark_theme_detects_builtin_light_and_dark_palettes() {
    assert!(!is_dark_theme(&Theme::Light));
    assert!(is_dark_theme(&Theme::Dark));
}

#[test]
fn is_dark_theme_uses_strict_combined_brightness_threshold() {
    let mut threshold_palette = iced::theme::Palette::LIGHT;
    threshold_palette.background = Color::from_rgb(0.5, 0.5, 0.5);
    let threshold = Theme::custom("threshold", threshold_palette);

    let mut below_threshold_palette = iced::theme::Palette::LIGHT;
    below_threshold_palette.background = Color::from_rgb(0.499, 0.5, 0.5);
    let below_threshold = Theme::custom("below-threshold", below_threshold_palette);

    assert!(!is_dark_theme(&threshold));
    assert!(is_dark_theme(&below_threshold));
}

#[test]
fn popover_style_uses_light_theme_surface_and_border() {
    let style = popover_style(&Theme::Light);

    assert_eq!(style.background, Some(Background::Color(Color::from_rgb8(0xF3, 0xF4, 0xF6))));
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, Color::from_rgba8(0x00, 0x00, 0x00, 0.10));
}

#[test]
fn popover_style_uses_dark_theme_palette() {
    let palette = Theme::Dark.extended_palette();
    let style = popover_style(&Theme::Dark);

    assert_eq!(style.background, Some(Background::Color(palette.background.weak.color)));
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, palette.background.strong.color);
}

#[test]
fn menu_button_style_keeps_text_readable_in_idle_state() {
    let style = menu_button_style(&Theme::Light, button::Status::Active);

    assert_eq!(style.background, None);
    assert_eq!(style.text_color, Theme::Light.palette().text);
    assert_eq!(style.border.width, 0.0);
}

#[test]
fn menu_button_style_uses_hover_background_for_light_theme() {
    let style = menu_button_style(&Theme::Light, button::Status::Hovered);

    assert_eq!(
        style.background,
        Some(Background::Color(Color::from_rgba8(0x00, 0x00, 0x00, 0.06)))
    );
}

#[test]
fn menu_button_style_uses_hover_background_for_dark_theme() {
    let palette = Theme::Dark.extended_palette();
    let style = menu_button_style(&Theme::Dark, button::Status::Hovered);

    assert_eq!(
        style.background,
        Some(Background::Color(palette.background.strong.color.scale_alpha(0.65)))
    );
}

#[test]
fn menu_button_style_uses_pressed_primary_background_for_light_theme() {
    let style = menu_button_style(&Theme::Light, button::Status::Pressed);

    assert_eq!(
        style.background,
        Some(Background::Color(Theme::Light.palette().primary.scale_alpha(0.14)))
    );
}

#[test]
fn menu_button_style_uses_pressed_primary_background_for_dark_theme() {
    let style = menu_button_style(&Theme::Dark, button::Status::Pressed);

    assert_eq!(
        style.background,
        Some(Background::Color(Theme::Dark.palette().primary.scale_alpha(0.28)))
    );
}

#[test]
fn menu_button_style_treats_disabled_status_as_idle() {
    let style = menu_button_style(&Theme::Dark, button::Status::Disabled);

    assert_eq!(style.background, None);
    assert_eq!(style.text_color, Theme::Dark.palette().text);
    assert_eq!(style.border.width, 0.0);
    assert_eq!(style.border.color, Color::TRANSPARENT);
}
