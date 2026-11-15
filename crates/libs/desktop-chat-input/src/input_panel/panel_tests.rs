use super::panel::summarize_names;

#[test]
fn summarize_names_lists_up_to_three_names_and_hidden_count() {
    assert_eq!(summarize_names(&[]), "");
    assert_eq!(summarize_names(&["a".into(), "b".into()]), "a、b");
    assert_eq!(summarize_names(&["a".into(), "b".into(), "c".into()]), "a、b、c");
    assert_eq!(summarize_names(&["a".into(), "b".into(), "c".into(), "d".into()]), "a、b、c +1");
}


use super::panel::{popover_item_style, bottom_bar_round_toggle};
use super::icons::icon_svg;
use desktop_assets::assets::Icon;
use iced::{Element, Theme};

#[test]
fn popover_item_style_delegates_selected_and_unselected_states() {
    let selected = popover_item_style(&Theme::Dark, iced::widget::button::Status::Hovered, true);
    let unselected = popover_item_style(&Theme::Dark, iced::widget::button::Status::Active, false);

    assert_ne!(selected.background, unselected.background);
}

#[test]
fn bottom_bar_round_toggle_builds_tooltip_wrapped_icon_button() {
    let icon: Element<'_, ()> = icon_svg(Icon::Sliders, 14.0).into();
    let element = bottom_bar_round_toggle(
        icon,
        "toggle",
        (),
    );

    assert_eq!(element.as_widget().children().len(), 2);
}
